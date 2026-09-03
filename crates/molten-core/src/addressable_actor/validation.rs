use super::*;

const REQUIRED_SELECTED_CONCEPT_COUNT: usize = 5;
const MAX_RESTORE_ISSUES_PER_CLASS: usize = 2;
const REQUIRED_SELECTED_CONCEPTS: [&str; REQUIRED_SELECTED_CONCEPT_COUNT] = [
    "keyed-addressability",
    "generation-fenced-runtime",
    "sleep-intent-and-rewake-separation",
    "persisted-state-and-scheduled-events",
    "runtime-and-durable-survival-separation",
];

// r[impl molten.addressable_actor.profile]
pub fn validate_actor_key(key: &ActorKey) -> Vec<ActorIssue> {
    let mut issues = Vec::new();
    if key.schema != ACTOR_KEY_SCHEMA {
        issues.push(ActorIssue::SchemaMismatch);
    }
    if !crate::fabric::valid_blake3_ref(&key.namespace_ref)
        || !crate::fabric::valid_fabric_token(&key.actor_type)
        || key.key.is_empty()
        || key.key.len() > MAX_ACTOR_KEY_BYTES
        || !key
            .key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/'))
    {
        issues.push(ActorIssue::MalformedActorKey);
    }
    issues
}

// r[impl molten.addressable_actor.profile]
// r[impl molten.addressable_actor.survival]
pub fn validate_addressable_actor_profile(profile: &AddressableActorProfile) -> Vec<ActorIssue> {
    let mut issues = Vec::new();
    if profile.schema != ACTOR_PROFILE_SCHEMA {
        issues.push(ActorIssue::SchemaMismatch);
    }
    if profile.profile_version != ADDRESSABLE_ACTOR_PROFILE_VERSION {
        issues.push(ActorIssue::UnsupportedProfileVersion);
    }
    if !crate::fabric::valid_fabric_token(&profile.profile_id) {
        issues.push(ActorIssue::MalformedActorKey);
    }
    if profile.reference_source.repository != RIVET_ACTORS_REPOSITORY
        || profile.reference_source.revision != RIVET_ACTORS_REVISION
        || profile.reference_source.license != RIVET_ACTORS_LICENSE
        || profile.reference_source.selected_concepts
            != REQUIRED_SELECTED_CONCEPTS.iter().map(|value| (*value).to_string()).collect::<Vec<_>>()
    {
        issues.push(ActorIssue::ReferenceSourceMismatch);
    }
    let references = [
        &profile.system_extension_profile_ref,
        &profile.placement_profile_ref,
        &profile.delivery_profile_ref,
        &profile.durable_state_profile_ref,
        &profile.time_profile_ref,
        &profile.resource_profile_ref,
        &profile.supervision_profile_ref,
        &profile.authority_profile_ref,
        &profile.evidence_profile_ref,
    ];
    if references.iter().any(|reference| !crate::fabric::valid_blake3_ref(reference)) {
        issues.push(ActorIssue::MalformedReference);
    }
    if profile.idle_after_ticks == 0 {
        issues.push(ActorIssue::InvalidIdlePolicy);
    }
    if profile.maximum_drain_items == 0 {
        issues.push(ActorIssue::InvalidDrainPolicy);
    }
    issues.extend(validate_survival_matrix(&profile.survival));
    if profile.non_claims != required_addressable_actor_non_claims() {
        issues.push(ActorIssue::MissingNonClaim);
    }
    issues
}

