use molten_core::world_commit::WorldCommitRef;
use molten_core::world_commit::WorldRootRef;
use molten_core::world_merge::WorldMergeMode;
use molten_core::world_merge::WorldMergeOutput;
use molten_core::world_merge::WorldMergePlan;
use molten_core::world_merge::WorldMergePolicyRef;
use molten_core::world_merge::WorldMergeRequest;
use molten_core::world_merge::WorldMergeValue;
use molten_core::world_merge::admit_world_migrations;
use molten_core::world_merge::plan_world_merge;
use molten_core::world_merge::validate_world_merge_plan;
use molten_core::world_merge::validate_world_migration_result;

use super::CanonicalWorldMergeConflict;
use super::CanonicalWorldMergeResult;
use super::WorldMergeAuthorityPort;
use super::WorldMergeCommitPort;
use super::WorldMergeConflictPort;
use super::WorldMergeHandlerPort;
use super::WorldMergeMigrationPort;
use super::WorldMergeObjectPort;
use super::WorldMergePortError;
use super::WorldMergeResultInput;
use super::canonical_generated_world_root;
use super::canonical_world_merge_conflict;
use super::canonical_world_merge_result;
use crate::error::Failure;
use crate::error::Result;

const DECISION_PUBLISHED: &str = "published";
const DECISION_CONFLICT: &str = "conflict";
const NO_AUTHORITY_OBSERVATION_DOMAIN: &str = "molten.world-merge.no-authority-observation.v1";

pub fn prepare_world_merge_plan<O, M, H>(
    objects: &mut O,
    migrations: &mut M,
    handlers: &mut H,
    request: &WorldMergeRequest,
) -> Result<WorldMergePlan>
where
    O: WorldMergeObjectPort,
    M: WorldMergeMigrationPort,
    H: WorldMergeHandlerPort,
{
    let mut loaded = request.clone();
    for root in &mut loaded.roots {
        load_value(objects, &mut root.base, loaded.bounds.max_value_bytes)?;
        load_value(objects, &mut root.left, loaded.bounds.max_value_bytes)?;
        load_value(objects, &mut root.right, loaded.bounds.max_value_bytes)?;
    }
    let conversions = admit_world_migrations(&loaded)
        .map_err(|issues| Failure::invalid_harness(format!("world merge migration admission denied: {issues:?}")))?;
    let mut materializations = Vec::with_capacity(conversions.len());
    for conversion in &conversions {
        let root = loaded
            .roots
            .iter()
            .find(|root| root.kind == conversion.kind())
            .ok_or_else(|| Failure::invalid_harness("admitted migration root is unavailable"))?;
        let source = conversion
            .side()
            .value(root)
            .canonical_bytes
            .as_deref()
            .ok_or_else(|| Failure::invalid_harness("admitted migration source bytes are unavailable"))?;
        let result = migrations.materialize_migration(conversion, source).map_err(port_error)?;
        materializations.push(validate_world_migration_result(conversion, result).map_err(migration_result_error)?);
    }
    let application_profiles = loaded
        .profile
        .root_modes
        .iter()
        .filter_map(|(kind, mode)| {
            (*mode == WorldMergeMode::ApplicationHandler).then(|| loaded.profile.handlers.get(kind)).flatten()
        })
        .collect::<Vec<_>>();
    if application_profiles.len() > 1 {
        return Err(Failure::invalid_harness("one merge plan supports at most one application handler profile"));
    }
    let handler = application_profiles
        .first()
        .map(|profile| handlers.load_handler(profile).map_err(port_error))
        .transpose()?;
    plan_world_merge(loaded, materializations, handler.as_deref())
        .map_err(|issues| Failure::invalid_harness(format!("world merge planning denied: {issues:?}")))
}

pub struct WorldMergePublicationRequest<'a> {
    pub plan: &'a WorldMergePlan,
    pub policy_ref: &'a WorldMergePolicyRef,
}

#[derive(Debug, Clone)]
pub struct WorldMergePublicationResult {
    pub result_commit: Option<WorldCommitRef>,
    pub output_roots: Vec<WorldRootRef>,
    pub conflicts: Vec<CanonicalWorldMergeConflict>,
    pub receipt: CanonicalWorldMergeResult,
}

