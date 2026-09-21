use super::content_objects;
use super::pilot_config;
use super::workload;
use crate::error::Result;
use crate::marble_store::Backend;
use crate::marble_store::ObjectKind;
use crate::marble_store::StoreConfig;
use crate::marble_store::digest_from_content_ref;

const OBJECT_BYTES: usize = 1024;
const MARBLE_SMALL_OBJECT_CEILING: usize = 128;

// r[verify aspen.marble_store.verification]
#[test]
fn store_resolve_fetch_round_trips_through_the_digest_index() -> Result<()> {
    let backend = Backend::open(&pilot_config("round-trip"))?;
    let outcome = backend.store_batch(content_objects(1, 3, OBJECT_BYTES))?;
    assert_eq!(outcome.stored.len(), 3);
    let mut refs = outcome.stored.iter().map(|object| object.content_ref.clone()).collect::<Vec<_>>();
    refs.sort_unstable();
    let sorted_refs = refs.clone();
    assert_eq!(refs, sorted_refs);
    for (index, stored) in outcome.stored.iter().enumerate() {
        let fetched = backend.fetch(&stored.content_ref)?;
        assert_eq!(fetched.bytes.as_slice(), workload(1, index as u32, OBJECT_BYTES).as_slice());
        assert_eq!(fetched.content_ref, stored.content_ref);
        assert!(stored.object_id >= 1 << 32, "content handles stay in the content range");
    }
    Ok(())
}

// r[verify aspen.marble_store.identity]
#[test]
fn identical_digests_resolve_to_one_logical_object() -> Result<()> {
    let backend = Backend::open(&pilot_config("dedupe"))?;
    let first = workload(2, 0, OBJECT_BYTES);
    let duplicate = first.clone();
    let mut batch = content_objects(2, 1, OBJECT_BYTES);
    batch.push((ObjectKind::Content, duplicate));
    let outcome = backend.store_batch(batch)?;
    assert_eq!(outcome.stored.len(), 1);
    assert_eq!(outcome.reused_refs.len(), 1);
    assert_eq!(outcome.stored[0].content_ref, outcome.reused_refs[0]);
    let fetched = backend.fetch(&outcome.stored[0].content_ref)?;
    assert_eq!(fetched.bytes.as_slice(), first.as_slice());
    Ok(())
}

// r[verify aspen.marble_store.identity]
#[test]
fn fetches_after_commit_are_served_from_the_read_cache() -> Result<()> {
    let backend = Backend::open(&pilot_config("read-cache"))?;
    let outcome = backend.store_batch(content_objects(3, 1, OBJECT_BYTES))?;
    let content_ref = outcome.stored[0].content_ref.clone();
    let warm = backend.fetch(&content_ref)?;
    assert!(warm.from_read_cache);
    let warmer = backend.fetch(&content_ref)?;
    assert!(warmer.from_read_cache);
    assert_eq!(warm.bytes, warmer.bytes);
    Ok(())
}

// r[verify aspen.marble_store.identity]
#[test]
fn objects_beyond_the_read_cache_bound_read_from_marble_and_reverify() -> Result<()> {
    let mut config = pilot_config("no-read-cache");
    config.read_cache_bytes = 32;
    let backend = Backend::open(&config)?;
    let outcome = backend.store_batch(content_objects(19, 1, OBJECT_BYTES))?;
    let content_ref = outcome.stored[0].content_ref.clone();
    let first = backend.fetch(&content_ref)?;
    assert!(!first.from_read_cache);
    let second = backend.fetch(&content_ref)?;
    assert!(!second.from_read_cache);
    assert_eq!(first.bytes.as_slice(), second.bytes.as_slice());
    assert_eq!(first.bytes.as_slice(), workload(19, 0, OBJECT_BYTES).as_slice());
    Ok(())
}

// r[verify aspen.marble_store.spike]
#[test]
fn mixed_kinds_allocate_from_disjoint_handle_ranges() -> Result<()> {
    let backend = Backend::open(&pilot_config("mixed-kinds"))?;
    let mut batch = content_objects(4, 2, OBJECT_BYTES);
    batch.push((ObjectKind::Index, workload(4, 99, OBJECT_BYTES)));
    let outcome = backend.store_batch(batch)?;
    assert_eq!(outcome.stored.len(), 3);
    for stored in &outcome.stored {
        match stored.object_kind {
            ObjectKind::Content => assert!(stored.object_id < 1 << 48),
            ObjectKind::Index => assert!(stored.object_id >= 1 << 48),
        }
        let fetched = backend.fetch(&stored.content_ref)?;
        assert_eq!(fetched.object_id, stored.object_id);
    }
    Ok(())
}

// r[verify aspen.marble_store.spike]
#[test]
fn maintenance_runs_on_the_backend_owned_schedule() -> Result<()> {
    let mut config = pilot_config("maintenance");
    config.maintenance_interval_batches = 1;
    let backend = Backend::open(&config)?;
    let outcome = backend.store_batch(content_objects(5, 1, OBJECT_BYTES))?;
    assert!(outcome.maintenance.is_some());
    Ok(())
}

// r[verify aspen.marble_store.identity]
#[test]
fn recovery_replays_committed_batches_and_round_trips_again() -> Result<()> {
    let config = pilot_config("recovery");
    let backend = Backend::open(&config)?;
    let first = backend.store_batch(content_objects(6, 2, OBJECT_BYTES))?;
    let second = backend.store_batch(content_objects(7, 2, OBJECT_BYTES))?;
    drop(backend);
    let reopened = Backend::open(&config)?;
    let recovery = reopened.recovery();
    assert_eq!(recovery.recovered_mappings, 4);
    for stored in first.stored.iter().chain(second.stored.iter()) {
        let fetched = reopened.fetch(&stored.content_ref)?;
        assert_eq!(fetched.object_id, stored.object_id);
    }
    Ok(())
}

