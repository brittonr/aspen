use super::WorldHeadConflictSet;
use super::WorldHeadTransitionPlan;

/// The closed set of world-head bounds an `InvalidBounds` issue can name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldHeadBound {
    MaxHistoryNodes,
    MaxParentsPerCommit,
    MaxSigners,
    MaxConflicts,
    SignatureThreshold,
    PolicyMaxConflicts,
    ChoregraphLimits,
    MaxMetadataEntries,
    MaxLabelBytes,
    ConflictIdentityLength,
}

impl WorldHeadBound {
    /// The stable kebab-case receipt code for this bound.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MaxHistoryNodes => "max-history-nodes",
            Self::MaxParentsPerCommit => "max-parents-per-commit",
            Self::MaxSigners => "max-signers",
            Self::MaxConflicts => "max-conflicts",
            Self::SignatureThreshold => "signature-threshold",
            Self::PolicyMaxConflicts => "policy-max-conflicts",
            Self::ChoregraphLimits => "choregraph-limits",
            Self::MaxMetadataEntries => "max-metadata-entries",
            Self::MaxLabelBytes => "max-label-bytes",
            Self::ConflictIdentityLength => "conflict-identity-length",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldHeadIssue {
    InvalidBounds(WorldHeadBound),
    PolicyMismatch,
    BranchMismatch,
    BranchClassMismatch,
    PurposeDenied,
    RecoveryDenied,
    CurrentHeadRequired,
    CurrentHeadUnexpected,
    ExpectedHeadMissing,
    StaleExpectedHead,
    OldGeneration,
    RepeatedGeneration,
    SkippedGeneration,
    GenerationOverflow,
    DurableGenerationUnavailable,
    IndependentCurrentnessRequired,
    AuthorityDenied,
    AuthorityPolicyMismatch,
    AuthorityGenerationMismatch,
    AuthenticationDenied,
    AuthenticationPurposeMismatch,
    AuthenticationPolicyMismatch,
    SignerLimitExceeded,
    SignerThresholdMiss,
    DuplicateSigner,
    UnknownSignerRole,
    SignerNotCurrent,
    SignerRevoked,
    SignerAuthorityDenied,
    HistoryLimitExceeded,
    HistoryEmpty,
    DuplicateHistoryNode,
    ParentLimitExceeded,
    MissingHistoryParent,
    HistoryCycle,
    SuccessorMissing,
    ExpectedHistoryHeadMissing,
    UnrelatedSuccessor,
    MergeSourceMissing,
    DuplicateMergeSource,
    MergeNeedsMultipleSources,
    ChoregraphDenied,
    ConflictLimitExceeded,
    ConflictStateMismatch,
}

impl WorldHeadIssue {
    /// The stable receipt code for this issue: a closed kebab-case tag, followed by the bounded
    /// field name for `InvalidBounds`. Receipt identity never derives from Rust `Debug` output.
    // r[impl molten.runtime_spine.canonical_preserves_identity_and_embedded_scan]
    pub fn code(&self) -> String {
        let tag = match self {
            Self::InvalidBounds(bound) => return format!("invalid-bounds:{}", bound.as_str()),
            Self::PolicyMismatch => "policy-mismatch",
            Self::BranchMismatch => "branch-mismatch",
            Self::BranchClassMismatch => "branch-class-mismatch",
            Self::PurposeDenied => "purpose-denied",
            Self::RecoveryDenied => "recovery-denied",
            Self::CurrentHeadRequired => "current-head-required",
            Self::CurrentHeadUnexpected => "current-head-unexpected",
            Self::ExpectedHeadMissing => "expected-head-missing",
            Self::StaleExpectedHead => "stale-expected-head",
            Self::OldGeneration => "old-generation",
            Self::RepeatedGeneration => "repeated-generation",
            Self::SkippedGeneration => "skipped-generation",
            Self::GenerationOverflow => "generation-overflow",
            Self::DurableGenerationUnavailable => "durable-generation-unavailable",
            Self::IndependentCurrentnessRequired => "independent-currentness-required",
            Self::AuthorityDenied => "authority-denied",
            Self::AuthorityPolicyMismatch => "authority-policy-mismatch",
            Self::AuthorityGenerationMismatch => "authority-generation-mismatch",
            Self::AuthenticationDenied => "authentication-denied",
            Self::AuthenticationPurposeMismatch => "authentication-purpose-mismatch",
            Self::AuthenticationPolicyMismatch => "authentication-policy-mismatch",
            Self::SignerLimitExceeded => "signer-limit-exceeded",
            Self::SignerThresholdMiss => "signer-threshold-miss",
            Self::DuplicateSigner => "duplicate-signer",
            Self::UnknownSignerRole => "unknown-signer-role",
            Self::SignerNotCurrent => "signer-not-current",
            Self::SignerRevoked => "signer-revoked",
            Self::SignerAuthorityDenied => "signer-authority-denied",
            Self::HistoryLimitExceeded => "history-limit-exceeded",
            Self::HistoryEmpty => "history-empty",
            Self::DuplicateHistoryNode => "duplicate-history-node",
            Self::ParentLimitExceeded => "parent-limit-exceeded",
            Self::MissingHistoryParent => "missing-history-parent",
            Self::HistoryCycle => "history-cycle",
            Self::SuccessorMissing => "successor-missing",
            Self::ExpectedHistoryHeadMissing => "expected-history-head-missing",
            Self::UnrelatedSuccessor => "unrelated-successor",
            Self::MergeSourceMissing => "merge-source-missing",
            Self::DuplicateMergeSource => "duplicate-merge-source",
            Self::MergeNeedsMultipleSources => "merge-needs-multiple-sources",
            Self::ChoregraphDenied => "choregraph-denied",
            Self::ConflictLimitExceeded => "conflict-limit-exceeded",
            Self::ConflictStateMismatch => "conflict-state-mismatch",
        };
        tag.to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldHeadDecision {
    Admitted(WorldHeadTransitionPlan),
    Denied(Vec<WorldHeadIssue>),
    Conflict(WorldHeadConflictSet),
}
