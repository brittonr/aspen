use super::*;
use crate::fabric_time::AdmittedTimeProfile;
use crate::fabric_time::SchedulerOrdering;
use crate::fabric_time::SchedulerOverloadPolicy;
use crate::fabric_time::SchedulerPolicy;
use crate::fabric_time::SchedulerReplayPolicy;
use crate::fabric_time::TimeDomain;
use crate::fabric_time::TimeEvidenceMode;
use crate::fabric_time::TimeNonClaim;
use crate::fabric_time::TimeProfileKind;

const SERVICE_GENERATION: u64 = 7;
const CONSISTENCY_EPOCH: u64 = 11;
const ENGINE_EPOCH: u64 = 13;
const INITIAL_TICK: u64 = 100;
const VISIBILITY_TICKS: u64 = 10;
const RETRY_TICKS: u64 = 5;
const RETENTION_TICKS: u64 = 20;
const CAPACITY: u32 = 8;
const METADATA_BYTES: u32 = 32;
const STATUS_LIMIT: u32 = CAPACITY;
const MAX_ATTEMPTS: u64 = 2;
const BLAKE3_HEX_LENGTH: usize = 64;
const CLEANUP_OFFSET_TICKS: u64 = 2;
const QUEUE_ID: &str = "queue:delivery";
const ACTOR_ID: &str = "consumer-a";
const OTHER_ACTOR_ID: &str = "consumer-b";
const RETRYABLE_FAILURE: &str = "transient";
const POISON_FAILURE: &str = "poison";

fn reference(hex: char) -> String {
    format!("blake3:{}", hex.to_string().repeat(BLAKE3_HEX_LENGTH))
}

fn policy() -> DeliveryPolicy {
    DeliveryPolicy {
        schema: DELIVERY_POLICY_SCHEMA.to_string(),
        policy_id: "delivery-policy-v1".to_string(),
        visibility_timeout_ticks: VISIBILITY_TICKS,
        maximum_attempts: MAX_ATTEMPTS,
        retry_base_delay_ticks: RETRY_TICKS,
        retry_maximum_delay_ticks: RETRY_TICKS,
        retry_backoff: DeliveryBackoff::Fixed,
        ordering: DeliveryOrdering::StrictFifo,
        dead_letter_queue_id: "queue:delivery-dlq".to_string(),
        dead_letter_retention_ticks: RETENTION_TICKS,
        ready_capacity: CAPACITY,
        in_flight_capacity: CAPACITY,
        retry_capacity: CAPACITY,
        dead_letter_capacity: CAPACITY,
        metadata_byte_limit: METADATA_BYTES,
        status_item_limit: STATUS_LIMIT,
        completion_authority_ref: reference('a'),
        expiry_authority_ref: reference('b'),
        redrive_authority_ref: reference('c'),
        retention_authority_ref: reference('d'),
        retryable_failure_classes: [RETRYABLE_FAILURE.to_string()].into_iter().collect(),
        poison_failure_classes: [POISON_FAILURE.to_string()].into_iter().collect(),
        poison_item_handling: PoisonItemHandling::DeadLetter,
        non_claims: required_delivery_non_claims(),
    }
}

fn manifest(policy: &DeliveryPolicy) -> DeliveryManifest {
    DeliveryManifest {
        schema: DELIVERY_MANIFEST_SCHEMA.to_string(),
        extension_id: "coordination-delivery".to_string(),
        service_id: "coordination-delivery-local".to_string(),
        service_generation: SERVICE_GENERATION,
        implementation_ref: reference('e'),
        time_profile_ref: reference('f'),
        policy_ref: identify_delivery_policy(policy),
        port_bindings: REQUIRED_DELIVERY_PORTS
            .into_iter()
            .enumerate()
            .map(|(index, port)| {
                let digits = ['1', '2', '3', '4', '5'];
                (port.to_string(), reference(digits[index]))
            })
            .collect(),
        non_claims: required_delivery_non_claims(),
    }
}

