use super::content_objects;
use super::pilot_config;
use crate::error::Result;
use crate::marble_store::Backend;
use crate::marble_store::canonical_batch_receipt;
use crate::marble_store::canonical_fetch_receipt;
use crate::marble_store::canonical_maintenance_receipt;
use crate::marble_store::canonical_profile;
use crate::marble_store::canonical_recovery_receipt;

const OBJECT_BYTES: usize = 512;

// r[verify aspen.marble_store.verification]
#[test]
fn batch_and_fetch_receipts_carry_canonical_identity() -> Result<()> {
    let backend = Backend::open(&pilot_config("receipts"))?;
    let outcome = backend.store_batch(content_objects(20, 2, OBJECT_BYTES))?;
    let receipt = canonical_batch_receipt(&outcome)?;
    assert_eq!(
        receipt.artifact_ref,
        crate::preserves_rail::canonical_hash(&receipt.value)?,
        "the receipt ref is the BLAKE3 of its canonical value"
    );
    let text = crate::preserves_rail::to_text(&receipt.value)?;
    assert!(text.contains("marble-store-batch-receipt"));
    assert!(text.contains("molten.marble-store.batch-receipt.v1"));
    for stored in &outcome.stored {
        let fetched = backend.fetch(&stored.content_ref)?;
        let fetch_receipt = canonical_fetch_receipt(&fetched)?;
        assert!(crate::preserves_rail::to_text(&fetch_receipt.value)?.contains(&stored.content_ref));
        assert!(
            !crate::preserves_rail::to_text(&fetch_receipt.value)?.contains("workload"),
            "receipts never embed object bytes"
        );
    }
    Ok(())
}

// r[verify aspen.marble_store.verification]
#[test]
fn receipts_omit_host_paths_and_private_kinds_stay_labels() -> Result<()> {
    let config = pilot_config("no-paths");
    let backend = Backend::open(&config)?;
    let profile = canonical_profile(&config)?;
    let text = crate::preserves_rail::to_text(&profile.value)?;
    assert!(!text.contains(config.heap_path.to_string_lossy().as_ref()), "profile omits the heap path");
    assert!(text.contains("marble-store-profile"));
    let outcome = backend.store_batch(content_objects(21, 1, OBJECT_BYTES))?;
    let batch_text = crate::preserves_rail::to_text(&canonical_batch_receipt(&outcome)?.value)?;
    assert!(!batch_text.contains(config.heap_path.to_string_lossy().as_ref()));
    Ok(())
}

// r[verify aspen.marble_store.identity]
#[test]
fn recovery_and_maintenance_receipts_bind_recorded_outcomes() -> Result<()> {
    let mut config = pilot_config("outcome-receipts");
    config.maintenance_interval_batches = 1;
    let backend = Backend::open(&config)?;
    let outcome = backend.store_batch(content_objects(22, 2, OBJECT_BYTES))?;
    assert!(outcome.maintenance.is_some());
    if let Some(run) = outcome.maintenance {
        let receipt = canonical_maintenance_receipt(&run)?;
        assert!(crate::preserves_rail::to_text(&receipt.value)?.contains("marble-store-maintenance-receipt"));
    }
    drop(backend);
    let reopened = Backend::open(&config)?;
    let recovery = canonical_recovery_receipt(&reopened.recovery())?;
    let text = crate::preserves_rail::to_text(&recovery.value)?;
    assert!(text.contains("marble-store-recovery-receipt"));
    assert!(text.contains("molten.marble-store.recovery-receipt.v1"));
    Ok(())
}

#[test]
fn profile_receipt_rejects_out_of_bounds_configs() {
    let mut config = pilot_config("bad-profile");
    config.read_cache_entries = 0;
    assert!(canonical_profile(&config).is_err());
}
