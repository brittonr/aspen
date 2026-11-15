use super::*;
mod acknowledgement;
mod mailbox;
mod queued;
mod replay;

fn reference(label: &str) -> String {
    crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
}

pub(super) fn time_profile() -> crate::fabric_time::AdmittedTimeProfile {
    use crate::fabric_time::*;
    AdmittedTimeProfile {
        profile_id: "actor-delivery-tests".to_string(),
        profile_ref: reference("actor-time-profile"),
        kind: TimeProfileKind::DeterministicSimulation,
        supported_domains: REQUIRED_TIME_DOMAINS.to_vec(),
        max_duration_ticks: 1_000,
        max_uncertainty_ticks: 0,
        max_timers: MAX_REQUESTS as u64,
        max_runnables: MAX_REQUESTS as u64,
        max_entropy_request_bytes: 1,
        max_entropy_total_bytes: 1,
        max_scheduler_concurrency: 1,
        max_scheduler_queue_depth: MAX_REQUESTS as u64,
        fairness_bound_turns: Some(1_000),
        scheduler_policy: SchedulerPolicy {
            ordering: SchedulerOrdering::Fifo,
            replay: SchedulerReplayPolicy::Deterministic,
            overload: SchedulerOverloadPolicy::Reject,
        },
        evidence_mode: TimeEvidenceMode::SelectedSemanticBoundaries,
        non_claims: REQUIRED_TIME_NON_CLAIMS.to_vec(),
    }
}

fn policy() -> DeliveryPolicy {
    DeliveryPolicy {
        schema: DELIVERY_POLICY_SCHEMA.to_string(),
        policy_id: "actor-delivery-status".to_string(),
        visibility_timeout_ticks: 10,
        maximum_attempts: 1,
        retry_base_delay_ticks: 1,
        retry_maximum_delay_ticks: 1,
        retry_backoff: DeliveryBackoff::Fixed,
        ordering: DeliveryOrdering::StrictFifo,
        dead_letter_queue_id: "actor-dead-letter".to_string(),
        dead_letter_retention_ticks: 10,
        ready_capacity: 1,
        in_flight_capacity: 1,
        retry_capacity: 1,
        dead_letter_capacity: 1,
        metadata_byte_limit: 1,
        status_item_limit: 1,
        completion_authority_ref: reference("complete"),
        expiry_authority_ref: reference("expire"),
        redrive_authority_ref: reference("redrive"),
        retention_authority_ref: reference("retain"),
        retryable_failure_classes: ["transient".to_string()].into_iter().collect(),
        poison_failure_classes: Default::default(),
        poison_item_handling: PoisonItemHandling::DeadLetter,
        non_claims: required_delivery_non_claims(),
    }
}

fn mailbox_state(manifest: &DeliveryManifest) -> DeliveryState {
    let mut state = DeliveryState::empty("actor-queue", &manifest.policy_ref, 1, 1);
    state.revision = 1;
    let item = DeliveryItem {
        item_ref: reference("request"),
        content_ref: reference("payload"),
        metadata_ref: reference("metadata"),
        metadata_bytes: 1,
        enqueue_sequence: 1,
        policy_ref: manifest.policy_ref.clone(),
    };
    state.ready.insert(item.item_ref.clone(), ReadyDelivery {
        item: item.clone(),
        eligible_at_tick: 1,
        cycle: 1,
        attempts_in_cycle: 0,
    });
    let mut completed_item = item;
    completed_item.item_ref = reference("completed");
    completed_item.enqueue_sequence = 2;
    state.next_sequence = 3;
    state.completed.insert(reference("completed"), CompletedDelivery {
        item: completed_item,
        delivery_id: reference("terminal-delivery"),
        acknowledged_at_tick: 1,
        total_attempts: 1,
    });
    state
}

fn observed_request(manifest: &DeliveryManifest, operation_id: &str, operation: DeliveryOperation) -> DeliveryRequest {
    DeliveryRequest {
        schema: DELIVERY_REQUEST_SCHEMA.to_string(),
        queue_id: "actor-queue".to_string(),
        operation_id: reference(operation_id),
        actor_id: "counter".to_string(),
        service_generation: 1,
        consistency_epoch: 1,
        engine_epoch: 1,
        time_profile_ref: manifest.time_profile_ref.clone(),
        logical_tick: 2,
        currentness: DeliveryCurrentness::Linearizable,
        authority_refs: vec![reference("authority")],
        policy_refs: vec![manifest.policy_ref.clone()],
        resource_refs: vec![reference("resources")],
        evidence_refs: vec![reference("native-commit")],
        operation,
    }
}
