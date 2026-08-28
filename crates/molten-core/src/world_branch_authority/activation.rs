use std::collections::BTreeSet;

use super::model::*;

const ACTIVATION_IDENTITY_DOMAIN: &str = "onixresearch.molten.world-branch-authority.activation.v1";

// r[impl molten.world_branch_authority.activation]
// r[impl molten.world_branch_authority.simulation]
pub fn decide_world_branch_activation(
    plan: &WorldBranchAuthorityPlan,
    observation: &WorldBranchRealizationObservation,
    current: &CurrentAuthorityFacts,
) -> WorldBranchActivationDecision {
    let diagnostic = activation_diagnostic(plan, observation, current);
    let is_allowed = diagnostic == WorldBranchAuthorityDiagnostic::Admitted;
    let decision_ref = activation_identity(
        plan.plan_ref.as_str(),
        observation.operation_ref.as_str(),
        current.observation_ref.as_str(),
        is_allowed,
        diagnostic,
    );
    WorldBranchActivationDecision {
        schema: WORLD_BRANCH_AUTHORITY_ACTIVATION_SCHEMA,
        decision_ref,
        allowed: is_allowed,
        plan_ref: plan.plan_ref.clone(),
        observation_ref: observation.operation_ref.clone(),
        diagnostic,
        non_claims: WORLD_BRANCH_AUTHORITY_NON_CLAIMS.iter().map(ToString::to_string).collect(),
    }
}

fn activation_diagnostic(
    plan: &WorldBranchAuthorityPlan,
    observation: &WorldBranchRealizationObservation,
    current: &CurrentAuthorityFacts,
) -> WorldBranchAuthorityDiagnostic {
    if !plan.allowed || plan.mode.is_none() || plan.obligations.contains(&WorldBranchObligation::DenyActivation) {
        return WorldBranchAuthorityDiagnostic::NonBranchable;
    }
    if !valid_content_ref(&plan.plan_ref)
        || !valid_content_ref(&plan.policy_ref)
        || !valid_content_ref(&plan.capability_ref)
        || !valid_content_ref(&observation.operation_ref)
        || !valid_content_ref(&current.observation_ref)
    {
        return WorldBranchAuthorityDiagnostic::InvalidInput;
    }
    if observation.plan_ref != plan.plan_ref
        || observation.policy_ref != plan.policy_ref
        || observation.capability_ref != plan.capability_ref
        || observation.destination_scope != plan.destination_scope
    {
        return WorldBranchAuthorityDiagnostic::ObservationMismatch;
    }
    if !current.policy_current {
        return WorldBranchAuthorityDiagnostic::PolicyStale;
    }
    if !current.all_current() {
        return WorldBranchAuthorityDiagnostic::CurrentnessDenied;
    }
    let Some(mode) = plan.mode else {
        return WorldBranchAuthorityDiagnostic::NonBranchable;
    };
    if mode_requires_ucan(mode) && !current.ucan_verified {
        return WorldBranchAuthorityDiagnostic::UcanCompositionMissing;
    }
    if observation.bearer_material_present {
        return WorldBranchAuthorityDiagnostic::BearerMaterialPresent;
    }
    if observation.receipt_claims_authority {
        return WorldBranchAuthorityDiagnostic::ReceiptAuthorityOverclaim;
    }
    if observation.destination_active {
        return WorldBranchAuthorityDiagnostic::ObservationMismatch;
    }
    if !complete_evidence(plan, observation) {
        return WorldBranchAuthorityDiagnostic::MissingObligationEvidence;
    }
    mode_diagnostic(mode, observation)
}

fn mode_diagnostic(
    mode: WorldBranchMode,
    observation: &WorldBranchRealizationObservation,
) -> WorldBranchAuthorityDiagnostic {
    match mode {
        WorldBranchMode::Copyable | WorldBranchMode::Attenuated | WorldBranchMode::ReplaceBeforeActivation => {
            if observation.destination_grant_current {
                WorldBranchAuthorityDiagnostic::Admitted
            } else {
                WorldBranchAuthorityDiagnostic::CurrentnessDenied
            }
        }
        WorldBranchMode::Linear => {
            if observation.source_active || observation.transfer_generation.is_none() {
                WorldBranchAuthorityDiagnostic::LinearOwnershipAmbiguous
            } else if observation.destination_grant_current {
                WorldBranchAuthorityDiagnostic::Admitted
            } else {
                WorldBranchAuthorityDiagnostic::CurrentnessDenied
            }
        }
        WorldBranchMode::SimulationOnly => {
            if !observation.simulation_adapter_ref.as_deref().is_some_and(valid_content_ref) {
                WorldBranchAuthorityDiagnostic::SimulationAdapterMissing
            } else if !observation.simulation_adapter_deterministic {
                WorldBranchAuthorityDiagnostic::SimulationLiveFallback
            } else {
                WorldBranchAuthorityDiagnostic::Admitted
            }
        }
        WorldBranchMode::PromotionGated => {
            if observation.release_reservation_ref.as_deref().is_some_and(valid_content_ref) {
                WorldBranchAuthorityDiagnostic::Admitted
            } else {
                WorldBranchAuthorityDiagnostic::PromotionReservationMissing
            }
        }
        WorldBranchMode::NonBranchable => WorldBranchAuthorityDiagnostic::NonBranchable,
    }
}

