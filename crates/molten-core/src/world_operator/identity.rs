use super::*;

const WORLD_WORKFLOW_PLAN_IDENTITY_DOMAIN: &str = "onixresearch.molten.world-workflow.plan.v1";
const WORLD_WORKFLOW_RECEIPT_IDENTITY_DOMAIN: &str = "onixresearch.molten.world-workflow.receipt.v1";
const WORLD_WORKFLOW_SUMMARY_IDENTITY_DOMAIN: &str = "onixresearch.molten.world-workflow.summary.v1";

pub fn identify_world_workflow_plan(
    request: &WorldWorkflowRequest,
    operations: &[WorldOperationPlanNode],
    first_blocker: Option<&WorldWorkflowBlocker>,
) -> Result<String, WorldWorkflowIssue> {
    let mut hasher = blake3::Hasher::new_derive_key(WORLD_WORKFLOW_PLAN_IDENTITY_DOMAIN);
    update_text(&mut hasher, WORLD_WORKFLOW_PLAN_SCHEMA)?;
    update_request_facts(&mut hasher, request)?;
    update_usize(&mut hasher, operations.len())?;
    for operation in operations {
        update_plan_operation(&mut hasher, operation)?;
    }
    update_optional_blocker(&mut hasher, first_blocker)?;
    update_non_claims(&mut hasher)?;
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

pub fn identify_world_workflow_receipt(receipt: &WorldWorkflowReceipt) -> Result<String, WorldWorkflowIssue> {
    let mut hasher = blake3::Hasher::new_derive_key(WORLD_WORKFLOW_RECEIPT_IDENTITY_DOMAIN);
    update_text(&mut hasher, receipt.schema)?;
    update_text(&mut hasher, &receipt.plan_ref)?;
    update_text(&mut hasher, receipt.completion.as_str())?;
    update_usize(&mut hasher, receipt.links.len())?;
    for link in &receipt.links {
        update_receipt_link(&mut hasher, link)?;
    }
    update_optional_blocker(&mut hasher, receipt.first_blocker.as_ref())?;
    update_strings(&mut hasher, &receipt.non_claims)?;
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

pub fn identify_world_workflow_summary(summary: &WorldWorkflowSummary) -> Result<String, WorldWorkflowIssue> {
    let mut hasher = blake3::Hasher::new_derive_key(WORLD_WORKFLOW_SUMMARY_IDENTITY_DOMAIN);
    update_text(&mut hasher, summary.schema)?;
    update_text(&mut hasher, &summary.plan_ref)?;
    update_text(&mut hasher, summary.completion.as_str())?;
    update_usize(&mut hasher, summary.operation_count)?;
    update_usize(&mut hasher, summary.linked_receipt_count)?;
    update_optional_blocker(&mut hasher, summary.first_blocker.as_ref())?;
    update_strings(&mut hasher, &summary.non_claims)?;
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

fn update_request_facts(hasher: &mut blake3::Hasher, request: &WorldWorkflowRequest) -> Result<(), WorldWorkflowIssue> {
    update_text(hasher, &request.schema)?;
    update_text(hasher, &request.request_ref)?;
    update_text(hasher, request.world_ref.as_str())?;
    update_text(hasher, request.branch_id.as_str())?;
    update_text(hasher, request.expected_head.as_str())?;
    update_number(hasher, request.expected_generation);
    update_text(hasher, request.policy_ref.as_str())?;
    update_text(hasher, &request.authority_observation_ref)?;
    update_limits(hasher, &request.limits)?;
    update_profiles(hasher, &request.profiles)?;
    update_observations(hasher, &request.observations)
}

fn update_limits(hasher: &mut blake3::Hasher, limits: &WorldWorkflowLimits) -> Result<(), WorldWorkflowIssue> {
    update_text(hasher, &limits.limits_ref)?;
    update_usize(hasher, limits.max_operations)?;
    update_usize(hasher, limits.max_dependencies_per_operation)?;
    update_usize(hasher, limits.max_receipt_links)?;
    update_usize(hasher, limits.max_canonical_bytes)
}

fn update_profiles(hasher: &mut blake3::Hasher, profiles: &[WorldProfileCapability]) -> Result<(), WorldWorkflowIssue> {
    let mut profiles = profiles.to_vec();
    profiles.sort_by(|left, right| left.profile_ref.cmp(&right.profile_ref));
    update_usize(hasher, profiles.len())?;
    for profile in profiles {
        update_text(hasher, &profile.profile_ref)?;
        update_text(hasher, profile.kind.as_str())?;
        update_text(hasher, profile.status.as_str())?;
        update_text(hasher, &profile.status_ref)?;
    }
    Ok(())
}

fn update_observations(
    hasher: &mut blake3::Hasher,
    observations: &[WorldExpectedObservation],
) -> Result<(), WorldWorkflowIssue> {
    let mut observations = observations.to_vec();
    observations.sort_by(|left, right| {
        (left.kind, left.observation_ref.as_str()).cmp(&(right.kind, right.observation_ref.as_str()))
    });
    update_usize(hasher, observations.len())?;
    for observation in observations {
        update_text(hasher, observation.kind.as_str())?;
        update_text(hasher, &observation.observation_ref)?;
        update_text(hasher, &observation.subject_ref)?;
        update_bool(hasher, observation.admitted);
    }
    Ok(())
}

fn update_plan_operation(
    hasher: &mut blake3::Hasher,
    operation: &WorldOperationPlanNode,
) -> Result<(), WorldWorkflowIssue> {
    update_text(hasher, &operation.operation_id)?;
    update_text(hasher, operation.kind.as_str())?;
    update_text(hasher, &operation.subject_ref)?;
    update_text(hasher, &operation.profile_ref)?;
    update_strings(hasher, &operation.dependencies)?;
    update_text(hasher, operation.state.as_str())?;
    update_optional_blocker(hasher, operation.blocker.as_ref())
}

fn update_receipt_link(hasher: &mut blake3::Hasher, link: &WorldReceiptLink) -> Result<(), WorldWorkflowIssue> {
    update_text(hasher, &link.operation_id)?;
    update_text(hasher, link.kind.as_str())?;
    update_text(hasher, link.owner.as_str())?;
    update_text(hasher, link.role.as_str())?;
    update_text(hasher, &link.component_ref)?;
    update_text(hasher, link.state.as_str())?;
    update_bool(hasher, link.sensitive_material_present);
    update_bool(hasher, link.claims_authority);
    update_bool(hasher, link.claims_deletion_authority);
    Ok(())
}

fn update_optional_blocker(
    hasher: &mut blake3::Hasher,
    blocker: Option<&WorldWorkflowBlocker>,
) -> Result<(), WorldWorkflowIssue> {
    match blocker {
        Some(blocker) => {
            update_bool(hasher, true);
            update_text(hasher, &blocker.operation_id)?;
            update_text(hasher, blocker.code.as_str())?;
            update_optional_text(hasher, blocker.evidence_ref.as_deref())
        }
        None => {
            update_bool(hasher, false);
            Ok(())
        }
    }
}

fn update_non_claims(hasher: &mut blake3::Hasher) -> Result<(), WorldWorkflowIssue> {
    let non_claims = world_operator_non_claims();
    update_strings(hasher, &non_claims)
}

fn update_strings(hasher: &mut blake3::Hasher, values: &[String]) -> Result<(), WorldWorkflowIssue> {
    update_usize(hasher, values.len())?;
    for value in values {
        update_text(hasher, value)?;
    }
    Ok(())
}

fn update_optional_text(hasher: &mut blake3::Hasher, value: Option<&str>) -> Result<(), WorldWorkflowIssue> {
    match value {
        Some(value) => {
            update_bool(hasher, true);
            update_text(hasher, value)
        }
        None => {
            update_bool(hasher, false);
            Ok(())
        }
    }
}

fn update_usize(hasher: &mut blake3::Hasher, value: usize) -> Result<(), WorldWorkflowIssue> {
    let value = u64::try_from(value).map_err(|_| WorldWorkflowIssue::IdentityLengthExceeded)?;
    update_number(hasher, value);
    Ok(())
}

fn update_text(hasher: &mut blake3::Hasher, value: &str) -> Result<(), WorldWorkflowIssue> {
    let length = u64::try_from(value.len()).map_err(|_| WorldWorkflowIssue::IdentityLengthExceeded)?;
    update_number(hasher, length);
    hasher.update(value.as_bytes());
    Ok(())
}

fn update_number(hasher: &mut blake3::Hasher, value: u64) {
    hasher.update(&value.to_be_bytes());
}

fn update_bool(hasher: &mut blake3::Hasher, value: bool) {
    hasher.update(&[u8::from(value)]);
}
