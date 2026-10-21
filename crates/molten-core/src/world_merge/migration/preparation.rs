use super::super::WorldMergeGenerated;
use super::super::WorldMergeIssue;
use super::super::WorldMergeOutput;
use super::super::WorldMergeRequest;
use super::super::WorldMergedRoot;
use super::MaterializedWorldMigration;
use super::WorldMigrationEvidence;
use super::admit_world_migrations;
use super::identity;
use crate::world_commit::WorldRootRef;

// Validate all results against the original request before replacing any value.
// r[impl molten.audit_f06.binding]
pub(in crate::world_merge) fn prepare_values(
    request: &mut WorldMergeRequest,
    mut materializations: Vec<MaterializedWorldMigration>,
) -> Result<(String, Vec<WorldMigrationEvidence>), Vec<WorldMergeIssue>> {
    let expected = admit_world_migrations(request)?;
    if materializations.len() != expected.len() {
        return Err(vec![
            expected.first().map_or(WorldMergeIssue::MigrationResultUnexpected, |conversion| {
                WorldMergeIssue::MigrationResultMissing(conversion.kind)
            }),
        ]);
    }
    materializations.sort_by_key(|result| (result.evidence.conversion.kind, result.evidence.conversion.side));
    for (conversion, result) in expected.iter().zip(&materializations) {
        if conversion != &result.evidence.conversion {
            return Err(vec![WorldMergeIssue::MigrationResultMismatch(conversion.kind)]);
        }
    }
    let original_ref = identity::request_ref(request).map_err(|issue| vec![issue])?;
    let mut evidence = Vec::with_capacity(materializations.len());
    for result in materializations {
        let conversion = &result.evidence.conversion;
        let root = request
            .roots
            .iter_mut()
            .find(|root| root.kind == conversion.kind)
            .ok_or_else(|| vec![WorldMergeIssue::MigrationResultMismatch(conversion.kind)])?;
        let value = conversion.side.value_mut(root);
        value.root = Some(
            WorldRootRef::parse(conversion.kind, result.evidence.result_ref.clone())
                .map_err(|_| vec![WorldMergeIssue::MigrationResultMismatch(conversion.kind)])?,
        );
        value.schema_ref = Some(conversion.binding.target_schema.clone());
        value.canonical_bytes = Some(result.canonical_bytes);
        value.keyed_values = result.keyed_values;
        evidence.push(result.evidence);
    }
    Ok((original_ref, evidence))
}

// A selected converted value has new content, so publish it as generated data.
// The original source root is retained only in immutable conversion evidence.
// r[impl molten.audit_f06.publication]
pub(in crate::world_merge) fn restore_generated_outputs(
    request: &mut WorldMergeRequest,
    migrations: &[WorldMigrationEvidence],
    outputs: &mut [WorldMergedRoot],
) -> Result<(), WorldMergeIssue> {
    for output in outputs {
        let WorldMergeOutput::Selected(selected) = output.output.clone() else {
            continue;
        };
        let Some(evidence) = migrations
            .iter()
            .find(|evidence| evidence.conversion.kind == output.kind && evidence.result_ref == selected.as_str())
        else {
            continue;
        };
        let Some(root) = request.roots.iter_mut().find(|root| root.kind == output.kind) else {
            continue;
        };
        let value = evidence.conversion.side.value_mut(root);
        let Some(schema) = value.schema_ref.clone() else {
            return Err(WorldMergeIssue::MissingOutputSchema(output.kind));
        };
        output.output = WorldMergeOutput::Generated(WorldMergeGenerated {
            schema,
            values: std::mem::take(&mut value.keyed_values),
            bytes: value.canonical_bytes.take(),
        });
    }
    Ok(())
}
