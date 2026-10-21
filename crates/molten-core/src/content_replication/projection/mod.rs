use super::*;

struct ActionProgress<'a> {
    verified_targets: std::collections::BTreeMap<&'a str, std::collections::BTreeSet<&'a str>>,
    active_operations: Vec<String>,
    failures: Vec<String>,
}

// r[impl molten.audit_f04.current_availability]
// r[impl molten.audit_f07.deficit_status]
// r[impl molten.audit_f07.per_content_accounting]
pub fn status(plan: &Plan, history: &[PriorOperation]) -> Status {
    let progress = action_progress(plan, history);
    let mut under_replicated = Vec::with_capacity(plan.replica_deficits.len());
    let mut verified_replicas = plan.verified_replicas;
    for deficit in &plan.replica_deficits {
        let verified = progress
            .verified_targets
            .get(deficit.content_ref.as_str())
            .map_or(0, std::collections::BTreeSet::len);
        verified_replicas = verified_replicas.saturating_add(verified.min(deficit.missing_replicas));
        if verified < deficit.missing_replicas {
            under_replicated.push(deficit.content_ref.clone());
        }
    }
    Status {
        plan_ref: plan.plan_ref.clone(),
        generation: plan.generation,
        placement_epoch: plan.placement_epoch,
        desired_replicas: plan.desired_replicas,
        verified_replicas: verified_replicas.min(plan.desired_replicas),
        under_replicated,
        active_operations: progress.active_operations,
        failures: progress.failures,
        pins: plan.required_pins.clone(),
        non_claims: plan.non_claims.clone(),
    }
}

// r[impl molten.audit_f04.replay_preservation]
pub fn operation_matches_plan_action(plan: &Plan, action: &Action, operation: &PriorOperation) -> bool {
    operation.operation_id == action.operation_id
        && operation.content_ref == action.content_ref
        && operation.source_peer == action.source_peer
        && operation.target_peer == action.target_peer
        && operation.generation == plan.generation
        && operation.membership_epoch == plan.membership_epoch
        && operation.placement_epoch == plan.placement_epoch
        && operation.attempt == action.attempt
}

fn action_progress<'a>(plan: &'a Plan, history: &[PriorOperation]) -> ActionProgress<'a> {
    let mut outcomes = std::collections::BTreeMap::new();
    for operation in history {
        outcomes
            .entry(operation.operation_id.as_str())
            .and_modify(|entry| *entry = None)
            .or_insert(Some(operation));
    }
    let mut progress = ActionProgress {
        verified_targets: std::collections::BTreeMap::new(),
        active_operations: Vec::with_capacity(plan.actions.len()),
        failures: Vec::with_capacity(plan.actions.len()),
    };
    for action in &plan.actions {
        if action.kind == ActionKind::Defer {
            continue;
        }
        let operation = outcomes
            .get(action.operation_id.as_str())
            .copied()
            .flatten()
            .filter(|operation| operation_matches_plan_action(plan, action, operation));
        match operation {
            Some(operation)
                if operation.outcome == OperationOutcome::Verified
                    && operation.result_ref.as_deref().is_some_and(valid_ref) =>
            {
                if action.kind != ActionKind::Cleanup {
                    progress
                        .verified_targets
                        .entry(action.content_ref.as_str())
                        .or_default()
                        .insert(action.target_peer.as_str());
                }
            }
            Some(operation) if operation.outcome != OperationOutcome::Planned => {
                progress.failures.push(action.operation_id.clone());
            }
            _ if action.kind != ActionKind::Cleanup => progress.active_operations.push(action.operation_id.clone()),
            _ => {}
        }
    }
    progress.active_operations.sort();
    progress.failures.sort();
    progress
}
