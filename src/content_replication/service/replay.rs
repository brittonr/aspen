use super::*;

// r[impl molten.audit_f04.replay_preservation]
pub(super) fn index_replays<'a>(
    plan: &Plan,
    history: &'a [PriorOperation],
) -> crate::error::Result<std::collections::BTreeMap<&'a str, &'a PriorOperation>> {
    let mut indexed = std::collections::BTreeMap::new();
    for operation in history {
        if indexed.insert(operation.operation_id.as_str(), operation).is_some() {
            return Err(crate::error::Failure::invalid_harness("replication replay has duplicate operation identity"));
        }
    }
    for action in &plan.actions {
        if !is_data_action(action) {
            continue;
        }
        let Some(operation) = indexed.get(action.operation_id.as_str()) else {
            continue;
        };
        if !operation_matches_plan_action(plan, action, operation)
            || operation.outcome == OperationOutcome::Planned
            || operation.result_ref.as_deref().is_some_and(|reference| !valid_ref(reference))
            || (operation.outcome == OperationOutcome::Verified && operation.result_ref.is_none())
        {
            return Err(crate::error::Failure::invalid_harness("replication saved operation binding conflicts"));
        }
    }
    Ok(indexed)
}

pub(super) fn is_data_action(action: &Action) -> bool {
    matches!(action.kind, ActionKind::Transfer | ActionKind::Repair | ActionKind::Handoff)
}
