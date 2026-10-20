use super::super::MAX_WORLD_MERGE_VALUE_BYTES;
use super::super::WorldMergeIssue;
use super::super::WorldMergeRequest;
use super::super::WorldMergeValue;
use super::super::admission::is_runtime_sensitive;
use super::super::admission::validate_request;
use super::AdmittedWorldMigration;
use super::WorldMergeInputSide;
use super::identity;

const MIGRATION_PROFILE_MAX_BYTES: u16 = 256;
const SIDES: [WorldMergeInputSide; 3] = [
    WorldMergeInputSide::Base,
    WorldMergeInputSide::Left,
    WorldMergeInputSide::Right,
];

// r[impl molten.audit_f06.admission]
pub fn admit_world_migrations(
    request: &WorldMergeRequest,
) -> Result<Vec<AdmittedWorldMigration>, Vec<WorldMergeIssue>> {
    let mut issues = validate_request(request);
    if request.bounds.max_value_bytes > MAX_WORLD_MERGE_VALUE_BYTES {
        issues.push(WorldMergeIssue::InvalidBounds("max_value_bytes"));
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    validate_root_migrations(request, &mut issues);
    if !issues.is_empty() {
        issues.sort();
        issues.dedup();
        return Err(issues);
    }
    let needs_conversion = request.roots.iter().any(|root| {
        request.profile.migrations.get(&root.kind).is_some_and(|binding| {
            SIDES.into_iter().any(|side| {
                let value = side.value(root);
                value.root.is_some() && value.schema_ref.as_ref() == Some(&binding.source_schema)
            })
        })
    });
    if !needs_conversion {
        return Ok(Vec::new());
    }
    let context_ref = identity::request_ref(request).map_err(|issue| vec![issue])?;
    let mut conversions = Vec::with_capacity(request.roots.len() * SIDES.len());
    for root in &request.roots {
        let Some(binding) = request.profile.migrations.get(&root.kind) else {
            continue;
        };
        for side in SIDES {
            let value = side.value(root);
            let Some(original_root) = value.root.as_ref() else {
                continue;
            };
            if value.schema_ref.as_ref() != Some(&binding.source_schema) {
                continue;
            }
            let source = value
                .canonical_bytes
                .as_deref()
                .ok_or_else(|| vec![WorldMergeIssue::MigrationSourceMalformed(root.kind)])?;
            let source_ref = identity::bytes_ref(source);
            let mut conversion = AdmittedWorldMigration {
                kind: root.kind,
                side,
                binding: binding.clone(),
                original_root: original_root.clone(),
                source_ref,
                conversion_ref: String::new(),
                max_output_bytes: request.bounds.max_value_bytes,
                max_keys: request.bounds.max_keys,
            };
            conversion.conversion_ref =
                identity::conversion_ref(&context_ref, &conversion).map_err(|issue| vec![issue])?;
            conversions.push(conversion);
        }
    }
    conversions.sort_by_key(|conversion| (conversion.kind, conversion.side));
    Ok(conversions)
}

fn validate_root_migrations(request: &WorldMergeRequest, issues: &mut Vec<WorldMergeIssue>) {
    for root in &request.roots {
        for side in SIDES {
            if let Err(issue) = validate_input(side.value(root), &request.bounds, root.kind) {
                issues.push(issue);
            }
        }
        if let Some(binding) = request.profile.migrations.get(&root.kind) {
            if !binding.admitted {
                issues.push(WorldMergeIssue::MigrationRequired(root.kind));
            }
            if binding.source_schema == binding.target_schema {
                issues.push(WorldMergeIssue::MigrationMismatch(root.kind));
            }
            let profile_valid = schema_migration_core::TextLimit::new(MIGRATION_PROFILE_MAX_BYTES)
                .and_then(|limit| schema_migration_core::MigrationProfileId::new(binding.profile_id.clone(), limit));
            if profile_valid.is_err() {
                issues.push(WorldMergeIssue::MigrationProfileInvalid(root.kind));
            }
            for side in SIDES {
                let value = side.value(root);
                if value.root.is_none() {
                    continue;
                }
                if value.schema_ref.as_ref() == Some(&binding.source_schema) {
                    if is_runtime_sensitive(root.kind) {
                        issues.push(WorldMergeIssue::RuntimeSensitiveRoot(root.kind));
                    }
                    if !request.profile.root_modes.contains_key(&root.kind) {
                        issues.push(WorldMergeIssue::ModeNotDeclared(root.kind));
                    }
                    if value.canonical_bytes.as_ref().is_none_or(Vec::is_empty) {
                        issues.push(WorldMergeIssue::MigrationSourceMalformed(root.kind));
                    }
                } else if value.schema_ref.as_ref() != Some(&binding.target_schema) {
                    issues.push(WorldMergeIssue::MigrationMismatch(root.kind));
                }
            }
            if [&root.base, &root.left, &root.right].into_iter().any(|value| value.root.is_some())
                && !SIDES.into_iter().any(|side| {
                    let value = side.value(root);
                    value.root.is_some() && value.schema_ref.as_ref() == Some(&binding.source_schema)
                })
            {
                issues.push(WorldMergeIssue::MigrationResultMissing(root.kind));
            }
        } else {
            let mut schemas = SIDES
                .into_iter()
                .map(|side| side.value(root))
                .filter(|value| value.root.is_some())
                .map(|value| &value.schema_ref);
            if let Some(first) = schemas.next()
                && schemas.any(|schema| schema != first)
            {
                issues.push(WorldMergeIssue::MigrationRequired(root.kind));
            }
        }
    }
}

fn validate_input(
    value: &WorldMergeValue,
    bounds: &super::super::WorldMergeBounds,
    kind: crate::world_commit::RootKind,
) -> Result<(), WorldMergeIssue> {
    if value.root.as_ref().is_some_and(|root| root.kind() != kind) {
        return Err(WorldMergeIssue::MigrationMismatch(kind));
    }
    if value.root.is_some() && !value.available {
        return Err(WorldMergeIssue::UnavailableRoot(kind));
    }
    if value.canonical_bytes.as_ref().is_some_and(|bytes| bytes.len() as u64 > bounds.max_value_bytes) {
        return Err(WorldMergeIssue::ValueLimitExceeded);
    }
    if value.keyed_values.len() as u64 > u64::from(bounds.max_keys) {
        return Err(WorldMergeIssue::KeyLimitExceeded);
    }
    let mut size = 0_u64;
    for (key, bytes) in &value.keyed_values {
        size = size
            .checked_add(key.len() as u64)
            .and_then(|total| total.checked_add(bytes.len() as u64))
            .ok_or(WorldMergeIssue::ValueLimitExceeded)?;
    }
    if size > bounds.max_value_bytes {
        return Err(WorldMergeIssue::ValueLimitExceeded);
    }
    Ok(())
}
