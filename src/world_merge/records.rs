use molten_core::world_commit::WorldCommitRef;
use molten_core::world_commit::WorldRootRef;
use molten_core::world_merge::WORLD_DIFF_SCHEMA;
use molten_core::world_merge::WORLD_MERGE_CONFLICT_SCHEMA;
use molten_core::world_merge::WORLD_MERGE_PLAN_SCHEMA;
use molten_core::world_merge::WORLD_MERGE_RESULT_SCHEMA;
use molten_core::world_merge::WorldDiffReport;
use molten_core::world_merge::WorldMergeConflict;
use molten_core::world_merge::WorldMergePlan;
use molten_core::world_merge::WorldMergeSchemaRef;
use molten_core::world_merge::WorldMergedRoot;
use preserves::IOValue;

use crate::error::MoltenError;
use crate::error::Result;

pub const WORLD_DIFF_RECORD: &str = "world-diff";
pub const WORLD_MERGE_PLAN_RECORD: &str = "world-merge-plan";
pub const WORLD_MERGE_CONFLICT_RECORD: &str = "world-merge-conflict";
pub const WORLD_MERGE_RESULT_RECORD: &str = "world-merge-result";
const WORLD_MERGE_GENERATED_ROOT_RECORD: &str = "world-merge-generated-root";
const WORLD_MERGE_NON_CLAIM_COUNT: usize = 7;

pub const WORLD_MERGE_NON_CLAIMS: [&str; WORLD_MERGE_NON_CLAIM_COUNT] = [
    "diff-does-not-grant-merge-authority",
    "schema-compatibility-does-not-prove-semantic-equivalence",
    "handler-identity-does-not-prove-handler-correctness",
    "migration-planning-does-not-prove-migrated-data-correctness",
    "conflict-identity-does-not-select-a-winner",
    "merge-publication-does-not-move-a-branch-head",
    "merge-receipt-does-not-prove-release-eligibility",
];

