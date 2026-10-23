use molten_core::marble_store::*;
use molten_core::prolly_map::*;

use super::*;

mod faults;
mod measure;

pub(super) fn marble_profile(block_ids: u64) -> MarbleStoreProfile {
    MarbleStoreProfile {
        layout: ObjectIdLayout {
            catalog: ObjectIdRange { start: 0, end: 1 },
            blocks: ObjectIdRange {
                start: 1_024,
                end: 1_024 + block_ids,
            },
        },
        maintenance: MaintenanceSchedule {
            every_committed_batches: 4,
        },
        executor_queue_depth: 8,
        read_cache_entries: 64,
        max_object_bytes: 1 << 20,
    }
}

fn heap(label: &str) -> crate::test_support::ProcessWorkspace {
    crate::test_support::process_workspace(label).expect("marble workspace")
}

fn block(label: &str) -> EncodedBlock {
    EncodedBlock {
        node_ref: node_ref_of_digest(blake3::hash(label.as_bytes()).as_bytes()),
        bytes: label.as_bytes().to_vec(),
    }
}

// r[verify aspen.marble_store.verification]
// r[verify aspen.marble_store.identity]
#[test]
fn marble_store_stages_publishes_restarts_and_executes_revalidated_gc() {
    let workspace = heap("marble_lifecycle");
    let mut store = MarbleArtBlockStore::open(&workspace, &marble_profile(1 << 16)).expect("marble store");
    let profile = profile();
    let initial = initial_plan();
    let expected = ExpectedProllyRoot {
        root_ref: None,
        generation: INITIAL_GENERATION,
    };
    let receipt = publish_prolly_edit(&mut store, MAP_ID, &expected, &initial).expect("initial publication");
    assert_eq!(receipt.status, ProllyPublicationStatus::Applied);
    let first = store.read_root(MAP_ID).expect("read root").expect("published root");
    assert_eq!(first.generation, FIRST_GENERATION);
    let mut collided = initial.staged_blocks.first().cloned().expect("staged block");
    collided.bytes.push(0);
    assert!(store.stage_blocks(&[collided]).expect_err("block collision").code.contains("collision"));

    let first_snapshot =
        load_prolly_snapshot(&store, &profile, first.root.clone()).expect("load first snapshot").snapshot;
    let update = update_plan(&first_snapshot);
    let expected = ExpectedProllyRoot {
        root_ref: Some(first.root.root_ref.clone()),
        generation: first.generation,
    };
    publish_prolly_edit(&mut store, MAP_ID, &expected, &update).expect("update publication");
    let second = store.read_root(MAP_ID).expect("read second").expect("second root");
    assert_eq!(second.generation, SECOND_GENERATION);

    drop(store);
    let mut reopened = MarbleArtBlockStore::open(&workspace, &marble_profile(1 << 16)).expect("reopened store");
    assert_eq!(reopened.read_root(MAP_ID).expect("reopened head"), Some(second.clone()));
    let second_snapshot =
        load_prolly_snapshot(&reopened, &profile, second.root.clone()).expect("restart snapshot").snapshot;
    assert_eq!(
        point_read(&profile, &second_snapshot, &entries()[UPDATE_INDEX].key)
            .expect("updated point")
            .expect("entry")
            .value,
        vec![b'z'; SHELL_VALUE_BYTES]
    );

    let all_nodes = first_snapshot
        .blocks
        .iter()
        .chain(&second_snapshot.blocks)
        .map(|block| block.node_ref.clone())
        .collect::<Vec<_>>();
    let facts = super::local::merged_facts(&profile, &[&first_snapshot, &second_snapshot]);
    let plan =
        plan_gc(&profile, &all_nodes, core::slice::from_ref(&second.root.top_node_ref), &[], &facts).expect("gc plan");
    let candidate = plan.candidate_unreachable[0].clone();
    let inventory = reopened.retention_inventory().expect("retention inventory");
    execute_prolly_gc(&mut reopened, &plan, &ProllyDeletionAdmission {
        roots: plan.roots.clone(),
        pins: plan.pins.clone(),
        candidate_unreachable: plan.candidate_unreachable.clone(),
        generation_current: true,
        retention_policy_allows: true,
        deletion_authority_present: true,
        retention_generation: inventory.retention_generation,
    })
    .expect("admitted gc");
    assert!(reopened.read_block(&candidate).expect("candidate read").is_none());
    drop(reopened);
    let recovered = MarbleArtBlockStore::open(&workspace, &marble_profile(1 << 16)).expect("after gc");
    assert!(
        recovered.read_block(&candidate).expect("deleted read").is_none(),
        "a deletion batch survives reopen"
    );
}

// r[verify aspen.marble_store.art_index]
#[test]
fn absent_and_non_canonical_digests_fail_lookup() {
    let workspace = heap("marble_absent");
    let mut store = MarbleArtBlockStore::open(&workspace, &marble_profile(16)).expect("marble store");
    store.stage_blocks(&[block("present")]).expect("stage");
    assert_eq!(store.read_block(&block("present").node_ref).expect("present"), Some(b"present".to_vec()));
    assert_eq!(store.read_block(&block("never-stored").node_ref).expect("absent"), None);
    let non_canonical = NodeRef::new("blake3:NOT-HEX".to_owned());
    assert_eq!(store.read_block(&non_canonical).expect("non-canonical"), None);
    let mut invalid = block("invalid");
    invalid.node_ref = non_canonical;
    assert_eq!(store.stage_blocks(&[invalid]).expect_err("refused").code, "prolly-marble-node-ref");
}

// r[verify aspen.marble_store.identity]
#[test]
fn identical_blocks_resolve_to_one_logical_object() {
    let workspace = heap("marble_dedupe");
    let mut store = MarbleArtBlockStore::open(&workspace, &marble_profile(16)).expect("marble store");
    store.stage_blocks(&[block("a"), block("a"), block("b")]).expect("stage");
    let before = store.statistics().expect("statistics");
    store.stage_blocks(&[block("a"), block("b")]).expect("restage");
    let after = store.statistics().expect("statistics");
    assert_eq!(before.indexed_blocks, 2);
    assert_eq!(after.indexed_blocks, 2);
    assert_eq!(after.stored_objects, before.stored_objects, "a restage writes no new object");
}

// r[verify aspen.marble_store.verification]
#[test]
fn object_id_exhaustion_fails_closed_and_survives_reopen() {
    let workspace = heap("marble_exhaustion");
    let mut store = MarbleArtBlockStore::open(&workspace, &marble_profile(2)).expect("marble store");
    let refused = store.stage_blocks(&[block("a"), block("b"), block("c")]).expect_err("exhausted");
    assert_eq!(refused.code, "prolly-marble-object-id-exhausted");
    assert!(!refused.outcome_unknown, "nothing was written");
    assert_eq!(store.read_block(&block("a").node_ref).expect("read"), None);
    store.stage_blocks(&[block("a"), block("b")]).expect("fits");
    drop(store);
    let mut reopened = MarbleArtBlockStore::open(&workspace, &marble_profile(2)).expect("reopened");
    assert_eq!(
        reopened.stage_blocks(&[block("c")]).expect_err("still exhausted").code,
        "prolly-marble-object-id-exhausted"
    );
    assert_eq!(reopened.read_block(&block("b").node_ref).expect("read"), Some(b"b".to_vec()));
}
