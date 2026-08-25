//! Imperative membership shell that orders admitted external effects.

#![allow(
    tigerstyle::function_length,
    tigerstyle::path_segment_repetition,
    reason = "the membership transaction keeps intent, effect, commit, and uncertainty order visible in one bounded shell"
)]

use super::*;

// r[impl molten.modularity.fabric_boundary.shell]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedProviderSnapshot {
    pub membership: CanonicalMembershipView,
    pub failures: CanonicalFailureObservationSet,
    pub reservations: Vec<CapacityReservation>,
}

// r[impl molten.fabric_membership.live_sim_parity]
// r[impl molten.fabric_membership.authority_separation]
pub fn observe_provider(
    provider: &mut dyn MembershipPlacementProvider,
) -> crate::error::Result<AdmittedProviderSnapshot> {
    let snapshot = provider.snapshot().map_err(crate::error::MoltenError::from)?;
    if provider.provider_kind() != snapshot.profile.provider_kind {
        return Err(crate::error::MoltenError::invalid_harness(
            "membership provider kind differs from its source profile",
        ));
    }
    let profile = canonical_membership_profile(&snapshot.profile)?;
    let membership = canonical_membership_view(
        &profile,
        &snapshot.view,
        &snapshot.descriptors,
        snapshot.observed_now_ticks,
        &snapshot.required_compatibility_ref,
    )?;
    let failures = canonical_failure_observations(
        &membership,
        &snapshot.detector_profiles,
        &snapshot.failure_observations,
        snapshot.observed_now_ticks,
    )?;
    Ok(AdmittedProviderSnapshot {
        membership,
        failures,
        reservations: snapshot.reservations,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignmentExecutionPhase {
    IntentPersistence,
    RoleEffect,
    CommitPersistence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UncertainAssignmentExecution {
    pub transition: AssignmentTransition,
    pub phase: AssignmentExecutionPhase,
    pub message: String,
    pub intent_ref: Option<String>,
    pub role_effect_ref: Option<String>,
    pub effect_may_have_happened: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignmentExecutionOutcome {
    Committed(CanonicalAssignmentTransition),
    Uncertain(UncertainAssignmentExecution),
}

// r[impl molten.fabric_membership.recruitment]
// r[impl molten.fabric_membership.drain_replace]
// r[impl molten.fabric_membership.evidence]
pub fn execute_assignment_command(
    persistence: &mut dyn AssignmentPersistence,
    lifecycle: &mut dyn ExtensionRoleLifecyclePort,
    current: &RoleAssignment,
    command: &AssignmentCommand,
    authority: &AssignmentAuthoritySnapshot,
) -> std::result::Result<AssignmentExecutionOutcome, Vec<AssignmentIssue>> {
    let authority_issues = validate_assignment_authority(current, authority);
    if !authority_issues.is_empty() {
        return Err(authority_issues);
    }
    let transition = apply_assignment_command(current, command)?;
    let intent_ref = match persistence.record_intent(current, &transition) {
        Ok(intent_ref) => intent_ref,
        Err(error) => {
            return Ok(AssignmentExecutionOutcome::Uncertain(UncertainAssignmentExecution {
                transition,
                phase: AssignmentExecutionPhase::IntentPersistence,
                message: error.to_string(),
                intent_ref: None,
                role_effect_ref: None,
                effect_may_have_happened: false,
            }));
        }
    };
    if !valid_evidence_ref(&intent_ref) {
        return Ok(AssignmentExecutionOutcome::Uncertain(UncertainAssignmentExecution {
            transition,
            phase: AssignmentExecutionPhase::IntentPersistence,
            message: "assignment persistence returned a malformed intent ref".to_string(),
            intent_ref: Some(intent_ref),
            role_effect_ref: None,
            effect_may_have_happened: false,
        }));
    }
    let role_effect_ref = match run_role_effect(lifecycle, &transition) {
        Ok(effect_ref) => effect_ref,
        Err(failure) => {
            return Ok(AssignmentExecutionOutcome::Uncertain(UncertainAssignmentExecution {
                transition,
                phase: AssignmentExecutionPhase::RoleEffect,
                message: failure.message,
                intent_ref: Some(intent_ref),
                role_effect_ref: None,
                effect_may_have_happened: failure.effect_may_have_happened,
            }));
        }
    };
    if role_effect_ref.as_deref().is_some_and(|effect_ref| !valid_evidence_ref(effect_ref)) {
        return Ok(AssignmentExecutionOutcome::Uncertain(UncertainAssignmentExecution {
            transition,
            phase: AssignmentExecutionPhase::RoleEffect,
            message: "extension lifecycle returned a malformed effect ref".to_string(),
            intent_ref: Some(intent_ref),
            role_effect_ref,
            effect_may_have_happened: true,
        }));
    }
    let persistence_ref = match persistence.commit(&transition, &intent_ref, role_effect_ref.as_deref()) {
        Ok(persistence_ref) => persistence_ref,
        Err(error) => {
            return Ok(AssignmentExecutionOutcome::Uncertain(UncertainAssignmentExecution {
                transition,
                phase: AssignmentExecutionPhase::CommitPersistence,
                message: error.to_string(),
                intent_ref: Some(intent_ref),
                role_effect_ref,
                effect_may_have_happened: true,
            }));
        }
    };
    if !valid_evidence_ref(&persistence_ref) {
        return Ok(AssignmentExecutionOutcome::Uncertain(UncertainAssignmentExecution {
            transition,
            phase: AssignmentExecutionPhase::CommitPersistence,
            message: "assignment persistence returned a malformed commit ref".to_string(),
            intent_ref: Some(intent_ref),
            role_effect_ref,
            effect_may_have_happened: true,
        }));
    }
    match canonical_assignment_transition(&transition, &intent_ref, role_effect_ref.as_deref(), &persistence_ref) {
        Ok(canonical) => Ok(AssignmentExecutionOutcome::Committed(canonical)),
        Err(error) => Ok(AssignmentExecutionOutcome::Uncertain(UncertainAssignmentExecution {
            transition,
            phase: AssignmentExecutionPhase::CommitPersistence,
            message: format!("canonical assignment evidence failed after commit: {error}"),
            intent_ref: Some(intent_ref),
            role_effect_ref,
            effect_may_have_happened: true,
        })),
    }
}

fn run_role_effect(
    lifecycle: &mut dyn ExtensionRoleLifecyclePort,
    transition: &AssignmentTransition,
) -> std::result::Result<Option<String>, RoleEffectFailure> {
    let effect = match transition.kind {
        AssignmentCommandKind::Activate => Some(lifecycle.activate(&transition.next)?),
        AssignmentCommandKind::BeginDrain => Some(lifecycle.begin_drain(&transition.next)?),
        AssignmentCommandKind::BeginReplacement => Some(lifecycle.begin_replacement(&transition.next)?),
        AssignmentCommandKind::Release => Some(lifecycle.release(&transition.next)?),
        AssignmentCommandKind::Fail => Some(lifecycle.fail(&transition.next)?),
        AssignmentCommandKind::Quarantine => Some(lifecycle.quarantine(&transition.next)?),
        AssignmentCommandKind::Reserve | AssignmentCommandKind::Assign | AssignmentCommandKind::Acknowledge => None,
    };
    Ok(effect)
}

fn valid_evidence_ref(value: &str) -> bool {
    const BLAKE3_PREFIX: &str = "blake3:";
    const BLAKE3_HEX_LENGTH: usize = 64;
    value.strip_prefix(BLAKE3_PREFIX).is_some_and(|hex| {
        hex.len() == BLAKE3_HEX_LENGTH && hex.chars().all(|character| matches!(character, '0'..='9' | 'a'..='f'))
    })
}