fn time_profile(manifest: &DeliveryManifest) -> AdmittedTimeProfile {
    AdmittedTimeProfile {
        profile_id: "delivery-logical-time".to_string(),
        profile_ref: manifest.time_profile_ref.clone(),
        kind: TimeProfileKind::DeterministicSimulation,
        supported_domains: vec![TimeDomain::Logical],
        max_duration_ticks: MAX_DELIVERY_TICKS,
        max_uncertainty_ticks: 0,
        max_timers: u64::from(CAPACITY),
        max_runnables: u64::from(CAPACITY),
        max_entropy_request_bytes: 1,
        max_entropy_total_bytes: 1,
        max_scheduler_concurrency: u64::from(CAPACITY),
        max_scheduler_queue_depth: u64::from(CAPACITY),
        fairness_bound_turns: Some(MAX_DELIVERY_TICKS),
        scheduler_policy: SchedulerPolicy {
            ordering: SchedulerOrdering::Fifo,
            replay: SchedulerReplayPolicy::Deterministic,
            overload: SchedulerOverloadPolicy::Reject,
        },
        evidence_mode: TimeEvidenceMode::SelectedSemanticBoundaries,
        non_claims: vec![
            TimeNonClaim::NoGlobalTime,
            TimeNonClaim::NoSynchronizedClocks,
            TimeNonClaim::NoDistributedLeaseExclusivity,
            TimeNonClaim::NoFairness,
            TimeNonClaim::NoLiveness,
            TimeNonClaim::NoSafeRetry,
            TimeNonClaim::NoPartitionAbsence,
            TimeNonClaim::NoRemoteDeadlineAgreement,
        ],
    }
}

fn state(manifest: &DeliveryManifest) -> DeliveryState {
    DeliveryState::empty(QUEUE_ID, manifest.policy_ref.clone(), SERVICE_GENERATION, CONSISTENCY_EPOCH)
}

fn request(
    manifest: &DeliveryManifest,
    operation_id: char,
    actor_id: &str,
    logical_tick: u64,
    operation: DeliveryOperation,
) -> DeliveryRequest {
    DeliveryRequest {
        schema: DELIVERY_REQUEST_SCHEMA.to_string(),
        queue_id: QUEUE_ID.to_string(),
        operation_id: reference(operation_id),
        actor_id: actor_id.to_string(),
        service_generation: SERVICE_GENERATION,
        consistency_epoch: CONSISTENCY_EPOCH,
        engine_epoch: ENGINE_EPOCH,
        time_profile_ref: manifest.time_profile_ref.clone(),
        logical_tick,
        currentness: DeliveryCurrentness::Linearizable,
        authority_refs: vec![reference('6')],
        policy_refs: vec![manifest.policy_ref.clone()],
        resource_refs: vec![reference('7')],
        evidence_refs: vec![reference('8')],
        operation,
    }
}

fn enqueue_request(manifest: &DeliveryManifest, operation_id: char, item_ref: char) -> DeliveryRequest {
    request(manifest, operation_id, ACTOR_ID, INITIAL_TICK, DeliveryOperation::Enqueue {
        item_ref: reference(item_ref),
        content_ref: reference('9'),
        metadata_ref: reference('0'),
        metadata_bytes: METADATA_BYTES,
    })
}

fn transition(
    manifest: &DeliveryManifest,
    policy: &DeliveryPolicy,
    state: &DeliveryState,
    request: &DeliveryRequest,
) -> DeliveryTransition {
    let time = time_profile(manifest);
    plan_delivery_transition(&DeliveryTransitionInput {
        manifest,
        policy,
        time_profile: &time,
        state,
        request,
    })
}

