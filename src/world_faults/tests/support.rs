use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use molten_core::fabric_simulation::EligibleChoice;
use molten_core::world_faults::*;
use molten_node_host::node_state::MAX_NODE_STATE_FILE_BYTES;
use molten_node_host::node_state::NodeStateNamespaceKind;
use molten_node_host::node_state::NodeStatePath;
use molten_node_host::node_state::NodeStateRoot;

use super::super::*;
use crate::error::MoltenError;
use crate::error::Result;

pub(super) const TEST_SOURCE_REVISION: &str = "06cd7ca465550d2a35e0511cbbd7989e434d8f51";
pub(super) const RESTART_MARKER_PATH: &str = "world-fault-restart.marker";
pub(super) const EXPECTED_SUPPORTED_CASES: usize = 64;
pub(super) const EXPECTED_RESTARTS: u32 = 16;
pub(super) const EXPECTED_RECEIPTS: usize = 1;

#[derive(Default)]
pub(super) struct DeterministicFaultControl {
    pub(super) events: Rc<RefCell<Vec<String>>>,
}

impl WorldFaultControlPort for DeterministicFaultControl {
    fn interrupt(&mut self, case: &WorldFaultCase) -> Result<WorldFaultInterruption> {
        self.events.borrow_mut().push(format!("interrupt:{}", case.case_id));
        let is_complete = case.expected_decision == RecoveryClass::AlreadyComplete;
        Ok(WorldFaultInterruption {
            submission: if case.phase == FaultPhase::BeforeSubmit {
                SubmissionObservation::NotSubmitted
            } else if is_complete {
                SubmissionObservation::DurablySubmitted
            } else {
                SubmissionObservation::PossiblySubmitted
            },
            response: if case.phase == FaultPhase::LostResponse {
                ResponseObservation::Lost
            } else if is_complete {
                ResponseObservation::Received
            } else {
                ResponseObservation::NotExpected
            },
            whole_store_rollback: false,
            cleanup_authorized: false,
        })
    }
}

pub(super) struct LocalRestartPort {
    pub(super) root: PathBuf,
    pub(super) restarts: u32,
    pub(super) events: Rc<RefCell<Vec<String>>>,
}

impl WorldFaultRestartPort for LocalRestartPort {
    fn restart(&mut self, case: &WorldFaultCase) -> Result<()> {
        let root = NodeStateRoot::open_existing(&self.root)?;
        let storage = root.namespace(NodeStateNamespaceKind::Storage)?;
        storage.write(&NodeStatePath::parse(RESTART_MARKER_PATH)?, case.case_id.as_bytes())?;
        self.restarts = self.restarts.saturating_add(1);
        self.events.borrow_mut().push(format!("restart:{}", case.case_id));
        Ok(())
    }
}

pub(super) struct LocalDurableObservationPort {
    pub(super) root: PathBuf,
    pub(super) events: Rc<RefCell<Vec<String>>>,
}

impl WorldFaultDurableObservationPort for LocalDurableObservationPort {
    fn read_back(&mut self, case: &WorldFaultCase) -> Result<DurableReadBack> {
        if matches!(case.phase, FaultPhase::ProcessRestart | FaultPhase::RecoveryReadBack) {
            let reopened = NodeStateRoot::open_existing(&self.root)?;
            let storage = reopened.namespace(NodeStateNamespaceKind::Storage)?;
            let marker =
                storage.read_to_string(&NodeStatePath::parse(RESTART_MARKER_PATH)?, MAX_NODE_STATE_FILE_BYTES)?;
            if marker != case.case_id {
                return Err(MoltenError::invalid_harness("world fault restart marker crossed case identity"));
            }
        }
        self.events.borrow_mut().push(format!("read-back:{}", case.case_id));
        Ok(read_back_for_case(case))
    }
}

pub(super) struct ExpectedOwnerCore {
    pub(super) calls: usize,
    pub(super) misclassify_lost_response: bool,
    pub(super) events: Rc<RefCell<Vec<String>>>,
}