fn complete_evidence(plan: &WorldBranchAuthorityPlan, observation: &WorldBranchRealizationObservation) -> bool {
    if observation.evidence_refs.len() > MAXIMUM_REALIZATION_EVIDENCE
        || observation.evidence_refs.iter().any(|evidence| !valid_content_ref(evidence))
    {
        return false;
    }
    let unique = observation.evidence_refs.iter().collect::<BTreeSet<_>>();
    if unique.len() != observation.evidence_refs.len() {
        return false;
    }
    let required = plan
        .obligations
        .iter()
        .filter(|obligation| **obligation != WorldBranchObligation::DenyActivation)
        .count();
    observation.evidence_refs.len() >= required
}

const fn mode_requires_ucan(mode: WorldBranchMode) -> bool {
    matches!(
        mode,
        WorldBranchMode::Copyable
            | WorldBranchMode::Attenuated
            | WorldBranchMode::Linear
            | WorldBranchMode::PromotionGated
    )
}

fn activation_identity(
    plan_ref: &str,
    operation_ref: &str,
    current_ref: &str,
    is_allowed: bool,
    diagnostic: WorldBranchAuthorityDiagnostic,
) -> String {
    let mut hasher = blake3::Hasher::new_derive_key(ACTIVATION_IDENTITY_DOMAIN);
    for value in [
        plan_ref,
        operation_ref,
        current_ref,
        if is_allowed { "allowed" } else { "denied" },
        diagnostic_text(diagnostic),
    ] {
        update_identity_text(&mut hasher, value);
    }
    format!("blake3:{}", hasher.finalize().to_hex())
}

fn update_identity_text(hasher: &mut blake3::Hasher, value: &str) {
    let length = match u64::try_from(value.len()) {
        Ok(length) => length,
        Err(_) => {
            let fallback = b"length-overflow";
            let Ok(fallback_length) = u64::try_from(fallback.len()) else {
                return;
            };
            hasher.update(&fallback_length.to_le_bytes());
            hasher.update(fallback);
            return;
        }
    };
    hasher.update(&length.to_le_bytes());
    hasher.update(value.as_bytes());
}

const fn diagnostic_text(diagnostic: WorldBranchAuthorityDiagnostic) -> &'static str {
    match diagnostic {
        WorldBranchAuthorityDiagnostic::Admitted => "admitted",
        WorldBranchAuthorityDiagnostic::InvalidInput => "invalid-input",
        WorldBranchAuthorityDiagnostic::PolicyMalformed => "policy-malformed",
        WorldBranchAuthorityDiagnostic::PolicyStale => "policy-stale",
        WorldBranchAuthorityDiagnostic::MappingLossy => "mapping-lossy",
        WorldBranchAuthorityDiagnostic::CurrentnessDenied => "currentness-denied",
        WorldBranchAuthorityDiagnostic::UcanCompositionMissing => "ucan-composition-missing",
        WorldBranchAuthorityDiagnostic::ScopeWidened => "scope-widened",
        WorldBranchAuthorityDiagnostic::AttenuationNotNarrower => "attenuation-not-narrower",
        WorldBranchAuthorityDiagnostic::ActionModeMismatch => "action-mode-mismatch",
        WorldBranchAuthorityDiagnostic::NonBranchable => "non-branchable",
        WorldBranchAuthorityDiagnostic::ObservationMismatch => "observation-mismatch",
        WorldBranchAuthorityDiagnostic::MissingObligationEvidence => "missing-obligation-evidence",
        WorldBranchAuthorityDiagnostic::LinearOwnershipAmbiguous => "linear-ownership-ambiguous",
        WorldBranchAuthorityDiagnostic::SimulationAdapterMissing => "simulation-adapter-missing",
        WorldBranchAuthorityDiagnostic::SimulationLiveFallback => "simulation-live-fallback",
        WorldBranchAuthorityDiagnostic::PromotionReservationMissing => "promotion-reservation-missing",
        WorldBranchAuthorityDiagnostic::BearerMaterialPresent => "bearer-material-present",
        WorldBranchAuthorityDiagnostic::ReceiptAuthorityOverclaim => "receipt-authority-overclaim",
        WorldBranchAuthorityDiagnostic::ActivationDenied => "activation-denied",
        WorldBranchAuthorityDiagnostic::ActivationOutcomeUnknown => "activation-outcome-unknown",
    }
}