fn enqueue_and_claim() -> (DeliveryPolicy, DeliveryManifest, DeliveryState, DeliveryToken) {
    let policy = policy();
    let manifest = manifest(&policy);
    let initial = state(&manifest);
    let enqueued = transition(&manifest, &policy, &initial, &enqueue_request(&manifest, '1', 'a'));
    assert_eq!(enqueued.decision, DeliveryDecisionKind::Applied);
    let claim = request(&manifest, '2', ACTOR_ID, INITIAL_TICK, DeliveryOperation::Claim);
    let claimed = transition(&manifest, &policy, &enqueued.next_state, &claim);
    assert_eq!(claimed.kind, DeliveryTransitionKind::Claimed);
    let token = claimed.token.clone().expect("claim token");
    (policy, manifest, claimed.next_state, token)
}

// r[verify molten.coordination_delivery.versioned_extension]
#[test]
fn policy_manifest_and_state_have_stable_exact_identities() {
    let policy = policy();
    let manifest = manifest(&policy);
    let state = state(&manifest);
    assert_eq!(validate_delivery_policy(&policy), Ok(()));
    assert_eq!(validate_delivery_manifest(&manifest, &policy), Ok(()));
    assert_eq!(validate_delivery_state(&state, &manifest, &policy), Ok(()));
    assert_eq!(
        identify_delivery_policy(&policy),
        "blake3:05be03f3c3a2af25a8ba2f4f603b205b8105abdc15c61b4438518370b5e09d8a"
    );
    assert_eq!(identify_delivery_policy(&policy), identify_delivery_policy(&policy.clone()));
    assert_eq!(identify_delivery_manifest(&manifest), identify_delivery_manifest(&manifest.clone()));
    assert_eq!(identify_delivery_state(&state), identify_delivery_state(&state.clone()));
}

// r[verify molten.coordination_delivery.claim_lease]
// r[verify molten.coordination_delivery.fenced_completion]
#[test]
fn enqueue_claim_and_ack_form_a_fenced_logical_lease() {
    let (policy, manifest, claimed_state, token) = enqueue_and_claim();
    assert_eq!(token.visibility_deadline_tick, INITIAL_TICK + VISIBILITY_TICKS);
    assert_eq!(token.token_ref, identify_delivery_token(&token));
    let ack =
        request(&manifest, '3', ACTOR_ID, INITIAL_TICK + 1, DeliveryOperation::Acknowledge { token: token.clone() });
    let acknowledged = transition(&manifest, &policy, &claimed_state, &ack);
    assert_eq!(acknowledged.decision, DeliveryDecisionKind::Applied);
    assert_eq!(acknowledged.kind, DeliveryTransitionKind::Acknowledged);
    assert!(acknowledged.next_state.in_flight.is_empty());
    assert!(acknowledged.next_state.completed.contains_key(&token.item_ref));
    assert!(!acknowledged.external_effect_exactly_once);
}

// r[verify molten.coordination_delivery.fenced_completion]
#[test]
fn stale_currentness_wrong_owner_and_expired_ack_preserve_state() {
    let (policy, manifest, claimed_state, token) = enqueue_and_claim();
    let before = identify_delivery_state(&claimed_state);

    let mut stale =
        request(&manifest, '3', ACTOR_ID, INITIAL_TICK + 1, DeliveryOperation::Acknowledge { token: token.clone() });
    stale.currentness = DeliveryCurrentness::LocalStale;
    let denied = transition(&manifest, &policy, &claimed_state, &stale);
    assert_eq!(denied.issue, Some(DeliveryIssue::CurrentnessRequired));
    assert_eq!(denied.after_state_ref, before);

    let wrong_owner = request(&manifest, '4', OTHER_ACTOR_ID, INITIAL_TICK + 1, DeliveryOperation::Acknowledge {
        token: token.clone(),
    });
    let denied = transition(&manifest, &policy, &claimed_state, &wrong_owner);
    assert_eq!(denied.issue, Some(DeliveryIssue::WrongOwner));
    assert_eq!(denied.after_state_ref, before);

    let expired =
        request(&manifest, '5', ACTOR_ID, token.visibility_deadline_tick, DeliveryOperation::Acknowledge { token });
    let denied = transition(&manifest, &policy, &claimed_state, &expired);
    assert_eq!(denied.issue, Some(DeliveryIssue::LeaseExpired));
    assert_eq!(denied.after_state_ref, before);
}

