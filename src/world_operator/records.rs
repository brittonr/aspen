use molten_core::world_operator::*;
use preserves::IOValue;

use crate::error::MoltenError;
use crate::error::Result;

pub const WORLD_WORKFLOW_REQUEST_RECORD: &str = "molten-world-workflow-request-v1";
pub const WORLD_WORKFLOW_PLAN_RECORD: &str = "molten-world-workflow-plan-v1";
pub const WORLD_WORKFLOW_RECEIPT_RECORD: &str = "molten-world-workflow-receipt-v1";
pub const WORLD_WORKFLOW_SUMMARY_RECORD: &str = "molten-world-workflow-summary-v1";

const WORLD_OPERATOR_RECORD_CONTEXT: &str = "onixresearch.molten.world-workflow.record.v1";

#[derive(Debug, Clone)]
pub struct CanonicalWorldOperatorRecord {
    pub kind: &'static str,
    pub record_ref: String,
    pub value: IOValue,
    pub bytes: Vec<u8>,
}

// r[impl molten.world_operator.receipt]
pub fn canonical_world_workflow_request(
    request: &WorldWorkflowRequest,
    plan: &WorldWorkflowPlan,
) -> Result<CanonicalWorldOperatorRecord> {
    core_issues(validate_world_workflow_request(request), "request")?;
    let mut profiles = request.profiles.clone();
    profiles.sort_by(|left, right| left.profile_ref.cmp(&right.profile_ref));
    let mut observations = request.observations.clone();
    observations.sort_by(|left, right| {
        (left.kind, left.observation_ref.as_str()).cmp(&(right.kind, right.observation_ref.as_str()))
    });
    canonical(
        "request",
        WORLD_WORKFLOW_REQUEST_RECORD,
        record(WORLD_WORKFLOW_REQUEST_RECORD, vec![
            string(&request.schema),
            field("request-ref", string(&request.request_ref)),
            field("world-ref", string(request.world_ref.as_str())),
            field("branch-id", string(request.branch_id.as_str())),
            field("expected-head", string(request.expected_head.as_str())),
            field("expected-generation", number(request.expected_generation)),
            field("policy-ref", string(request.policy_ref.as_str())),
            field("authority-observation-ref", string(&request.authority_observation_ref)),
            limits_value(&request.limits),
            field("profiles", sequence(profiles.iter().map(profile_value).collect())),
            field("observations", sequence(observations.iter().map(observation_value).collect())),
            field("operations", sequence(plan.operations.iter().map(plan_operation_value).collect())),
        ]),
    )
}

pub fn canonical_world_workflow_plan(plan: &WorldWorkflowPlan) -> Result<CanonicalWorldOperatorRecord> {
    if plan.schema != WORLD_WORKFLOW_PLAN_SCHEMA || plan.non_claims != world_operator_non_claims() {
        return Err(MoltenError::invalid_harness("world workflow plan schema or non-claims are invalid"));
    }
    canonical(
        "plan",
        WORLD_WORKFLOW_PLAN_RECORD,
        record(WORLD_WORKFLOW_PLAN_RECORD, vec![
            string(plan.schema),
            field("plan-ref", string(&plan.plan_ref)),
            field("request-ref", string(&plan.request_ref)),
            field("world-ref", string(plan.world_ref.as_str())),
            field("branch-id", string(plan.branch_id.as_str())),
            field("expected-head", string(plan.expected_head.as_str())),
            field("expected-generation", number(plan.expected_generation)),
            field("policy-ref", string(plan.policy_ref.as_str())),
            field("authority-observation-ref", string(&plan.authority_observation_ref)),
            field("limits-ref", string(&plan.limits_ref)),
            field("operations", sequence(plan.operations.iter().map(plan_operation_value).collect())),
            field("first-blocker", optional_blocker_value(plan.first_blocker.as_ref())),
            non_claims_value(&plan.non_claims),
        ]),
    )
}

