use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use molten_core::fabric_simulation::EligibleChoice;
use molten_core::world_faults::*;
use molten_node_host::node_state::MAX_NODE_STATE_FILE_BYTES;
use molten_node_host::node_state::NodeStateNamespaceKind;
use molten_node_host::node_state::NodeStatePath;
use molten_node_host::node_state::NodeStateRoot;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;

const TEST_SOURCE_REVISION: &str = "51646db62379c6790f21211630ff648f4a0446d1";
const RESTART_MARKER_PATH: &str = "world-fault-restart.marker";
const EXPECTED_SUPPORTED_CASES: usize = 64;
const EXPECTED_RESTARTS: u32 = 16;
const EXPECTED_RECEIPTS: usize = 1;

#[derive(Default)]
struct DeterministicFaultControl {
    events: Rc<RefCell<Vec<String>>>,
}

impl WorldFaultControlPort for DeterministicFaultControl {
    fn interrupt(&mut self, case: &WorldFaultCase) -> Result<WorldFaultInterruption> {
        self.events.borrow_mut().push(format!("interrupt:{}", case.case_id));
        let complete = case.expected_decision == RecoveryClass::AlreadyComplete;
        Ok(WorldFaultInterruption {
            submission: if case.phase == FaultPhase::BeforeSubmit {
                SubmissionObservation::NotSubmitted
            } else if complete {
                SubmissionObservation::DurablySubmitted
            } else {
                SubmissionObservation::PossiblySubmitted
            },
            response: if case.phase == FaultPhase::LostResponse {
                ResponseObservation::Lost
            } else if complete {
                ResponseObservation::Received
            } else {
                ResponseObservation::NotExpected
            },
            whole_store_rollback: false,
            cleanup_authorized: false,
        })
    }
}

