use std::collections::BTreeSet;

use super::CaptureIssue;
use super::ClosureIssue;
use super::CompletenessClaim;
use super::MAX_WORLD_COMMIT_CANONICAL_BYTES;
use super::MAX_WORLD_COMMIT_CLOSURE_OBJECTS;
use super::MAX_WORLD_COMMIT_PARENTS;
use super::MAX_WORLD_COMMIT_REVISION_FENCES;
use super::MAX_WORLD_COMMIT_ROOTS;
use super::RootKind;
use super::RootReplayClass;
use super::SnapshotProfile;
use super::SnapshotProfileKind;
use super::WorldCommitBounds;
use super::WorldCommitCore;
use super::WorldCommitRef;
use super::WorldRootRef;

// r[impl molten.world_commit.core]
// r[impl molten.world_commit.typed_roots]
pub fn validate_and_normalize_core(
    core: &WorldCommitCore,
    bounds: &WorldCommitBounds,
) -> Result<WorldCommitCore, Vec<CaptureIssue>> {
    let mut issues = validate_bounds(bounds);
    validate_collection_bounds(core, bounds, &mut issues);
    if !issues.is_empty() {
        issues.sort();
        issues.dedup();
        return Err(issues);
    }
    validate_profile(&core.profile, &mut issues);
    validate_parents(core, &mut issues);
    validate_roots(core, &mut issues);
    validate_completeness(core, &mut issues);
    if !issues.is_empty() {
        issues.sort();
        issues.dedup();
        return Err(issues);
    }
    let mut normalized = core.clone();
    normalized.parents.sort();
    normalized
        .roots
        .sort_by(|left, right| left.kind().cmp(&right.kind()).then_with(|| left.as_str().cmp(right.as_str())));
    normalized.completeness.required_roots.sort();
    Ok(normalized)
}

pub fn validate_bounds(bounds: &WorldCommitBounds) -> Vec<CaptureIssue> {
    let mut issues = Vec::new();
    validate_bound("max-parents", bounds.max_parents, MAX_WORLD_COMMIT_PARENTS, &mut issues);
    validate_bound("max-roots", bounds.max_roots, MAX_WORLD_COMMIT_ROOTS, &mut issues);
    validate_bound("max-revision-fences", bounds.max_revision_fences, MAX_WORLD_COMMIT_REVISION_FENCES, &mut issues);
    validate_bound("max-closure-objects", bounds.max_closure_objects, MAX_WORLD_COMMIT_CLOSURE_OBJECTS, &mut issues);
    issues
}

fn validate_bound(field: &'static str, value: usize, hard_limit: usize, issues: &mut Vec<CaptureIssue>) {
    if value == 0 {
        issues.push(CaptureIssue::ZeroBound(field));
    } else if value > hard_limit {
        issues.push(CaptureIssue::BoundAboveHardLimit {
            field,
            actual: value,
            maximum: hard_limit,
        });
    }
}

fn validate_profile(profile: &SnapshotProfile, issues: &mut Vec<CaptureIssue>) {
    match (profile.kind, profile.cohort_ref.is_some()) {
        (SnapshotProfileKind::Logical, true) => issues.push(CaptureIssue::LogicalProfileHasCohort),
        (SnapshotProfileKind::Opaque | SnapshotProfileKind::Mixed, false) => {
            issues.push(CaptureIssue::OpaqueProfileMissingCohort);
        }
        _ => {}
    }
}

fn validate_collection_bounds(core: &WorldCommitCore, bounds: &WorldCommitBounds, issues: &mut Vec<CaptureIssue>) {
    if core.parents.len() > bounds.max_parents {
        issues.push(CaptureIssue::BoundExceeded {
            field: "parents",
            actual: core.parents.len(),
            maximum: bounds.max_parents,
        });
    }
    if core.roots.len() > bounds.max_roots {
        issues.push(CaptureIssue::BoundExceeded {
            field: "roots",
            actual: core.roots.len(),
            maximum: bounds.max_roots,
        });
    }
}

fn validate_parents(core: &WorldCommitCore, issues: &mut Vec<CaptureIssue>) {
    let mut seen = BTreeSet::new();
    for parent in &core.parents {
        if !seen.insert(parent.as_str()) {
            issues.push(CaptureIssue::DuplicateParent(parent.as_str().to_string()));
        }
    }
}

fn validate_roots(core: &WorldCommitCore, issues: &mut Vec<CaptureIssue>) {
    let required = core.profile.kind.required_roots();
    let mut seen_kinds = BTreeSet::new();
    let mut seen_refs = BTreeSet::new();
    for root in &core.roots {
        let kind = root.kind();
        if !seen_kinds.insert(kind) {
            issues.push(CaptureIssue::DuplicateRootKind(kind));
        }
        if !seen_refs.insert(root.as_str()) {
            issues.push(CaptureIssue::DuplicateRootRef(root.as_str().to_string()));
        }
        if !required.contains(&kind) && kind != RootKind::AuthorityObservation {
            issues.push(CaptureIssue::UnexpectedRoot(kind));
        }
    }
    for kind in required {
        if !seen_kinds.contains(kind) {
            issues.push(CaptureIssue::MissingRequiredRoot(*kind));
        }
    }
}

