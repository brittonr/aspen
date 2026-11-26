//! Reachability and cycle walks over an observed world-commit parent graph.

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;

use super::super::WorldCommitRef;

const WALK_EVENT_FACTOR: usize = 2;

pub(super) fn unreachable_observations<'a>(
    root: &WorldCommitRef,
    graph: &'a BTreeMap<WorldCommitRef, Vec<WorldCommitRef>>,
) -> Vec<&'a WorldCommitRef> {
    let mut reachable = BTreeSet::new();
    let mut stack = Vec::with_capacity(graph.len());
    stack.push(root);
    while let Some(node) = stack.pop() {
        if !reachable.insert(node) {
            continue;
        }
        if let Some(parents) = graph.get(node) {
            stack.extend(parents.iter().filter(|parent| !reachable.contains(parent)));
        }
    }
    graph.keys().filter(|commit_ref| !reachable.contains(commit_ref)).collect()
}

pub(super) fn first_cycle(
    root: &WorldCommitRef,
    graph: &BTreeMap<WorldCommitRef, Vec<WorldCommitRef>>,
) -> Option<WorldCommitRef> {
    let capacity = graph.len().saturating_mul(WALK_EVENT_FACTOR);
    let mut stack = Vec::with_capacity(capacity);
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for start in core::iter::once(root).chain(graph.keys()) {
        if visited.contains(start) {
            continue;
        }
        stack.push((start.clone(), false));
        while let Some((node, is_exit)) = stack.pop() {
            if is_exit {
                visiting.remove(&node);
                visited.insert(node);
                continue;
            }
            if visited.contains(&node) {
                continue;
            }
            if !visiting.insert(node.clone()) {
                return Some(node);
            }
            stack.push((node.clone(), true));
            if let Some(parents) = graph.get(&node) {
                for parent in parents.iter().rev() {
                    stack.push((parent.clone(), false));
                }
            }
        }
    }
    None
}
