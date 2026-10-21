//! Pure command-pattern resource reservation over authoritative state.
//!
//! The core owns capacity, worker selection, and allocation decisions. A
//! reservation is one command evaluated against current authoritative state;
//! there is no client-visible read-then-write window. A rejected command
//! preserves state.

use std::collections::BTreeMap;

use crate::fabric_time::valid_time_id;

mod batch;

pub use batch::*;

pub const MAX_RESERVATION_BATCH_ITEMS: usize = 1_024;
pub const MAX_RESERVATION_REQUIREMENT_BYTES: u64 = 1_073_741_824;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceRequirements {
    pub units: u64,
    pub bytes: u64,
}

impl ResourceRequirements {
    pub fn new(units: u64, bytes: u64) -> Self {
        Self { units, bytes }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservationRequest {
    pub job_id: String,
    pub operation_ref: String,
    pub requirements: ResourceRequirements,
    pub expected_generation: u64,
    pub max_waiting_ticks: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservationOutcome {
    Committed,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerCapacity {
    pub worker_id: String,
    pub units: u64,
    pub bytes: u64,
}

/// Admitted capacity with reserved and allocated quantities kept distinct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapacityPool {
    pub total_units: u64,
    pub total_bytes: u64,
    pub reserved_units: u64,
    pub reserved_bytes: u64,
    pub allocated_units: u64,
    pub allocated_bytes: u64,
}

impl CapacityPool {
    pub const fn available_units(&self) -> u64 {
        self.total_units.saturating_sub(self.reserved_units)
    }

    pub const fn available_bytes(&self) -> u64 {
        self.total_bytes.saturating_sub(self.reserved_bytes)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservationState {
    Committed,
    /// The lease expired and the capacity is still counted against the
    /// reservation because no enforcement or termination fact exists.
    Stale,
    /// The lease expired and an enforcement or termination fact released the
    /// capacity.
    Released,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservationRecord {
    pub job_id: String,
    pub operation_ref: String,
    pub worker_id: String,
    pub requirements: ResourceRequirements,
    pub state: ReservationState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservationState_ {
    pub generation: u64,
    pub pool: CapacityPool,
    pub workers: Vec<WorkerCapacity>,
    pub reservations: Vec<ReservationRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchOutcome {
    pub next: ReservationState_,
    pub items: Vec<BatchItemOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservationTransition {
    pub next: ReservationState_,
    pub outcome: ReservationOutcome,
    pub reservation: Option<ReservationRecord>,
    pub issue: Option<ReservationIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReservationIssue {
    MalformedIdentifier {
        field: &'static str,
    },
    ZeroGeneration,
    StaleGeneration {
        expected: u64,
        actual: u64,
    },
    DuplicateJob(String),
    NoWorkerAvailable,
    InsufficientCapacity {
        required_units: u64,
        available_units: u64,
        required_bytes: u64,
        available_bytes: u64,
    },
    RequirementOverflow,
    CapacityInvariantViolated,
}

pub fn new_reservation_state(
    generation: u64,
    total_units: u64,
    total_bytes: u64,
    workers: Vec<WorkerCapacity>,
) -> Result<ReservationState_, ReservationIssue> {
    if generation == 0 {
        return Err(ReservationIssue::ZeroGeneration);
    }
    for worker in &workers {
        if !valid_time_id(&worker.worker_id) {
            return Err(ReservationIssue::MalformedIdentifier { field: "worker-id" });
        }
    }
    let mut sorted = workers;
    sorted.sort_by(|left, right| left.worker_id.cmp(&right.worker_id));
    if sorted.windows(2).any(|pair| pair[0].worker_id == pair[1].worker_id) {
        return Err(ReservationIssue::DuplicateJob("worker-id".to_string()));
    }
    Ok(ReservationState_ {
        generation,
        pool: CapacityPool {
            total_units,
            total_bytes,
            reserved_units: 0,
            reserved_bytes: 0,
            allocated_units: 0,
            allocated_bytes: 0,
        },
        workers: sorted,
        reservations: Vec::new(),
    })
}

/// Evaluates one reservation command against current authoritative state.
// r[impl molten.reservation_command.atomic_eval]
pub fn apply_reservation(
    state: &ReservationState_,
    request: &ReservationRequest,
) -> Result<ReservationTransition, ReservationIssue> {
    validate_request(request)?;
    if request.expected_generation != state.generation {
        return Ok(rejected(state.clone(), ReservationIssue::StaleGeneration {
            expected: request.expected_generation,
            actual: state.generation,
        }));
    }
    if state.reservations.iter().any(|record| record.job_id == request.job_id) {
        return Ok(rejected(state.clone(), ReservationIssue::DuplicateJob(request.job_id.clone())));
    }
    let requirements = request.requirements;
    let available_units = state.pool.available_units();
    let available_bytes = state.pool.available_bytes();
    if requirements.units > available_units || requirements.bytes > available_bytes {
        return Ok(rejected(state.clone(), ReservationIssue::InsufficientCapacity {
            required_units: requirements.units,
            available_units,
            required_bytes: requirements.bytes,
            available_bytes,
        }));
    }
    let Some(index) = select_worker(state, requirements) else {
        return Ok(rejected(state.clone(), ReservationIssue::NoWorkerAvailable));
    };
    let mut next = state.clone();
    let worker_id = next.workers[index].worker_id.clone();
    next.pool.reserved_units = next
        .pool
        .reserved_units
        .checked_add(requirements.units)
        .ok_or(ReservationIssue::RequirementOverflow)?;
    next.pool.reserved_bytes = next
        .pool
        .reserved_bytes
        .checked_add(requirements.bytes)
        .ok_or(ReservationIssue::RequirementOverflow)?;
    next.pool.allocated_units = next
        .pool
        .allocated_units
        .checked_add(requirements.units)
        .ok_or(ReservationIssue::RequirementOverflow)?;
    next.pool.allocated_bytes = next
        .pool
        .allocated_bytes
        .checked_add(requirements.bytes)
        .ok_or(ReservationIssue::RequirementOverflow)?;
    next.workers[index].units = next.workers[index].units.saturating_sub(requirements.units);
    next.workers[index].bytes = next.workers[index].bytes.saturating_sub(requirements.bytes);
    let reservation = ReservationRecord {
        job_id: request.job_id.clone(),
        operation_ref: request.operation_ref.clone(),
        worker_id,
        requirements,
        state: ReservationState::Committed,
    };
    next.reservations.push(reservation.clone());
    if !capacity_invariant_holds(&next) {
        return Err(ReservationIssue::CapacityInvariantViolated);
    }
    Ok(ReservationTransition {
        next,
        outcome: ReservationOutcome::Committed,
        reservation: Some(reservation),
        issue: None,
    })
}

fn validate_request(request: &ReservationRequest) -> Result<(), ReservationIssue> {
    if !valid_time_id(&request.job_id) {
        return Err(ReservationIssue::MalformedIdentifier { field: "job-id" });
    }
    if !valid_time_id(&request.operation_ref) {
        return Err(ReservationIssue::MalformedIdentifier { field: "operation-ref" });
    }
    if request.expected_generation == 0 {
        return Err(ReservationIssue::ZeroGeneration);
    }
    if request.requirements.bytes > MAX_RESERVATION_REQUIREMENT_BYTES {
        return Err(ReservationIssue::RequirementOverflow);
    }
    Ok(())
}

fn select_worker(state: &ReservationState_, requirements: ResourceRequirements) -> Option<usize> {
    state
        .workers
        .iter()
        .position(|worker| worker.units >= requirements.units && worker.bytes >= requirements.bytes)
}

pub(super) fn capacity_invariant_holds(state: &ReservationState_) -> bool {
    let pool = state.pool;
    pool.reserved_units <= pool.total_units
        && pool.reserved_bytes <= pool.total_bytes
        && pool.allocated_units <= pool.reserved_units
        && pool.allocated_bytes <= pool.reserved_bytes
}

fn rejected(next: ReservationState_, issue: ReservationIssue) -> ReservationTransition {
    ReservationTransition {
        next,
        outcome: ReservationOutcome::Rejected,
        reservation: None,
        issue: Some(issue),
    }
}

/// Reservations by job, for bounded operator readback.
pub fn reservations_by_job(state: &ReservationState_) -> BTreeMap<&str, &ReservationRecord> {
    state.reservations.iter().map(|record| (record.job_id.as_str(), record)).collect()
}