pub fn canonical_world_workflow_receipt(receipt: &WorldWorkflowReceipt) -> Result<CanonicalWorldOperatorRecord> {
    let identity = identify_world_workflow_receipt(receipt).map_err(core_issue)?;
    if receipt.schema != WORLD_WORKFLOW_RECEIPT_SCHEMA
        || receipt.receipt_ref != identity
        || receipt.non_claims != world_operator_non_claims()
    {
        return Err(MoltenError::invalid_harness("world workflow receipt identity or non-claims are invalid"));
    }
    canonical(
        "receipt",
        WORLD_WORKFLOW_RECEIPT_RECORD,
        record(WORLD_WORKFLOW_RECEIPT_RECORD, vec![
            string(receipt.schema),
            field("receipt-ref", string(&receipt.receipt_ref)),
            field("plan-ref", string(&receipt.plan_ref)),
            field("links", sequence(receipt.links.iter().map(receipt_link_value).collect())),
            field("completion", string(receipt.completion.as_str())),
            field("first-blocker", optional_blocker_value(receipt.first_blocker.as_ref())),
            non_claims_value(&receipt.non_claims),
        ]),
    )
}

pub fn canonical_world_workflow_summary(summary: &WorldWorkflowSummary) -> Result<CanonicalWorldOperatorRecord> {
    let identity = identify_world_workflow_summary(summary).map_err(core_issue)?;
    if summary.schema != WORLD_WORKFLOW_SUMMARY_SCHEMA
        || summary.summary_ref != identity
        || summary.non_claims != world_operator_non_claims()
    {
        return Err(MoltenError::invalid_harness("world workflow summary identity or non-claims are invalid"));
    }
    canonical(
        "summary",
        WORLD_WORKFLOW_SUMMARY_RECORD,
        record(WORLD_WORKFLOW_SUMMARY_RECORD, vec![
            string(summary.schema),
            field("summary-ref", string(&summary.summary_ref)),
            field("plan-ref", string(&summary.plan_ref)),
            field("completion", string(summary.completion.as_str())),
            field("operation-count", usize_value(summary.operation_count)?),
            field("linked-receipt-count", usize_value(summary.linked_receipt_count)?),
            field("first-blocker", optional_blocker_value(summary.first_blocker.as_ref())),
            non_claims_value(&summary.non_claims),
        ]),
    )
}

fn limits_value(limits: &WorldWorkflowLimits) -> IOValue {
    record("limits", vec![
        field("limits-ref", string(&limits.limits_ref)),
        field("max-operations", usize_value_lossless(limits.max_operations)),
        field("max-dependencies-per-operation", usize_value_lossless(limits.max_dependencies_per_operation)),
        field("max-receipt-links", usize_value_lossless(limits.max_receipt_links)),
        field("max-canonical-bytes", usize_value_lossless(limits.max_canonical_bytes)),
    ])
}

fn profile_value(profile: &WorldProfileCapability) -> IOValue {
    record("profile", vec![
        field("profile-ref", string(&profile.profile_ref)),
        field("kind", string(profile.kind.as_str())),
        field("status", string(profile.status.as_str())),
        field("status-ref", string(&profile.status_ref)),
    ])
}

fn observation_value(observation: &WorldExpectedObservation) -> IOValue {
    record("observation", vec![
        field("kind", string(observation.kind.as_str())),
        field("observation-ref", string(&observation.observation_ref)),
        field("subject-ref", string(&observation.subject_ref)),
        field("admitted", boolean(observation.admitted)),
    ])
}

fn plan_operation_value(operation: &WorldOperationPlanNode) -> IOValue {
    record("operation", vec![
        field("operation-id", string(&operation.operation_id)),
        field("kind", string(operation.kind.as_str())),
        field("subject-ref", string(&operation.subject_ref)),
        field("profile-ref", string(&operation.profile_ref)),
        field("dependencies", sequence(operation.dependencies.iter().map(string).collect())),
        field("state", string(operation.state.as_str())),
        field("blocker", optional_blocker_value(operation.blocker.as_ref())),
    ])
}

