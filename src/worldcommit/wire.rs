use molten_core::world_commit::CaptureIssue;
use molten_core::world_commit::CapturePlan;
use molten_core::world_commit::WorldCommitBounds;
use molten_core::world_commit::WorldCommitCore;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_commit::WorldRootRef;
use preserves::IOValue;

use crate::error::MoltenError;
use crate::error::Result;

pub const WORLD_COMMIT_RECORD: &str = "molten-world-commit-v1";
pub const WORLD_COMMIT_CAPTURE_RECEIPT_RECORD: &str = "molten-world-commit-capture-receipt-v1";
pub const WORLD_COMMIT_CLOSURE_REPORT_RECORD: &str = "molten-world-commit-closure-report-v1";
pub const WORLD_COMMIT_RESTORE_PLAN_RECORD: &str = "molten-world-commit-restore-plan-v1";

const WORLD_COMMIT_NON_CLAIM_COUNT: usize = 7;
const MAX_CAPTURE_RECEIPT_ISSUES: usize = 64;
const MAX_CAPTURE_RECEIPT_ISSUE_BYTES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldCommitNonClaim {
    IntegrityIsNotAuthority,
    CoherentCutIsNotCrossStoreAtomicity,
    ClosureIsNotRestorability,
    RestorePlanIsNotAdmission,
    HistoricalAuthorityIsNotCurrent,
    OpaqueStateIsNotSemanticMerge,
    ExternalRealmCompatibilityIsNotClaimed,
}

impl WorldCommitNonClaim {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::IntegrityIsNotAuthority => "world-commit-integrity-is-not-authority",
            Self::CoherentCutIsNotCrossStoreAtomicity => "fenced-coherent-cut-is-not-cross-store-atomicity",
            Self::ClosureIsNotRestorability => "root-closure-is-not-restorability",
            Self::RestorePlanIsNotAdmission => "restore-plan-is-not-current-runtime-admission",
            Self::HistoricalAuthorityIsNotCurrent => "historical-authority-observation-is-not-current-authority",
            Self::OpaqueStateIsNotSemanticMerge => "opaque-state-is-not-logical-state-or-semantic-merge",
            Self::ExternalRealmCompatibilityIsNotClaimed => "no-external-realmcommit-compatibility-claim",
        }
    }
}