#[derive(Debug, Clone)]
pub struct CanonicalWorldDiff {
    pub report_ref: String,
    pub value: IOValue,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct CanonicalWorldMergePlan {
    pub plan_ref: String,
    pub value: IOValue,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct CanonicalWorldMergeConflict {
    pub conflict_ref: String,
    pub value: IOValue,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct CanonicalWorldMergeResult {
    pub result_ref: String,
    pub value: IOValue,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct WorldMergeResultInput<'a> {
    pub plan: &'a WorldMergePlan,
    pub result_commit: Option<&'a WorldCommitRef>,
    pub output_roots: &'a [WorldRootRef],
    pub authority_ref: &'a str,
    pub decision: &'a str,
    pub issues: &'a [String],
}

pub fn canonical_world_diff(report: &WorldDiffReport) -> Result<CanonicalWorldDiff> {
    let roots = report
        .roots
        .iter()
        .map(|root| {
            crate::preserves_rail::record("root-diff", vec![
                crate::preserves_rail::string(root.kind.as_str()),
                crate::preserves_rail::string(root.class.as_str()),
            ])
        })
        .collect();
    let value = crate::preserves_rail::record(WORLD_DIFF_RECORD, vec![
        crate::preserves_rail::string(WORLD_DIFF_SCHEMA),
        field("base-head", crate::preserves_rail::string(report.base_head.as_str())),
        field("source-heads", refs(report.source_heads.iter().map(WorldCommitRef::as_str))),
        field("roots", crate::preserves_rail::sequence(roots)),
        non_claims(),
    ]);
    canonical_diff(value)
}

pub fn canonical_world_merge_plan(plan: &WorldMergePlan) -> Result<CanonicalWorldMergePlan> {
    let outputs = plan.outputs.iter().map(output_value).collect();
    let conflicts = plan.conflicts.iter().map(conflict_value).collect();
    let value = crate::preserves_rail::record(WORLD_MERGE_PLAN_RECORD, vec![
        crate::preserves_rail::string(WORLD_MERGE_PLAN_SCHEMA),
        field("plan-ref", crate::preserves_rail::string(plan.plan_ref.as_str())),
        field("base-head", crate::preserves_rail::string(plan.base_head.as_str())),
        field("source-heads", refs(plan.source_heads.iter().map(WorldCommitRef::as_str))),
        field("outputs", crate::preserves_rail::sequence(outputs)),
        field("conflicts", crate::preserves_rail::sequence(conflicts)),
        non_claims(),
    ]);
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    Ok(CanonicalWorldMergePlan {
        plan_ref: crate::preserves_rail::content_ref_from_bytes(&bytes),
        value,
        bytes,
    })
}

pub fn canonical_world_merge_conflict(
    plan: &WorldMergePlan,
    conflict: &WorldMergeConflict,
) -> Result<CanonicalWorldMergeConflict> {
    let value = crate::preserves_rail::record(WORLD_MERGE_CONFLICT_RECORD, vec![
        crate::preserves_rail::string(WORLD_MERGE_CONFLICT_SCHEMA),
        field("plan-ref", crate::preserves_rail::string(plan.plan_ref.as_str())),
        field("root-kind", crate::preserves_rail::string(conflict.kind.as_str())),
        field(
            "key",
            conflict.key.as_deref().map_or_else(
                || crate::preserves_rail::record("none", Vec::new()),
                |key| crate::preserves_rail::record("some", vec![crate::preserves_rail::string(key)]),
            ),
        ),
        field("code", crate::preserves_rail::string(conflict.code)),
        non_claims(),
    ]);
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    Ok(CanonicalWorldMergeConflict {
        conflict_ref: crate::preserves_rail::content_ref_from_bytes(&bytes),
        value,
        bytes,
    })
}

pub fn canonical_generated_world_root(output: &WorldMergedRoot) -> Result<(WorldRootRef, Vec<u8>)> {
    let schema = output
        .output_schema
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("generated merge root requires an output schema"))?;
    let mut entries = output.generated_values.iter().collect::<Vec<_>>();
    entries.sort_by(|left, right| left.0.cmp(right.0));
    let entries = entries
        .into_iter()
        .map(|(key, value)| {
            crate::preserves_rail::record("entry", vec![crate::preserves_rail::string(key), bytes_value(value)])
        })
        .collect();
    let value = crate::preserves_rail::record(WORLD_MERGE_GENERATED_ROOT_RECORD, vec![
        crate::preserves_rail::string(WORLD_MERGE_RESULT_SCHEMA),
        field("root-kind", crate::preserves_rail::string(output.kind.as_str())),
        field("schema-ref", crate::preserves_rail::string(schema.as_str())),
        field("entries", crate::preserves_rail::sequence(entries)),
        field(
            "handler-bytes",
            output.generated_bytes.as_ref().map_or_else(
                || crate::preserves_rail::record("none", Vec::new()),
                |bytes| crate::preserves_rail::record("some", vec![bytes_value(bytes)]),
            ),
        ),
    ]);
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    let root = WorldRootRef::parse(output.kind, crate::preserves_rail::content_ref_from_bytes(&bytes))
        .map_err(|error| MoltenError::invalid_harness(format!("generated root identity failed: {error:?}")))?;
    Ok((root, bytes))
}

pub fn canonical_world_merge_result(input: &WorldMergeResultInput<'_>) -> Result<CanonicalWorldMergeResult> {
    let value = crate::preserves_rail::record(WORLD_MERGE_RESULT_RECORD, vec![
        crate::preserves_rail::string(WORLD_MERGE_RESULT_SCHEMA),
        field("decision", crate::preserves_rail::string(input.decision)),
        field("plan-ref", crate::preserves_rail::string(input.plan.plan_ref.as_str())),
        field(
            "result-commit",
            crate::preserves_rail::optional_ref_value(input.result_commit.map(WorldCommitRef::as_str)),
        ),
        field("output-roots", refs(input.output_roots.iter().map(WorldRootRef::as_str))),
        field("authority-ref", crate::preserves_rail::string(input.authority_ref)),
        field(
            "issues",
            crate::preserves_rail::sequence(input.issues.iter().map(crate::preserves_rail::string).collect()),
        ),
        non_claims(),
    ]);
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    Ok(CanonicalWorldMergeResult {
        result_ref: crate::preserves_rail::content_ref_from_bytes(&bytes),
        value,
        bytes,
    })
}

fn canonical_diff(value: IOValue) -> Result<CanonicalWorldDiff> {
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    Ok(CanonicalWorldDiff {
        report_ref: crate::preserves_rail::content_ref_from_bytes(&bytes),
        value,
        bytes,
    })
}

fn output_value(output: &WorldMergedRoot) -> IOValue {
    crate::preserves_rail::record("merge-output", vec![
        crate::preserves_rail::string(output.kind.as_str()),
        crate::preserves_rail::optional_ref_value(output.selected_root.as_ref().map(WorldRootRef::as_str)),
        crate::preserves_rail::optional_ref_value(output.output_schema.as_ref().map(WorldMergeSchemaRef::as_str)),
        crate::preserves_rail::string(if output.generated_values.is_empty() && output.generated_bytes.is_none() {
            "selected"
        } else {
            "generated"
        }),
    ])
}

fn conflict_value(conflict: &WorldMergeConflict) -> IOValue {
    crate::preserves_rail::record("merge-conflict", vec![
        crate::preserves_rail::string(conflict.kind.as_str()),
        conflict.key.as_deref().map_or_else(
            || crate::preserves_rail::record("none", Vec::new()),
            |key| crate::preserves_rail::record("some", vec![crate::preserves_rail::string(key)]),
        ),
        crate::preserves_rail::string(conflict.code),
    ])
}

fn bytes_value(bytes: &[u8]) -> IOValue {
    crate::preserves_rail::sequence(
        bytes.iter().map(|byte| crate::preserves_rail::u64_value(u64::from(*byte))).collect(),
    )
}

fn field(label: &'static str, value: IOValue) -> IOValue {
    crate::preserves_rail::record(label, vec![value])
}

fn refs<'a>(values: impl Iterator<Item = &'a str>) -> IOValue {
    crate::preserves_rail::sequence(values.map(crate::preserves_rail::string).collect())
}

fn non_claims() -> IOValue {
    field(
        "non-claims",
        crate::preserves_rail::sequence(WORLD_MERGE_NON_CLAIMS.iter().map(crate::preserves_rail::string).collect()),
    )
}