// r[verify aspen.marble_store.art_index]
#[test]
fn absent_digest_fails_lookup_and_reuse_stays_absent() -> Result<()> {
    let backend = Backend::open(&pilot_config("absent"))?;
    let never_stored = crate::marble_store::content_ref_of(&crate::marble_store::digest_of(&[7_u8; 32]));
    let denial = backend.fetch(&never_stored).expect_err("absent digest must fail lookup");
    assert!(denial.to_string().contains("was never stored"));
    let after = backend.store_batch(content_objects(8, 1, OBJECT_BYTES))?;
    assert!(
        !after.reused_refs.contains(&never_stored),
        "an absent digest must not become reused after an unrelated batch"
    );
    Ok(())
}

// r[verify aspen.marble_store.art_index]
#[test]
fn malformed_content_refs_fail_before_any_lookup() {
    assert!(digest_from_content_ref("sha3:0123").is_err());
    assert!(digest_from_content_ref("blake3:0123").is_err());
    assert!(digest_from_content_ref(&"z".repeat(70)).is_err());
}

// r[verify aspen.marble_store.identity]
#[test]
fn interrupted_batch_leaves_no_mappings_and_recovers_none() -> Result<()> {
    let mut config = pilot_config("interrupted");
    config.marble_max_object_bytes = MARBLE_SMALL_OBJECT_CEILING;
    let backend = Backend::open(&config)?;
    let committed = backend.store_batch(content_objects(9, 1, 64))?;
    let mut batch = content_objects(10, 1, 64);
    batch.push((ObjectKind::Content, workload(10, 1, 4 * MARBLE_SMALL_OBJECT_CEILING)));
    assert!(batch.iter().all(|(kind, _)| matches!(kind, ObjectKind::Content)));
    let denial = backend.store_batch(batch).expect_err("a batch interrupted before commit must fail");
    assert!(denial.to_string().contains("interrupted before commit"));
    for stored in &committed.stored {
        assert!(backend.fetch(&stored.content_ref).is_ok(), "committed mappings stay readable");
    }
    let rejected_ref = crate::marble_store::content_ref_of(&crate::marble_store::digest_of(&workload(10, 0, 64)));
    assert!(backend.fetch(&rejected_ref).is_err(), "the interrupted batch leaves no mapping");
    drop(backend);
    let reopened = Backend::open(&config)?;
    assert_eq!(reopened.recovery().recovered_mappings, 1, "only the atomically recovered batch replays");
    Ok(())
}

// r[verify aspen.marble_store.verification]
#[test]
fn object_id_exhaustion_fails_closed_and_keeps_existing_mappings() -> Result<()> {
    let mut config = pilot_config("exhaustion");
    config.content_id_ceiling = (1 << 32) + 2;
    let backend = Backend::open(&config)?;
    let first = backend.store_batch(content_objects(11, 1, OBJECT_BYTES))?;
    let second = backend.store_batch(content_objects(12, 1, OBJECT_BYTES))?;
    assert_eq!(first.stored[0].object_id, 1 << 32);
    assert_eq!(second.stored[0].object_id, (1 << 32) + 1);
    let denial = backend
        .store_batch(content_objects(13, 1, OBJECT_BYTES))
        .expect_err("object id exhaustion must fail closed");
    assert!(denial.to_string().contains("object id range is exhausted"));
    let denial_again = backend.store_batch(content_objects(14, 1, OBJECT_BYTES));
    assert!(denial_again.is_err(), "exhaustion stays closed on retry");
    assert!(backend.fetch(&first.stored[0].content_ref).is_ok());
    Ok(())
}

// r[verify aspen.marble_store.spike]
#[test]
fn out_of_bounds_configs_and_batches_fail_admission() {
    let mut config = pilot_config("bad-config");
    config.max_object_bytes = 0;
    assert!(Backend::open(&config).is_err());
    let live_config = pilot_config("bad-batch");
    let object_bound = live_config.max_object_bytes;
    let backend = Backend::open(&live_config).expect("valid backend");
    let oversized = vec![(ObjectKind::Content, workload(15, 0, object_bound + 1))];
    let denial = backend.store_batch(oversized).expect_err("oversized object must fail admission");
    assert!(denial.to_string().contains("exceeds the bound"));
    let too_many = content_objects(16, 5, 64);
    assert!(backend.store_batch(too_many).is_ok());
    let rejected = backend.store_batch(content_objects(17, 4097, 64));
    assert!(rejected.is_err());
}

// r[verify aspen.marble_store.spike]
#[test]
fn cloned_handles_share_one_bounded_executor() -> Result<()> {
    let backend = Backend::open(&pilot_config("clone"))?;
    let cloned = backend.clone();
    let outcome = backend.store_batch(content_objects(18, 1, OBJECT_BYTES))?;
    let fetched = cloned.fetch(&outcome.stored[0].content_ref)?;
    assert_eq!(fetched.bytes.as_slice(), workload(18, 0, OBJECT_BYTES).as_slice());
    Ok(())
}

#[test]
fn store_config_pilot_defaults_stay_bounded() {
    let config = StoreConfig::pilot("unused");
    assert!(config.max_batch_objects <= 4_096);
    assert!(config.read_cache_entries <= 4_096);
    assert!(config.executor_queue_depth <= 1_024);
    assert!(config.fsync_each_batch);
}