// r[verify molten.coordination_delivery.fenced_completion]
#[test]
fn delegated_completion_needs_the_exact_policy_authority() {
    let (policy, manifest, claimed_state, token) = enqueue_and_claim();
    let mut delegated =
        request(&manifest, '3', OTHER_ACTOR_ID, INITIAL_TICK + 1, DeliveryOperation::Acknowledge { token });
    delegated.authority_refs.push(policy.completion_authority_ref.clone());
    let acknowledged = transition(&manifest, &policy, &claimed_state, &delegated);
    assert_eq!(acknowledged.kind, DeliveryTransitionKind::Acknowledged);
}

// r[verify molten.coordination_delivery.retry_dlq_policy]
#[test]
fn retry_waits_for_logical_time_then_exhaustion_enters_the_dlq() {
    let (policy, manifest, claimed_state, first_token) = enqueue_and_claim();
    let nack = request(&manifest, '3', ACTOR_ID, INITIAL_TICK + 1, DeliveryOperation::NegativeAcknowledge {
        token: first_token,
        failure_class: RETRYABLE_FAILURE.to_string(),
    });
    let retry = transition(&manifest, &policy, &claimed_state, &nack);
    assert_eq!(retry.kind, DeliveryTransitionKind::RetryScheduled);
    let retry_entry = retry.next_state.ready.values().next().expect("retry entry");
    let eligible_at = retry_entry.eligible_at_tick;

    let early_claim = request(&manifest, '4', ACTOR_ID, eligible_at - 1, DeliveryOperation::Claim);
    let denied = transition(&manifest, &policy, &retry.next_state, &early_claim);
    assert_eq!(denied.issue, Some(DeliveryIssue::NoEligibleItem));

    let second_claim = request(&manifest, '5', ACTOR_ID, eligible_at, DeliveryOperation::Claim);
    let second = transition(&manifest, &policy, &retry.next_state, &second_claim);
    let second_token = second.token.clone().expect("second token");
    assert_eq!(second_token.attempt, MAX_ATTEMPTS);
    let second_nack = request(&manifest, '6', ACTOR_ID, eligible_at + 1, DeliveryOperation::NegativeAcknowledge {
        token: second_token.clone(),
        failure_class: RETRYABLE_FAILURE.to_string(),
    });
    let dead_letter = transition(&manifest, &policy, &second.next_state, &second_nack);
    assert_eq!(dead_letter.kind, DeliveryTransitionKind::DeadLettered);
    assert!(dead_letter.next_state.dead_letter.contains_key(&second_token.item_ref));
}

// r[verify molten.coordination_delivery.logical_time]
#[test]
fn expiry_requires_current_logical_time_and_exact_expiry_authority() {
    let (policy, manifest, claimed_state, token) = enqueue_and_claim();
    let before = identify_delivery_state(&claimed_state);
    let early = request(&manifest, '3', ACTOR_ID, token.visibility_deadline_tick - 1, DeliveryOperation::ExpireLease {
        token: token.clone(),
    });
    let denied = transition(&manifest, &policy, &claimed_state, &early);
    assert_eq!(denied.issue, Some(DeliveryIssue::ExpiryAuthorityRequired));
    assert_eq!(denied.after_state_ref, before);

    let mut expiry =
        request(&manifest, '4', OTHER_ACTOR_ID, token.visibility_deadline_tick, DeliveryOperation::ExpireLease {
            token,
        });
    expiry.authority_refs.push(policy.expiry_authority_ref.clone());
    let expired = transition(&manifest, &policy, &claimed_state, &expiry);
    assert_eq!(expired.kind, DeliveryTransitionKind::RetryScheduled);
}

