use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::*;

const BLAKE3_REFERENCE_PREFIX: &str = "blake3:";
const BLAKE3_HEX_DIGITS: usize = 64;
const PLAN_AND_SHELL_BLOCKER_CAPACITY: usize = 2;

// r[impl molten.world_operator.receipt]
pub fn build_world_workflow_receipt(
    plan: &WorldWorkflowPlan,
    links: Vec<WorldReceiptLink>,
    shell_blocker: Option<WorldWorkflowBlocker>,
) -> Result<WorldWorkflowReceipt, Vec<WorldWorkflowIssue>> {
    let mut issues = validate_links(plan, &links);
    let first_blocker = select_first_blocker(plan, &links, shell_blocker);
    validate_blocker_boundary(plan, &links, first_blocker.as_ref(), &mut issues);
    if !issues.is_empty() {
        issues.sort();
        issues.dedup();
        return Err(issues);
    }
    let completion = classify_completion(plan, &links, first_blocker.as_ref());
    let mut receipt = WorldWorkflowReceipt {
        schema: WORLD_WORKFLOW_RECEIPT_SCHEMA,
        receipt_ref: String::new(),
        plan_ref: plan.plan_ref.clone(),
        links,
        completion,
        first_blocker,
        non_claims: world_operator_non_claims(),
    };
    receipt.receipt_ref = identify_world_workflow_receipt(&receipt).map_err(|issue| vec![issue])?;
    Ok(receipt)
}

pub fn summarize_world_workflow(
    plan: &WorldWorkflowPlan,
    receipt: &WorldWorkflowReceipt,
) -> Result<WorldWorkflowSummary, WorldWorkflowIssue> {
    let mut summary = WorldWorkflowSummary {
        schema: WORLD_WORKFLOW_SUMMARY_SCHEMA,
        summary_ref: String::new(),
        plan_ref: plan.plan_ref.clone(),
        completion: receipt.completion,
        operation_count: plan.operations.len(),
        linked_receipt_count: receipt.links.len(),
        first_blocker: receipt.first_blocker.clone(),
        non_claims: world_operator_non_claims(),
    };
    summary.summary_ref = identify_world_workflow_summary(&summary)?;
    Ok(summary)
}

// r[impl molten.world_operator.diagnostics]
pub fn render_world_workflow_summary(summary: &WorldWorkflowSummary) -> Result<String, WorldWorkflowIssue> {
    let blocker = summary
        .first_blocker
        .as_ref()
        .map_or_else(|| "none".to_string(), |blocker| format!("{}:{}", blocker.operation_id, blocker.code.as_str()));
    let rendered = format!(
        "plan_ref={} summary_ref={} completion={} operations={} links={} first_blocker={} authority_granted=false release_eligible=false deletion_authorized=false",
        summary.plan_ref,
        summary.summary_ref,
        summary.completion.as_str(),
        summary.operation_count,
        summary.linked_receipt_count,
        blocker,
    );
    if rendered.len() > MAX_WORLD_OPERATOR_TEXT_BYTES {
        Err(WorldWorkflowIssue::IdentityLengthExceeded)
    } else {
        Ok(rendered)
    }
}

fn validate_links(plan: &WorldWorkflowPlan, links: &[WorldReceiptLink]) -> Vec<WorldWorkflowIssue> {
    let mut issues = Vec::with_capacity(MAX_WORLD_OPERATOR_DIAGNOSTICS);
    if links.len() > MAX_WORLD_OPERATOR_RECEIPT_LINKS {
        issues.push(WorldWorkflowIssue::ReceiptLimitExceeded);
        return issues;
    }
    let operation_positions = plan
        .operations
        .iter()
        .enumerate()
        .map(|(position, operation)| (operation.operation_id.as_str(), (position, operation.kind)))
        .collect::<BTreeMap<_, _>>();
    let mut last_position = None;
    for link in links {
        validate_link(link, &operation_positions, &mut last_position, &mut issues);
    }
    issues
}