impl WorldFaultOwnerDecisionPort for ExpectedOwnerCore {
    fn decide(
        &mut self,
        case: &WorldFaultCase,
        _interruption: WorldFaultInterruption,
        _read_back: &DurableReadBack,
    ) -> Result<RecoveryClass> {
        self.calls = self.calls.saturating_add(1);
        self.events.borrow_mut().push(format!("owner-decision:{}", case.case_id));
        if self.misclassify_lost_response && case.phase == FaultPhase::LostResponse {
            Ok(RecoveryClass::SafeToRetry)
        } else {
            Ok(case.expected_decision)
        }
    }
}

#[derive(Default)]
pub(super) struct DeterministicSchedulePort {
    pub(super) events: Rc<RefCell<Vec<String>>>,
}

impl WorldConcurrentSchedulePort for DeterministicSchedulePort {
    fn execute_schedule(
        &mut self,
        schedule: &ConcurrentSchedule,
        eligible_by_position: &[Vec<EligibleChoice>],
    ) -> Result<WorldConcurrentExecution> {
        self.events.borrow_mut().push(format!("schedule:{}", schedule.schedule_id));
        let mut operations = schedule.steps.iter().map(|step| step.operation_id.clone()).collect::<Vec<_>>();
        operations.sort();
        operations.dedup();
        if operations.len() != WORLD_FAULT_CONTENDER_COUNT {
            return Err(MoltenError::invalid_harness("world fault schedule contender count is invalid"));
        }
        let release_count =
            u32::from(matches!(schedule.mutation, WorldMutationKind::Promotion | WorldMutationKind::Outbox));
        let observations = vec![
            schedule_observation(schedule, &operations[0], ConcurrentOutcome::Applied, release_count),
            schedule_observation(schedule, &operations[1], ConcurrentOutcome::Stale, 0),
        ];
        let scheduler_choices = eligible_by_position.iter().flat_map(|choices| choices.iter().cloned()).collect();
        Ok(WorldConcurrentExecution {
            observations,
            scheduler_choices,
        })
    }
}

pub(super) struct RecordingReceiptPort {
    pub(super) records: Vec<Vec<u8>>,
    pub(super) return_crossed_ref: bool,
    pub(super) events: Rc<RefCell<Vec<String>>>,
}

impl WorldFaultReceiptPort for RecordingReceiptPort {
    fn publish_receipt(&mut self, record: &CanonicalWorldFaultReceipt) -> Result<String> {
        self.records.push(record.bytes.clone());
        self.events.borrow_mut().push("receipt".to_string());
        if self.return_crossed_ref {
            Ok(test_ref("crossed-receipt"))
        } else {
            Ok(record.record_ref.clone())
        }
    }
}

fn read_back_for_case(case: &WorldFaultCase) -> DurableReadBack {
    let is_complete = case.expected_decision == RecoveryClass::AlreadyComplete;
    let is_prior = case.expected_decision == RecoveryClass::SafeToRetry;
    DurableReadBack {
        status: if is_complete {
            DurableReadBackStatus::Applied
        } else if is_prior {
            DurableReadBackStatus::Prior
        } else {
            DurableReadBackStatus::Missing
        },
        state_ref: if is_complete || is_prior {
            Some(test_ref(&format!("{}:state", case.case_id)))
        } else {
            None
        },
        record_ref: if is_complete {
            Some(test_ref(&format!("{}:record", case.case_id)))
        } else {
            None
        },
        observed_generation: if is_complete || is_prior {
            Some(case.expected_generation)
        } else {
            None
        },
        independent_witness: false,
    }
}

fn schedule_observation(
    schedule: &ConcurrentSchedule,
    operation_id: &str,
    outcome: ConcurrentOutcome,
    effect_release_count: u32,
) -> ConcurrentOperationObservation {
    let step = schedule.steps.iter().find(|step| step.operation_id == operation_id).expect("schedule operation");
    ConcurrentOperationObservation {
        operation_id: operation_id.to_string(),
        mutation: schedule.mutation,
        expected_generation: step.expected_generation,
        pre_state_ref: step.pre_state_ref.clone(),
        outcome,
        effect_release_count,
    }
}

fn test_ref(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}