// r[verify molten.coordination_delivery.retry_dlq_policy]
#[test]
fn redrive_and_cleanup_require_distinct_authorities() {
    let (policy, manifest, claimed_state, token) = enqueue_and_claim();
    let mut poison = request(&manifest, '3', ACTOR_ID, INITIAL_TICK + 1, DeliveryOperation::NegativeAcknowledge {
        token: token.clone(),
        failure_class: POISON_FAILURE.to_string(),
    });
    poison.authority_refs.push(policy.completion_authority_ref.clone());
    let dead_letter = transition(&manifest, &policy, &claimed_state, &poison);
    assert_eq!(dead_letter.kind, DeliveryTransitionKind::DeadLettered);

    let redrive = request(&manifest, '4', ACTOR_ID, INITIAL_TICK + 2, DeliveryOperation::Redrive {
        item_ref: token.item_ref.clone(),
    });
    let denied = transition(&manifest, &policy, &dead_letter.next_state, &redrive);
    assert_eq!(denied.issue, Some(DeliveryIssue::RedriveAuthorityRequired));

    let mut authorized = redrive;
    authorized.operation_id = reference('5');
    authorized.authority_refs.push(policy.redrive_authority_ref.clone());
    let redriven = transition(&manifest, &policy, &dead_letter.next_state, &authorized);
    assert_eq!(redriven.kind, DeliveryTransitionKind::Redriven);
    assert!(redriven.next_state.ready.contains_key(&token.item_ref));

    let cleanup_tick = INITIAL_TICK + RETENTION_TICKS + CLEANUP_OFFSET_TICKS;
    let mut cleanup = request(&manifest, '6', ACTOR_ID, cleanup_tick, DeliveryOperation::CleanupDeadLetter {
        through_tick: cleanup_tick,
    });
    cleanup.authority_refs.push(policy.retention_authority_ref.clone());
    let cleaned = transition(&manifest, &policy, &dead_letter.next_state, &cleanup);
    assert_eq!(cleaned.kind, DeliveryTransitionKind::DeadLetterCleaned);
    assert!(cleaned.next_state.dead_letter.is_empty());
}

// r[verify molten.coordination_delivery.claim_lease]
#[test]
fn duplicate_replay_is_stable_and_conflicting_operation_id_denies() {
    let policy = policy();
    let manifest = manifest(&policy);
    let initial = state(&manifest);
    let enqueue = enqueue_request(&manifest, '1', 'a');
    let first = transition(&manifest, &policy, &initial, &enqueue);
    let duplicate = transition(&manifest, &policy, &first.next_state, &enqueue);
    assert_eq!(duplicate.decision, DeliveryDecisionKind::DuplicateReplay);
    assert_eq!(duplicate.after_state_ref, first.after_state_ref);
    assert_eq!(duplicate.prior_operation_ref.as_deref(), Some(first.operation_ref.as_str()));

    let conflicting = enqueue_request(&manifest, '1', 'b');
    let denied = transition(&manifest, &policy, &first.next_state, &conflicting);
    assert_eq!(denied.issue, Some(DeliveryIssue::ConflictingDuplicateOperation));
    assert_eq!(denied.after_state_ref, first.after_state_ref);
}

// r[verify molten.coordination_delivery.content_refs]
#[test]
fn metadata_bounds_and_worker_admission_fail_before_payload_effects() {
    let policy = policy();
    let manifest = manifest(&policy);
    let initial = state(&manifest);
    let mut oversized = enqueue_request(&manifest, '1', 'a');
    if let DeliveryOperation::Enqueue { metadata_bytes, .. } = &mut oversized.operation {
        *metadata_bytes = METADATA_BYTES + 1;
    }
    let denied = transition(&manifest, &policy, &initial, &oversized);
    assert_eq!(denied.issue, Some(DeliveryIssue::MetadataLimitExceeded));

    let (_, _, claimed_state, token) = enqueue_and_claim();
    let active = claimed_state.in_flight.get(&token.item_ref).expect("active claim");
    let incomplete = plan_delivery_worker_dispatch(active, &DeliveryWorkerAdmission {
        content_verified: true,
        provenance_current: true,
        authority_current: false,
        policy_current: true,
        resource_admitted: true,
        execution_admitted: true,
        evidence_refs: vec![reference('1')],
    });
    assert!(!incomplete.admitted);
    assert!(!incomplete.external_effect_authorized);

    let complete = plan_delivery_worker_dispatch(active, &DeliveryWorkerAdmission {
        content_verified: true,
        provenance_current: true,
        authority_current: true,
        policy_current: true,
        resource_admitted: true,
        execution_admitted: true,
        evidence_refs: vec![reference('1')],
    });
    assert!(complete.admitted);
    assert!(!complete.external_effect_authorized);
    assert!(!complete.exact_once_claimed);
}

