use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::CompletenessClaim;
use super::RootKind;
use super::SnapshotProfile;
use super::WorldCommitBounds;
use super::WorldCommitCore;
use super::WorldCommitRef;
use super::WorldCommitReferenceError;
use super::WorldCommitVersion;
use super::WorldRootRef;
use super::validate_and_normalize_core;
use super::validate_bounds;
use super::validate_source_id;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RevisionFence {
    pub root_kind: RootKind,
    pub source_id: String,
    pub observed_revision: u64,
}

impl RevisionFence {
    pub fn new(
        root_kind: RootKind,
        source_id: impl Into<String>,
        observed_revision: u64,
    ) -> Result<Self, WorldCommitReferenceError> {
        let source_id = source_id.into();
        validate_source_id(&source_id)?;
        Ok(Self {
            root_kind,
            source_id,
            observed_revision,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationStability {
    Immutable,
    Mutable(RevisionFence),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootObservation {
    pub root: WorldRootRef,
    pub source_kind: RootKind,
    pub schema_validated: bool,
    pub stability: ObservationStability,
    pub durable: bool,
    pub inventory_complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureRequest {
    pub version: WorldCommitVersion,
    pub profile: SnapshotProfile,
    pub parents: Vec<WorldCommitRef>,
    pub observations: Vec<RootObservation>,
    pub bounds: WorldCommitBounds,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturePlan {
    pub core: WorldCommitCore,
    pub roots_to_persist: Vec<WorldRootRef>,
    pub revision_fences: Vec<RevisionFence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionRecheck {
    pub root_kind: RootKind,
    pub source_id: String,
    pub current_revision: u64,
    pub inventory_complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionComparison {
    pub current: bool,
    pub issues: Vec<CaptureIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CaptureIssue {
    ZeroBound(&'static str),
    BoundAboveHardLimit {
        field: &'static str,
        actual: usize,
        maximum: usize,
    },
    BoundExceeded {
        field: &'static str,
        actual: usize,
        maximum: usize,
    },
    LogicalProfileHasCohort,
    OpaqueProfileMissingCohort,
    DuplicateParent(String),
    DuplicateRootKind(RootKind),
    DuplicateRootRef(String),
    MissingRequiredRoot(RootKind),
    UnexpectedRoot(RootKind),
    CompletenessMismatch,
    RootDomainMismatch {
        expected: RootKind,
        actual: RootKind,
    },
    RootSchemaNotValidated(RootKind),
    IncompleteInventory(RootKind),
    FenceRootMismatch {
        root: RootKind,
        fence: RootKind,
    },
    DuplicateFenceSource(String),
    MissingRecheck(String),
    UnexpectedRecheck(String),
    DuplicateRecheck(String),
    RevisionDrift {
        source_id: String,
        expected: u64,
        actual: u64,
    },
    RecheckInventoryIncomplete(String),
}

// r[impl molten.world_commit.capture]
pub fn plan_capture(request: &CaptureRequest) -> Result<CapturePlan, Vec<CaptureIssue>> {
    let mut issues = validate_bounds(&request.bounds);
    validate_observation_bound(request, &mut issues);
    if request.parents.len() > request.bounds.max_parents {
        issues.push(CaptureIssue::BoundExceeded {
            field: "parents",
            actual: request.parents.len(),
            maximum: request.bounds.max_parents,
        });
    }
    if !issues.is_empty() {
        issues.sort();
        issues.dedup();
        return Err(issues);
    }
    let mut roots = Vec::with_capacity(request.observations.len());
    let mut roots_to_persist = Vec::with_capacity(request.observations.len());
    let mut revision_fences = Vec::with_capacity(request.bounds.max_revision_fences);
    let mut fence_sources = BTreeSet::new();
    for observation in &request.observations {
        validate_observation(observation, &mut issues);
        roots.push(observation.root.clone());
        if !observation.durable {
            roots_to_persist.push(observation.root.clone());
        }
        if let ObservationStability::Mutable(fence) = &observation.stability {
            if !fence_sources.insert(fence.source_id.as_str()) {
                issues.push(CaptureIssue::DuplicateFenceSource(fence.source_id.clone()));
            }
            revision_fences.push(fence.clone());
        }
    }
    if revision_fences.len() > request.bounds.max_revision_fences {
        issues.push(CaptureIssue::BoundExceeded {
            field: "revision-fences",
            actual: revision_fences.len(),
            maximum: request.bounds.max_revision_fences,
        });
    }
    let candidate = WorldCommitCore {
        version: request.version,
        profile: request.profile.clone(),
        parents: request.parents.clone(),
        roots,
        completeness: CompletenessClaim::for_profile(request.profile.kind),
    };
    let normalized = match validate_and_normalize_core(&candidate, &request.bounds) {
        Ok(normalized) => Some(normalized),
        Err(core_issues) => {
            issues.extend(core_issues);
            None
        }
    };
    if !issues.is_empty() {
        issues.sort();
        issues.dedup();
        return Err(issues);
    }
    roots_to_persist
        .sort_by(|left, right| left.kind().cmp(&right.kind()).then_with(|| left.as_str().cmp(right.as_str())));
    revision_fences.sort();
    let Some(core) = normalized else {
        return Err(vec![CaptureIssue::CompletenessMismatch]);
    };
    Ok(CapturePlan {
        core,
        roots_to_persist,
        revision_fences,
    })
}

// r[impl molten.world_commit.capture]
pub fn compare_revision_rechecks(plan: &CapturePlan, rechecks: &[RevisionRecheck]) -> RevisionComparison {
    let issue_capacity = plan.revision_fences.len().saturating_add(rechecks.len());
    let mut issues = Vec::with_capacity(issue_capacity);
    let expected = plan
        .revision_fences
        .iter()
        .map(|fence| (fence.source_id.as_str(), fence))
        .collect::<BTreeMap<_, _>>();
    let mut actual = BTreeMap::new();
    for recheck in rechecks {
        if actual.insert(recheck.source_id.as_str(), recheck).is_some() {
            issues.push(CaptureIssue::DuplicateRecheck(recheck.source_id.clone()));
        }
    }
    for (source_id, fence) in expected {
        let Some(recheck) = actual.remove(source_id) else {
            issues.push(CaptureIssue::MissingRecheck(source_id.to_string()));
            continue;
        };
        compare_recheck(fence, recheck, &mut issues);
    }
    for source_id in actual.keys() {
        issues.push(CaptureIssue::UnexpectedRecheck((*source_id).to_string()));
    }
    issues.sort();
    issues.dedup();
    RevisionComparison {
        current: issues.is_empty(),
        issues,
    }
}

fn validate_observation_bound(request: &CaptureRequest, issues: &mut Vec<CaptureIssue>) {
    if request.observations.len() > request.bounds.max_roots {
        issues.push(CaptureIssue::BoundExceeded {
            field: "root-observations",
            actual: request.observations.len(),
            maximum: request.bounds.max_roots,
        });
    }
}

fn validate_observation(observation: &super::RootObservation, issues: &mut Vec<CaptureIssue>) {
    if observation.root.kind() != observation.source_kind {
        issues.push(CaptureIssue::RootDomainMismatch {
            expected: observation.root.kind(),
            actual: observation.source_kind,
        });
    }
    if !observation.schema_validated {
        issues.push(CaptureIssue::RootSchemaNotValidated(observation.root.kind()));
    }
    if !observation.inventory_complete {
        issues.push(CaptureIssue::IncompleteInventory(observation.root.kind()));
    }
    if let ObservationStability::Mutable(fence) = &observation.stability
        && fence.root_kind != observation.root.kind()
    {
        issues.push(CaptureIssue::FenceRootMismatch {
            root: observation.root.kind(),
            fence: fence.root_kind,
        });
    }
}

fn compare_recheck(fence: &RevisionFence, recheck: &RevisionRecheck, issues: &mut Vec<CaptureIssue>) {
    if recheck.root_kind != fence.root_kind {
        issues.push(CaptureIssue::FenceRootMismatch {
            root: fence.root_kind,
            fence: recheck.root_kind,
        });
    }
    if recheck.current_revision != fence.observed_revision {
        issues.push(CaptureIssue::RevisionDrift {
            source_id: fence.source_id.clone(),
            expected: fence.observed_revision,
            actual: recheck.current_revision,
        });
    }
    if !recheck.inventory_complete {
        issues.push(CaptureIssue::RecheckInventoryIncomplete(fence.source_id.clone()));
    }
}
