use std::collections::BTreeSet;

// r[impl molten.world_branch_authority.derivation]
// r[impl molten.world_branch_authority.evidence]

pub const MAXIMUM_SCOPE_ABILITIES: usize = 32;
pub const MAXIMUM_SCOPE_TEXT_BYTES: usize = 512;
pub const MAXIMUM_REALIZATION_EVIDENCE: usize = 16;
pub const MAXIMUM_DIAGNOSTICS: usize = 16;
pub const WORLD_BRANCH_AUTHORITY_PLAN_SCHEMA: &str = "molten.world-branch-authority-plan.v1";
pub const WORLD_BRANCH_AUTHORITY_ACTIVATION_SCHEMA: &str = "molten.world-branch-authority-activation.v1";
pub const BASALT_BRANCH_AUTHORITY_REVISION: &str = "89675cd4f585f837323c049e4a25f7b94c903038";

pub const WORLD_BRANCH_AUTHORITY_NON_CLAIMS: &[&str] = &[
    "policy decisions do not mint or move capabilities",
    "policy and realization receipts are not current authority",
    "world commits and receipts exclude bearer material",
    "linear plans do not prove source deactivation or destination activation",
    "simulation plans do not prove parity or prevent host escape",
    "promotion plans do not authorize effect dispatch",
    "activation observations do not prove future enforcement",
    "branch-authority evidence does not prove release eligibility",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CapabilityKind {
    PublicArtifact,
    ScopedService,
    ExclusiveLease,
    ExternalEffect,
    DeferredEffect,
    HostSecret,
    BearerCredential,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldBranchAction {
    Create,
    Activate,
    Promote,
    Simulate,
    Transfer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldBranchMode {
    Copyable,
    Attenuated,
    Linear,
    SimulationOnly,
    PromotionGated,
    ReplaceBeforeActivation,
    NonBranchable,
}

impl WorldBranchMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Copyable => "copyable",
            Self::Attenuated => "attenuated",
            Self::Linear => "linear",
            Self::SimulationOnly => "simulation-only",
            Self::PromotionGated => "promotion-gated",
            Self::ReplaceBeforeActivation => "replace-before-activation",
            Self::NonBranchable => "non-branchable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldBranchObligation {
    IndependentDestinationGrant,
    NarrowerDerivation,
    GenerationFencedTransfer,
    SourceDeactivation,
    ExactSimulationAdapter,
    PromotionRecheck,
    ReleaseReservation,
    FreshDestinationGrant,
    DenyActivation,
}

impl WorldBranchObligation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::IndependentDestinationGrant => "independent-destination-grant",
            Self::NarrowerDerivation => "narrower-derivation",
            Self::GenerationFencedTransfer => "generation-fenced-transfer",
            Self::SourceDeactivation => "source-deactivation",
            Self::ExactSimulationAdapter => "exact-simulation-adapter",
            Self::PromotionRecheck => "promotion-recheck",
            Self::ReleaseReservation => "release-reservation",
            Self::FreshDestinationGrant => "fresh-destination-grant",
            Self::DenyActivation => "deny-activation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldBranchAuthorityDiagnostic {
    Admitted,
    InvalidInput,
    PolicyMalformed,
    PolicyStale,
    MappingLossy,
    CurrentnessDenied,
    UcanCompositionMissing,
    ScopeWidened,
    AttenuationNotNarrower,
    ActionModeMismatch,
    NonBranchable,
    ObservationMismatch,
    MissingObligationEvidence,
    LinearOwnershipAmbiguous,
    SimulationAdapterMissing,
    SimulationLiveFallback,
    PromotionReservationMissing,
    BearerMaterialPresent,
    ReceiptAuthorityOverclaim,
    ActivationDenied,
    ActivationOutcomeUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedCapabilityScope {
    resource: String,
    abilities: BTreeSet<String>,
    limit: Option<u64>,
}

impl NormalizedCapabilityScope {
    pub fn new(
        resource: impl Into<String>,
        abilities: impl IntoIterator<Item = String>,
        limit: Option<u64>,
    ) -> Result<Self, WorldBranchAuthorityDiagnostic> {
        let resource = resource.into();
        let abilities = abilities.into_iter().collect::<BTreeSet<_>>();
        if !valid_normalized_text(&resource)
            || abilities.is_empty()
            || abilities.len() > MAXIMUM_SCOPE_ABILITIES
            || abilities.iter().any(|ability| !valid_normalized_text(ability))
        {
            return Err(WorldBranchAuthorityDiagnostic::InvalidInput);
        }
        Ok(Self {
            resource,
            abilities,
            limit,
        })
    }

    pub fn resource(&self) -> &str {
        &self.resource
    }

    pub const fn abilities(&self) -> &BTreeSet<String> {
        &self.abilities
    }

    pub const fn limit(&self) -> Option<u64> {
        self.limit
    }

    pub fn is_within(&self, source: &Self) -> bool {
        self.resource.starts_with(&source.resource)
            && self.abilities.is_subset(&source.abilities)
            && match (self.limit, source.limit) {
                (Some(destination), Some(origin)) => destination <= origin,
                (None, Some(_)) => false,
                _ => true,
            }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBranchAuthorityFacts {
    pub capability_kind: CapabilityKind,
    pub action: WorldBranchAction,
    pub source_branch_ref: String,
    pub destination_branch_ref: String,
    pub capability_ref: String,
    pub source_scope: NormalizedCapabilityScope,
    pub destination_scope: NormalizedCapabilityScope,
    pub policy_generation: u64,
    pub mapping_lossless: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentAuthorityFacts {
    pub observation_ref: String,
    pub policy_current: bool,
    pub capability_current: bool,
    pub revocation_current: bool,
    pub replay_current: bool,
    pub scope_current: bool,
    pub ucan_verified: bool,
}

impl CurrentAuthorityFacts {
    pub const fn all_current(&self) -> bool {
        self.policy_current
            && self.capability_current
            && self.revocation_current
            && self.replay_current
            && self.scope_current
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBranchAuthorityPlan {
    pub schema: &'static str,
    pub plan_ref: String,
    pub allowed: bool,
    pub policy_ref: String,
    pub request_ref: String,
    pub authority_input_ref: String,
    pub capability_ref: String,
    pub source_branch_ref: String,
    pub destination_branch_ref: String,
    pub source_scope: NormalizedCapabilityScope,
    pub destination_scope: NormalizedCapabilityScope,
    pub mode: Option<WorldBranchMode>,
    pub obligations: Vec<WorldBranchObligation>,
    pub diagnostic: WorldBranchAuthorityDiagnostic,
    pub non_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBranchRealizationObservation {
    pub plan_ref: String,
    pub policy_ref: String,
    pub capability_ref: String,
    pub operation_ref: String,
    pub evidence_refs: Vec<String>,
    pub destination_scope: NormalizedCapabilityScope,
    pub destination_grant_current: bool,
    pub source_active: bool,
    pub destination_active: bool,
    pub transfer_generation: Option<u64>,
    pub simulation_adapter_ref: Option<String>,
    pub simulation_adapter_deterministic: bool,
    pub release_reservation_ref: Option<String>,
    pub bearer_material_present: bool,
    pub receipt_claims_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBranchActivationDecision {
    pub schema: &'static str,
    pub decision_ref: String,
    pub allowed: bool,
    pub plan_ref: String,
    pub observation_ref: String,
    pub diagnostic: WorldBranchAuthorityDiagnostic,
    pub non_claims: Vec<String>,
}

pub fn valid_content_ref(value: &str) -> bool {
    const PREFIX: &str = "blake3:";
    const HEX_LENGTH: usize = 64;
    value.strip_prefix(PREFIX).is_some_and(|hex| {
        hex.len() == HEX_LENGTH && hex.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn valid_normalized_text(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAXIMUM_SCOPE_TEXT_BYTES
        && !value.chars().any(char::is_control)
        && !value.contains("token")
        && !value.contains("secret=")
}
