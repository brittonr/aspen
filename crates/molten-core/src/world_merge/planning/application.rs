use super::super::WorldApplicationHandlerProfile;
use super::super::WorldMergeGenerated;
use super::super::WorldMergeHandler;
use super::super::WorldMergeIssue;
use super::super::WorldMergeOutput;
use super::super::WorldMergeRequest;
use super::super::WorldMergeRootInput;
use super::super::WorldMergedRoot;

/// Admits the declared handler profile for a root before any handler input is formed.
///
/// The profile must be pure, bound to the merge policy, and declare the exact schema of every
/// present base, left, and right root; a handler never interprets bytes of another schema.
// r[impl molten.world_commit.merge_conservative_precedence_and_full_graph_closure]
pub(in crate::world_merge) fn admitted_handler_profile<'a>(
    request: &'a WorldMergeRequest,
    root: &WorldMergeRootInput,
) -> Result<&'a WorldApplicationHandlerProfile, WorldMergeIssue> {
    let Some(profile) = request.profile.handlers.get(&root.kind) else {
        return Err(WorldMergeIssue::HandlerMissing(root.kind));
    };
    if !profile.pure || profile.policy_ref != request.profile.policy_ref {
        return Err(WorldMergeIssue::HandlerEffectRequested(root.kind));
    }
    let is_input_schema_admitted = [&root.base, &root.left, &root.right]
        .into_iter()
        .filter(|value| value.root.is_some())
        .all(|value| value.schema_ref.as_ref() == Some(&profile.input_schema));
    if !is_input_schema_admitted {
        return Err(WorldMergeIssue::HandlerInputSchemaMismatch(root.kind));
    }
    Ok(profile)
}

pub(in crate::world_merge) fn add_application_handler(
    request: &WorldMergeRequest,
    root: &WorldMergeRootInput,
    handler: Option<&dyn WorldMergeHandler>,
    outputs: &mut Vec<WorldMergedRoot>,
    issues: &mut Vec<WorldMergeIssue>,
) {
    let profile = match admitted_handler_profile(request, root) {
        Ok(profile) => profile,
        Err(issue) => {
            issues.push(issue);
            return;
        }
    };
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
        output: WorldMergeOutput::Generated(WorldMergeGenerated {
            schema: profile.output_schema.clone(),
            values: alloc::collections::BTreeMap::new(),
            bytes: Some(output.canonical_bytes),
        }),
    });
}
