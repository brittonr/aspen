use super::*;

mod batch;
mod codec;
mod decision;
mod recovery;

const MAX_OBJECT: u32 = 4_096;

type ProfileMutation = fn(&mut MarbleStoreProfile);

fn digest(label: &str) -> Digest {
    *blake3::hash(label.as_bytes()).as_bytes()
}

fn layout(block_count: u64) -> ObjectIdLayout {
    ObjectIdLayout {
        catalog: ObjectIdRange { start: 0, end: 1 },
        blocks: ObjectIdRange {
            start: 1_024,
            end: 1_024 + block_count,
        },
    }
}

fn profile() -> MarbleStoreProfile {
    MarbleStoreProfile {
        layout: layout(1 << 20),
        maintenance: MaintenanceSchedule {
            every_committed_batches: 8,
        },
        executor_queue_depth: 16,
        read_cache_entries: 256,
        max_object_bytes: MAX_OBJECT,
    }
}

// r[verify aspen.marble_store.spike]
#[test]
fn a_reviewed_profile_validates_and_each_unbounded_field_is_refused() {
    assert!(validate_store_profile(&profile()).is_empty());
    let cases: [(ProfileMutation, MarbleStoreIssue); 6] = [
        (|p| p.layout.blocks.end = p.layout.blocks.start, MarbleStoreIssue::EmptyRange(ObjectKind::Block)),
        (|p| p.layout.blocks.start = 0, MarbleStoreIssue::OverlappingRanges),
        (|p| p.maintenance.every_committed_batches = 0, MarbleStoreIssue::ZeroMaintenanceInterval),
        (|p| p.executor_queue_depth = 0, MarbleStoreIssue::ExecutorQueueOutOfBounds),
        (
            |p| p.executor_queue_depth = MAX_EXECUTOR_QUEUE_DEPTH + 1,
            MarbleStoreIssue::ExecutorQueueOutOfBounds,
        ),
        (|p| p.max_object_bytes = FRAME_HEADER_BYTES, MarbleStoreIssue::ObjectBoundOutOfRange),
    ];
    for (mutate, expected) in cases {
        let mut candidate = profile();
        mutate(&mut candidate);
        assert!(validate_store_profile(&candidate).contains(&expected), "{expected:?}");
    }
}

#[test]
fn maintenance_is_due_only_after_the_scheduled_batch_count() {
    let schedule = MaintenanceSchedule {
        every_committed_batches: 3,
    };
    assert!(!schedule.is_due(2));
    assert!(schedule.is_due(3));
    assert!(
        !MaintenanceSchedule {
            every_committed_batches: 0
        }
        .is_due(u32::MAX)
    );
}

#[test]
fn the_read_cache_evicts_its_oldest_entry_and_deleted_blocks() {
    let mut cache = ReadCache::new(2);
    cache.insert(digest("a"), b"a");
    cache.insert(digest("b"), b"b");
    cache.insert(digest("c"), b"c");
    assert_eq!(cache.get(&digest("a")), None, "the oldest entry is evicted");
    assert_eq!(cache.get(&digest("c")), Some(b"c".as_slice()));
    cache.evict(&digest("b"));
    assert_eq!(cache.get(&digest("b")), None);
    let mut disabled = ReadCache::new(0);
    disabled.insert(digest("a"), b"a");
    assert_eq!(disabled.get(&digest("a")), None);
}
