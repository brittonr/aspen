use molten_core::world_commit::CaptureRequest;
use molten_core::world_commit::RestorePlan;
use molten_core::world_commit::RevisionRecheck;
use molten_core::world_commit::RootKind;
use molten_core::world_commit::SnapshotProfile;
use molten_core::world_commit::WorldCommitBounds;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_commit::WorldCommitVersion;
use molten_core::world_commit::compare_revision_rechecks;
use molten_core::world_commit::plan_capture;

use super::CanonicalCaptureReceipt;
use super::CanonicalWorldCommit;
use super::CaptureDecision;
use super::CaptureReceipt;
use super::PublicationOutcome;
use super::WORLD_COMMIT_NON_CLAIMS;
use super::WorldCommitPortError;
use super::WorldCommitPublicationPort;
use super::WorldImmutableObjectPort;
use super::WorldRestorePort;
use super::WorldRevisionRecheckPort;
use super::WorldRootObservationPort;
use super::canonical_capture_receipt;
use super::canonical_world_commit;
use super::denied_capture_receipt;
use crate::error::MoltenError;
use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureShellInput {
    pub version: WorldCommitVersion,
    pub profile: SnapshotProfile,
    pub parents: Vec<WorldCommitRef>,
    pub root_kinds: Vec<RootKind>,
    pub bounds: WorldCommitBounds,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureExecution {
    pub commit: Option<CanonicalWorldCommit>,
    pub receipt: CanonicalCaptureReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreExecution {
    pub evidence_refs: Vec<String>,
}

// r[impl molten.world_commit.capture]
pub fn capture_world_commit<O, S, R>(
    input: &CaptureShellInput,
    observations: &mut O,
    store: &mut S,
    revisions: &mut R,
) -> Result<CaptureExecution>
where
    O: WorldRootObservationPort,
    S: WorldImmutableObjectPort + WorldCommitPublicationPort,
    R: WorldRevisionRecheckPort,
{
    let profile_ref = input.profile.profile_ref.as_str();
    let preflight_issues = super::preflight_shell(input);
    if !preflight_issues.is_empty() {
        return denied_execution(profile_ref, None, PublicationOutcome::NotAttempted, preflight_issues);
    }
    let materials = match observe_materials(input, observations, store) {
        Ok(materials) => materials,
        Err(error) => {
            return denied_execution(profile_ref, None, PublicationOutcome::NotAttempted, [port_issue(&error)]);
        }
    };
    let request = CaptureRequest {
        version: input.version,
        profile: input.profile.clone(),
        parents: input.parents.clone(),
        observations: materials.iter().map(|material| material.observation.clone()).collect(),
        bounds: input.bounds.clone(),
    };
    let plan = match plan_capture(&request) {
        Ok(plan) => plan,
        Err(issues) => {
            return denied_execution(
                profile_ref,
                None,
                PublicationOutcome::NotAttempted,
                issues.into_iter().map(|issue| format!("{issue:?}")),
            );
        }
    };
    if let Err(error) = persist_missing_roots(&plan, &materials, store) {
        return denied_execution(profile_ref, Some(&plan), PublicationOutcome::NotAttempted, [port_issue(&error)]);
    }
    if let Err(error) = verify_durable_roots(&plan, store) {
        return denied_execution(profile_ref, Some(&plan), PublicationOutcome::NotAttempted, [port_issue(&error)]);
    }
    let rechecks = match collect_rechecks(&plan.revision_fences, revisions) {
        Ok(rechecks) => rechecks,
        Err(error) => {
            return denied_execution(profile_ref, Some(&plan), PublicationOutcome::NotAttempted, [port_issue(&error)]);
        }
    };
    let comparison = compare_revision_rechecks(&plan, &rechecks);
    if !comparison.current {
        return denied_execution(
            profile_ref,
            Some(&plan),
            PublicationOutcome::NotAttempted,
            comparison.issues.into_iter().map(|issue| format!("{issue:?}")),
        );
    }
    publish_capture(input, profile_ref, plan, store)
}

fn publish_capture<S>(
    input: &CaptureShellInput,
    profile_ref: &str,
    plan: molten_core::world_commit::CapturePlan,
    store: &mut S,
) -> Result<CaptureExecution>
where
    S: WorldCommitPublicationPort,
{
    let commit = canonical_world_commit(&plan.core, &input.bounds)?;
    let outcome = match store.publish_commit(&commit.commit_ref, &commit.bytes) {
        Ok(outcome) => outcome,
        Err(error) => {
            return denied_execution(profile_ref, Some(&plan), PublicationOutcome::Uncertain, [port_issue(&error)]);
        }
    };
    if !outcome.is_success() {
        return denied_execution(profile_ref, Some(&plan), outcome, [
            "world commit publication did not reach a known successful outcome".to_string(),
        ]);
    }
    let receipt = canonical_capture_receipt(&CaptureReceipt {
        decision: CaptureDecision::Published,
        commit_ref: Some(commit.commit_ref.clone()),
        profile_ref: profile_ref.to_string(),
        persisted_roots: plan.roots_to_persist,
        revision_fences: plan.revision_fences,
        issues: Vec::new(),
        publication: outcome,
        non_claims: WORLD_COMMIT_NON_CLAIMS.to_vec(),
    })?;
    Ok(CaptureExecution {
        commit: Some(commit),
        receipt,
    })
}

// r[impl molten.world_commit.restore]
pub fn execute_restore_plan<P: WorldRestorePort>(plan: &RestorePlan, port: &mut P) -> Result<RestoreExecution> {
    let mut evidence_refs = Vec::with_capacity(plan.steps.len());
    for step in &plan.steps {
        let outcome = port.execute_restore_step(step).map_err(port_error)?;
        crate::preserves_rail::validate_content_ref(&outcome.evidence_ref)?;
        if outcome.step != *step {
            return Err(MoltenError::invalid_harness(
                "world restore port returned evidence for a different restore step",
            ));
        }
        evidence_refs.push(outcome.evidence_ref);
    }
    Ok(RestoreExecution { evidence_refs })
}

fn observe_materials<O, S>(
    input: &CaptureShellInput,
    observations: &mut O,
    objects: &S,
) -> std::result::Result<Vec<super::ObservedRootMaterial>, WorldCommitPortError>
where
    O: WorldRootObservationPort,
    S: WorldImmutableObjectPort,
{
    let mut materials = Vec::with_capacity(input.root_kinds.len());
    for kind in &input.root_kinds {
        let mut material = observations.observe_root(*kind)?;
        material.observation.durable = objects.contains_root(&material.observation.root)?;
        materials.push(material);
    }
    Ok(materials)
}

fn persist_missing_roots<S: WorldImmutableObjectPort>(
    plan: &molten_core::world_commit::CapturePlan,
    materials: &[super::ObservedRootMaterial],
    objects: &mut S,
) -> std::result::Result<(), WorldCommitPortError> {
    for root in &plan.roots_to_persist {
        let material = materials
            .iter()
            .find(|material| material.observation.root == *root)
            .ok_or_else(|| WorldCommitPortError::new("missing-root-material", root.kind().as_str()))?;
        verify_root_material(root, &material.canonical_bytes)?;
        objects.persist_root(root, &material.canonical_bytes)?;
        if !objects.contains_root(root)? {
            return Err(WorldCommitPortError::new("root-durability-unconfirmed", root.kind().as_str()));
        }
    }
    Ok(())
}

fn verify_root_material(
    root: &molten_core::world_commit::WorldRootRef,
    bytes: &[u8],
) -> std::result::Result<(), WorldCommitPortError> {
    crate::preserves_rail::strict_canonical_decode(bytes)
        .map_err(|error| WorldCommitPortError::new("root-noncanonical", error.to_string()))?;
    let observed = crate::preserves_rail::content_ref_from_bytes(bytes);
    if observed != root.as_str() {
        return Err(WorldCommitPortError::new(
            "root-identity-mismatch",
            format!("{} expected {}, got {observed}", root.kind().as_str(), root.as_str()),
        ));
    }
    Ok(())
}

fn verify_durable_roots<S: WorldImmutableObjectPort>(
    plan: &molten_core::world_commit::CapturePlan,
    objects: &S,
) -> std::result::Result<(), WorldCommitPortError> {
    for root in &plan.core.roots {
        let bytes = objects.read_root(root)?;
        verify_root_material(root, &bytes)?;
    }
    Ok(())
}

fn collect_rechecks<R: WorldRevisionRecheckPort>(
    fences: &[molten_core::world_commit::RevisionFence],
    revisions: &mut R,
) -> std::result::Result<Vec<RevisionRecheck>, WorldCommitPortError> {
    fences.iter().map(|fence| revisions.recheck_revision(fence)).collect()
}

fn denied_execution(
    profile_ref: &str,
    plan: Option<&molten_core::world_commit::CapturePlan>,
    publication: PublicationOutcome,
    issues: impl IntoIterator<Item = String>,
) -> Result<CaptureExecution> {
    Ok(CaptureExecution {
        commit: None,
        receipt: denied_capture_receipt(profile_ref, plan, publication, issues)?,
    })
}

pub(crate) fn port_issue(error: &WorldCommitPortError) -> String {
    format!("port:{}", error.class)
}

fn port_error(error: WorldCommitPortError) -> MoltenError {
    MoltenError::invalid_harness(format!("world commit port failed: {error}"))
}
