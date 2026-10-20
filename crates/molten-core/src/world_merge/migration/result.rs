use std::collections::BTreeMap;

use super::super::WorldMergeIssue;
use super::AdmittedWorldMigration;
use super::MaterializedWorldMigration;
use super::WorldMigrationEvidence;
use super::WorldMigrationResult;
use super::identity;

pub(super) fn validate_payload(
    conversion: &AdmittedWorldMigration,
    bytes: &[u8],
    keyed: &BTreeMap<String, Vec<u8>>,
) -> Result<(), WorldMergeIssue> {
    if bytes.is_empty() {
        return Err(WorldMergeIssue::MigrationResultMalformed(conversion.kind));
    }
    if keyed.len() as u64 > u64::from(conversion.max_keys) {
        return Err(WorldMergeIssue::KeyLimitExceeded);
    }
    let mut actual = bytes.len() as u64;
    for (key, value) in keyed {
        actual = actual
            .checked_add(key.len() as u64)
            .and_then(|total| total.checked_add(value.len() as u64))
            .ok_or(WorldMergeIssue::ValueLimitExceeded)?;
    }
    if actual > conversion.max_output_bytes {
        return Err(WorldMergeIssue::MigrationResultOversized {
            kind: conversion.kind,
            actual,
            maximum: conversion.max_output_bytes,
        });
    }
    Ok(())
}

// r[impl molten.audit_f06.binding]
// r[impl molten.audit_f06.publication]
pub fn validate_world_migration_result(
    conversion: &AdmittedWorldMigration,
    result: WorldMigrationResult,
) -> Result<MaterializedWorldMigration, WorldMergeIssue> {
    if result.conversion_ref != conversion.conversion_ref || result.target_schema != conversion.binding.target_schema {
        return Err(WorldMergeIssue::MigrationResultMismatch(conversion.kind));
    }
    validate_payload(conversion, &result.canonical_bytes, &result.keyed_values)?;
    if identity::payload_ref(&result.canonical_bytes, &result.keyed_values)? != result.result_ref {
        return Err(WorldMergeIssue::MigrationResultMismatch(conversion.kind));
    }
    Ok(MaterializedWorldMigration {
        evidence: WorldMigrationEvidence {
            conversion: conversion.clone(),
            result_ref: result.result_ref,
        },
        canonical_bytes: result.canonical_bytes,
        keyed_values: result.keyed_values,
    })
}
