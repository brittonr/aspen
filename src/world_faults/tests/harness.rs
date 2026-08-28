use std::cell::RefCell;
use std::rc::Rc;

use molten_core::world_faults::*;
use molten_node_host::node_state::NodeStateRoot;

use super::super::*;
use super::support::*;

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
