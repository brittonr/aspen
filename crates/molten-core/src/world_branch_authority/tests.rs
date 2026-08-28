use super::*;

const POLICY_GENERATION: u64 = 1;
const TRANSFER_GENERATION: u64 = 2;
const SOURCE_LIMIT: u64 = 100;
const DESTINATION_LIMIT: u64 = 50;

fn content_ref(label: &str) -> String {
    let mut hasher = blake3::Hasher::new_derive_key("onixresearch.molten.world-branch-authority.test.v1");
    hasher.update(label.as_bytes());
    format!("blake3:{}", hasher.finalize().to_hex())
}

fn scope(resource: &str, abilities: &[&str], limit: Option<u64>) -> NormalizedCapabilityScope {
    NormalizedCapabilityScope::new(resource, abilities.iter().map(|ability| (*ability).to_string()), limit)
        .expect("valid normalized test scope")
}

fn facts(kind: CapabilityKind, action: WorldBranchAction) -> WorldBranchAuthorityFacts {
    let source_scope = scope("service/root", &["read", "write"], Some(SOURCE_LIMIT));
    let destination_scope = if kind == CapabilityKind::ScopedService {
        scope("service/root/child", &["read"], Some(DESTINATION_LIMIT))
    } else {
        source_scope.clone()
    };
    WorldBranchAuthorityFacts {
        capability_kind: kind,
        action,
        source_branch_ref: content_ref("source-branch"),
        destination_branch_ref: content_ref("destination-branch"),
        capability_ref: content_ref("capability"),
        source_scope,
        destination_scope,
        policy_generation: POLICY_GENERATION,
        mapping_lossless: true,
    }
}

fn current() -> CurrentAuthorityFacts {
    CurrentAuthorityFacts {
        observation_ref: content_ref("current-authority"),
        policy_current: true,
        capability_current: true,
        revocation_current: true,
        replay_current: true,
        scope_current: true,
        ucan_verified: true,
    }
}

fn plan(kind: CapabilityKind, action: WorldBranchAction) -> WorldBranchAuthorityPlan {
    plan_world_branch_authority(
        basalt::world_branch_authority::DEFAULT_WORLD_BRANCH_AUTHORITY_POLICY_JSON,
        &facts(kind, action),
        &current(),
    )
}

fn observation(plan: &WorldBranchAuthorityPlan) -> WorldBranchRealizationObservation {
    let evidence_refs = plan
        .obligations
        .iter()
        .enumerate()
        .filter(|(_, obligation)| **obligation != WorldBranchObligation::DenyActivation)
        .map(|(index, _)| content_ref(&format!("evidence-{index}")))
        .collect();
    WorldBranchRealizationObservation {
        plan_ref: plan.plan_ref.clone(),
        policy_ref: plan.policy_ref.clone(),
        capability_ref: plan.capability_ref.clone(),
        operation_ref: content_ref("operation"),
        evidence_refs,
        destination_scope: plan.destination_scope.clone(),
        destination_grant_current: true,
        source_active: plan.mode != Some(WorldBranchMode::Linear),
        destination_active: false,
        transfer_generation: (plan.mode == Some(WorldBranchMode::Linear)).then_some(TRANSFER_GENERATION),
        simulation_adapter_ref: (plan.mode == Some(WorldBranchMode::SimulationOnly))
            .then(|| content_ref("simulation-adapter")),
        simulation_adapter_deterministic: plan.mode == Some(WorldBranchMode::SimulationOnly),
        release_reservation_ref: (plan.mode == Some(WorldBranchMode::PromotionGated))
            .then(|| content_ref("release-reservation")),
        bearer_material_present: false,
        receipt_claims_authority: false,
    }
}

