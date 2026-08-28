use basalt::world_branch_authority;

use super::super::model::*;

pub(super) fn basalt_scope(
    scope: &NormalizedCapabilityScope,
) -> Result<world_branch_authority::CapabilityScope, WorldBranchAuthorityDiagnostic> {
    world_branch_authority::CapabilityScope::new(
        scope.resource().to_string(),
        scope.abilities().iter().cloned(),
        scope.limit(),
    )
    .map_err(|_| WorldBranchAuthorityDiagnostic::InvalidInput)
}

pub(super) const fn basalt_capability_kind(kind: CapabilityKind) -> world_branch_authority::CapabilityClass {
    match kind {
        CapabilityKind::PublicArtifact => world_branch_authority::CapabilityClass::PublicArtifact,
        CapabilityKind::ScopedService => world_branch_authority::CapabilityClass::ScopedService,
        CapabilityKind::ExclusiveLease => world_branch_authority::CapabilityClass::ExclusiveLease,
        CapabilityKind::ExternalEffect => world_branch_authority::CapabilityClass::ExternalEffect,
        CapabilityKind::DeferredEffect => world_branch_authority::CapabilityClass::DeferredEffect,
        CapabilityKind::HostSecret => world_branch_authority::CapabilityClass::HostSecret,
        CapabilityKind::BearerCredential => world_branch_authority::CapabilityClass::BearerCredential,
    }
}

pub(super) const fn basalt_action(action: WorldBranchAction) -> world_branch_authority::BranchAction {
    match action {
        WorldBranchAction::Create => world_branch_authority::BranchAction::Create,
        WorldBranchAction::Activate => world_branch_authority::BranchAction::Activate,
        WorldBranchAction::Promote => world_branch_authority::BranchAction::Promote,
        WorldBranchAction::Simulate => world_branch_authority::BranchAction::Simulate,
        WorldBranchAction::Transfer => world_branch_authority::BranchAction::Transfer,
    }
}

pub(super) const fn current_fact(current: bool) -> world_branch_authority::CurrentFact {
    if current {
        world_branch_authority::CurrentFact::Current
    } else {
        world_branch_authority::CurrentFact::Stale
    }
}

pub(super) const fn world_mode(mode: world_branch_authority::BranchMode) -> WorldBranchMode {
    match mode {
        world_branch_authority::BranchMode::Copyable => WorldBranchMode::Copyable,
        world_branch_authority::BranchMode::Attenuated => WorldBranchMode::Attenuated,
        world_branch_authority::BranchMode::Linear => WorldBranchMode::Linear,
        world_branch_authority::BranchMode::SimulationOnly => WorldBranchMode::SimulationOnly,
        world_branch_authority::BranchMode::PromotionGated => WorldBranchMode::PromotionGated,
        world_branch_authority::BranchMode::ReplaceBeforeActivation => WorldBranchMode::ReplaceBeforeActivation,
        world_branch_authority::BranchMode::NonBranchable => WorldBranchMode::NonBranchable,
    }
}

pub(super) const fn world_obligation(obligation: world_branch_authority::BranchObligation) -> WorldBranchObligation {
    match obligation {
        world_branch_authority::BranchObligation::IndependentDestinationGrant => {
            WorldBranchObligation::IndependentDestinationGrant
        }
        world_branch_authority::BranchObligation::NarrowerDerivation => WorldBranchObligation::NarrowerDerivation,
        world_branch_authority::BranchObligation::GenerationFencedTransfer => {
            WorldBranchObligation::GenerationFencedTransfer
        }
        world_branch_authority::BranchObligation::SourceDeactivation => WorldBranchObligation::SourceDeactivation,
        world_branch_authority::BranchObligation::ExactSimulationAdapter => {
            WorldBranchObligation::ExactSimulationAdapter
        }
        world_branch_authority::BranchObligation::PromotionRecheck => WorldBranchObligation::PromotionRecheck,
        world_branch_authority::BranchObligation::ReleaseReservation => WorldBranchObligation::ReleaseReservation,
        world_branch_authority::BranchObligation::FreshDestinationGrant => WorldBranchObligation::FreshDestinationGrant,
        world_branch_authority::BranchObligation::DenyActivation => WorldBranchObligation::DenyActivation,
    }
}

pub(super) const fn world_diagnostic(
    diagnostic: world_branch_authority::BranchAuthorityDiagnostic,
) -> WorldBranchAuthorityDiagnostic {
    match diagnostic {
        world_branch_authority::BranchAuthorityDiagnostic::Admitted => WorldBranchAuthorityDiagnostic::Admitted,
        world_branch_authority::BranchAuthorityDiagnostic::PolicyMalformed
        | world_branch_authority::BranchAuthorityDiagnostic::UnknownCapabilityClass
        | world_branch_authority::BranchAuthorityDiagnostic::AmbiguousRules => {
            WorldBranchAuthorityDiagnostic::PolicyMalformed
        }
        world_branch_authority::BranchAuthorityDiagnostic::PolicyStale => WorldBranchAuthorityDiagnostic::PolicyStale,
        world_branch_authority::BranchAuthorityDiagnostic::MappingLossy => WorldBranchAuthorityDiagnostic::MappingLossy,
        world_branch_authority::BranchAuthorityDiagnostic::CurrentnessDenied => {
            WorldBranchAuthorityDiagnostic::CurrentnessDenied
        }
        world_branch_authority::BranchAuthorityDiagnostic::UcanCompositionMissing => {
            WorldBranchAuthorityDiagnostic::UcanCompositionMissing
        }
        world_branch_authority::BranchAuthorityDiagnostic::ScopeWidened => WorldBranchAuthorityDiagnostic::ScopeWidened,
        world_branch_authority::BranchAuthorityDiagnostic::AttenuationNotNarrower => {
            WorldBranchAuthorityDiagnostic::AttenuationNotNarrower
        }
        world_branch_authority::BranchAuthorityDiagnostic::ActionModeMismatch => {
            WorldBranchAuthorityDiagnostic::ActionModeMismatch
        }
        world_branch_authority::BranchAuthorityDiagnostic::NonBranchable => {
            WorldBranchAuthorityDiagnostic::NonBranchable
        }
    }
}
