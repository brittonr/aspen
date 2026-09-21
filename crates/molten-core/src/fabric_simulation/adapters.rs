use std::collections::BTreeSet;

use super::*;
use crate::world_faults::FaultPhase;
use crate::world_faults::RecoveryClass;
use crate::world_faults::expected_recovery_for_phase;

pub const MAX_PENDING_TRANSMISSIONS: usize = 1_024;
pub const MAX_ACTIVE_PARTITIONS: usize = 64;
pub const MAX_SUBMITTED_OPERATIONS: usize = 1_024;
pub const NEVER_HEALS_TICK: u64 = u64::MAX;

const DELIVERED_SUFFIX: &str = "message-delivery";
const COMPLETION_SUFFIX: &str = "storage-completion";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulatedTransmission {
    pub transmission_id: String,
    pub destination: String,
    pub port_id: String,
    pub request_ref: String,
    pub generation: u64,
    pub submitted_at_tick: u64,
    pub eligible_at_tick: u64,
}

impl SimulatedTransmission {
    // r[impl molten.fabric_simulation.stateful_transport]
    pub fn effective_ready_tick(&self, partitions: &[SimulatedPartition]) -> u64 {
        partitions
            .iter()
            .filter(|partition| partition.destination == self.destination)
            .map(|partition| partition.heals_at_tick)
            .fold(self.eligible_at_tick, u64::max)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulatedPartition {
    pub fault_id: String,
    pub destination: String,
    pub heals_at_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportIssue {
    PendingOverflow { actual: usize, maximum: usize },
    PartitionOverflow { actual: usize, maximum: usize },
    DuplicateTransmissionId(String),
    DuplicatePartition(String),
    UnknownTransmission(String),
    UnknownPartition(String),
    NotDeliverable(String),
    Overflow(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulatedTransportState {
    pending: Vec<SimulatedTransmission>,
    dropped: Vec<SimulatedTransmission>,
    delivered: Vec<SimulatedTransmission>,
    partitions: Vec<SimulatedPartition>,
    healed: Vec<SimulatedPartition>,
}

impl Default for SimulatedTransportState {
    fn default() -> Self {
        Self::new()
    }
}

impl SimulatedTransportState {
    /// Transport state with no in-flight, dropped, delivered, or partitioned path.
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
            dropped: Vec::new(),
            delivered: Vec::new(),
            partitions: Vec::new(),
            healed: Vec::new(),
        }
    }

    pub fn submit(&mut self, transmission: SimulatedTransmission) -> Result<(), TransportIssue> {
        if self.pending.len() >= MAX_PENDING_TRANSMISSIONS {
            return Err(TransportIssue::PendingOverflow {
                actual: self.pending.len().checked_add(1).ok_or(TransportIssue::Overflow("pending"))?,
                maximum: MAX_PENDING_TRANSMISSIONS,
            });
        }
        if self.pending.iter().any(|existing| existing.transmission_id == transmission.transmission_id) {
            return Err(TransportIssue::DuplicateTransmissionId(transmission.transmission_id));
        }
        self.pending.push(transmission);
        Ok(())
    }

    // r[impl molten.fabric_simulation.stateful_transport]
    pub fn delay_transmission(&mut self, transmission_id: &str, eligible_at_tick: u64) -> Result<(), TransportIssue> {
        let transmission = self
            .pending
            .iter_mut()
            .find(|existing| existing.transmission_id == transmission_id)
            .ok_or_else(|| TransportIssue::UnknownTransmission(transmission_id.to_string()))?;
        transmission.eligible_at_tick = transmission.eligible_at_tick.max(eligible_at_tick);
        Ok(())
    }

    // r[impl molten.fabric_simulation.stateful_transport]
    pub fn drop_transmission(&mut self, transmission_id: &str) -> Result<SimulatedTransmission, TransportIssue> {
        let index = self
            .pending
            .iter()
            .position(|existing| existing.transmission_id == transmission_id)
            .ok_or_else(|| TransportIssue::UnknownTransmission(transmission_id.to_string()))?;
        let dropped = self.pending.remove(index);
        self.dropped.push(dropped.clone());
        Ok(dropped)
    }

    pub fn open_partition(&mut self, partition: SimulatedPartition) -> Result<(), TransportIssue> {
        if self.partitions.len() >= MAX_ACTIVE_PARTITIONS {
            return Err(TransportIssue::PartitionOverflow {
                actual: self.partitions.len().checked_add(1).ok_or(TransportIssue::Overflow("partitions"))?,
                maximum: MAX_ACTIVE_PARTITIONS,
            });
        }
        if self.partitions.iter().any(|existing| existing.fault_id == partition.fault_id) {
            return Err(TransportIssue::DuplicatePartition(partition.fault_id));
        }
        self.partitions.push(partition);
        Ok(())
    }

    // r[impl molten.fabric_simulation.stateful_transport]
    pub fn heal_ready_partitions(&mut self, tick: u64) {
        let mut still_blocked = Vec::with_capacity(self.partitions.len());
        for partition in self.partitions.drain(..) {
            if tick >= partition.heals_at_tick {
                self.healed.push(partition);
            } else {
                still_blocked.push(partition);
            }
        }
        self.partitions = still_blocked;
    }

    pub fn is_destination_blocked(&self, destination: &str) -> bool {
        self.partitions.iter().any(|partition| partition.destination == destination)
    }

    // r[impl molten.fabric_simulation.stateful_transport]
    pub fn eligible_deliveries(&self, tick: u64) -> Vec<EligibleChoice> {
        self.pending
            .iter()
            .filter(|transmission| {
                transmission.eligible_at_tick <= tick && !self.is_destination_blocked(&transmission.destination)
            })
            .map(|transmission| EligibleChoice {
                kind: SchedulerChoiceKind::MessageDelivery,
                choice_id: message_delivery_choice_id(&transmission.transmission_id),
                node_id: transmission.destination.clone(),
                generation: transmission.generation,
                ready_at_tick: transmission.eligible_at_tick,
            })
            .collect()
    }

    // r[impl molten.fabric_simulation.stateful_transport]
    pub fn deliver(&mut self, transmission_id: &str, tick: u64) -> Result<SimulatedTransmission, TransportIssue> {
        let index = self
            .pending
            .iter()
            .position(|existing| existing.transmission_id == transmission_id)
            .ok_or_else(|| TransportIssue::UnknownTransmission(transmission_id.to_string()))?;
        let transmission = &self.pending[index];
        if transmission.eligible_at_tick > tick || self.is_destination_blocked(&transmission.destination) {
            return Err(TransportIssue::NotDeliverable(transmission_id.to_string()));
        }
        let delivered = self.pending.remove(index);
        self.delivered.push(delivered.clone());
        Ok(delivered)
    }

    pub fn pending(&self) -> &[SimulatedTransmission] {
        &self.pending
    }

    pub fn dropped(&self) -> &[SimulatedTransmission] {
        &self.dropped
    }

    pub fn delivered(&self) -> &[SimulatedTransmission] {
        &self.delivered
    }

    pub fn partitions(&self) -> &[SimulatedPartition] {
        &self.partitions
    }

    pub fn healed(&self) -> &[SimulatedPartition] {
        &self.healed
    }

    pub fn next_readiness(&self, tick: u64) -> Option<u64> {
        self.pending
            .iter()
            .filter(|transmission| self.effective_readiness(transmission) > tick)
            .map(|transmission| self.effective_readiness(transmission))
            .min()
    }

    fn effective_readiness(&self, transmission: &SimulatedTransmission) -> u64 {
        transmission.effective_ready_tick(&self.partitions)
    }
}

pub fn message_delivery_choice_id(transmission_id: &str) -> String {
    format!("{DELIVERED_SUFFIX}-{transmission_id}")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulatedStorageOperation {
    pub operation_id: String,
    pub port_id: String,
    pub owner_node_id: String,
    pub request_ref: String,
    pub phase: FaultPhase,
    pub submission_ordinal: u64,
    pub submitted_at_tick: u64,
    pub eligible_at_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulatedDurableEntry {
    pub entry_id: String,
    pub request_ref: String,
    pub submission_ordinal: u64,
    pub completed_at_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulatedDurableImage {
    entries: Vec<SimulatedDurableEntry>,
}

impl Default for SimulatedDurableImage {
    fn default() -> Self {
        Self::new()
    }
}

impl SimulatedDurableImage {
    /// Durable image with no committed entry.
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn entries(&self) -> &[SimulatedDurableEntry] {
        &self.entries
    }

    pub fn request_refs(&self) -> Vec<String> {
        self.entries.iter().map(|entry| entry.request_ref.clone()).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageIssue {
    SubmittedOverflow { actual: usize, maximum: usize },
    DuplicateOperationId(String),
    UnknownOperation(String),
    NotEligible(String),
    Overflow(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulatedStorageState {
    submitted: Vec<SimulatedStorageOperation>,
    completed_ids: BTreeSet<String>,
    durable: SimulatedDurableImage,
    lost_on_crash: Vec<SimulatedStorageOperation>,
}

impl Default for SimulatedStorageState {
    fn default() -> Self {
        Self::new()
    }
}

impl SimulatedStorageState {
    /// Storage state with no submitted operation, completion, durable entry, or crash loss.
    pub fn new() -> Self {
        Self {
            submitted: Vec::new(),
            completed_ids: BTreeSet::new(),
            durable: SimulatedDurableImage::new(),
            lost_on_crash: Vec::new(),
        }
    }

    pub fn submit(&mut self, operation: SimulatedStorageOperation) -> Result<(), StorageIssue> {
        if self.submitted.len() >= MAX_SUBMITTED_OPERATIONS {
            return Err(StorageIssue::SubmittedOverflow {
                actual: self.submitted.len().checked_add(1).ok_or(StorageIssue::Overflow("submitted"))?,
                maximum: MAX_SUBMITTED_OPERATIONS,
            });
        }
        if self.submitted.iter().any(|existing| existing.operation_id == operation.operation_id) {
            return Err(StorageIssue::DuplicateOperationId(operation.operation_id));
        }
        self.submitted.push(operation);
        Ok(())
    }

    // r[impl molten.fabric_simulation.stateful_storage]
    pub fn hold_completion(&mut self, operation_id: &str, eligible_at_tick: u64) -> Result<(), StorageIssue> {
        let operation = self
            .submitted
            .iter_mut()
            .find(|existing| existing.operation_id == operation_id)
            .ok_or_else(|| StorageIssue::UnknownOperation(operation_id.to_string()))?;
        operation.eligible_at_tick = operation.eligible_at_tick.max(eligible_at_tick);
        Ok(())
    }

    fn head(&self) -> Option<&SimulatedStorageOperation> {
        self.submitted.first()
    }

    // r[impl molten.fabric_simulation.stateful_storage]
    pub fn eligible_completions(&self, tick: u64) -> Vec<EligibleChoice> {
        let Some(head) = self.head() else {
            return Vec::new();
        };
        if head.eligible_at_tick > tick {
            return Vec::new();
        }
        vec![EligibleChoice {
            kind: SchedulerChoiceKind::StorageCompletion,
            choice_id: storage_completion_choice_id(&head.operation_id),
            node_id: head.owner_node_id.clone(),
            generation: INITIAL_EXTENSION_GENERATION,
            ready_at_tick: head.eligible_at_tick,
        }]
    }

    // r[impl molten.fabric_simulation.stateful_storage]
    pub fn complete(&mut self, operation_id: &str, tick: u64) -> Result<SimulatedDurableEntry, StorageIssue> {
        let head = self.head().ok_or_else(|| StorageIssue::UnknownOperation(operation_id.to_string()))?;
        if head.operation_id != operation_id {
            return Err(StorageIssue::NotEligible(operation_id.to_string()));
        }
        if head.eligible_at_tick > tick {
            return Err(StorageIssue::NotEligible(operation_id.to_string()));
        }
        let mut operation = self.submitted.remove(0);
        operation.phase = FaultPhase::AfterDurableWrite;
        let entry = SimulatedDurableEntry {
            entry_id: operation.operation_id.clone(),
            request_ref: operation.request_ref.clone(),
            submission_ordinal: operation.submission_ordinal,
            completed_at_tick: tick,
        };
        self.completed_ids.insert(operation.operation_id.clone());
        self.durable.entries.push(entry.clone());
        Ok(entry)
    }

    // r[impl molten.fabric_simulation.stateful_storage]
    pub fn crash_and_recover(&mut self) -> Vec<SimulatedStorageOperation> {
        let lost = self.submitted.drain(..).collect::<Vec<_>>();
        for operation in &lost {
            self.lost_on_crash.push(operation.clone());
        }
        self.completed_ids.clear();
        lost
    }

    pub fn durable_image(&self) -> &SimulatedDurableImage {
        &self.durable
    }

    pub fn submitted(&self) -> &[SimulatedStorageOperation] {
        &self.submitted
    }

    pub fn completed_ids(&self) -> &BTreeSet<String> {
        &self.completed_ids
    }

    pub fn lost_on_crash(&self) -> &[SimulatedStorageOperation] {
        &self.lost_on_crash
    }

    // r[impl molten.fabric_simulation.stateful_storage]
    pub fn next_readiness(&self, tick: u64) -> Option<u64> {
        self.head()
            .filter(|operation| operation.eligible_at_tick > tick)
            .map(|operation| operation.eligible_at_tick)
    }
}

pub fn storage_completion_choice_id(operation_id: &str) -> String {
    format!("{COMPLETION_SUFFIX}-{operation_id}")
}

// r[impl molten.fabric_simulation.stateful_storage]
// r[impl molten.fabric_simulation.stateful_transport]
pub fn simulation_fault_phase(kind: SimulationFaultKind) -> FaultPhase {
    match kind {
        SimulationFaultKind::Delay | SimulationFaultKind::Pause | SimulationFaultKind::ClockSkew => {
            FaultPhase::AfterPossibleSubmit
        }
        SimulationFaultKind::Drop
        | SimulationFaultKind::Duplicate
        | SimulationFaultKind::Reorder
        | SimulationFaultKind::Partition
        | SimulationFaultKind::ClockJump => FaultPhase::BeforeResponse,
        SimulationFaultKind::Reset | SimulationFaultKind::BoundedCorruption => FaultPhase::AfterDurableWrite,
        SimulationFaultKind::CapacityExhaustion
        | SimulationFaultKind::AuthorityRevocation
        | SimulationFaultKind::MembershipChange
        | SimulationFaultKind::PlacementReplacement
        | SimulationFaultKind::ConsistencyQuorumLoss => FaultPhase::BeforeSubmit,
        SimulationFaultKind::Crash | SimulationFaultKind::Restart => FaultPhase::ProcessRestart,
    }
}

// r[impl molten.fabric_simulation.stateful_storage]
pub fn recovery_class_for_operation(operation: &SimulatedStorageOperation) -> RecoveryClass {
    expected_recovery_for_phase(operation.phase)
}
