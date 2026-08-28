use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::super::*;

#[allow(
    tigerstyle::unbounded_collection_growth,
    reason = "request validation bounds every ordered map and set by MAX_WORLD_OPERATOR_OPERATIONS; BTreeMap has no reservation API"
)]
pub(super) fn order_operations(
    request: &WorldWorkflowRequest,
) -> Result<Vec<WorldOperationRequest>, Vec<WorldWorkflowIssue>> {
    if request.operations.len() > MAX_WORLD_OPERATOR_OPERATIONS {
        return Err(vec![WorldWorkflowIssue::OperationLimitExceeded]);
    }
    let mut operations = BTreeMap::new();
    let mut incoming = BTreeMap::new();
    let mut outgoing = BTreeMap::<String, Vec<String>>::new();
    for operation in &request.operations {
        operations.insert(operation.operation_id.clone(), operation.clone());
        incoming.insert(operation.operation_id.clone(), operation.dependencies.len());
        for dependency in &operation.dependencies {
            outgoing.entry(dependency.clone()).or_default().push(operation.operation_id.clone());
        }
    }
    for targets in outgoing.values_mut() {
        targets.sort();
    }
    let mut ready = initial_ready(&request.operations, &incoming);
    let mut ordered = Vec::with_capacity(request.operations.len());
    while let Some((_, operation_id)) = ready.pop_first() {
        let Some(operation) = operations.get(&operation_id).cloned() else {
            return Err(vec![WorldWorkflowIssue::MissingDependency(operation_id)]);
        };
        ordered.push(operation);
        release_dependents(&operation_id, &operations, &outgoing, &mut incoming, &mut ready)?;
    }
    if ordered.len() == request.operations.len() {
        Ok(ordered)
    } else {
        Err(vec![WorldWorkflowIssue::DependencyCycle])
    }
}

fn initial_ready(
    operations: &[WorldOperationRequest],
    incoming: &BTreeMap<String, usize>,
) -> BTreeSet<(WorldOperationKind, String)> {
    operations
        .iter()
        .filter(|operation| incoming.get(&operation.operation_id) == Some(&0))
        .map(|operation| (operation.kind, operation.operation_id.clone()))
        .collect()
}

fn release_dependents(
    operation_id: &str,
    operations: &BTreeMap<String, WorldOperationRequest>,
    outgoing: &BTreeMap<String, Vec<String>>,
    incoming: &mut BTreeMap<String, usize>,
    ready: &mut BTreeSet<(WorldOperationKind, String)>,
) -> Result<(), Vec<WorldWorkflowIssue>> {
    let Some(targets) = outgoing.get(operation_id) else {
        return Ok(());
    };
    for target in targets {
        let Some(count) = incoming.get_mut(target) else {
            return Err(vec![WorldWorkflowIssue::MissingDependency(target.clone())]);
        };
        let Some(next_count) = count.checked_sub(1) else {
            return Err(vec![WorldWorkflowIssue::DependencyCycle]);
        };
        *count = next_count;
        if next_count == 0 {
            let Some(operation) = operations.get(target) else {
                return Err(vec![WorldWorkflowIssue::MissingDependency(target.clone())]);
            };
            ready.insert((operation.kind, target.clone()));
        }
    }
    Ok(())
}
