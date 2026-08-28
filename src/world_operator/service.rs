use molten_core::world_operator::*;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;

const MAX_LINKS_PER_EXECUTED_OPERATION: usize = 2;

#[derive(Debug)]
pub struct WorldOperatorRun {
    pub plan: WorldWorkflowPlan,
    pub request_record: CanonicalWorldOperatorRecord,
    pub plan_record: CanonicalWorldOperatorRecord,
    pub receipt: WorldWorkflowReceipt,
    pub receipt_record: CanonicalWorldOperatorRecord,
    pub summary: WorldWorkflowSummary,
    pub summary_record: CanonicalWorldOperatorRecord,
    pub rendered_summary: String,
}

// r[impl molten.world_operator.plan]
pub fn plan_world_operator_request(request: &WorldWorkflowRequest) -> Result<WorldOperatorRun> {
    let plan = core_plan(request)?;
    build_run(request, plan, Vec::new(), None)
}

// r[impl molten.world_operator.composition]
pub fn preview_world_operator_with_handlers(
    request: &WorldWorkflowRequest,
    handlers: &mut [&mut dyn WorldOperationHandler],
) -> Result<WorldOperatorRun> {
    validate_handler_registry(handlers)?;
    let plan = core_plan(request)?;
    let mut links = Vec::with_capacity(plan.operations.len());
    let mut shell_blocker = None;
    for operation in &plan.operations {
        if operation.state == WorldOperationPlanState::Blocked {
            break;
        }
        let Some(handler) = handler_for(handlers, operation.kind) else {
            shell_blocker = Some(handler_unavailable(operation));
            break;
        };
        let link = handler.preview(&plan, operation)?;
        let should_stop = link.state != WorldComponentCompletionState::Planned;
        links.push(link);
        if should_stop {
            break;
        }
    }
    build_run(request, plan, links, shell_blocker)
}

// r[impl molten.world_operator.preview_apply]
pub fn apply_world_operator_with_handlers(
    request: &WorldWorkflowRequest,
    submitted_plan_ref: &str,
    handlers: &mut [&mut dyn WorldOperationHandler],
    current_facts: &mut dyn WorldOperationCurrentFactsPort,
) -> Result<WorldOperatorRun> {
    validate_handler_registry(handlers)?;
    let plan = core_plan(request)?;
    if submitted_plan_ref != plan.plan_ref {
        let blocker = stale_plan_blocker(&plan, submitted_plan_ref);
        return build_run(request, plan, Vec::new(), blocker);
    }
    execute_plan(request, plan, handlers, current_facts)
}

pub fn publish_world_operator_run(
    run: &WorldOperatorRun,
    port: &mut dyn WorldWorkflowRecordPort,
) -> Result<Vec<String>> {
    let records = [
        &run.request_record,
        &run.plan_record,
        &run.receipt_record,
        &run.summary_record,
    ];
    records.into_iter().map(|record| port.publish(record)).collect()
}

fn execute_plan(
    request: &WorldWorkflowRequest,
    plan: WorldWorkflowPlan,
    handlers: &mut [&mut dyn WorldOperationHandler],
    current_facts: &mut dyn WorldOperationCurrentFactsPort,
) -> Result<WorldOperatorRun> {
    let mut links = Vec::with_capacity(plan.operations.len().saturating_mul(MAX_LINKS_PER_EXECUTED_OPERATION));
    let mut shell_blocker = None;
    for operation in &plan.operations {
        if operation.state == WorldOperationPlanState::Blocked {
            break;
        }
        let Some(handler) = handler_for(handlers, operation.kind) else {
            shell_blocker = Some(handler_unavailable(operation));
            break;
        };
        let preview = handler.preview(&plan, operation)?;
        let preview_state = preview.state;
        links.push(preview);
        if preview_state != WorldComponentCompletionState::Planned {
            break;
        }
        let admission = if operation.kind.is_mutating() {
            match current_admission(&plan, operation, current_facts)? {
                Ok(admission) => Some(admission),
                Err(blocker) => {
                    shell_blocker = Some(blocker);
                    break;
                }
            }
        } else {
            None
        };
        let outcome = handler.execute(&plan, operation, admission.as_ref())?;
        let outcome_state = outcome.state;
        links.push(outcome);
        if outcome_state != WorldComponentCompletionState::Complete {
            break;
        }
    }
    build_run(request, plan, links, shell_blocker)
}

