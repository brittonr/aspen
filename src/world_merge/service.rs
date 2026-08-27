use molten_core::world_commit::WorldCommitRef;
use molten_core::world_commit::WorldRootRef;
use molten_core::world_merge::WorldMergeMode;
use molten_core::world_merge::WorldMergePlan;
use molten_core::world_merge::WorldMergePolicyRef;
use molten_core::world_merge::WorldMergeRequest;
use molten_core::world_merge::WorldMergeValue;
use molten_core::world_merge::plan_world_merge;

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
use crate::error::MoltenError;
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
        if let Some(binding) = loaded.profile.migrations.get(&root.kind) {
            materialize_value(migrations, binding, &mut root.base)?;
            materialize_value(migrations, binding, &mut root.left)?;
            materialize_value(migrations, binding, &mut root.right)?;
        }
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
        return Err(MoltenError::invalid_harness("one merge plan supports at most one application handler profile"));
    }
    let handler = application_profiles
        .first()
        .map(|profile| handlers.load_handler(profile).map_err(port_error))
        .transpose()?;
    plan_world_merge(&loaded, handler.as_deref())
        .map_err(|issues| MoltenError::invalid_harness(format!("world merge planning denied: {issues:?}")))
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
    if !request.plan.conflicts.is_empty() {
        return persist_conflicts(conflicts, request.plan);
    }
    let authority_ref = authority
        .recheck_merge_authority(&request.plan.source_heads, request.policy_ref)
        .map_err(port_error)?;
    let mut output_roots = Vec::with_capacity(request.plan.outputs.len());
    for output in &request.plan.outputs {
        if let Some(root) = &output.selected_root {
            output_roots.push(root.clone());
            continue;
        }
        let (expected_root, bytes) = canonical_generated_world_root(output)?;
        let schema_ref = output
            .output_schema
            .as_ref()
            .ok_or_else(|| MoltenError::invalid_harness("generated merge output has no schema"))?;
        let observed_root = objects.persist_generated_root(output.kind, schema_ref, &bytes).map_err(port_error)?;
        if observed_root != expected_root {
            return Err(MoltenError::invalid_harness("generated merge root identity changed during publication"));
        }
        output_roots.push(observed_root);
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

fn materialize_value<M: WorldMergeMigrationPort>(
    migrations: &mut M,
    binding: &molten_core::world_merge::WorldMigrationBinding,
    value: &mut WorldMergeValue,
) -> Result<()> {
    if value.schema_ref.as_ref() != Some(&binding.source_schema) {
        return Ok(());
    }
    let source = value
        .canonical_bytes
        .as_deref()
        .ok_or_else(|| MoltenError::invalid_harness("migration source bytes are unavailable"))?;
    value.canonical_bytes = Some(migrations.materialize_migration(binding, source).map_err(port_error)?);
    value.schema_ref = Some(binding.target_schema.clone());
    Ok(())
}

fn port_error(error: WorldMergePortError) -> MoltenError {
    MoltenError::invalid_harness(format!("world-merge port failed: {error}"))
}
