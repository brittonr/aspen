use std::process::Command;

use molten_core::coordination_delivery::*;
use molten_node_host::node_state::NodeStateNamespaceKind;
use molten_node_host::node_state::NodeStateRoot;

use super::super::*;
use super::support::*;

const CHILD_MODE_ENV: &str = "MOLTEN_COORDINATION_DELIVERY_CHILD_MODE";
const CHILD_TOKEN_ENV: &str = "MOLTEN_COORDINATION_DELIVERY_CHILD_TOKEN";
const CHILD_MODE_STALE_ACK: &str = "stale-ack";
const CHILD_MODE_CURRENT_ACK: &str = "current-ack";
const CHILD_TEST_NAME: &str = "coordination_delivery::tests::multiprocess::child_delivery_process";
const TEST_EXACT_ARGUMENT: &str = "--exact";
const TEST_NOCAPTURE_ARGUMENT: &str = "--nocapture";
const CHILD_SUCCESS: i32 = 0;

// r[verify molten.coordination_delivery.consistency_durability]
// r[verify molten.coordination_delivery.final_validation]
#[test]
fn multiprocess_restart_recovers_claim_and_fences_stale_consumer() {
    let workspace =
        crate::test_support::process_workspace("coordination-delivery-multiprocess").expect("process workspace");
    let workspace_path: &std::path::Path = workspace.as_ref();
    let directory = cap_std::fs::Dir::open_ambient_dir(workspace_path, cap_std::ambient_authority())
        .expect("open process workspace");
    let root = NodeStateRoot::from_dir(directory);
    root.create_layout().expect("node state layout");
    let storage = root.namespace(NodeStateNamespaceKind::Storage).expect("storage namespace");
    let policy = policy();
    let manifest = manifest(&policy);
    let time = time_profile(&manifest);
    let mut timers = timer_port(false);
    let mut statuses = MemoryStatusPort::default();

    let enqueued = apply_local(
        &storage,
        &mut timers,
        &mut statuses,
        &manifest,
        &policy,
        &time,
        empty_expected(),
        &enqueue_request(&manifest, '1'),
    );
    let claim = request(&manifest, '2', INITIAL_TICK, DeliveryOperation::Claim);
    let first_claim = apply_local(
        &storage,
        &mut timers,
        &mut statuses,
        &manifest,
        &policy,
        &time,
        expected_from_outcome(&enqueued),
        &claim,
    );
    let first_token = first_claim.transition.token.clone().expect("first token");

    let mut expiry = request(&manifest, '3', first_token.visibility_deadline_tick, DeliveryOperation::ExpireLease {
        token: first_token.clone(),
    });
    expiry.authority_refs.push(policy.expiry_authority_ref.clone());
    let expired = apply_local(
        &storage,
        &mut timers,
        &mut statuses,
        &manifest,
        &policy,
        &time,
        expected_from_outcome(&first_claim),
        &expiry,
    );
    let retry_tick = expired.transition.next_state.ready.values().next().expect("retry").eligible_at_tick;
    let second_claim_request = request(&manifest, '4', retry_tick, DeliveryOperation::Claim);
    let second_claim = apply_local(
        &storage,
        &mut timers,
        &mut statuses,
        &manifest,
        &policy,
        &time,
        expected_from_outcome(&expired),
        &second_claim_request,
    );
    let second_token = second_claim.transition.token.clone().expect("second token");
    drop(storage);

    run_child(workspace_path, CHILD_MODE_STALE_ACK, &first_token);
    run_child(workspace_path, CHILD_MODE_CURRENT_ACK, &second_token);

    let directory = cap_std::fs::Dir::open_ambient_dir(workspace_path, cap_std::ambient_authority())
        .expect("reopen process workspace");
    let root = NodeStateRoot::from_dir(directory);
    let storage = root.namespace(NodeStateNamespaceKind::Storage).expect("reopened storage namespace");
    let store = LocalDeliveryStore::open(&storage, ENGINE_EPOCH).expect("reopened delivery store");
    let observed = store.load(QUEUE_ID).expect("read final delivery state").expect("final delivery state");
    assert!(observed.state.in_flight.is_empty());
    assert!(observed.state.completed.contains_key(&second_token.item_ref));
}