fn current_admission(
    plan: &WorldWorkflowPlan,
    operation: &WorldOperationPlanNode,
    current_facts: &mut dyn WorldOperationCurrentFactsPort,
) -> Result<std::result::Result<WorldOperationApplyAdmission, WorldWorkflowBlocker>> {
    let facts = current_facts.observe_current(plan, operation)?;
    match admit_world_operation_apply(plan, &facts) {
        Ok(admission) => Ok(Ok(admission)),
        Err(_) => Ok(Err(WorldWorkflowBlocker {
            operation_id: operation.operation_id.clone(),
            code: WorldWorkflowBlockerCode::MutableObservationDrift,
            evidence_ref: Some(facts.authority_observation_ref),
        })),
    }
}

fn core_plan(request: &WorldWorkflowRequest) -> Result<WorldWorkflowPlan> {
    plan_world_workflow(request)
        .map_err(|issues| MoltenError::invalid_harness(format!("world workflow planning denied: {issues:?}")))
}

fn build_run(
    request: &WorldWorkflowRequest,
    plan: WorldWorkflowPlan,
    links: Vec<WorldReceiptLink>,
    shell_blocker: Option<WorldWorkflowBlocker>,
) -> Result<WorldOperatorRun> {
    let receipt = build_world_workflow_receipt(&plan, links, shell_blocker)
        .map_err(|issues| MoltenError::invalid_harness(format!("world workflow receipt denied: {issues:?}")))?;
    let summary = summarize_world_workflow(&plan, &receipt)
        .map_err(|issue| MoltenError::invalid_harness(format!("world workflow summary denied: {issue:?}")))?;
    let rendered_summary = render_world_workflow_summary(&summary)
        .map_err(|issue| MoltenError::invalid_harness(format!("world workflow rendering denied: {issue:?}")))?;
    let request_record = canonical_world_workflow_request(request, &plan)?;
    let plan_record = canonical_world_workflow_plan(&plan)?;
    let receipt_record = canonical_world_workflow_receipt(&receipt)?;
    let summary_record = canonical_world_workflow_summary(&summary)?;
    Ok(WorldOperatorRun {
        plan,
        request_record,
        plan_record,
        receipt,
        receipt_record,
        summary,
        summary_record,
        rendered_summary,
    })
}

fn validate_handler_registry(handlers: &[&mut dyn WorldOperationHandler]) -> Result<()> {
    let mut kinds = std::collections::BTreeSet::new();
    for handler in handlers {
        let kind = handler.kind();
        if handler.owner() != kind.owner() {
            return Err(MoltenError::invalid_harness(
                "world workflow handler registry crosses a component owner boundary",
            ));
        }
        if !kinds.insert(kind) {
            return Err(MoltenError::invalid_harness(
                "world workflow handler registry contains a duplicate operation kind",
            ));
        }
    }
    Ok(())
}

fn handler_for<'a>(
    handlers: &'a mut [&mut dyn WorldOperationHandler],
    kind: WorldOperationKind,
) -> Option<&'a mut dyn WorldOperationHandler> {
    for handler in handlers {
        if handler.kind() == kind {
            return Some(&mut **handler);
        }
    }
    None
}

fn handler_unavailable(operation: &WorldOperationPlanNode) -> WorldWorkflowBlocker {
    WorldWorkflowBlocker {
        operation_id: operation.operation_id.clone(),
        code: WorldWorkflowBlockerCode::HandlerUnavailable,
        evidence_ref: None,
    }
}

fn stale_plan_blocker(plan: &WorldWorkflowPlan, submitted_plan_ref: &str) -> Option<WorldWorkflowBlocker> {
    plan.operations
        .iter()
        .find(|operation| operation.kind.is_mutating())
        .or_else(|| plan.operations.first())
        .map(|operation| WorldWorkflowBlocker {
            operation_id: operation.operation_id.clone(),
            code: WorldWorkflowBlockerCode::StalePlan,
            evidence_ref: canonical_ref(submitted_plan_ref),
        })
}

fn canonical_ref(value: &str) -> Option<String> {
    crate::preserves_rail::validate_content_ref(value).ok().map(|()| value.to_string())
}
