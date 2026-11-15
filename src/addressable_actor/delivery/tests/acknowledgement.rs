use super::*;

#[test]
fn refused_original_ack_is_not_satisfied_by_separately_authorized_terminal_row() {
    let workspace =
        crate::test_support::process_workspace("actor-refused-then-separate-ack").expect("scoped workspace");
    let root = crate::node_state::DirectoryRoot::open_existing(workspace.as_ref()).expect("node root");
    root.create_layout().expect("node layout");
    let storage = root.namespace(crate::node_state::NamespaceKind::Storage).expect("storage namespace");
    let (policy, profile, manifest) = setup();
    let (token, state) = active_claim(&manifest);
    let original =
        observed_request(&manifest, "refused-original", DeliveryOperation::Acknowledge { token: token.clone() });
    let mut later = original.clone();
    later.operation_id = reference("separately-authorized");
    later.authority_refs.push(policy.completion_authority_ref.clone());
    let refused = plan_delivery_transition(&DeliveryTransitionInput {
        manifest: &manifest,
        policy: &policy,
        time_profile: &profile,
        state: &state,
        request: &original,
    });
    assert_eq!(refused.decision, DeliveryDecisionKind::Denied);
    assert_eq!(refused.issue, Some(DeliveryIssue::WrongOwner));
    assert_eq!(refused.next_state, state, "original refusal must preserve the active claim");
    let acknowledged = plan_delivery_transition(&DeliveryTransitionInput {
        manifest: &manifest,
        policy: &policy,
        time_profile: &profile,
        state: &state,
        request: &later,
    });
    assert_eq!(acknowledged.decision, DeliveryDecisionKind::Applied);
    let mut local = crate::coordination_delivery::Local::open(&storage, 1).expect("delivery owner");
    local
        .compare_and_commit(&crate::coordination_delivery::CommitRequest {
            queue_id: state.queue_id.clone(),
            expected: crate::coordination_delivery::ExpectedState {
                state_ref: None,
                revision: 0,
            },
            next: crate::coordination_delivery::PublishedState::from_state(acknowledged.next_state),
            requested_engine_epoch: 1,
        })
        .expect("retain separately authorized terminal delivery through actual owner");
    let clock = crate::fabric_time::VirtualClockAdapter::new(&profile, 0, 0).expect("clock");
    let bridge = Bridge::new(local, manifest, policy, profile, clock).expect("delivery bridge");
    let before = bridge.queue_state_ref("actor-queue").expect("queue evidence");
    assert!(bridge.completed_delivery("actor-queue", &token.item_ref).expect("terminal payload row").is_some());
    assert!(bridge.completed_acknowledgement(&original).expect("original acknowledgement read").is_none());
    assert!(bridge.completed_acknowledgement(&later).expect("separate acknowledgement read").is_some());
    let actor = crate::addressable_actor::Local::open(&storage, 1).expect("actual actor evidence owner");
    let key = reference("actor-key");
    let ingress = reference("original-ingress");
    for original_acknowledgement in [None, Some(&original), Some(&later)] {
        let observed = crate::addressable_actor::native::observation::completed_evidence(
            &actor,
            &bridge,
            crate::addressable_actor::native::observation::OriginalCompletion {
                actor_key_ref: &key,
                ingress_ref: &ingress,
                queue_id: "actor-queue",
                item_ref: &token.item_ref,
                acknowledgement: original_acknowledgement,
            },
        )
        .expect("consumer result original-evidence read");
        assert!(observed.is_none(), "later terminal row cannot create an original successful actor receipt");
    }
    assert_eq!(bridge.queue_state_ref("actor-queue").expect("read-only queue evidence"), before);
    assert!(actor.completion(&key, &ingress).expect("read-only actor evidence").is_none());
}

fn setup() -> (DeliveryPolicy, crate::fabric_time::AdmittedTimeProfile, DeliveryManifest) {
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
    (policy, profile, manifest)
}

fn active_claim(manifest: &DeliveryManifest) -> (DeliveryToken, DeliveryState) {
    let mut token = DeliveryToken {
        token_ref: String::new(),
        delivery_id: reference("delivery"),
        queue_id: "actor-queue".to_string(),
        item_ref: reference("item"),
        consumer_id: "original-consumer".to_string(),
        attempt: 1,
        cycle: 1,
        fencing_token: 1,
        claimed_at_tick: 1,
        visibility_deadline_tick: 10,
        consistency_epoch: 1,
        service_generation: 1,
        policy_ref: manifest.policy_ref.clone(),
    };
    token.token_ref = identify_delivery_token(&token);
    let mut state = DeliveryState::empty(&token.queue_id, &token.policy_ref, 1, 1);
    state.revision = 1;
    state.next_sequence = 2;
    state.next_fencing_token = 2;
    state.in_flight.insert(token.item_ref.clone(), ActiveDelivery {
        item: DeliveryItem {
            item_ref: token.item_ref.clone(),
            content_ref: token.item_ref.clone(),
            metadata_ref: token.item_ref.clone(),
            metadata_bytes: 0,
            enqueue_sequence: 1,
            policy_ref: token.policy_ref.clone(),
        },
        token: token.clone(),
    });
    (token, state)
}