fn receipt_link_value(link: &WorldReceiptLink) -> IOValue {
    record("link", vec![
        field("operation-id", string(&link.operation_id)),
        field("kind", string(link.kind.as_str())),
        field("owner", string(link.owner.as_str())),
        field("role", string(link.role.as_str())),
        field("component-ref", string(&link.component_ref)),
        field("state", string(link.state.as_str())),
        field("sensitive-material-present", boolean(link.sensitive_material_present)),
        field("claims-authority", boolean(link.claims_authority)),
        field("claims-deletion-authority", boolean(link.claims_deletion_authority)),
    ])
}

fn optional_blocker_value(blocker: Option<&WorldWorkflowBlocker>) -> IOValue {
    blocker.map_or_else(
        || record("none", Vec::new()),
        |blocker| {
            record("some", vec![record("blocker", vec![
                field("operation-id", string(&blocker.operation_id)),
                field("code", string(blocker.code.as_str())),
                field("evidence-ref", optional_ref(blocker.evidence_ref.as_deref())),
            ])])
        },
    )
}

fn canonical(identity_kind: &str, record_kind: &'static str, value: IOValue) -> Result<CanonicalWorldOperatorRecord> {
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    if bytes.len() > MAX_WORLD_OPERATOR_CANONICAL_BYTES {
        return Err(MoltenError::invalid_harness("world workflow canonical record exceeds the byte bound"));
    }
    let mut hasher = blake3::Hasher::new_derive_key(WORLD_OPERATOR_RECORD_CONTEXT);
    update(&mut hasher, identity_kind)?;
    let length = u64::try_from(bytes.len())
        .map_err(|_| MoltenError::invalid_harness("world workflow canonical length exceeds u64"))?;
    hasher.update(&length.to_be_bytes());
    hasher.update(&bytes);
    Ok(CanonicalWorldOperatorRecord {
        kind: record_kind,
        record_ref: format!("blake3:{}", hasher.finalize().to_hex()),
        value,
        bytes,
    })
}

fn core_issues(issues: Vec<WorldWorkflowIssue>, kind: &str) -> Result<()> {
    if issues.is_empty() {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("world workflow {kind} denied: {issues:?}")))
    }
}

fn core_issue(issue: WorldWorkflowIssue) -> MoltenError {
    MoltenError::invalid_harness(format!("world workflow identity denied: {issue:?}"))
}

fn update(hasher: &mut blake3::Hasher, value: &str) -> Result<()> {
    let length = u64::try_from(value.len())
        .map_err(|_| MoltenError::invalid_harness("world workflow identity field exceeds u64"))?;
    hasher.update(&length.to_be_bytes());
    hasher.update(value.as_bytes());
    Ok(())
}

fn non_claims_value(non_claims: &[String]) -> IOValue {
    field("non-claims", sequence(non_claims.iter().map(string).collect()))
}

fn optional_ref(reference: Option<&str>) -> IOValue {
    reference.map_or_else(|| record("none", Vec::new()), |reference| record("some", vec![string(reference)]))
}

fn boolean(value: bool) -> IOValue {
    record(if value { "true" } else { "false" }, Vec::new())
}

fn usize_value(value: usize) -> Result<IOValue> {
    let value = u64::try_from(value).map_err(|_| MoltenError::invalid_harness("world workflow count exceeds u64"))?;
    Ok(number(value))
}

fn usize_value_lossless(value: usize) -> IOValue {
    u64::try_from(value).map_or_else(|_| number(u64::MAX), number)
}

fn number(value: u64) -> IOValue {
    crate::preserves_rail::u64_value(value)
}

fn field(label: &'static str, value: IOValue) -> IOValue {
    record(label, vec![value])
}

fn string(value: impl AsRef<str>) -> IOValue {
    crate::preserves_rail::string(value.as_ref())
}

fn sequence(values: Vec<IOValue>) -> IOValue {
    crate::preserves_rail::sequence(values)
}

fn record(label: &'static str, fields: Vec<IOValue>) -> IOValue {
    crate::preserves_rail::record(label, fields)
}
