use molten_core::coordination_delivery::*;

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
    let enqueued = apply_request(&mut commit, &mut timers, &mut statuses, &Request {
        manifest: &manifest,
        policy: &policy,
        time_profile: &time,
        host_binding: &host_binding(&manifest),
        expected: empty_expected(),
        request: &enqueue,
    })
    .expect("enqueue");
    assert_eq!(enqueued.receipt.status, ServiceStatus::Applied);
    assert!(timers.observed.is_empty());
    assert_eq!(statuses.status_refs.len(), 1);

    let published = commit.head.clone().expect("published enqueue");
    let claim = request(&manifest, '2', INITIAL_TICK, DeliveryOperation::Claim);
    let claimed = apply_request(&mut commit, &mut timers, &mut statuses, &Request {
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
    let result = apply_request(&mut commit, &mut timers, &mut statuses, &Request {
        manifest: &manifest,
        policy: &policy,
        time_profile: &time,
        host_binding: &host,
        expected: empty_expected(),
        request: &request,
    });
    assert_eq!(result.expect_err("stopped host"), Error::Host(DeliveryIssue::HostNotRunning));
    assert_eq!(commit.compare_calls, 0);
}

// r[verify molten.coordination_delivery.consistency_durability]
// r[verify molten.coordination_delivery.final_validation]
#[test]
fn capability_rooted_store_reopens_exact_committed_state() {
    let temporary = cap_tempfile::tempdir(cap_std::ambient_authority()).expect("temporary state root");
    let root = molten_node_host::node_state::DirectoryRoot::from_dir(temporary.try_clone().expect("clone root"));
    root.create_layout().expect("node state layout");
    let storage = root.namespace(molten_node_host::node_state::NamespaceKind::Storage).expect("storage namespace");
    let policy = policy();
    let manifest = manifest(&policy);
    let time = time_profile(&manifest);
    let request = enqueue_request(&manifest, '1');
    let mut timers = timer_port(false);
    let mut statuses = MemoryStatusPort::default();
    let expected_state = {
        let mut store = Local::open(&storage, ENGINE_EPOCH).expect("delivery store");
        let outcome = apply_request(&mut store, &mut timers, &mut statuses, &Request {
            manifest: &manifest,
            policy: &policy,
            time_profile: &time,
            host_binding: &host_binding(&manifest),
            expected: empty_expected(),
            request: &request,
        })
        .expect("publish delivery state");
        assert_eq!(outcome.receipt.status, ServiceStatus::Applied);
        outcome.transition.after_state_ref
    };
    let reopened = Local::open(&storage, ENGINE_EPOCH).expect("reopened delivery store");
    let observed = reopened.load(QUEUE_ID).expect("read delivery state").expect("published state");
    assert_eq!(observed.state_ref, expected_state);
    assert_eq!(observed.state.ready.len(), 1);
}

// r[verify molten.audit_f08.state_authority]
#[test]
fn duplicate_and_denied_service_requests_preserve_state_and_effects() {
    let policy = policy();
    let manifest = manifest(&policy);
    let mut commit = MemoryCommitPort::new(CommitMode::Apply);
    let mut timers = timer_port(false);
    let mut statuses = MemoryStatusPort::default();
    let enqueue = enqueue_request(&manifest, '1');
    apply_current_request(&mut commit, &mut timers, &mut statuses, &manifest, &policy, &enqueue);
    let claim = request(&manifest, '2', INITIAL_TICK, DeliveryOperation::Claim);
    let claimed = apply_current_request(&mut commit, &mut timers, &mut statuses, &manifest, &policy, &claim);
    let token = claimed.transition.token.expect("claimed token");
    let before = commit.head.clone();
    let before_calls = commit.compare_calls;
    let before_timers = timers.observed.clone();
    let before_statuses = statuses.status_refs.clone();

    let mut changed_claim = claim.clone();
    changed_claim.actor_id = OTHER_ACTOR_ID.to_string();
    let mut wrong_owner =
        request(&manifest, '3', INITIAL_TICK + 1, DeliveryOperation::Acknowledge { token: token.clone() });
    wrong_owner.actor_id = OTHER_ACTOR_ID.to_string();
    let expired = request(&manifest, '4', token.visibility_deadline_tick, DeliveryOperation::Acknowledge {
        token: token.clone(),
    });
    let mut stale_token = token.clone();
    stale_token.fencing_token += 1;
    stale_token.token_ref = identify_delivery_token(&stale_token);
    let stale = request(&manifest, '5', INITIAL_TICK + 1, DeliveryOperation::Acknowledge { token: stale_token });
    for (request, issue) in [
        (&changed_claim, DeliveryIssue::ConflictingDuplicateOperation),
        (&wrong_owner, DeliveryIssue::WrongOwner),
        (&expired, DeliveryIssue::LeaseExpired),
        (&stale, DeliveryIssue::TokenMismatch),
    ] {
        let denied = apply_current_request(&mut commit, &mut timers, &mut statuses, &manifest, &policy, request);
        assert_eq!(denied.transition.decision, DeliveryDecisionKind::Denied);
        assert_eq!(denied.transition.issue, Some(issue));
        assert_eq!(commit.head, before);
        assert_eq!(commit.compare_calls, before_calls);
        assert_eq!(timers.observed, before_timers);
        assert_eq!(statuses.status_refs, before_statuses);
        assert!(denied.commit_observation.is_none());
        assert!(denied.transition.timer_intents.is_empty());
        assert!(!denied.transition.worker_dispatch_authorized);
    }
    for (request, expected_token) in [(&claim, Some(token.clone())), (&enqueue, None)] {
        let replay = apply_current_request(&mut commit, &mut timers, &mut statuses, &manifest, &policy, request);
        assert_eq!(replay.receipt.status, ServiceStatus::DuplicateReplay);
        assert_eq!(replay.transition.token, expected_token);
        assert_eq!(commit.head, before);
        assert_eq!(commit.compare_calls, before_calls);
        assert_eq!(timers.observed, before_timers);
        assert_eq!(statuses.status_refs, before_statuses);
        assert!(replay.commit_observation.is_none());
        assert!(!replay.transition.worker_dispatch_authorized);
    }

    let mut delegated = wrong_owner;
    delegated.operation_id = reference('6');
    delegated.authority_refs.push(policy.completion_authority_ref.clone());
    let completed = apply_current_request(&mut commit, &mut timers, &mut statuses, &manifest, &policy, &delegated);
    assert_eq!(completed.receipt.status, ServiceStatus::Applied);
    assert_eq!(completed.transition.kind, DeliveryTransitionKind::Acknowledged);
    assert!(completed.transition.next_state.completed.contains_key(&token.item_ref));
    assert_eq!(commit.compare_calls, before_calls + 1);
}

// r[verify molten.audit_f08.original_binding]
// r[verify molten.audit_f08.unavailable_result]
// r[verify molten.audit_f08.validation_claims]
#[test]
fn duplicate_claim_after_redb_reopen_never_returns_reclaimed_token() {
    let temporary = cap_tempfile::tempdir(cap_std::ambient_authority()).expect("temporary state root");
    let root = molten_node_host::node_state::DirectoryRoot::from_dir(temporary.try_clone().expect("clone root"));
    root.create_layout().expect("node state layout");
    let storage = root.namespace(molten_node_host::node_state::NamespaceKind::Storage).expect("storage");
    let policy = policy();
    let manifest = manifest(&policy);
    let claim = request(&manifest, '2', INITIAL_TICK, DeliveryOperation::Claim);
    let mut timers = timer_port(false);
    let mut statuses = MemoryStatusPort::default();
    let (original_operation_ref, before) = {
        let mut store = Local::open(&storage, ENGINE_EPOCH).expect("delivery store");
        let enqueue = enqueue_request(&manifest, '1');
        apply_current_request(&mut store, &mut timers, &mut statuses, &manifest, &policy, &enqueue);
        let claimed = apply_current_request(&mut store, &mut timers, &mut statuses, &manifest, &policy, &claim);
        let token = claimed.transition.token.expect("original token");
        let mut expiry =
            request(&manifest, '3', token.visibility_deadline_tick, DeliveryOperation::ExpireLease { token });
        expiry.authority_refs.push(policy.expiry_authority_ref.clone());
        let expired = apply_current_request(&mut store, &mut timers, &mut statuses, &manifest, &policy, &expiry);
        assert_eq!(expired.transition.kind, DeliveryTransitionKind::RetryScheduled);
        let mut reclaim =
            request(&manifest, '4', INITIAL_TICK + VISIBILITY_TICKS + RETRY_TICKS, DeliveryOperation::Claim);
        reclaim.actor_id = OTHER_ACTOR_ID.to_string();
        let reclaimed = apply_current_request(&mut store, &mut timers, &mut statuses, &manifest, &policy, &reclaim);
        assert_eq!(reclaimed.transition.token.as_ref().expect("reclaimed token").consumer_id, OTHER_ACTOR_ID);
        (claimed.transition.operation_ref, store.load(QUEUE_ID).expect("load before reopen").expect("state"))
    };
    let mut reopened = Local::open(&storage, ENGINE_EPOCH).expect("reopen delivery store");
    assert_eq!(reopened.load(QUEUE_ID).expect("reopened state"), Some(before.clone()));
    let before_timers = timers.observed.clone();
    let before_statuses = statuses.status_refs.clone();
    let duplicate = apply_current_request(&mut reopened, &mut timers, &mut statuses, &manifest, &policy, &claim);
    assert_eq!(duplicate.receipt.status, ServiceStatus::DuplicateReplay);
    assert_eq!(duplicate.transition.token, None);
    assert_eq!(duplicate.transition.operation_ref, original_operation_ref);
    assert_eq!(duplicate.transition.prior_operation_ref, Some(original_operation_ref));
    assert_eq!(duplicate.transition.next_state, before.state);
    assert_eq!(reopened.load(QUEUE_ID).expect("unchanged durable state"), Some(before));
    assert_eq!(timers.observed, before_timers);
    assert_eq!(statuses.status_refs, before_statuses);
    assert!(duplicate.commit_observation.is_none());
    assert!(!duplicate.transition.worker_dispatch_authorized);
}
