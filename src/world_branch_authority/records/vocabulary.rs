use molten_core::world_branch_authority::WorldBranchAuthorityDiagnostic;

pub(super) fn valid_obligation(value: &str) -> bool {
    matches!(
        value,
        "independent-destination-grant"
            | "narrower-derivation"
            | "generation-fenced-transfer"
            | "source-deactivation"
            | "exact-simulation-adapter"
            | "promotion-recheck"
            | "release-reservation"
            | "fresh-destination-grant"
            | "deny-activation"
    )
}

pub(super) fn valid_mode(value: &str) -> bool {
    matches!(
        value,
        "copyable"
            | "attenuated"
            | "linear"
            | "simulation-only"
            | "promotion-gated"
            | "replace-before-activation"
            | "non-branchable"
    )
}

pub(super) fn valid_diagnostic(value: &str) -> bool {
    matches!(
        value,
        "admitted"
            | "invalid-input"
            | "policy-malformed"
            | "policy-stale"
            | "mapping-lossy"
            | "currentness-denied"
            | "ucan-composition-missing"
            | "scope-widened"
            | "attenuation-not-narrower"
            | "action-mode-mismatch"
            | "non-branchable"
            | "observation-mismatch"
            | "missing-obligation-evidence"
            | "linear-ownership-ambiguous"
            | "simulation-adapter-missing"
            | "simulation-live-fallback"
            | "promotion-reservation-missing"
            | "bearer-material-present"
            | "receipt-authority-overclaim"
            | "activation-denied"
            | "activation-outcome-unknown"
    )
}

pub(super) fn valid_activation_outcome(value: &str) -> bool {
    matches!(value, "activated" | "denied" | "unknown")
}

pub(super) const fn diagnostic_text(diagnostic: WorldBranchAuthorityDiagnostic) -> &'static str {
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