// r[verify molten.world_branch_authority.verification]
#[test]
fn maps_every_closed_basalt_mode_without_minting_authority() {
    let cases = [
        (CapabilityKind::PublicArtifact, WorldBranchAction::Create, WorldBranchMode::Copyable),
        (CapabilityKind::ScopedService, WorldBranchAction::Create, WorldBranchMode::Attenuated),
        (CapabilityKind::ExclusiveLease, WorldBranchAction::Transfer, WorldBranchMode::Linear),
        (CapabilityKind::ExternalEffect, WorldBranchAction::Simulate, WorldBranchMode::SimulationOnly),
        (CapabilityKind::DeferredEffect, WorldBranchAction::Promote, WorldBranchMode::PromotionGated),
        (CapabilityKind::HostSecret, WorldBranchAction::Activate, WorldBranchMode::ReplaceBeforeActivation),
    ];
    for (kind, action, expected_mode) in cases {
        let plan = plan(kind, action);
        assert!(plan.allowed);
        assert_eq!(plan.mode, Some(expected_mode));
        assert!(!plan.obligations.is_empty());
        assert_eq!(plan.non_claims.len(), WORLD_BRANCH_AUTHORITY_NON_CLAIMS.len());
    }
    let denied = plan(CapabilityKind::BearerCredential, WorldBranchAction::Create);
    assert!(!denied.allowed);
    assert_eq!(denied.mode, Some(WorldBranchMode::NonBranchable));
    assert_eq!(denied.diagnostic, WorldBranchAuthorityDiagnostic::ActionModeMismatch);
}

#[test]
fn mapping_widening_attenuation_and_currentness_fail_closed() {
    let mut lossy = facts(CapabilityKind::PublicArtifact, WorldBranchAction::Create);
    lossy.mapping_lossless = false;
    let denied = plan_world_branch_authority(
        basalt::world_branch_authority::DEFAULT_WORLD_BRANCH_AUTHORITY_POLICY_JSON,
        &lossy,
        &current(),
    );
    assert_eq!(denied.diagnostic, WorldBranchAuthorityDiagnostic::MappingLossy);

    let mut widened = facts(CapabilityKind::ScopedService, WorldBranchAction::Create);
    widened.destination_scope = scope("service", &["read", "write"], None);
    let denied = plan_world_branch_authority(
        basalt::world_branch_authority::DEFAULT_WORLD_BRANCH_AUTHORITY_POLICY_JSON,
        &widened,
        &current(),
    );
    assert_eq!(denied.diagnostic, WorldBranchAuthorityDiagnostic::ScopeWidened);

    let mut not_narrower = facts(CapabilityKind::ScopedService, WorldBranchAction::Create);
    not_narrower.destination_scope = not_narrower.source_scope.clone();
    let denied = plan_world_branch_authority(
        basalt::world_branch_authority::DEFAULT_WORLD_BRANCH_AUTHORITY_POLICY_JSON,
        &not_narrower,
        &current(),
    );
    assert_eq!(denied.diagnostic, WorldBranchAuthorityDiagnostic::AttenuationNotNarrower);

    let mut stale = current();
    stale.revocation_current = false;
    let denied = plan_world_branch_authority(
        basalt::world_branch_authority::DEFAULT_WORLD_BRANCH_AUTHORITY_POLICY_JSON,
        &facts(CapabilityKind::PublicArtifact, WorldBranchAction::Create),
        &stale,
    );
    assert_eq!(denied.diagnostic, WorldBranchAuthorityDiagnostic::CurrentnessDenied);

    let malformed_policy = basalt::world_branch_authority::DEFAULT_WORLD_BRANCH_AUTHORITY_POLICY_JSON
        .replace("\"copyable\"", "\"future-copy\"");
    let denied = plan_world_branch_authority(
        &malformed_policy,
        &facts(CapabilityKind::PublicArtifact, WorldBranchAction::Create),
        &current(),
    );
    assert_eq!(denied.diagnostic, WorldBranchAuthorityDiagnostic::PolicyMalformed);

    let mut missing_ucan = current();
    missing_ucan.ucan_verified = false;
    let denied = plan_world_branch_authority(
        basalt::world_branch_authority::DEFAULT_WORLD_BRANCH_AUTHORITY_POLICY_JSON,
        &facts(CapabilityKind::PublicArtifact, WorldBranchAction::Create),
        &missing_ucan,
    );
    assert_eq!(denied.diagnostic, WorldBranchAuthorityDiagnostic::UcanCompositionMissing);

    let mut replayed = current();
    replayed.replay_current = false;
    let denied = plan_world_branch_authority(
        basalt::world_branch_authority::DEFAULT_WORLD_BRANCH_AUTHORITY_POLICY_JSON,
        &facts(CapabilityKind::PublicArtifact, WorldBranchAction::Create),
        &replayed,
    );
    assert_eq!(denied.diagnostic, WorldBranchAuthorityDiagnostic::CurrentnessDenied);

    let denied = plan_world_branch_authority(
        basalt::world_branch_authority::DEFAULT_WORLD_BRANCH_AUTHORITY_POLICY_JSON,
        &facts(CapabilityKind::ExclusiveLease, WorldBranchAction::Create),
        &current(),
    );
    assert_eq!(denied.diagnostic, WorldBranchAuthorityDiagnostic::ActionModeMismatch);
}