fn validate_completeness(core: &WorldCommitCore, issues: &mut Vec<CaptureIssue>) {
    let mut expected = CompletenessClaim::for_profile(core.profile.kind).required_roots;
    expected.sort();
    let mut actual = core.completeness.required_roots.clone();
    actual.sort();
    if actual != expected {
        issues.push(CaptureIssue::CompletenessMismatch);
    }
}

pub fn root_for_kind(roots: &[WorldRootRef], kind: RootKind) -> Option<&WorldRootRef> {
    roots.iter().find(|root| root.kind() == kind)
}

pub const fn replay_class(kind: RootKind) -> RootReplayClass {
    match kind {
        RootKind::Artifact | RootKind::Schema | RootKind::RuntimeProfile | RootKind::Policy => {
            RootReplayClass::VerifyOnly
        }
        RootKind::AuthorityObservation => RootReplayClass::HistoricalEvidenceOnly,
        RootKind::OpaqueMachineSnapshot => RootReplayClass::RestoreOpaqueState,
        RootKind::DurableState
        | RootKind::Tasks
        | RootKind::History
        | RootKind::Effects
        | RootKind::Scheduler
        | RootKind::Time
        | RootKind::Entropy => RootReplayClass::ReplayLogicalState,
    }
}

pub(crate) fn validate_parent_edges(
    commit_ref: &WorldCommitRef,
    parents: &[WorldCommitRef],
    bounds: &WorldCommitBounds,
    issues: &mut Vec<ClosureIssue>,
) {
    if parents.len() > bounds.max_parents {
        issues.push(ClosureIssue::ParentEdgeBoundExceeded {
            commit_ref: commit_ref.as_str().to_string(),
            actual: parents.len(),
            maximum: bounds.max_parents,
        });
    }
    let mut seen = BTreeSet::new();
    for parent in parents {
        if !seen.insert(parent) {
            issues.push(ClosureIssue::DuplicateParentEdge {
                commit_ref: commit_ref.as_str().to_string(),
                parent_ref: parent.as_str().to_string(),
            });
        }
    }
}

pub(crate) fn protocol_bounds() -> WorldCommitBounds {
    WorldCommitBounds {
        max_parents: MAX_WORLD_COMMIT_PARENTS,
        max_roots: MAX_WORLD_COMMIT_ROOTS,
        max_revision_fences: MAX_WORLD_COMMIT_REVISION_FENCES,
        max_closure_objects: MAX_WORLD_COMMIT_CLOSURE_OBJECTS,
    }
}

const WORLD_COMMIT_HASH_CONTEXT: &str = "onixresearch.molten.world-commit.identity.v1";
const WORLD_COMMIT_FRAME_VERSION: &[u8] = b"molten-world-commit-frame-v1";
const FRAME_SEPARATOR: &[u8] = &[0];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldCommitIdentityIssue {
    EmptyCanonicalBytes,
    CanonicalBytesExceeded { actual: usize, maximum: usize },
    ByteLengthOverflow,
    InvalidDerivedReference,
}

// r[impl molten.world_commit.core]
pub fn identify_world_commit(canonical_preserves_bytes: &[u8]) -> Result<WorldCommitRef, WorldCommitIdentityIssue> {
    if canonical_preserves_bytes.is_empty() {
        return Err(WorldCommitIdentityIssue::EmptyCanonicalBytes);
    }
    if canonical_preserves_bytes.len() > MAX_WORLD_COMMIT_CANONICAL_BYTES {
        return Err(WorldCommitIdentityIssue::CanonicalBytesExceeded {
            actual: canonical_preserves_bytes.len(),
            maximum: MAX_WORLD_COMMIT_CANONICAL_BYTES,
        });
    }
    let byte_length =
        u64::try_from(canonical_preserves_bytes.len()).map_err(|_| WorldCommitIdentityIssue::ByteLengthOverflow)?;
    let mut hasher = blake3::Hasher::new_derive_key(WORLD_COMMIT_HASH_CONTEXT);
    hasher.update(WORLD_COMMIT_FRAME_VERSION);
    hasher.update(FRAME_SEPARATOR);
    hasher.update(&byte_length.to_be_bytes());
    hasher.update(canonical_preserves_bytes);
    WorldCommitRef::new(format!("blake3:{}", hasher.finalize().to_hex()))
        .map_err(|_| WorldCommitIdentityIssue::InvalidDerivedReference)
}
