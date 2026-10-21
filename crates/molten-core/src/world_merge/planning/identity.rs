use serde::Serialize;

use super::super::WorldMergeConflict;
use super::super::WorldMergeIssue;
use super::super::WorldMergeOutput;
use super::super::WorldMergePlan;
use super::super::WorldMergePlanRef;
use super::super::WorldMergedRoot;
use super::super::WorldMigrationEvidence;
use super::super::migration::canonical_ref;
use super::super::migration::payload_ref;
use crate::world_commit::WorldCommitRef;

#[derive(Serialize)]
#[serde(rename = "world-merge-plan-identity-v3")]
struct PlanIdentity<'a> {
    original_request: &'a str,
    base_head: &'a str,
    source_heads: Vec<&'a str>,
    outputs: Vec<OutputIdentity<'a>>,
    conflicts: Vec<(&'a str, Option<&'a str>, &'a str)>,
    migrations: Vec<(&'a str, &'a str)>,
}

#[derive(Serialize)]
struct OutputIdentity<'a> {
    kind: &'a str,
    case: OutputCase<'a>,
}

#[derive(Serialize)]
#[serde(tag = "case", rename_all = "kebab-case")]
enum OutputCase<'a> {
    Absent,
    Selected {
        root: &'a str,
    },
    Generated {
        schema: &'a str,
        bytes_present: bool,
        payload: String,
    },
}

pub(in crate::world_merge) struct PlanIdentityInput<'a> {
    pub original_request: &'a str,
    pub base_head: &'a WorldCommitRef,
    pub source_heads: &'a [WorldCommitRef],
    pub outputs: &'a [WorldMergedRoot],
    pub conflicts: &'a [WorldMergeConflict],
    pub migrations: &'a [WorldMigrationEvidence],
}

pub(in crate::world_merge) fn identify_plan(
    input: PlanIdentityInput<'_>,
) -> Result<WorldMergePlanRef, WorldMergeIssue> {
    let outputs = input
        .outputs
        .iter()
        .map(|output| {
            let case = match &output.output {
                WorldMergeOutput::Absent => OutputCase::Absent,
                WorldMergeOutput::Selected(root) => OutputCase::Selected { root: root.as_str() },
                WorldMergeOutput::Generated(generated) => OutputCase::Generated {
                    schema: generated.schema.as_str(),
                    bytes_present: generated.bytes.is_some(),
                    payload: payload_ref(generated.bytes.as_deref().unwrap_or_default(), &generated.values)?,
                },
            };
            Ok(OutputIdentity {
                kind: output.kind.as_str(),
                case,
            })
        })
        .collect::<Result<Vec<_>, WorldMergeIssue>>()?;
    let value = PlanIdentity {
        original_request: input.original_request,
        base_head: input.base_head.as_str(),
        source_heads: input.source_heads.iter().map(WorldCommitRef::as_str).collect(),
        outputs,
        conflicts: input
            .conflicts
            .iter()
            .map(|conflict| (conflict.kind.as_str(), conflict.key.as_deref(), conflict.code))
            .collect(),
        migrations: input
            .migrations
            .iter()
            .map(|evidence| (evidence.conversion().conversion_ref(), evidence.result_ref()))
            .collect(),
    };
    WorldMergePlanRef::new(canonical_ref(&value, "world-merge-plan-identity-v3")?)
        .map_err(|_| WorldMergeIssue::InvalidBounds("plan_identity"))
}

// r[impl molten.audit_f06.publication]
// r[impl molten.audit_f13.compatibility]
pub fn validate_world_merge_plan(plan: &WorldMergePlan) -> Result<(), WorldMergeIssue> {
    let expected = identify_plan(PlanIdentityInput {
        original_request: &plan.original_request_ref,
        base_head: &plan.base_head,
        source_heads: &plan.source_heads,
        outputs: &plan.outputs,
        conflicts: &plan.conflicts,
        migrations: &plan.migrations,
    })?;
    if expected != plan.plan_ref {
        return Err(WorldMergeIssue::PlanIdentityMismatch);
    }
    Ok(())
}