pub const WORLD_COMMIT_NON_CLAIMS: [WorldCommitNonClaim; WORLD_COMMIT_NON_CLAIM_COUNT] = [
    WorldCommitNonClaim::IntegrityIsNotAuthority,
    WorldCommitNonClaim::CoherentCutIsNotCrossStoreAtomicity,
    WorldCommitNonClaim::ClosureIsNotRestorability,
    WorldCommitNonClaim::RestorePlanIsNotAdmission,
    WorldCommitNonClaim::HistoricalAuthorityIsNotCurrent,
    WorldCommitNonClaim::OpaqueStateIsNotSemanticMerge,
    WorldCommitNonClaim::ExternalRealmCompatibilityIsNotClaimed,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalWorldCommit {
    pub core: WorldCommitCore,
    pub commit_ref: WorldCommitRef,
    pub value: IOValue,
    pub bytes: Vec<u8>,
}

// r[impl molten.world_commit.core]
pub fn canonical_world_commit(core: &WorldCommitCore, bounds: &WorldCommitBounds) -> Result<CanonicalWorldCommit> {
    let normalized =
        molten_core::world_commit::validate_and_normalize_core(core, bounds).map_err(core_validation_error)?;
    let value = world_commit_value(&normalized);
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    let commit_ref = molten_core::world_commit::identify_world_commit(&bytes)
        .map_err(|issue| MoltenError::invalid_harness(format!("world commit identity denied: {issue:?}")))?;
    Ok(CanonicalWorldCommit {
        core: normalized,
        commit_ref,
        value,
        bytes,
    })
}

pub fn world_commit_value(core: &WorldCommitCore) -> IOValue {
    crate::preserves_rail::record(WORLD_COMMIT_RECORD, vec![
        crate::preserves_rail::string(molten_core::world_commit::WORLD_COMMIT_SCHEMA),
        crate::preserves_rail::record("version", vec![crate::preserves_rail::string(core.version.as_str())]),
        crate::preserves_rail::record("profile", vec![
            crate::preserves_rail::string(core.profile.kind.as_str()),
            crate::preserves_rail::string(core.profile.profile_ref.as_str()),
            crate::preserves_rail::optional_ref_value(
                core.profile.cohort_ref.as_ref().map(|reference| reference.as_str()),
            ),
        ]),
        crate::preserves_rail::record("parents", vec![crate::preserves_rail::sequence(
            core.parents.iter().map(|parent| crate::preserves_rail::string(parent.as_str())).collect(),
        )]),
        crate::preserves_rail::record("roots", vec![crate::preserves_rail::sequence(
            core.roots.iter().map(root_value).collect(),
        )]),
        crate::preserves_rail::record("completeness", vec![crate::preserves_rail::sequence(
            core.completeness
                .required_roots
                .iter()
                .map(|kind| crate::preserves_rail::string(kind.as_str()))
                .collect(),
        )]),
    ])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureDecision {
    Published,
    Denied,
}

impl CaptureDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Published => "published",
            Self::Denied => "denied",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicationOutcome {
    Published,
    AlreadyPresent,
    NotAttempted,
    Uncertain,
}

impl PublicationOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Published => "published",
            Self::AlreadyPresent => "already-present",
            Self::NotAttempted => "not-attempted",
            Self::Uncertain => "uncertain",
        }
    }

    pub const fn is_success(self) -> bool {
        matches!(self, Self::Published | Self::AlreadyPresent)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureReceipt {
    pub decision: CaptureDecision,
    pub commit_ref: Option<WorldCommitRef>,
    pub profile_ref: String,
    pub persisted_roots: Vec<WorldRootRef>,
    pub revision_fences: Vec<molten_core::world_commit::RevisionFence>,
    pub issues: Vec<String>,
    pub publication: PublicationOutcome,
    pub non_claims: Vec<WorldCommitNonClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalCaptureReceipt {
    pub receipt: CaptureReceipt,
    pub receipt_ref: String,
    pub value: IOValue,
    pub bytes: Vec<u8>,
}

// r[impl molten.world_commit.capture]
pub fn canonical_capture_receipt(receipt: &CaptureReceipt) -> Result<CanonicalCaptureReceipt> {
    validate_capture_receipt(receipt)?;
    let value = capture_receipt_value(receipt);
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    let receipt_ref = crate::preserves_rail::content_ref_from_bytes(&bytes);
    Ok(CanonicalCaptureReceipt {
        receipt: receipt.clone(),
        receipt_ref,
        value,
        bytes,
    })
}

pub fn denied_capture_receipt(
    profile_ref: &str,
    plan: Option<&CapturePlan>,
    publication: PublicationOutcome,
    issues: impl IntoIterator<Item = String>,
) -> Result<CanonicalCaptureReceipt> {
    canonical_capture_receipt(&CaptureReceipt {
        decision: CaptureDecision::Denied,
        commit_ref: None,
        profile_ref: profile_ref.to_string(),
        persisted_roots: Vec::new(),
        revision_fences: plan.map_or_else(Vec::new, |plan| plan.revision_fences.clone()),
        issues: issues.into_iter().collect(),
        publication,
        non_claims: WORLD_COMMIT_NON_CLAIMS.to_vec(),
    })
}

fn validate_capture_receipt(receipt: &CaptureReceipt) -> Result<()> {
    crate::preserves_rail::validate_content_ref(&receipt.profile_ref)?;
    if receipt.persisted_roots.len() > molten_core::world_commit::MAX_WORLD_COMMIT_ROOTS {
        return Err(MoltenError::invalid_harness("world commit capture receipt root count exceeds bound"));
    }
    if receipt.revision_fences.len() > molten_core::world_commit::MAX_WORLD_COMMIT_REVISION_FENCES {
        return Err(MoltenError::invalid_harness("world commit capture receipt revision-fence count exceeds bound"));
    }
    if receipt.issues.len() > MAX_CAPTURE_RECEIPT_ISSUES {
        return Err(MoltenError::invalid_harness("world commit capture receipt issue count exceeds bound"));
    }
    if receipt
        .issues
        .iter()
        .any(|issue| issue.len() > MAX_CAPTURE_RECEIPT_ISSUE_BYTES || issue.chars().any(char::is_control))
    {
        return Err(MoltenError::invalid_harness(
            "world commit capture receipt issue is oversized or contains control text",
        ));
    }
    let has_commit = receipt.commit_ref.is_some();
    let is_success = receipt.decision == CaptureDecision::Published && receipt.publication.is_success();
    if is_success != has_commit {
        return Err(MoltenError::invalid_harness("world commit capture receipt success and commit identity disagree"));
    }
    if receipt.decision == CaptureDecision::Published && !receipt.issues.is_empty() {
        return Err(MoltenError::invalid_harness("world commit capture receipt cannot publish with denial issues"));
    }
    if receipt.non_claims != WORLD_COMMIT_NON_CLAIMS {
        return Err(MoltenError::invalid_harness("world commit capture receipt non-claims are incomplete"));
    }
    Ok(())
}

fn capture_receipt_value(receipt: &CaptureReceipt) -> IOValue {
    crate::preserves_rail::record(WORLD_COMMIT_CAPTURE_RECEIPT_RECORD, vec![
        crate::preserves_rail::string(molten_core::world_commit::WORLD_COMMIT_CAPTURE_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(receipt.decision.as_str())]),
        crate::preserves_rail::record("commit-ref", vec![crate::preserves_rail::optional_ref_value(
            receipt.commit_ref.as_ref().map(WorldCommitRef::as_str),
        )]),
        crate::preserves_rail::record("profile-ref", vec![crate::preserves_rail::string(&receipt.profile_ref)]),
        crate::preserves_rail::record("persisted-roots", vec![crate::preserves_rail::sequence(
            receipt.persisted_roots.iter().map(root_value).collect(),
        )]),
        crate::preserves_rail::record("revision-fences", vec![crate::preserves_rail::sequence(
            receipt
                .revision_fences
                .iter()
                .map(|fence| {
                    crate::preserves_rail::record("revision-fence", vec![
                        crate::preserves_rail::string(fence.root_kind.as_str()),
                        crate::preserves_rail::string(&fence.source_id),
                        crate::preserves_rail::u64_value(fence.observed_revision),
                    ])
                })
                .collect(),
        )]),
        crate::preserves_rail::record("publication", vec![crate::preserves_rail::string(receipt.publication.as_str())]),
        crate::preserves_rail::record("issues", vec![crate::preserves_rail::sequence(
            receipt.issues.iter().map(crate::preserves_rail::string).collect(),
        )]),
        non_claims_value(),
    ])
}

pub(crate) fn root_value(root: &WorldRootRef) -> IOValue {
    crate::preserves_rail::record("typed-root", vec![
        crate::preserves_rail::string(root.kind().as_str()),
        crate::preserves_rail::string(root.as_str()),
    ])
}

pub(crate) fn non_claims_value() -> IOValue {
    crate::preserves_rail::record("non-claims", vec![crate::preserves_rail::sequence(
        WORLD_COMMIT_NON_CLAIMS
            .iter()
            .map(|non_claim| crate::preserves_rail::string(non_claim.as_str()))
            .collect(),
    )])
}

fn core_validation_error(issues: Vec<CaptureIssue>) -> MoltenError {
    MoltenError::invalid_harness(format!("world commit core validation denied: {issues:?}"))
}