#[allow(
    tigerstyle::borrowed_argument_types,
    reason = "the bounded validator appends typed issues to one preallocated local sink"
)]
fn validate_link(
    link: &WorldReceiptLink,
    operation_positions: &BTreeMap<&str, (usize, WorldOperationKind)>,
    last_position: &mut Option<usize>,
    issues: &mut Vec<WorldWorkflowIssue>,
) {
    let Some((position, kind)) = operation_positions.get(link.operation_id.as_str()).copied() else {
        issues.push(WorldWorkflowIssue::ReceiptOperationMissing(link.operation_id.clone()));
        return;
    };
    if link.kind != kind || last_position.is_some_and(|last| position < last) {
        issues.push(WorldWorkflowIssue::ReceiptOrderMismatch);
    }
    if link.owner != kind.owner() {
        issues.push(WorldWorkflowIssue::ReceiptOwnerMismatch);
    }
    *last_position = Some(position);
    if !is_content_ref(&link.operation_id) || !is_content_ref(&link.component_ref) {
        issues.push(WorldWorkflowIssue::InvalidReference("receipt-link"));
    }
    if link.claims_authority {
        issues.push(WorldWorkflowIssue::ReceiptOverclaimsAuthority);
    }
    if link.claims_deletion_authority {
        issues.push(WorldWorkflowIssue::ReceiptOverclaimsDeletionAuthority);
    }
    if link.sensitive_material_present {
        issues.push(WorldWorkflowIssue::ReceiptContainsSensitiveMaterial);
    }
}

fn select_first_blocker(
    plan: &WorldWorkflowPlan,
    links: &[WorldReceiptLink],
    shell_blocker: Option<WorldWorkflowBlocker>,
) -> Option<WorldWorkflowBlocker> {
    let mut blockers = Vec::with_capacity(links.len().saturating_add(PLAN_AND_SHELL_BLOCKER_CAPACITY));
    if let Some(blocker) = &plan.first_blocker {
        blockers.push(blocker.clone());
    }
    if let Some(blocker) = shell_blocker {
        blockers.push(blocker);
    }
    for link in links {
        let code = match link.state {
            WorldComponentCompletionState::Blocked => Some(WorldWorkflowBlockerCode::ComponentDenied),
            WorldComponentCompletionState::Unknown => Some(WorldWorkflowBlockerCode::ComponentOutcomeUnknown),
            WorldComponentCompletionState::Planned | WorldComponentCompletionState::Complete => None,
        };
        if let Some(code) = code {
            blockers.push(WorldWorkflowBlocker {
                operation_id: link.operation_id.clone(),
                code,
                evidence_ref: Some(link.component_ref.clone()),
            });
        }
    }
    blockers.into_iter().min_by_key(|blocker| blocker_position(plan, blocker))
}

fn blocker_position(plan: &WorldWorkflowPlan, blocker: &WorldWorkflowBlocker) -> usize {
    plan.operations
        .iter()
        .position(|operation| operation.operation_id == blocker.operation_id)
        .unwrap_or(plan.operations.len())
}

#[allow(
    tigerstyle::borrowed_argument_types,
    reason = "the bounded validator appends typed issues to one preallocated local sink"
)]
fn validate_blocker_boundary(
    plan: &WorldWorkflowPlan,
    links: &[WorldReceiptLink],
    first_blocker: Option<&WorldWorkflowBlocker>,
    issues: &mut Vec<WorldWorkflowIssue>,
) {
    let Some(first_blocker) = first_blocker else {
        return;
    };
    let boundary = blocker_position(plan, first_blocker);
    for link in links {
        let position = plan
            .operations
            .iter()
            .position(|operation| operation.operation_id == link.operation_id)
            .unwrap_or(plan.operations.len());
        if position > boundary {
            issues.push(WorldWorkflowIssue::ReceiptAfterBlocker);
        }
    }
}

fn classify_completion(
    plan: &WorldWorkflowPlan,
    links: &[WorldReceiptLink],
    first_blocker: Option<&WorldWorkflowBlocker>,
) -> WorldWorkflowCompletionState {
    if first_blocker.is_some_and(|blocker| blocker.code == WorldWorkflowBlockerCode::ComponentOutcomeUnknown) {
        return WorldWorkflowCompletionState::Unknown;
    }
    if first_blocker.is_some() {
        return WorldWorkflowCompletionState::Blocked;
    }
    let completed = links
        .iter()
        .filter(|link| link.state == WorldComponentCompletionState::Complete)
        .map(|link| link.operation_id.as_str())
        .collect::<BTreeSet<_>>();
    let planned = links
        .iter()
        .filter(|link| link.state == WorldComponentCompletionState::Planned)
        .map(|link| link.operation_id.as_str())
        .collect::<BTreeSet<_>>();
    if completed.len() == plan.operations.len() {
        WorldWorkflowCompletionState::Complete
    } else if planned.len() == plan.operations.len() || links.is_empty() {
        WorldWorkflowCompletionState::Planned
    } else {
        WorldWorkflowCompletionState::Partial
    }
}

fn is_content_ref(value: &str) -> bool {
    let Some(digest) = value.strip_prefix(BLAKE3_REFERENCE_PREFIX) else {
        return false;
    };
    digest.len() == BLAKE3_HEX_DIGITS && digest.bytes().all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}