pub fn validate_actor_state(state: &ActorState) -> Vec<ActorIssue> {
    let mut issues = Vec::new();
    if state.schema != ACTOR_STATE_SCHEMA {
        issues.push(ActorIssue::SchemaMismatch);
    }
    let required_refs = [
        &state.actor_key_ref,
        &state.profile_ref,
        &state.system_extension_manifest_ref,
        &state.placement_ref,
    ];
    if required_refs.iter().any(|reference| !crate::fabric::valid_blake3_ref(reference))
        || state.checkpoint_ref.as_ref().is_some_and(|reference| !crate::fabric::valid_blake3_ref(reference))
        || state
            .durable_state_ref
            .as_ref()
            .is_some_and(|reference| !crate::fabric::valid_blake3_ref(reference))
        || state.active_wake_ref.as_ref().is_some_and(|reference| !crate::fabric::valid_blake3_ref(reference))
        || state
            .unknown_effect_ref
            .as_ref()
            .is_some_and(|reference| !crate::fabric::valid_blake3_ref(reference))
    {
        issues.push(ActorIssue::MalformedReference);
    }
    if state.extension_generation < ADDRESSABLE_ACTOR_INITIAL_GENERATION {
        issues.push(ActorIssue::StaleGeneration);
    }
    if state.applied_operations.len() > MAX_ACTOR_OPERATIONS {
        issues.push(ActorIssue::OperationCapacityExceeded);
    }
    if state.completed_event_refs.len() > MAX_ACTOR_COMPLETED_EVENTS {
        issues.push(ActorIssue::CompletedEventCapacityExceeded);
    }
    if !sorted_unique_refs(&state.completed_event_refs)
        || state.applied_operations.values().any(|operation| {
            !crate::fabric::valid_blake3_ref(&operation.request_ref)
                || !crate::fabric::valid_blake3_ref(&operation.operation_ref)
        })
    {
        issues.push(ActorIssue::StateIdentityMismatch);
    }
    issues
}

pub fn validate_actor_request(request: &ActorRequest) -> Vec<ActorIssue> {
    let mut issues = Vec::new();
    if request.schema != ACTOR_REQUEST_SCHEMA {
        issues.push(ActorIssue::SchemaMismatch);
    }
    if request.operation_id.is_empty()
        || request.operation_id.len() > MAX_ACTOR_OPERATION_ID_BYTES
        || !request
            .operation_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        issues.push(ActorIssue::MalformedOperationId);
    }
    let references = [
        &request.actor_key_ref,
        &request.placement_ref,
        &request.admission.profile_ref,
        &request.admission.system_extension_manifest_ref,
        &request.admission.authority_ref,
        &request.admission.resource_ref,
        &request.admission.adapter_ref,
    ];
    if references.iter().any(|reference| !crate::fabric::valid_blake3_ref(reference))
        || operation_refs(&request.operation)
            .iter()
            .any(|reference| !crate::fabric::valid_blake3_ref(reference))
    {
        issues.push(ActorIssue::MalformedReference);
    }
    issues
}

// r[impl molten.addressable_actor.survival]
pub fn validate_restore_classes(matrix: &ActorSurvivalMatrix, classes: &[SurvivalClass]) -> Vec<ActorIssue> {
    let issue_capacity = classes.len().saturating_mul(MAX_RESTORE_ISSUES_PER_CLASS);
    let mut issues = Vec::with_capacity(issue_capacity);
    let mut seen = std::collections::BTreeSet::new();
    for class in classes {
        if !seen.insert(*class) {
            issues.push(ActorIssue::DuplicateRestoreClass { class: *class });
        }
        if !matrix.disposition(*class).is_some_and(SurvivalDisposition::permits_restore) {
            issues.push(ActorIssue::RestoreClassDenied { class: *class });
        }
    }
    issues
}

// r[impl molten.addressable_actor.lifecycle]
pub fn validate_system_extension_binding(
    actor: &ActorState,
    extension: &crate::system_extension::LifecycleState,
) -> Vec<ActorIssue> {
    let mut issues = Vec::new();
    if actor.extension_generation != extension.generation {
        issues.push(ActorIssue::SystemExtensionGenerationMismatch);
    }
    if !actor_phase_matches_system_extension(actor.phase, extension.phase) {
        issues.push(ActorIssue::SystemExtensionPhaseMismatch);
    }
    if actor.checkpoint_ref != extension.checkpoint_ref {
        issues.push(ActorIssue::SystemExtensionCheckpointMismatch);
    }
    issues
}

