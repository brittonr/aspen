//! Canonical Preserves receipts for the marble object-store pilot.
//!
//! Receipts bind the recorded backend outcomes without host paths or raw
//! object bytes. Physical handles appear only as private diagnostic handles,
//! never as content identity.

use crate::error::MoltenError;
use crate::error::Result;
use crate::marble_store::BatchOutcome;
use crate::marble_store::FetchOutcome;
use crate::marble_store::MaintenanceOutcome;
use crate::marble_store::RecoveryOutcome;
use crate::marble_store::StoreConfig;

// r[impl aspen.marble_store.verification]
const PROFILE_SCHEMA: &str = crate::preserves_rail::MARBLE_STORE_PROFILE_SCHEMA;
const BATCH_RECEIPT_SCHEMA: &str = crate::preserves_rail::MARBLE_STORE_BATCH_RECEIPT_SCHEMA;
const FETCH_RECEIPT_SCHEMA: &str = crate::preserves_rail::MARBLE_STORE_FETCH_RECEIPT_SCHEMA;
const RECOVERY_RECEIPT_SCHEMA: &str = crate::preserves_rail::MARBLE_STORE_RECOVERY_RECEIPT_SCHEMA;
const MAINTENANCE_RECEIPT_SCHEMA: &str = crate::preserves_rail::MARBLE_STORE_MAINTENANCE_RECEIPT_SCHEMA;

/// Canonical projection of one pilot artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalArtifact<T> {
    pub artifact: T,
    pub artifact_ref: String,
    pub value: preserves::IOValue,
}

// r[impl aspen.marble_store.spike]
pub fn canonical_profile(config: &StoreConfig) -> Result<CanonicalArtifact<StoreConfig>> {
    config.validate()?;
    let value = record("marble-store-profile", vec![
        string(PROFILE_SCHEMA),
        u64_value(config.max_object_bytes as u64),
        u64_value(config.max_batch_objects as u64),
        u64_value(config.executor_queue_depth as u64),
        u64_value(config.read_cache_entries as u64),
        u64_value(config.read_cache_bytes as u64),
        u64_value(config.maintenance_interval_batches as u64),
        u64_value(config.content_id_ceiling),
        bool_value(config.fsync_each_batch),
        optional_level(config.zstd_compression_level),
    ]);
    canonical_artifact(config.clone(), value)
}

// r[impl aspen.marble_store.identity]
pub fn canonical_batch_receipt(outcome: &BatchOutcome) -> Result<CanonicalArtifact<BatchOutcome>> {
    let stored = sequence(outcome.stored.iter().map(stored_value).collect());
    let reused = strings(&outcome.reused_refs);
    let maintenance = match outcome.maintenance {
        Some(run) => record("some", vec![maintenance_value(&run)]),
        None => string("none"),
    };
    let value = record("marble-store-batch-receipt", vec![
        string(BATCH_RECEIPT_SCHEMA),
        stored,
        reused,
        u64_value(outcome.batch_bytes),
        maintenance,
    ]);
    canonical_artifact(outcome.clone(), value)
}

// r[impl aspen.marble_store.identity]
// r[impl aspen.marble_store.art_index]
pub fn canonical_fetch_receipt(outcome: &FetchOutcome) -> Result<CanonicalArtifact<FetchOutcome>> {
    let value = record("marble-store-fetch-receipt", vec![
        string(FETCH_RECEIPT_SCHEMA),
        string(&outcome.content_ref),
        u64_value(outcome.bytes.len() as u64),
        u64_value(outcome.object_id),
        bool_value(outcome.from_read_cache),
    ]);
    canonical_artifact(outcome.clone(), value)
}

// r[impl aspen.marble_store.identity]
pub fn canonical_recovery_receipt(outcome: &RecoveryOutcome) -> Result<CanonicalArtifact<RecoveryOutcome>> {
    let value = record("marble-store-recovery-receipt", vec![
        string(RECOVERY_RECEIPT_SCHEMA),
        u64_value(outcome.recovered_mappings as u64),
        u64_value(outcome.max_content_object_id),
        u64_value(outcome.heap_bytes),
    ]);
    canonical_artifact(*outcome, value)
}

// r[impl aspen.marble_store.spike]
pub fn canonical_maintenance_receipt(outcome: &MaintenanceOutcome) -> Result<CanonicalArtifact<MaintenanceOutcome>> {
    let value = record("marble-store-maintenance-receipt", vec![
        string(MAINTENANCE_RECEIPT_SCHEMA),
        u64_value(outcome.rewritten_files as u64),
    ]);
    canonical_artifact(*outcome, value)
}

fn stored_value(object: &crate::marble_store::StoredObject) -> preserves::IOValue {
    record("stored-object", vec![
        string(&object.content_ref),
        u64_value(object.object_id),
        string(object.object_kind.as_str()),
        u64_value(object.byte_length),
    ])
}

fn maintenance_value(outcome: &MaintenanceOutcome) -> preserves::IOValue {
    record("maintenance-run", vec![u64_value(outcome.rewritten_files as u64)])
}

fn optional_level(level: Option<i32>) -> preserves::IOValue {
    match level {
        Some(level) => record("some", vec![string(format!("zstd-{level}"))]),
        None => string("none"),
    }
}

fn canonical_artifact<T>(artifact: T, value: preserves::IOValue) -> Result<CanonicalArtifact<T>> {
    let artifact_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalArtifact {
        artifact,
        artifact_ref,
        value,
    })
}

pub(crate) fn require_no_host_path(text: &str, label: &str) -> Result<()> {
    if text.contains('/') || text.contains(std::path::MAIN_SEPARATOR) {
        Err(MoltenError::invalid_harness(format!("{label} leaked a host path into canonical evidence")))
    } else {
        Ok(())
    }
}

fn bool_value(value: bool) -> preserves::IOValue {
    crate::preserves_rail::bool_value(value)
}

fn record(label: &'static str, fields: Vec<preserves::IOValue>) -> preserves::IOValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<preserves::IOValue>) -> preserves::IOValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> preserves::IOValue {
    crate::preserves_rail::string(value.as_ref())
}

fn strings(values: &[String]) -> preserves::IOValue {
    sequence(values.iter().map(|value| string(value)).collect())
}

fn u64_value(value: u64) -> preserves::IOValue {
    crate::preserves_rail::u64_value(value)
}
