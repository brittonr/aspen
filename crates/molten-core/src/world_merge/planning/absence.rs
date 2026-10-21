use super::super::WorldMergeIssue;
use super::super::WorldMergeMode;
use super::super::WorldMergeOutput;
use super::super::WorldMergeRequest;
use super::super::WorldMergeRootInput;
use super::super::WorldMergedRoot;
use super::super::admission::is_runtime_sensitive;

// Absence is an admitted inventory result, not generated data. A profile that
// declares a content-producing mode for the kind requires a present root.
// r[impl molten.audit_f13.output_kind]
// r[impl molten.audit_f13.absence]
pub(in crate::world_merge) fn reduce_absent(
    request: &WorldMergeRequest,
    root: &WorldMergeRootInput,
    outputs: &mut Vec<WorldMergedRoot>,
    issues: &mut Vec<WorldMergeIssue>,
) {
    if is_runtime_sensitive(root.kind) {
        issues.push(WorldMergeIssue::RuntimeSensitiveRoot(root.kind));
        return;
    }
    if root.left.schema_ref != root.right.schema_ref {
        issues.push(WorldMergeIssue::IncompatibleSchema(root.kind));
        return;
    }
    match request.profile.root_modes.get(&root.kind) {
        Some(WorldMergeMode::IdenticalOnly | WorldMergeMode::AncestorReplacement) => {
            outputs.push(WorldMergedRoot {
                kind: root.kind,
                output: WorldMergeOutput::Absent,
            });
        }
        Some(_) => issues.push(WorldMergeIssue::UnsupportedAbsence(root.kind)),
        None => issues.push(WorldMergeIssue::ModeNotDeclared(root.kind)),
    }
}
