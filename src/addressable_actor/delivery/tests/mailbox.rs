use super::*;
use crate::coordination_delivery::DeliveryStatusPort;

#[test]
fn observed_status_does_not_claim_publication_durability() {
    let mut status_port = ObservedStatus::default();
    let policy = policy();
    let state = DeliveryState::empty("actor-queue", identify_delivery_policy(&policy), 1, 1);
    let status = project_delivery_status(&state, &StatusProjectionInput {
        policy: &policy,
        requested_limit: 1,
        resource_refs: &[reference("resources")],
        evidence_refs: &[reference("evidence")],
    })
    .expect("canonical status");
    let observation = status_port.publish_status(&status).expect("observe status");
    assert!(status_port.observed_ref.is_some());
    assert_eq!(observation.published_status_ref, None);
    assert!(!observation.outcome_unknown);
}

#[test]
fn pending_count_excludes_quarantined_and_completed() {
    let mut state = DeliveryState::empty("actor-queue", reference("policy"), 1, 1);
    assert_eq!(pending_items(&state).expect("empty mailbox"), 0);
    let item = DeliveryItem {
        item_ref: reference("request"),
        content_ref: reference("payload"),
        metadata_ref: reference("metadata"),
        metadata_bytes: 1,
        enqueue_sequence: 1,
        policy_ref: state.policy_ref.clone(),
    };
    state.ready.insert(item.item_ref.clone(), ReadyDelivery {
        item: item.clone(),
        eligible_at_tick: 1,
        cycle: 1,
        attempts_in_cycle: 0,
    });
    assert_eq!(pending_items(&state).expect("ready mailbox"), 1);
    state.dead_letter.insert(reference("quarantined"), DeadLetterDelivery {
        item: item.clone(),
        entered_at_tick: 1,
        cycle: 1,
        attempts_in_cycle: 1,
        total_attempts: 1,
        reason: "poison".to_string(),
    });
    state.completed.insert(reference("completed"), CompletedDelivery {
        item,
        delivery_id: reference("completed-delivery"),
        acknowledged_at_tick: 1,
        total_attempts: 1,
    });
    assert_eq!(pending_items(&state).expect("quarantined and completed are not runnable"), 1);
}

#[test]
fn observations_read_the_existing_redb_head_across_reopening() {
    let temporary = cap_tempfile::tempdir(cap_std::ambient_authority()).expect("temporary scoped node root");
    let root = molten_node_host::node_state::DirectoryRoot::from_dir(temporary.try_clone().expect("clone root"));
    root.create_layout().expect("node state layout");
    let storage = root.namespace(molten_node_host::node_state::NamespaceKind::Storage).expect("storage namespace");
    let policy = policy();
    let profile = time_profile();
    let manifest = DeliveryManifest {
        schema: DELIVERY_MANIFEST_SCHEMA.to_string(),
        extension_id: "actor-delivery".to_string(),
        service_id: "actor-delivery-service".to_string(),
        service_generation: 1,
        implementation_ref: reference("implementation"),
        time_profile_ref: profile.profile_ref.clone(),
        policy_ref: identify_delivery_policy(&policy),
        port_bindings: REQUIRED_DELIVERY_PORTS.into_iter().map(|port| (port.to_string(), reference(port))).collect(),
        non_claims: required_delivery_non_claims(),
    };
    let state = mailbox_state(&manifest);
    let expected_ref = identify_delivery_state(&state);
    let mut local = crate::coordination_delivery::Local::open(&storage, 1).expect("actual Redb delivery owner");
    local
        .compare_and_commit(&crate::coordination_delivery::CommitRequest {
            queue_id: state.queue_id.clone(),
            expected: crate::coordination_delivery::ExpectedState {
                state_ref: None,
                revision: 0,
            },
            next: crate::coordination_delivery::PublishedState::from_state(state),
            requested_engine_epoch: 1,
        })
        .expect("persist actual mailbox head");
    drop(local);
    let local = crate::coordination_delivery::Local::open(&storage, 1).expect("reopened Redb owner");
    let clock = crate::fabric_time::VirtualClockAdapter::new(&profile, 0, 0).expect("selected clock");
    let bridge = Bridge::new(local, manifest, policy, profile, clock).expect("admitted delivery bridge");
    assert_eq!(bridge.pending_items("actor-queue").expect("current pending count"), 1);
    assert_eq!(bridge.queue_state_ref("actor-queue").expect("persisted state ref"), Some(expected_ref));
    assert_eq!(
        bridge
            .completed_delivery("actor-queue", &reference("completed"))
            .expect("terminal row")
            .expect("completed item")
            .delivery_id,
        reference("terminal-delivery")
    );
    assert_eq!(bridge.pending_items("absent-queue").expect("absent queue"), 0);
    assert!(bridge.completed_delivery("actor-queue", &reference("absent-item")).expect("absent item").is_none());
}

#[test]
fn fifo_claim_preview_does_not_claim_older_work_for_new_request() {
    let workspace = crate::test_support::process_workspace("actor-fifo-claim-preview").expect("scoped workspace");
    let root = crate::node_state::DirectoryRoot::open_existing(workspace.as_ref()).expect("node root");
    root.create_layout().expect("node layout");
    let storage = root.namespace(crate::node_state::NamespaceKind::Storage).expect("storage namespace");
    let mut policy = policy();
    policy.ready_capacity = 2;
    let profile = time_profile();
    let manifest = DeliveryManifest {
        schema: DELIVERY_MANIFEST_SCHEMA.to_string(),
        extension_id: "actor-delivery".to_string(),
        service_id: "actor-delivery-service".to_string(),
        service_generation: 1,
        implementation_ref: reference("implementation"),
        time_profile_ref: profile.profile_ref.clone(),
        policy_ref: identify_delivery_policy(&policy),
        port_bindings: REQUIRED_DELIVERY_PORTS.into_iter().map(|port| (port.to_string(), reference(port))).collect(),
        non_claims: required_delivery_non_claims(),
    };
    let mut state = mailbox_state(&manifest);
    let older = reference("request");
    let newer = reference("new-request");
    let mut ready = state.ready[&older].clone();
    ready.item.item_ref = newer.clone();
    ready.item.content_ref = newer.clone();
    ready.item.enqueue_sequence = 3;
    state.next_sequence = 4;
    state.ready.insert(newer.clone(), ready);
    let before = identify_delivery_state(&state);
    let mut local = crate::coordination_delivery::Local::open(&storage, 1).expect("delivery owner");
    local
        .compare_and_commit(&crate::coordination_delivery::CommitRequest {
            queue_id: state.queue_id.clone(),
            expected: crate::coordination_delivery::ExpectedState {
                state_ref: None,
                revision: 0,
            },
            next: crate::coordination_delivery::PublishedState::from_state(state),
            requested_engine_epoch: 1,
        })
        .expect("persist FIFO ready work");
    let clock = crate::fabric_time::VirtualClockAdapter::new(&profile, 0, 0).expect("clock");
    let claim = observed_request(&manifest, "new-request-claim", DeliveryOperation::Claim);
    let bridge = Bridge::new(local, manifest, policy, profile, clock).expect("delivery bridge");
    assert_eq!(bridge.claimable_item(&claim).expect("pure claim preview"), Some(older.clone()));
    assert_eq!(bridge.queue_state_ref("actor-queue").expect("read-only queue"), Some(before));
    assert!(bridge.active_token("actor-queue", &older).expect("older token").is_none());
    assert!(bridge.active_token("actor-queue", &newer).expect("newer token").is_none());
    assert_eq!(bridge.pending_items("actor-queue").expect("unchanged runnable work"), 2);
}