#[must_use]
pub const fn actor_phase_matches_system_extension(
    actor: ActorPhase,
    extension: crate::system_extension::LifecyclePhase,
) -> bool {
    match actor {
        ActorPhase::Dormant => matches!(extension, crate::system_extension::LifecyclePhase::Drained),
        ActorPhase::Starting => matches!(
            extension,
            crate::system_extension::LifecyclePhase::Starting | crate::system_extension::LifecyclePhase::Initializing
        ),
        ActorPhase::Running => matches!(extension, crate::system_extension::LifecyclePhase::Running),
        ActorPhase::Draining => matches!(extension, crate::system_extension::LifecyclePhase::Draining),
        ActorPhase::Stopped => matches!(extension, crate::system_extension::LifecyclePhase::Stopped),
        ActorPhase::Degraded => matches!(
            extension,
            crate::system_extension::LifecyclePhase::Failed | crate::system_extension::LifecyclePhase::Quarantined
        ),
        ActorPhase::Recovering => matches!(
            extension,
            crate::system_extension::LifecyclePhase::Recovering | crate::system_extension::LifecyclePhase::Restarting
        ),
    }
}

fn validate_survival_matrix(matrix: &ActorSurvivalMatrix) -> Vec<ActorIssue> {
    let mut issues = Vec::with_capacity(MAX_RESTORE_ISSUES_PER_CLASS);
    if matrix.schema != ACTOR_SURVIVAL_MATRIX_SCHEMA
        || matrix.profile_version != ADDRESSABLE_ACTOR_PROFILE_VERSION
        || matrix != &standard_actor_survival_matrix()
    {
        issues.push(ActorIssue::SurvivalMatrixMismatch);
    }
    let mut seen = std::collections::BTreeSet::new();
    for rule in &matrix.rules {
        if !seen.insert(rule.class) {
            issues.push(ActorIssue::DuplicateSurvivalClass);
            break;
        }
    }
    issues
}

fn sorted_unique_refs(values: &[String]) -> bool {
    values.windows(2).all(|window| window[0] < window[1])
        && values.iter().all(|reference| crate::fabric::valid_blake3_ref(reference))
}

fn operation_refs(operation: &ActorOperation) -> Vec<&str> {
    match operation {
        ActorOperation::Wake { reason } => match reason {
            WakeReason::Message {
                delivery_item_ref,
                delivery_token_ref,
            } => vec![delivery_item_ref, delivery_token_ref],
            WakeReason::Timer { timer_ref } => vec![timer_ref],
            WakeReason::Connection { connection_ref } => vec![connection_ref],
            WakeReason::Operator { operator_request_ref } => vec![operator_request_ref],
        },
        ActorOperation::StartSucceeded { wake_ref } => vec![wake_ref],
        ActorOperation::IdleSleep { checkpoint_ref, .. }
        | ActorOperation::DrainSucceeded { checkpoint_ref, .. }
        | ActorOperation::BeginRecovery { checkpoint_ref } => vec![checkpoint_ref],
        ActorOperation::BeginDrain | ActorOperation::Stop => Vec::new(),
        ActorOperation::Degrade { failure_ref } | ActorOperation::RecoveryFailed { failure_ref } => {
            vec![failure_ref]
        }
        ActorOperation::RecoverySucceeded {
            checkpoint_ref,
            durable_state_ref,
            ..
        } => vec![checkpoint_ref, durable_state_ref],
        ActorOperation::CompleteDelivery {
            delivery_item_ref,
            delivery_token_ref,
            semantic_event_ref,
            semantic_commit_ref,
        } => vec![
            delivery_item_ref,
            delivery_token_ref,
            semantic_event_ref,
            semantic_commit_ref,
        ],
        ActorOperation::RecordUnknownEffect { effect_ref } => vec![effect_ref],
        ActorOperation::ResolveUnknownEffect {
            effect_ref,
            resolution_ref,
            checkpoint_ref,
        } => vec![effect_ref, resolution_ref, checkpoint_ref],
    }
}