#[test]
fn child_delivery_process() {
    let Ok(mode) = std::env::var(CHILD_MODE_ENV) else {
        return;
    };
    let token_json = std::env::var(CHILD_TOKEN_ENV).expect("child token");
    let token = serde_json::from_str::<DeliveryToken>(&token_json).expect("parse child token");
    let directory = cap_std::fs::Dir::open_ambient_dir(
        std::env::current_dir().expect("child current directory"),
        cap_std::ambient_authority(),
    )
    .expect("open child root");
    let root = NodeStateRoot::from_dir(directory);
    let storage = root.namespace(NodeStateNamespaceKind::Storage).expect("child storage namespace");
    let policy = policy();
    let manifest = manifest(&policy);
    let time = time_profile(&manifest);
    let mut store = LocalDeliveryStore::open(&storage, ENGINE_EPOCH).expect("child delivery store");
    let published = store.load(QUEUE_ID).expect("child state read").expect("child published state");
    let operation_id = if mode == CHILD_MODE_STALE_ACK { '6' } else { '7' };
    let tick = published
        .state
        .in_flight
        .values()
        .next()
        .map_or(INITIAL_TICK, |active| active.token.claimed_at_tick + 1);
    let ack = request(&manifest, operation_id, tick, DeliveryOperation::Acknowledge { token });
    let mut timers = timer_port(false);
    let mut statuses = MemoryStatusPort::default();
    let outcome = apply_delivery_request(&mut store, &mut timers, &mut statuses, &DeliveryServiceRequest {
        manifest: &manifest,
        policy: &policy,
        time_profile: &time,
        host_binding: &host_binding(&manifest),
        expected: expected(&published),
        request: &ack,
    })
    .expect("child delivery outcome");
    match mode.as_str() {
        CHILD_MODE_STALE_ACK => {
            assert_eq!(outcome.receipt.status, DeliveryServiceStatus::Denied);
            assert_eq!(outcome.transition.issue, Some(DeliveryIssue::TokenMismatch));
        }
        CHILD_MODE_CURRENT_ACK => {
            assert_eq!(outcome.receipt.status, DeliveryServiceStatus::Applied);
            assert_eq!(outcome.transition.kind, DeliveryTransitionKind::Acknowledged);
        }
        other => panic!("unsupported child mode {other}"),
    }
}

fn apply_local(
    storage: &molten_node_host::node_state::NodeStateNamespace,
    timers: &mut MemoryTimerPort,
    statuses: &mut MemoryStatusPort,
    manifest: &DeliveryManifest,
    policy: &DeliveryPolicy,
    time: &molten_core::fabric_time::AdmittedTimeProfile,
    expected: ExpectedDeliveryState,
    request: &DeliveryRequest,
) -> DeliveryServiceOutcome {
    let mut store = LocalDeliveryStore::open(storage, ENGINE_EPOCH).expect("delivery store");
    apply_delivery_request(&mut store, timers, statuses, &DeliveryServiceRequest {
        manifest,
        policy,
        time_profile: time,
        host_binding: &host_binding(manifest),
        expected,
        request,
    })
    .expect("local delivery request")
}

fn expected_from_outcome(outcome: &DeliveryServiceOutcome) -> ExpectedDeliveryState {
    ExpectedDeliveryState {
        state_ref: Some(outcome.transition.after_state_ref.clone()),
        revision: outcome.transition.next_state.revision,
    }
}

fn run_child(root: &std::path::Path, mode: &str, token: &DeliveryToken) {
    let output = Command::new(std::env::current_exe().expect("current test executable"))
        .arg(TEST_EXACT_ARGUMENT)
        .arg(CHILD_TEST_NAME)
        .arg(TEST_NOCAPTURE_ARGUMENT)
        .env(CHILD_MODE_ENV, mode)
        .env(CHILD_TOKEN_ENV, serde_json::to_string(token).expect("serialize child token"))
        .current_dir(root)
        .output()
        .expect("run delivery child");
    assert_eq!(
        output.status.code(),
        Some(CHILD_SUCCESS),
        "child failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