struct LocalRestartPort {
    root: PathBuf,
    restarts: u32,
    events: Rc<RefCell<Vec<String>>>,
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

struct LocalDurableObservationPort {
    root: PathBuf,
    events: Rc<RefCell<Vec<String>>>,
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

struct ExpectedOwnerCore {
    calls: usize,
    misclassify_lost_response: bool,
    events: Rc<RefCell<Vec<String>>>,
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
struct DeterministicSchedulePort {
    events: Rc<RefCell<Vec<String>>>,
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

struct RecordingReceiptPort {
    records: Vec<Vec<u8>>,
    return_crossed_ref: bool,
    events: Rc<RefCell<Vec<String>>>,
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

// r[verify molten.world_faults.profile]
#[test]
fn nickel_profile_is_an_exact_checked_projection_of_the_rust_profile() {
    let value = serde_json::from_str::<serde_json::Value>(include_str!(
        "../../config/world-faults/generated/local-deterministic.json"
    ))
    .expect("generated world fault profile JSON");
    let profile = standard_world_fault_profile(TEST_SOURCE_REVISION).expect("Rust world fault profile");
    assert_eq!(json_string(&value, "schema"), profile.schema);
    assert_eq!(json_string(&value, "profile_name"), profile.profile_name);
    assert_eq!(json_string(&value, "source_revision"), profile.source_revision);
    assert_eq!(json_string(&value, "inventory_ref"), profile.inventory_ref);
    assert_eq!(
        json_string(&value, "rust_profile_ref"),
        identify_world_fault_profile(&profile).expect("Rust profile identity")
    );
    let limits = value.get("limits").expect("limits");
    assert_eq!(json_usize(limits, "max_cases"), profile.limits.max_cases);
    assert_eq!(json_usize(limits, "max_schedules"), profile.limits.max_schedules);
    assert_eq!(json_usize(limits, "max_schedule_steps"), profile.limits.max_schedule_steps);
    assert_eq!(json_usize(limits, "max_adapters"), profile.limits.max_adapters);
    assert_eq!(json_usize(limits, "max_observations"), profile.limits.max_observations);
    assert_eq!(json_usize(limits, "max_unsupported_rows"), profile.limits.max_unsupported_rows);
    assert_eq!(json_u64(limits, "max_restarts"), u64::from(profile.limits.max_restarts));

    let adapters = value.get("adapters").and_then(serde_json::Value::as_array).expect("adapters");
    assert_eq!(adapters.len(), profile.adapters.len());
    for expected in &profile.adapters {
        let observed = adapters
            .iter()
            .find(|adapter| json_string(adapter, "adapter_id") == expected.adapter_id)
            .expect("projected adapter");
        assert_eq!(json_string(observed, "owner"), expected.owner.as_str());
        assert_eq!(json_string(observed, "profile"), expected.profile);
        assert_eq!(json_string(observed, "implementation_ref"), expected.implementation_ref);
        assert_eq!(json_string(observed, "semantic_phase_map_ref"), expected.semantic_phase_map_ref);
    }

    let cases = value.get("cases").and_then(serde_json::Value::as_array).expect("cases");
    assert_eq!(cases.len(), profile.cases.len());
    for expected in &profile.cases {
        let observed =
            cases.iter().find(|case| json_string(case, "case_id") == expected.case_id).expect("projected case");
        assert_eq!(json_string(observed, "mutation"), expected.mutation.as_str());
        assert_eq!(json_string(observed, "operation_id"), expected.operation_id);
        assert_eq!(json_string(observed, "phase"), expected.phase.as_str());
        assert_eq!(json_string(observed, "adapter_id"), expected.adapter_id);
        assert_eq!(json_u64(observed, "expected_generation"), expected.expected_generation);
        assert_eq!(json_string(observed, "pre_state_ref"), expected.pre_state_ref);
        assert_eq!(json_string(observed, "expected_decision"), expected.expected_decision.as_str());
    }

    let schedules = value.get("schedules").and_then(serde_json::Value::as_array).expect("schedules");
    assert_eq!(schedules.len(), profile.schedules.len());
    for expected in &profile.schedules {
        let observed = schedules
            .iter()
            .find(|schedule| json_string(schedule, "schedule_id") == expected.schedule_id)
            .expect("projected schedule");
        assert_eq!(json_string(observed, "mutation"), expected.mutation.as_str());
        let steps = observed.get("steps").and_then(serde_json::Value::as_array).expect("schedule steps");
        assert_eq!(steps.len(), expected.steps.len());
        for expected_step in &expected.steps {
            let observed_step = steps
                .iter()
                .find(|step| {
                    json_u64(step, "position") == u64::from(expected_step.position)
                        && json_string(step, "operation_id") == expected_step.operation_id
                })
                .expect("projected schedule step");
            assert_eq!(json_string(observed_step, "mutation"), expected_step.mutation.as_str());
            assert_eq!(json_u64(observed_step, "expected_generation"), expected_step.expected_generation);
            assert_eq!(json_string(observed_step, "pre_state_ref"), expected_step.pre_state_ref);
            assert_eq!(json_string(observed_step, "interleaving"), expected_step.interleaving.as_str());
            assert_eq!(json_string(observed_step, "node_id"), expected_step.node_id);
            assert_eq!(json_u64(observed_step, "node_generation"), expected_step.node_generation);
        }
    }
    assert_eq!(
        value
            .get("unexplained_numeric_thresholds")
            .and_then(serde_json::Value::as_array)
            .expect("unexplained thresholds")
            .len(),
        0
    );
    assert_eq!(value.get("claims_independent_witness").and_then(serde_json::Value::as_bool), Some(false));
    assert_eq!(value.get("claims_physical_power_loss").and_then(serde_json::Value::as_bool), Some(false));
}

// r[verify molten.world_faults.interruption]
// r[verify molten.world_faults.recovery]
// r[verify molten.world_faults.receipt]
#[test]
fn full_harness_reopens_local_state_and_publishes_bounded_receipt_last() {
    let inventory = standard_world_mutation_inventory();
    let profile = standard_world_fault_profile(TEST_SOURCE_REVISION).expect("fault profile");
    let workspace = crate::test_support::process_workspace("world-fault-restart").expect("process workspace");
    let root = NodeStateRoot::open(&workspace).expect("node state root");
    root.create_layout().expect("node state layout");
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut control = DeterministicFaultControl { events: events.clone() };
    let mut restart = LocalRestartPort {
        root: workspace.to_path_buf(),
        restarts: 0,
        events: events.clone(),
    };
    let mut durable = LocalDurableObservationPort {
        root: workspace.to_path_buf(),
        events: events.clone(),
    };
    let mut owner = ExpectedOwnerCore {
        calls: 0,
        misclassify_lost_response: false,
        events: events.clone(),
    };
    let mut schedules = DeterministicSchedulePort { events: events.clone() };
    let mut receipts = RecordingReceiptPort {
        records: Vec::new(),
        return_crossed_ref: false,
        events: events.clone(),
    };

    let outcome = run_world_fault_conformance(&inventory, &profile, WorldFaultHarnessPorts {
        fault_control: &mut control,
        restart: &mut restart,
        durable_observation: &mut durable,
        owner_decision: &mut owner,
        concurrent_schedule: &mut schedules,
        receipts: &mut receipts,
    })
    .expect("world fault conformance");

    assert_eq!(outcome.receipt.decision, ConformanceDisposition::Passed);
    assert_eq!(restart.restarts, EXPECTED_RESTARTS);
    assert_eq!(owner.calls, EXPECTED_SUPPORTED_CASES);
    assert_eq!(receipts.records.len(), EXPECTED_RECEIPTS);
    assert_eq!(outcome.persisted_receipt_ref, outcome.record.record_ref);
    assert_eq!(events.borrow().last().map(String::as_str), Some("receipt"));
}

// r[verify molten.world_faults.shell_boundary]
#[test]
fn owner_core_misclassification_is_recorded_as_failure_without_shell_compensation() {
    let inventory = standard_world_mutation_inventory();
    let profile = standard_world_fault_profile(TEST_SOURCE_REVISION).expect("fault profile");
    let workspace = crate::test_support::process_workspace("world-fault-owner-negative").expect("workspace");
    let root = NodeStateRoot::open(&workspace).expect("node state root");
    root.create_layout().expect("node state layout");
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut control = DeterministicFaultControl { events: events.clone() };
    let mut restart = LocalRestartPort {
        root: workspace.to_path_buf(),
        restarts: 0,
        events: events.clone(),
    };
    let mut durable = LocalDurableObservationPort {
        root: workspace.to_path_buf(),
        events: events.clone(),
    };
    let mut owner = ExpectedOwnerCore {
        calls: 0,
        misclassify_lost_response: true,
        events: events.clone(),
    };
    let mut schedules = DeterministicSchedulePort { events: events.clone() };
    let mut receipts = RecordingReceiptPort {
        records: Vec::new(),
        return_crossed_ref: false,
        events,
    };
    let outcome = run_world_fault_conformance(&inventory, &profile, WorldFaultHarnessPorts {
        fault_control: &mut control,
        restart: &mut restart,
        durable_observation: &mut durable,
        owner_decision: &mut owner,
        concurrent_schedule: &mut schedules,
        receipts: &mut receipts,
    })
    .expect("failed conformance is still a receipt");
    assert_eq!(outcome.receipt.decision, ConformanceDisposition::Failed);
    assert!(outcome.receipt.results.iter().any(|result| {
        result.phase == FaultPhase::LostResponse
            && result
                .diagnostics
                .iter()
                .any(|issue| matches!(issue, WorldFaultIssue::UnsafeRetryAfterPossibleSubmit(_)))
    }));
}

// r[verify molten.world_faults.receipt]
#[test]
fn receipt_port_cannot_cross_the_canonical_identity() {
    let inventory = standard_world_mutation_inventory();
    let profile = standard_world_fault_profile(TEST_SOURCE_REVISION).expect("fault profile");
    let workspace = crate::test_support::process_workspace("world-fault-crossed-receipt").expect("workspace");
    let root = NodeStateRoot::open(&workspace).expect("node state root");
    root.create_layout().expect("node state layout");
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut control = DeterministicFaultControl { events: events.clone() };
    let mut restart = LocalRestartPort {
        root: workspace.to_path_buf(),
        restarts: 0,
        events: events.clone(),
    };
    let mut durable = LocalDurableObservationPort {
        root: workspace.to_path_buf(),
        events: events.clone(),
    };
    let mut owner = ExpectedOwnerCore {
        calls: 0,
        misclassify_lost_response: false,
        events: events.clone(),
    };
    let mut schedules = DeterministicSchedulePort { events: events.clone() };
    let mut receipts = RecordingReceiptPort {
        records: Vec::new(),
        return_crossed_ref: true,
        events,
    };
    let error = run_world_fault_conformance(&inventory, &profile, WorldFaultHarnessPorts {
        fault_control: &mut control,
        restart: &mut restart,
        durable_observation: &mut durable,
        owner_decision: &mut owner,
        concurrent_schedule: &mut schedules,
        receipts: &mut receipts,
    })
    .expect_err("crossed receipt identity must fail");
    assert!(error.to_string().contains("crossed record identity"));
}

fn read_back_for_case(case: &WorldFaultCase) -> DurableReadBack {
    let complete = case.expected_decision == RecoveryClass::AlreadyComplete;
    let prior = case.expected_decision == RecoveryClass::SafeToRetry;
    DurableReadBack {
        status: if complete {
            DurableReadBackStatus::Applied
        } else if prior {
            DurableReadBackStatus::Prior
        } else {
            DurableReadBackStatus::Missing
        },
        state_ref: if complete || prior {
            Some(test_ref(&format!("{}:state", case.case_id)))
        } else {
            None
        },
        record_ref: if complete {
            Some(test_ref(&format!("{}:record", case.case_id)))
        } else {
            None
        },
        observed_generation: if complete || prior {
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

fn json_string<'a>(value: &'a serde_json::Value, field: &str) -> &'a str {
    value.get(field).and_then(serde_json::Value::as_str).expect("JSON string field")
}

fn json_u64(value: &serde_json::Value, field: &str) -> u64 {
    value.get(field).and_then(serde_json::Value::as_u64).expect("JSON u64 field")
}

fn json_usize(value: &serde_json::Value, field: &str) -> usize {
    usize::try_from(json_u64(value, field)).expect("JSON usize field")
}

fn test_ref(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}
