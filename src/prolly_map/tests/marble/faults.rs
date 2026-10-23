use std::os::unix::fs::PermissionsExt;

use molten_core::marble_store::*;
use molten_core::prolly_map::*;

use super::super::*;
use super::marble_profile;

const HEAP_DIRECTORY: &str = "heap";

fn block(label: &str) -> EncodedBlock {
    EncodedBlock {
        node_ref: node_ref_of_digest(blake3::hash(label.as_bytes()).as_bytes()),
        bytes: label.as_bytes().to_vec(),
    }
}

fn heap_files(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files = std::fs::read_dir(root.join(HEAP_DIRECTORY))
        .expect("heap directory")
        .map(|entry| entry.expect("heap entry").path())
        .filter(|path| path.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.contains('-')))
        .collect::<Vec<_>>();
    files.sort();
    files
}

/// A batch whose file was written but never renamed into place is a batch
/// marble did not atomically commit: recovery drops it and maps nothing.
// r[verify aspen.marble_store.identity]
// r[verify aspen.marble_store.verification]
#[test]
fn an_interrupted_batch_leaves_no_mapping_after_recovery() {
    let target = crate::test_support::process_workspace("marble_interrupted").expect("target");
    let scratch = crate::test_support::process_workspace("marble_interrupted_scratch").expect("scratch");
    let profile = marble_profile(64);
    let mut store = MarbleArtBlockStore::open(&target, &profile).expect("target store");
    store.stage_blocks(&[block("committed")]).expect("committed batch");
    drop(store);
    let mut other = MarbleArtBlockStore::open(&scratch, &profile).expect("scratch store");
    other.stage_blocks(&[block("interrupted-1"), block("interrupted-2")]).expect("scratch batch");
    drop(other);
    let written = heap_files(&scratch).pop().expect("scratch batch file");
    let unrenamed = target.join(HEAP_DIRECTORY).join("900-tmp");
    std::fs::copy(&written, &unrenamed).expect("plant the unrenamed batch");

    let recovered = MarbleArtBlockStore::open(&target, &profile).expect("recovered store");
    assert_eq!(recovered.read_block(&block("committed").node_ref).expect("committed"), Some(b"committed".to_vec()));
    assert_eq!(recovered.read_block(&block("interrupted-1").node_ref).expect("interrupted"), None);
    assert_eq!(recovered.read_block(&block("interrupted-2").node_ref).expect("interrupted"), None);
    assert!(!unrenamed.exists(), "recovery removes the partial batch file");
    assert_eq!(recovered.statistics().expect("statistics").indexed_blocks, 1);
}

/// A committed batch file whose trailer is torn fails the open closed.
// r[verify aspen.marble_store.verification]
#[test]
fn a_torn_batch_trailer_refuses_the_open() {
    let workspace = crate::test_support::process_workspace("marble_torn").expect("workspace");
    let profile = marble_profile(64);
    let mut store = MarbleArtBlockStore::open(&workspace, &profile).expect("store");
    store.stage_blocks(&[block("a"), block("b")]).expect("batch");
    drop(store);
    let file = heap_files(&workspace).pop().expect("batch file");
    let mut bytes = std::fs::read(&file).expect("read batch");
    let last = bytes.len() - 1;
    bytes[last] ^= 0xff;
    std::fs::write(&file, bytes).expect("tear trailer");
    assert!(MarbleArtBlockStore::open(&workspace, &profile).is_err(), "a torn trailer never opens");
}

/// A failed write batch poisons the instance; reopening recovers exactly the
/// committed batches.
// r[verify aspen.marble_store.identity]
#[test]
fn a_failed_write_batch_poisons_the_store_until_reopen() {
    let workspace = crate::test_support::process_workspace("marble_poison").expect("workspace");
    let profile = marble_profile(64);
    let mut store = MarbleArtBlockStore::open(&workspace, &profile).expect("store");
    store.stage_blocks(&[block("before")]).expect("first batch");
    let heap = workspace.join(HEAP_DIRECTORY);
    std::fs::set_permissions(&heap, std::fs::Permissions::from_mode(0o555)).expect("read-only heap");
    let probe = heap.join("probe");
    if std::fs::write(&probe, b"").is_ok() {
        std::fs::remove_file(&probe).expect("remove probe");
        std::fs::set_permissions(&heap, std::fs::Permissions::from_mode(0o755)).expect("restore heap");
        return;
    }
    let failed = store.stage_blocks(&[block("after")]).expect_err("write refused");
    assert!(failed.outcome_unknown, "a failed batch commit is an unknown outcome");
    assert_eq!(store.read_block(&block("before").node_ref).expect_err("poisoned").code, "prolly-marble-poisoned");
    assert!(store.statistics().expect("statistics").is_poisoned);
    std::fs::set_permissions(&heap, std::fs::Permissions::from_mode(0o755)).expect("restore heap");
    assert_eq!(store.stage_blocks(&[block("after")]).expect_err("still poisoned").code, "prolly-marble-poisoned");
    drop(store);

    let reopened = MarbleArtBlockStore::open(&workspace, &profile).expect("reopened");
    assert_eq!(reopened.read_block(&block("before").node_ref).expect("before"), Some(b"before".to_vec()));
    assert_eq!(reopened.read_block(&block("after").node_ref).expect("after"), None);
}

#[test]
fn the_backend_runs_scheduled_and_explicit_maintenance() {
    let workspace = crate::test_support::process_workspace("marble_maintenance").expect("workspace");
    let mut store = MarbleArtBlockStore::open(&workspace, &marble_profile(64)).expect("store");
    for index in 0..9 {
        store.stage_blocks(&[block(&format!("block-{index}"))]).expect("batch");
    }
    store.maintain().expect("explicit maintenance");
    let statistics = store.statistics().expect("statistics");
    assert_eq!(statistics.indexed_blocks, 9);
    assert!(statistics.heap_bytes > 0);
    for index in 0..9 {
        let label = format!("block-{index}");
        assert_eq!(store.read_block(&block(&label).node_ref).expect("read"), Some(label.into_bytes()));
    }
}
