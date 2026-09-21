mod backend;
mod canonical;
mod measure;
mod multiprocess;

use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use crate::marble_store::StoreConfig;

pub(crate) const CHILD_HEAP_ENV: &str = "MOLTEN_MARBLE_STORE_CHILD_HEAP";
pub(crate) const CHILD_JOURNAL_ENV: &str = "MOLTEN_MARBLE_STORE_CHILD_JOURNAL";
pub(crate) const CHILD_BATCHES_ENV: &str = "MOLTEN_MARBLE_STORE_CHILD_BATCHES";
pub(crate) const CHILD_MODE_TEST_NAME: &str = "marble_store::tests::multiprocess::child_crash_writer";
pub(crate) const CHILD_BATCHES: u32 = 64;
pub(crate) const CHILD_OBJECTS_PER_BATCH: usize = 8;
pub(crate) const CHILD_OBJECT_BYTES: usize = 3 * 1024;

/// Creates a per-process temporary heap directory for one test.
pub(crate) fn heap_root(label: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "molten-marble-store-{}-{label}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&root).expect("create marble store test root");
    root
}

pub(crate) fn pilot_config(label: &str) -> StoreConfig {
    StoreConfig::pilot(heap_root(label))
}

pub(crate) fn workload(batch: u32, object: u32, len: usize) -> Vec<u8> {
    crate::marble_store::workload_bytes(0x7069_6c6f_7431 ^ (u64::from(batch) << 32) ^ u64::from(object), len)
}

pub(crate) fn content_objects(batch: u32, count: usize, len: usize) -> Vec<(crate::marble_store::ObjectKind, Vec<u8>)> {
    (0..count)
        .map(|object| (crate::marble_store::ObjectKind::Content, workload(batch, object as u32, len)))
        .collect()
}
