use super::*;

#[test]
fn terminal_row_requires_exact_original_acknowledgement() {
    let mut token = DeliveryToken {
        token_ref: String::new(),
        delivery_id: reference("delivery"),
        queue_id: "actor-queue".to_string(),
        item_ref: reference("logical-request"),
        consumer_id: "counter".to_string(),
        attempt: 1,
        cycle: 0,
        fencing_token: 1,
        claimed_at_tick: 1,
        visibility_deadline_tick: 10,
        consistency_epoch: 1,
        service_generation: 1,
        policy_ref: reference("policy"),
    };
    token.token_ref = identify_delivery_token(&token);
    let mut request = DeliveryRequest {
        schema: DELIVERY_REQUEST_SCHEMA.to_string(),
        queue_id: token.queue_id.clone(),
        operation_id: "ack-1".to_string(),
        actor_id: token.consumer_id.clone(),
        service_generation: 1,
        consistency_epoch: 1,
        engine_epoch: 1,
        time_profile_ref: reference("time"),
        logical_tick: 2,
        currentness: DeliveryCurrentness::Linearizable,
        authority_refs: vec![reference("authority")],
        policy_refs: vec![token.policy_ref.clone()],
        resource_refs: vec![reference("resources")],
        evidence_refs: vec![reference("native-commit")],
        operation: DeliveryOperation::Acknowledge { token: token.clone() },
    };
    let mut state = DeliveryState::empty(&token.queue_id, &token.policy_ref, 1, 1);
    state.completed.insert(token.item_ref.clone(), CompletedDelivery {
        item: DeliveryItem {
            item_ref: token.item_ref.clone(),
            content_ref: reference("payload"),
            metadata_ref: reference("metadata"),
            metadata_bytes: 1,
            enqueue_sequence: 1,
            policy_ref: token.policy_ref.clone(),
        },
        delivery_id: token.delivery_id.clone(),
        acknowledged_at_tick: 2,
        total_attempts: 1,
    });
    state.operations.insert(request.operation_id.clone(), AppliedDeliveryOperation {
        request_ref: identify_delivery_request(&request),
        operation_ref: reference("operation"),
        operation_kind: "acknowledge".to_string(),
        item_ref: Some(token.item_ref.clone()),
        token_ref: Some(token.token_ref.clone()),
    });
    assert!(exact_ack_replay(&state, &request, &token));
    request.evidence_refs.push(reference("different-native-commit"));
    assert!(!exact_ack_replay(&state, &request, &token));
    request.evidence_refs.pop();
    let mut stale_token = token.clone();
    stale_token.fencing_token += 1;
    stale_token.token_ref = identify_delivery_token(&stale_token);
    assert!(!exact_ack_replay(&state, &request, &stale_token));
    state.completed.get_mut(&token.item_ref).expect("completed item").delivery_id = reference("other-delivery");
    assert!(!exact_ack_replay(&state, &request, &token));
}

#[test]
fn authority_preview_binds_full_time_admission_not_only_contract_reference() {
    let mut profile = time_profile();
    let original = admit_time_profile(&profile).expect("canonical time admission");
    profile.max_duration_ticks += 1;
    assert_ne!(admit_time_profile(&profile).expect("changed canonical admission"), original);
    profile.supported_domains.clear();
    assert!(admit_time_profile(&profile).is_err());
}