pub fn publish_world_merge<O, C, A, P>(
    objects: &mut O,
    conflicts: &mut C,
    authority: &mut A,
    commits: &mut P,
    request: &WorldMergePublicationRequest<'_>,
) -> Result<WorldMergePublicationResult>
where
    O: WorldMergeObjectPort,
    C: WorldMergeConflictPort,
    A: WorldMergeAuthorityPort,
    P: WorldMergeCommitPort,
{
    validate_world_merge_plan(request.plan)
        .map_err(|issue| Failure::invalid_harness(format!("world merge publication denied: {issue:?}")))?;
    if !request.plan.conflicts.is_empty() {
        return persist_conflicts(conflicts, request.plan);
    }
    let authority_ref = authority
        .recheck_merge_authority(&request.plan.source_heads, request.policy_ref)
        .map_err(port_error)?;
    let mut output_roots = Vec::with_capacity(request.plan.outputs.len());
    for output in &request.plan.outputs {
        match &output.output {
            WorldMergeOutput::Absent => {}
            WorldMergeOutput::Selected(root) => output_roots.push(root.clone()),
            WorldMergeOutput::Generated(generated) => {
                let (expected_root, bytes) = canonical_generated_world_root(output)?;
                let observed =
                    objects.persist_generated_root(output.kind, &generated.schema, &bytes).map_err(port_error)?;
                if observed != expected_root {
                    return Err(Failure::invalid_harness("generated merge root identity changed during publication"));
                }
                output_roots.push(observed);
            }
        }
    }
    let result_commit = commits
        .publish_merge_commit(&request.plan.base_head, &request.plan.source_heads, &output_roots)
        .map_err(port_error)?;
    let receipt = canonical_world_merge_result(&WorldMergeResultInput {
        plan: request.plan,
        result_commit: Some(&result_commit),
        output_roots: &output_roots,
        authority_ref: &authority_ref,
        decision: DECISION_PUBLISHED,
        issues: &[],
    })?;
    Ok(WorldMergePublicationResult {
        result_commit: Some(result_commit),
        output_roots,
        conflicts: Vec::new(),
        receipt,
    })
}

fn persist_conflicts<C: WorldMergeConflictPort>(
    store: &mut C,
    plan: &WorldMergePlan,
) -> Result<WorldMergePublicationResult> {
    let mut canonical_conflicts = Vec::with_capacity(plan.conflicts.len());
    for conflict in &plan.conflicts {
        let canonical = canonical_world_merge_conflict(plan, conflict)?;
        store.persist_conflict(&canonical.conflict_ref, &canonical.bytes).map_err(port_error)?;
        canonical_conflicts.push(canonical);
    }
    let authority_ref = format!("blake3:{}", blake3::hash(NO_AUTHORITY_OBSERVATION_DOMAIN.as_bytes()).to_hex());
    let issue_codes = vec!["unresolved-conflicts".to_string()];
    let receipt = canonical_world_merge_result(&WorldMergeResultInput {
        plan,
        result_commit: None,
        output_roots: &[],
        authority_ref: &authority_ref,
        decision: DECISION_CONFLICT,
        issues: &issue_codes,
    })?;
    Ok(WorldMergePublicationResult {
        result_commit: None,
        output_roots: Vec::new(),
        conflicts: canonical_conflicts,
        receipt,
    })
}

fn load_value<O: WorldMergeObjectPort>(objects: &mut O, value: &mut WorldMergeValue, maximum_bytes: u64) -> Result<()> {
    if value.canonical_bytes.is_some() {
        return Ok(());
    }
    let Some(root) = value.root.as_ref() else {
        return Ok(());
    };
    value.canonical_bytes = Some(objects.load_root(root, maximum_bytes).map_err(port_error)?);
    Ok(())
}

fn migration_result_error(issue: molten_core::world_merge::WorldMergeIssue) -> Failure {
    Failure::invalid_harness(format!("world merge migration result denied: {issue:?}"))
}

fn port_error(error: WorldMergePortError) -> Failure {
    Failure::invalid_harness(format!("world-merge port failed: {error}"))
}
