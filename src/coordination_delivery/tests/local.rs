use molten_core::coordination_delivery::*;
use molten_node_host::node_state::NodeStateNamespaceKind;
use molten_node_host::node_state::NodeStateRoot;

use super::super::*;
use super::support::*;

const EXPECTED_STATUS_COUNT_AFTER_CLAIM: usize = 2;

// r[verify molten.coordination_delivery.consistency_durability]
#[test]
fn service_commits_then_schedules_claim_timer_and_publishes_bounded_status() {
    let policy = policy();
    let manifest = manifest(&policy);
    let time = time_profile(&manifest);
    let mut commit = MemoryCommitPort::new(CommitMode::Apply);
    let mut timers = timer_port(false);
    let mut statuses = MemoryStatusPort::default();
    let enqueue = enqueue_request(&manifest, '1');
    let enqueued = apply_delivery_request(&mut commit, &mut timers, &mut statuses, &DeliveryServiceRequest {
        manifest: &manifest,
        policy: &policy,
        time_profile: &time,
        host_binding: &host_binding(&manifest),
        expected: empty_expected(),
        request: &enqueue,
    })
    .expect("enqueue");
    assert_eq!(enqueued.receipt.status, DeliveryServiceStatus::Applied);
    assert!(timers.observed.is_empty());
    assert_eq!(statuses.status_refs.len(), 1);

    let published = commit.head.clone().expect("published enqueue");
    let claim = request(&manifest, '2', INITIAL_TICK, DeliveryOperation::Claim);
    let claimed = apply_delivery_request(&mut commit, &mut timers, &mut statuses, &DeliveryServiceRequest {
        manifest: &manifest,
        policy: &policy,
        time_profile: &time,
        host_binding: &host_binding(&manifest),
        expected: expected(&published),
        request: &claim,
    })
    .expect("claim");
    assert_eq!(claimed.transition.kind, DeliveryTransitionKind::Claimed);
    assert_eq!(timers.observed.len(), 1);
    assert_eq!(statuses.status_refs.len(), EXPECTED_STATUS_COUNT_AFTER_CLAIM);
    assert!(!claimed.receipt.bytes.is_empty());
}

#[test]
fn stopped_or_mismatched_system_extension_host_denies_before_storage() {
    let policy = policy();
    let manifest = manifest(&policy);
    let time = time_profile(&manifest);
    let request = enqueue_request(&manifest, '1');
    let mut host = host_binding(&manifest);
    host.lifecycle_running = false;
    let mut commit = MemoryCommitPort::new(CommitMode::Apply);
    let mut timers = timer_port(false);
    let mut statuses = MemoryStatusPort::default();
    let result = apply_delivery_request(&mut commit, &mut timers, &mut statuses, &DeliveryServiceRequest {
        manifest: &manifest,
        policy: &policy,
        time_profile: &time,
        host_binding: &host,
        expected: empty_expected(),
        request: &request,
    });
    assert_eq!(result.expect_err("stopped host"), DeliveryServiceError::Host(DeliveryIssue::HostNotRunning));
    assert_eq!(commit.compare_calls, 0);
}

// r[verify molten.coordination_delivery.consistency_durability]
// r[verify molten.coordination_delivery.final_validation]
#[test]
fn capability_rooted_store_reopens_exact_committed_state() {
    let temporary = cap_tempfile::tempdir(cap_std::ambient_authority()).expect("temporary state root");
    let root = NodeStateRoot::from_dir(temporary.try_clone().expect("clone root"));
    root.create_layout().expect("node state layout");
    let storage = root.namespace(NodeStateNamespaceKind::Storage).expect("storage namespace");
    let policy = policy();
    let manifest = manifest(&policy);
    let time = time_profile(&manifest);
    let request = enqueue_request(&manifest, '1');
    let mut timers = timer_port(false);
    let mut statuses = MemoryStatusPort::default();
    let expected_state = {
        let mut store = LocalDeliveryStore::open(&storage, ENGINE_EPOCH).expect("delivery store");
        let outcome = apply_delivery_request(&mut store, &mut timers, &mut statuses, &DeliveryServiceRequest {
            manifest: &manifest,
            policy: &policy,
            time_profile: &time,
            host_binding: &host_binding(&manifest),
            expected: empty_expected(),
            request: &request,
        })
        .expect("publish delivery state");
        assert_eq!(outcome.receipt.status, DeliveryServiceStatus::Applied);
        outcome.transition.after_state_ref
    };
    let reopened = LocalDeliveryStore::open(&storage, ENGINE_EPOCH).expect("reopened delivery store");
    let observed = reopened.load(QUEUE_ID).expect("read delivery state").expect("published state");
    assert_eq!(observed.state_ref, expected_state);
    assert_eq!(observed.state.ready.len(), 1);
}
