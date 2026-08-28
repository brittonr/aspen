use molten_core::fabric_simulation::EligibleChoice;
use molten_core::world_faults::*;

use super::CanonicalWorldFaultReceipt;
use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldFaultInterruption {
    pub submission: SubmissionObservation,
    pub response: ResponseObservation,
    pub whole_store_rollback: bool,
    pub cleanup_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldConcurrentExecution {
    pub observations: Vec<ConcurrentOperationObservation>,
    pub scheduler_choices: Vec<EligibleChoice>,
}

// r[impl molten.world_faults.shell_boundary]
pub trait WorldFaultControlPort {
    fn interrupt(&mut self, case: &WorldFaultCase) -> Result<WorldFaultInterruption>;
}

// r[impl molten.world_faults.shell_boundary]
pub trait WorldFaultRestartPort {
    fn restart(&mut self, case: &WorldFaultCase) -> Result<()>;
}

// r[impl molten.world_faults.shell_boundary]
pub trait WorldFaultDurableObservationPort {
    fn read_back(&mut self, case: &WorldFaultCase) -> Result<DurableReadBack>;
}

// r[impl molten.world_faults.shell_boundary]
pub trait WorldFaultOwnerDecisionPort {
    fn decide(
        &mut self,
        case: &WorldFaultCase,
        interruption: WorldFaultInterruption,
        read_back: &DurableReadBack,
    ) -> Result<RecoveryClass>;
}

// r[impl molten.world_faults.concurrency]
pub trait WorldConcurrentSchedulePort {
    fn execute_schedule(
        &mut self,
        schedule: &ConcurrentSchedule,
        eligible_by_position: &[Vec<EligibleChoice>],
    ) -> Result<WorldConcurrentExecution>;
}

// r[impl molten.world_faults.receipt]
pub trait WorldFaultReceiptPort {
    fn publish_receipt(&mut self, record: &CanonicalWorldFaultReceipt) -> Result<String>;
}

pub struct WorldFaultHarnessPorts<'a> {
    pub fault_control: &'a mut dyn WorldFaultControlPort,
    pub restart: &'a mut dyn WorldFaultRestartPort,
    pub durable_observation: &'a mut dyn WorldFaultDurableObservationPort,
    pub owner_decision: &'a mut dyn WorldFaultOwnerDecisionPort,
    pub concurrent_schedule: &'a mut dyn WorldConcurrentSchedulePort,
    pub receipts: &'a mut dyn WorldFaultReceiptPort,
}
