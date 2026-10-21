//! Bounded batch admission, expiry-aware release, and operator status.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchRequest {
    pub items: Vec<ReservationRequest>,
    pub max_items: usize,
    pub max_bytes: u64,
    pub max_waiting_ticks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchItemOutcome {
    pub job_id: String,
    pub outcome: ReservationOutcome,
    pub issue: Option<ReservationIssue>,
    pub reservation: Option<ReservationRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchIssue {
    TooManyItems { items: usize, maximum: usize },
    TooManyBytes { bytes: u64, maximum: u64 },
    WaitingTimeExceeded { ticks: u64, maximum: u64 },
    CapacityInvariantViolated,
}

/// The fact that permits capacity behind an expired lease to be reused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnforcementFact {
    /// An observed stop of the external work.
    ObservedStop { observation_ref: String },
    /// A fencing takeover that carries termination authority.
    FencingTakeover { authority_ref: String },
    /// The contract declares the reserved work non-external.
    NonExternalWork,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservationExpiry {
    pub job_id: String,
    pub enforcement: Option<EnforcementFact>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpiryOutcome {
    /// The reservation is stale and its capacity is still counted.
    StaleCapacityRetained,
    /// The reservation was released and its capacity is available again.
    Released,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpiryTransition {
    pub next: ReservationState_,
    pub outcome: ExpiryOutcome,
    pub issue: Option<ExpiryIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpiryIssue {
    UnknownJob(String),
    NotStale(String),
    CapacityInvariantViolated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReservationStatus {
    pub available_units: u64,
    pub reserved_units: u64,
    pub allocated_units: u64,
    pub stale_held_units: u64,
    pub stale_held_bytes: u64,
    pub shortfall_units: u64,
    pub shortfall_bytes: u64,
}

/// Admits a bounded batch and evaluates every item independently.
// r[impl molten.reservation_command.bounded_batches]
pub fn admit_batch(state: &ReservationState_, request: &BatchRequest) -> Result<BatchOutcome, BatchIssue> {
    if request.items.len() > request.max_items || request.items.len() > MAX_RESERVATION_BATCH_ITEMS {
        return Err(BatchIssue::TooManyItems {
            items: request.items.len(),
            maximum: request.max_items.min(MAX_RESERVATION_BATCH_ITEMS),
        });
    }
    let mut bytes = 0_u64;
    for item in &request.items {
        if item.max_waiting_ticks > request.max_waiting_ticks {
            return Err(BatchIssue::WaitingTimeExceeded {
                ticks: item.max_waiting_ticks,
                maximum: request.max_waiting_ticks,
            });
        }
        bytes = bytes.checked_add(item.requirements.bytes).ok_or(BatchIssue::TooManyBytes {
            bytes: u64::MAX,
            maximum: request.max_bytes,
        })?;
    }
    if bytes > request.max_bytes {
        return Err(BatchIssue::TooManyBytes {
            bytes,
            maximum: request.max_bytes,
        });
    }
    let mut next = state.clone();
    let mut items = Vec::with_capacity(request.items.len());
    for item in &request.items {
        // Each item is its own atomic command. A batch never enlarges the set
        // of operations that commit together.
        match apply_reservation(&next, item) {
            Ok(transition) => {
                let outcome = BatchItemOutcome {
                    job_id: item.job_id.clone(),
                    outcome: transition.outcome,
                    issue: transition.issue,
                    reservation: transition.reservation,
                };
                next = transition.next;
                items.push(outcome);
            }
            Err(issue) => {
                items.push(BatchItemOutcome {
                    job_id: item.job_id.clone(),
                    outcome: ReservationOutcome::Rejected,
                    issue: Some(issue),
                    reservation: None,
                });
            }
        }
    }
    if !capacity_invariant_holds(&next) {
        return Err(BatchIssue::CapacityInvariantViolated);
    }
    Ok(BatchOutcome { next, items })
}

/// Marks an expired lease stale and releases its capacity only with an
/// enforcement or termination fact.
// r[impl molten.reservation_command.expiry_not_release]
pub fn expire_reservation(
    state: &ReservationState_,
    expiry: &ReservationExpiry,
) -> Result<ExpiryTransition, ExpiryIssue> {
    let Some(index) = state.reservations.iter().position(|record| record.job_id == expiry.job_id) else {
        return Err(ExpiryIssue::UnknownJob(expiry.job_id.clone()));
    };
    if state.reservations[index].state != ReservationState::Committed {
        return Err(ExpiryIssue::NotStale(expiry.job_id.clone()));
    }
    let mut next = state.clone();
    next.reservations[index].state = ReservationState::Stale;
    let outcome = match expiry.enforcement.as_ref().filter(|fact| fact_is_admitted(fact)) {
        Some(_) => {
            let record = next.reservations[index].clone();
            next.reservations[index].state = ReservationState::Released;
            next.pool.reserved_units = next.pool.reserved_units.saturating_sub(record.requirements.units);
            next.pool.reserved_bytes = next.pool.reserved_bytes.saturating_sub(record.requirements.bytes);
            next.pool.allocated_units = next.pool.allocated_units.saturating_sub(record.requirements.units);
            next.pool.allocated_bytes = next.pool.allocated_bytes.saturating_sub(record.requirements.bytes);
            ExpiryOutcome::Released
        }
        None => ExpiryOutcome::StaleCapacityRetained,
    };
    if !capacity_invariant_holds(&next) {
        return Err(ExpiryIssue::CapacityInvariantViolated);
    }
    Ok(ExpiryTransition {
        next,
        outcome,
        issue: None,
    })
}

/// The operator-visible capacity status, including the shortfall behind stale
/// reservations that cannot be released yet.
pub fn reservation_status(state: &ReservationState_) -> ReservationStatus {
    let stale_units = state
        .reservations
        .iter()
        .filter(|record| record.state == ReservationState::Stale)
        .map(|record| record.requirements.units)
        .sum::<u64>();
    let stale_bytes = state
        .reservations
        .iter()
        .filter(|record| record.state == ReservationState::Stale)
        .map(|record| record.requirements.bytes)
        .sum::<u64>();
    ReservationStatus {
        available_units: state.pool.available_units(),
        reserved_units: state.pool.reserved_units,
        allocated_units: state.pool.allocated_units,
        stale_held_units: stale_units,
        stale_held_bytes: stale_bytes,
        shortfall_units: stale_units,
        shortfall_bytes: stale_bytes,
    }
}

fn fact_is_admitted(fact: &EnforcementFact) -> bool {
    match fact {
        EnforcementFact::ObservedStop { observation_ref } => valid_time_ref(observation_ref),
        EnforcementFact::FencingTakeover { authority_ref } => valid_time_ref(authority_ref),
        EnforcementFact::NonExternalWork => true,
    }
}

fn valid_time_ref(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("blake3:") else {
        return false;
    };
    const BLAKE3_HEX_BYTES: usize = 64;
    hex.len() == BLAKE3_HEX_BYTES && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}
