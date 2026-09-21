use std::io::Write as _;
use std::process::Child;
use std::process::Command;
use std::process::Stdio;
use std::time::Duration;
use std::time::Instant;

use super::CHILD_BATCHES;
use super::CHILD_BATCHES_ENV;
use super::CHILD_HEAP_ENV;
use super::CHILD_JOURNAL_ENV;
use super::CHILD_MODE_TEST_NAME;
use super::CHILD_OBJECT_BYTES;
use super::CHILD_OBJECTS_PER_BATCH;
use super::content_objects;
use super::pilot_config;
use super::workload;
use crate::error::Result;
use crate::marble_store::Backend;
use crate::marble_store::StoreConfig;
use crate::marble_store::canonical_batch_receipt;
use crate::marble_store::canonical_recovery_receipt;
use crate::marble_store::content_ref_of;
use crate::marble_store::digest_of;

const TEST_EXACT_ARGUMENT: &str = "--exact";
const MINIMUM_COMMITTED_BATCHES: usize = 4;
const CHILD_WAIT_LIMIT: Duration = Duration::from_secs(30);
const POLL_PAUSE: Duration = Duration::from_millis(10);

// r[verify aspen.marble_store.identity]
// r[verify aspen.marble_store.verification]
#[test]
fn crash_mid_flight_replays_only_atomically_recovered_batches() -> Result<()> {
    let config = pilot_config("crash");
    let journal_path = config.heap_path.join("journal.tsv");
    let mut child = spawn_crash_child(&config.heap_path, &journal_path);
    wait_for_committed_batches(&journal_path, MINIMUM_COMMITTED_BATCHES);
    child.kill().expect("terminate the crash child mid-flight");
    let _ignored = child.wait();
    let committed = committed_batch_count(&journal_path);
    let reopened = Backend::open(&config)?;
    let recovery = reopened.recovery();
    let lower_bound = committed * CHILD_OBJECTS_PER_BATCH;
    let upper_bound = lower_bound + CHILD_OBJECTS_PER_BATCH;
    assert!(
        recovery.recovered_mappings >= lower_bound,
        "every journaled batch must replay after the crash; got {} mappings for {committed} batches",
        recovery.recovered_mappings
    );
    assert!(
        recovery.recovered_mappings <= upper_bound,
        "recovery may exceed the committed journal by at most one in-flight batch; got {} mappings for {committed} batches",
        recovery.recovered_mappings
    );
    for batch in 0..=committed as u32 {
        for object in 0..CHILD_OBJECTS_PER_BATCH {
            let bytes = workload(batch, object as u32, CHILD_OBJECT_BYTES);
            let content_ref = content_ref_of(&digest_of(&bytes));
            if let Ok(fetched) = reopened.fetch(&content_ref) {
                assert_eq!(
                    fetched.bytes.as_slice(),
                    bytes.as_slice(),
                    "a replayed mapping must round-trip its exact bytes after the crash"
                );
            }
        }
    }
    let receipt = canonical_recovery_receipt(&recovery)?;
    assert!(
        crate::preserves_rail::to_text(&receipt.value)?.contains("marble-store-recovery-receipt"),
        "recovery emits its canonical receipt"
    );
    Ok(())
}

/// Child entry: writes bounded batches, journaling each committed batch.
#[test]
fn child_crash_writer() -> Result<()> {
    let (Ok(heap_path), Ok(journal_path), Ok(batches)) =
        (std::env::var(CHILD_HEAP_ENV), std::env::var(CHILD_JOURNAL_ENV), std::env::var(CHILD_BATCHES_ENV))
    else {
        return Ok(());
    };
    let batches: u32 = batches.parse().expect("child batch count");
    let backend = Backend::open(&StoreConfig::pilot(heap_path))?;
    let mut journal = std::fs::File::create(&journal_path).expect("child journal");
    for batch in 0..batches {
        let outcome = backend.store_batch(content_objects(batch, CHILD_OBJECTS_PER_BATCH, CHILD_OBJECT_BYTES))?;
        let receipt = canonical_batch_receipt(&outcome)?;
        assert_eq!(outcome.stored.len(), CHILD_OBJECTS_PER_BATCH);
        writeln!(&mut journal, "{}", receipt.artifact_ref).expect("journal committed batch");
        journal.sync_all().expect("journal flush");
    }
    Ok(())
}

fn spawn_crash_child(heap_path: &std::path::Path, journal_path: &std::path::Path) -> Child {
    Command::new(std::env::current_exe().expect("current test executable"))
        .arg(TEST_EXACT_ARGUMENT)
        .arg(CHILD_MODE_TEST_NAME)
        .env(CHILD_HEAP_ENV, heap_path)
        .env(CHILD_JOURNAL_ENV, journal_path)
        .env(CHILD_BATCHES_ENV, CHILD_BATCHES.to_string())
        .stdout(Stdio::null())
        .spawn()
        .expect("spawn marble store crash child")
}

fn wait_for_committed_batches(journal_path: &std::path::Path, minimum: usize) {
    let deadline = Instant::now() + CHILD_WAIT_LIMIT;
    while committed_batch_count(journal_path) < minimum {
        assert!(Instant::now() < deadline, "crash child never committed {minimum} batches");
        std::thread::sleep(POLL_PAUSE);
    }
}

fn committed_batch_count(journal_path: &std::path::Path) -> usize {
    std::fs::read_to_string(journal_path)
        .map(|text| text.lines().filter(|line| !line.is_empty()).count())
        .unwrap_or(0)
}