// r[verify molten.coordination_delivery.retry_dlq_policy]
#[test]
fn status_is_bounded_and_never_renders_payloads() {
    let (policy, manifest, claimed_state, token) = enqueue_and_claim();
    let resource_refs = vec![reference('7')];
    let evidence_refs = vec![reference('8')];
    let status = project_delivery_status(&claimed_state, &StatusProjectionInput {
        policy: &policy,
        requested_limit: STATUS_LIMIT,
        resource_refs: &resource_refs,
        evidence_refs: &evidence_refs,
    })
    .expect("status");
    assert_eq!(status.in_flight_count, 1);
    assert_eq!(status.retry_count, 0);
    assert_eq!(status.active_claims[0].delivery_id, token.delivery_id);
    assert_eq!(status.resource_refs, resource_refs);
    assert_eq!(status.evidence_refs, evidence_refs);
    assert!(!status.payloads_rendered);

    let nack = request(&manifest, '3', ACTOR_ID, INITIAL_TICK + 1, DeliveryOperation::NegativeAcknowledge {
        token,
        failure_class: RETRYABLE_FAILURE.to_string(),
    });
    let retry = transition(&manifest, &policy, &claimed_state, &nack);
    let retry_status = project_delivery_status(&retry.next_state, &StatusProjectionInput {
        policy: &policy,
        requested_limit: STATUS_LIMIT,
        resource_refs: &resource_refs,
        evidence_refs: &evidence_refs,
    })
    .expect("retry status");
    assert_eq!(retry_status.retry_count, 1);
    assert_eq!(retry_status.failed_attempt_count, 1);
    assert_eq!(retry_status.maximum_attempts, MAX_ATTEMPTS);

    assert_eq!(
        project_delivery_status(&claimed_state, &StatusProjectionInput {
            policy: &policy,
            requested_limit: 0,
            resource_refs: &status.resource_refs,
            evidence_refs: &status.evidence_refs,
        },),
        Err(DeliveryIssue::InvalidPolicy)
    );
}

// r[verify molten.coordination_delivery.final_validation]
#[test]
fn policy_drift_wall_clock_and_missing_port_fail_closed() {
    let mut invalid_policy = policy();
    invalid_policy.maximum_attempts = 0;
    assert!(validate_delivery_policy(&invalid_policy).is_err());

    let valid_policy = policy();
    let mut invalid_manifest = manifest(&valid_policy);
    invalid_manifest.port_bindings.remove(PORT_DURABLE_STATE);
    assert!(validate_delivery_manifest(&invalid_manifest, &valid_policy).is_err());

    let valid_manifest = manifest(&valid_policy);
    let initial = state(&valid_manifest);
    let request = enqueue_request(&valid_manifest, '1', 'a');
    let mut wall_only = time_profile(&valid_manifest);
    wall_only.supported_domains = vec![TimeDomain::WallClock];
    let denied = plan_delivery_transition(&DeliveryTransitionInput {
        manifest: &valid_manifest,
        policy: &valid_policy,
        time_profile: &wall_only,
        state: &initial,
        request: &request,
    });
    assert_eq!(denied.issue, Some(DeliveryIssue::LogicalTimeRequired));
    assert_eq!(denied.before_state_ref, denied.after_state_ref);
}
