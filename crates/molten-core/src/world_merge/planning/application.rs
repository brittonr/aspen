use super::super::WorldMergeHandler;
use super::super::WorldMergeIssue;
use super::super::WorldMergeRequest;
use super::super::WorldMergeRootInput;
use super::super::WorldMergedRoot;

pub(in crate::world_merge) fn add_application_handler(
    request: &WorldMergeRequest,
    root: &WorldMergeRootInput,
    handler: Option<&dyn WorldMergeHandler>,
    outputs: &mut Vec<WorldMergedRoot>,
    issues: &mut Vec<WorldMergeIssue>,
) {
    let Some(profile) = request.profile.handlers.get(&root.kind) else {
        issues.push(WorldMergeIssue::HandlerMissing(root.kind));
        return;
    };
    if !profile.pure || profile.policy_ref != request.profile.policy_ref {
        issues.push(WorldMergeIssue::HandlerEffectRequested(root.kind));
        return;
    }
    let Some(handler) = handler else {
        issues.push(WorldMergeIssue::HandlerMissing(root.kind));
        return;
    };
    if handler.profile() != profile {
        issues.push(WorldMergeIssue::HandlerMismatch(root.kind));
        return;
    }
    let (Some(base), Some(left), Some(right)) = (
        root.base.canonical_bytes.as_deref(),
        root.left.canonical_bytes.as_deref(),
        root.right.canonical_bytes.as_deref(),
    ) else {
        issues.push(WorldMergeIssue::UnavailableRoot(root.kind));
        return;
    };
    let input = super::super::WorldApplicationMergeInput {
        kind: root.kind,
        base,
        left,
        right,
    };
    let Ok(output) = handler.merge(&input) else {
        issues.push(WorldMergeIssue::HandlerFailed(root.kind));
        return;
    };
    if output.requested_effect {
        issues.push(WorldMergeIssue::HandlerEffectRequested(root.kind));
        return;
    }
    let Ok(output_bytes) = u64::try_from(output.canonical_bytes.len()) else {
        issues.push(WorldMergeIssue::ValueLimitExceeded);
        return;
    };
    if output_bytes > profile.max_output_bytes || output_bytes > request.bounds.max_value_bytes {
        issues.push(WorldMergeIssue::ValueLimitExceeded);
        return;
    }
    outputs.push(WorldMergedRoot {
        kind: root.kind,
        selected_root: None,
        generated_values: std::collections::BTreeMap::new(),
        generated_bytes: Some(output.canonical_bytes),
        output_schema: Some(profile.output_schema.clone()),
    });
}