#[test]
fn complete_realizations_admit_each_supported_mode() {
    let cases = [
        (CapabilityKind::PublicArtifact, WorldBranchAction::Create),
        (CapabilityKind::ScopedService, WorldBranchAction::Create),
        (CapabilityKind::ExclusiveLease, WorldBranchAction::Transfer),
        (CapabilityKind::ExternalEffect, WorldBranchAction::Simulate),
        (CapabilityKind::DeferredEffect, WorldBranchAction::Promote),
        (CapabilityKind::HostSecret, WorldBranchAction::Activate),
    ];
    for (kind, action) in cases {
        let plan = plan(kind, action);
        let decision = decide_world_branch_activation(&plan, &observation(&plan), &current());
        assert!(decision.allowed, "mode {:?} denied: {:?}", plan.mode, decision.diagnostic);
        assert_eq!(decision.non_claims.len(), WORLD_BRANCH_AUTHORITY_NON_CLAIMS.len());
    }
}

#[test]
fn incomplete_uncertain_and_overclaiming_realizations_deny() {
    let linear = plan(CapabilityKind::ExclusiveLease, WorldBranchAction::Transfer);
    let mut still_active = observation(&linear);
    still_active.source_active = true;
    assert_eq!(
        decide_world_branch_activation(&linear, &still_active, &current()).diagnostic,
        WorldBranchAuthorityDiagnostic::LinearOwnershipAmbiguous
    );

    let simulation = plan(CapabilityKind::ExternalEffect, WorldBranchAction::Simulate);
    let mut missing_adapter = observation(&simulation);
    missing_adapter.simulation_adapter_ref = None;
    assert_eq!(
        decide_world_branch_activation(&simulation, &missing_adapter, &current()).diagnostic,
        WorldBranchAuthorityDiagnostic::SimulationAdapterMissing
    );

    let mut live_fallback = observation(&simulation);
    live_fallback.simulation_adapter_deterministic = false;
    assert_eq!(
        decide_world_branch_activation(&simulation, &live_fallback, &current()).diagnostic,
        WorldBranchAuthorityDiagnostic::SimulationLiveFallback
    );

    let copyable = plan(CapabilityKind::PublicArtifact, WorldBranchAction::Create);
    let mut stale_policy = current();
    stale_policy.policy_current = false;
    assert_eq!(
        decide_world_branch_activation(&copyable, &observation(&copyable), &stale_policy).diagnostic,
        WorldBranchAuthorityDiagnostic::PolicyStale
    );

    let mut missing_evidence = observation(&copyable);
    missing_evidence.evidence_refs.clear();
    assert_eq!(
        decide_world_branch_activation(&copyable, &missing_evidence, &current()).diagnostic,
        WorldBranchAuthorityDiagnostic::MissingObligationEvidence
    );

    let mut already_active = observation(&copyable);
    already_active.destination_active = true;
    assert_eq!(
        decide_world_branch_activation(&copyable, &already_active, &current()).diagnostic,
        WorldBranchAuthorityDiagnostic::ObservationMismatch
    );

    let mut bearer = observation(&copyable);
    bearer.bearer_material_present = true;
    assert_eq!(
        decide_world_branch_activation(&copyable, &bearer, &current()).diagnostic,
        WorldBranchAuthorityDiagnostic::BearerMaterialPresent
    );

    let mut overclaim = observation(&copyable);
    overclaim.receipt_claims_authority = true;
    assert_eq!(
        decide_world_branch_activation(&copyable, &overclaim, &current()).diagnostic,
        WorldBranchAuthorityDiagnostic::ReceiptAuthorityOverclaim
    );
}
