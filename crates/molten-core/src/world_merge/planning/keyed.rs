use super::super::WorldMergeConflict;
use super::super::WorldMergeGenerated;
use super::super::WorldMergeIssue;
use super::super::WorldMergeOutput;
use super::super::WorldMergeRequest;
use super::super::WorldMergeRootInput;
use super::super::WorldMergeSchemaRef;
use super::super::WorldMergedRoot;
use super::super::merge_keyed_values;
use crate::world_commit::RootKind;

pub(in crate::world_merge) fn add_keyed(
    request: &WorldMergeRequest,
    root: &WorldMergeRootInput,
    outputs: &mut Vec<WorldMergedRoot>,
    conflicts: &mut Vec<WorldMergeConflict>,
    issues: &mut Vec<WorldMergeIssue>,
) {
    if root.kind != RootKind::DurableState {
        issues.push(WorldMergeIssue::RuntimeSensitiveRoot(root.kind));
        return;
    }
    match merge_keyed_values(
        root.kind,
        &root.base.keyed_values,
        &root.left.keyed_values,
        &root.right.keyed_values,
        &request.bounds,
    ) {
        Ok(mut result) => {
            if result.conflicts.is_empty() {
                let Some(schema) = present_schema(root) else {
                    issues.push(WorldMergeIssue::MissingOutputSchema(root.kind));
                    return;
                };
                outputs.push(WorldMergedRoot {
                    kind: root.kind,
                    output: WorldMergeOutput::Generated(WorldMergeGenerated {
                        schema,
                        values: result.values,
                        bytes: None,
                    }),
                });
            }
            conflicts.append(&mut result.conflicts);
        }
        Err(issue) => issues.push(issue),
    }
}

fn present_schema(root: &WorldMergeRootInput) -> Option<WorldMergeSchemaRef> {
    [&root.base, &root.left, &root.right]
        .into_iter()
        .find(|value| value.root.is_some())
        .and_then(|value| value.schema_ref.clone())
}
