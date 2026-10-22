use std::collections::BTreeMap;

use molten_core::prolly_map::*;
use molten_node_host::node_state::DirectoryRoot;
use molten_node_host::node_state::NamespaceKind;

use super::*;

// r[verify molten.prolly_map.storage_boundary]
#[test]
fn local_store_stages_publishes_restarts_and_executes_revalidated_gc() {
    let temporary = cap_tempfile::tempdir(cap_std::ambient_authority()).expect("temporary state root");
    let root = DirectoryRoot::from_dir(temporary.try_clone().expect("clone root"));
    root.create_layout().expect("node state layout");
    let storage = root.namespace(NamespaceKind::Storage).expect("storage namespace");
    let mut store = LocalProllyBlockStore::open(&storage).expect("Prolly store");
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

    assert_unexpected_generation_is_stale(&mut store, &profile);

    let first_snapshot = load_prolly_snapshot(&store, &profile, first.root.clone()).expect("load first snapshot");
    assert_eq!(first_snapshot.root.entry_count, u32::try_from(SHELL_ENTRY_COUNT).expect("entry count"));

    let update = update_plan(&first_snapshot);
    let expected = ExpectedProllyRoot {
        root_ref: Some(first.root.root_ref.clone()),
        generation: first.generation,
    };
    let receipt = publish_prolly_edit(&mut store, MAP_ID, &expected, &update).expect("update publication");
    assert_eq!(receipt.status, ProllyPublicationStatus::Applied);
    let second = store.read_root(MAP_ID).expect("read second").expect("second root");
    assert_eq!(second.generation, SECOND_GENERATION);

    drop(store);
    let mut reopened = LocalProllyBlockStore::open(&storage).expect("reopened store");
    assert_eq!(reopened.read_root(MAP_ID).expect("reopened head"), Some(second.clone()));
    let second_snapshot = load_prolly_snapshot(&reopened, &profile, second.root.clone()).expect("restart snapshot");
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
    let facts = merged_facts(&profile, &[&first_snapshot, &second_snapshot]);
    let plan =
        plan_gc(&profile, &all_nodes, core::slice::from_ref(&second.root.top_node_ref), &[], &facts).expect("gc plan");
    assert!(plan.complete);
    assert!(!plan.candidate_unreachable.is_empty());
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
}

/// Publishing against the initial generation after the first publication is stale.
fn assert_unexpected_generation_is_stale(store: &mut LocalProllyBlockStore, profile: &ProllyProfile) {
    let empty = build_map(profile, &[]).expect("empty map");
    let alternative = plan_edits(profile, &empty.snapshot, &[MapEdit::Insert(SemanticEntry {
        key: b"alternative".to_vec(),
        value: b"state".to_vec(),
    })])
    .expect("alternative plan");
    let stale = publish_prolly_edit(
        store,
        MAP_ID,
        &ExpectedProllyRoot {
            root_ref: None,
            generation: INITIAL_GENERATION,
        },
        &alternative,
    )
    .expect("stale publication receipt");
    assert_eq!(stale.status, ProllyPublicationStatus::Stale);
}

// r[verify molten.prolly_map.snapshot_profile_preflight]
#[test]
fn prolly_snapshot_validates_profile_before_block_read() {
    let temporary = cap_tempfile::tempdir(cap_std::ambient_authority()).expect("temporary state root");
    let root = DirectoryRoot::from_dir(temporary.try_clone().expect("clone root"));
    root.create_layout().expect("node state layout");
    let storage = root.namespace(NamespaceKind::Storage).expect("storage namespace");
    let mut store = LocalProllyBlockStore::open(&storage).expect("typed Prolly block store");
    let profile = profile();
    assert_eq!(profile.limits.max_graph_facts, MAX_GRAPH_FACTS);

    let mut over_limit = profile.clone();
    over_limit.limits.max_graph_facts = MAX_GRAPH_FACTS + 1;
    over_limit.profile_ref = derive_profile_ref(&over_limit).expect("forged profile identity");
    let plan = initial_plan();
    let error = load_prolly_snapshot(&store, &over_limit, plan.next.snapshot.root.clone())
        .expect_err("invalid profile must deny before reading missing blocks");
    assert!(matches!(
        &error,
        ProllyServiceError::Domain(issues)
            if issues.contains(&ProllyIssue::ProfileBoundInvalid("max-graph-facts"))
    ));

    store.stage_blocks(&plan.staged_blocks).expect("stage real encoded blocks");
    let loaded = load_prolly_snapshot(&store, &profile, plan.next.snapshot.root.clone())
        .expect("exact maximum graph profile remains accepted");
    assert_eq!(loaded.root, plan.next.snapshot.root);
    assert_eq!(loaded.blocks.len(), plan.next.snapshot.blocks.len());
}

#[test]
fn gc_denies_incomplete_stale_unpinned_and_unauthorized_admission() {
    let profile = profile();
    let build = build_map(&profile, &entries()).expect("build");
    let all_nodes = build.snapshot.blocks.iter().map(|block| block.node_ref.clone()).collect::<Vec<_>>();
    let facts = facts_from_snapshot(&profile, &build.snapshot).expect("facts");
    let plan = plan_gc(&profile, &all_nodes, core::slice::from_ref(&build.snapshot.root.top_node_ref), &[], &facts)
        .expect("plan");
    let mut port = MemoryPort::new(UnknownMode::None);
    let denied = execute_prolly_gc(&mut port, &plan, &ProllyDeletionAdmission {
        roots: plan.roots.clone(),
        pins: plan.pins.clone(),
        candidate_unreachable: plan.candidate_unreachable.clone(),
        generation_current: false,
        retention_policy_allows: true,
        deletion_authority_present: true,
        retention_generation: 0,
    });
    assert_eq!(denied, Err(ProllyServiceError::GcAdmissionDenied));

    let mut crossed_candidates = plan.candidate_unreachable.clone();
    crossed_candidates.push(build.snapshot.root.top_node_ref.clone());
    let crossed = execute_prolly_gc(&mut port, &plan, &ProllyDeletionAdmission {
        roots: plan.roots.clone(),
        pins: plan.pins.clone(),
        candidate_unreachable: crossed_candidates,
        generation_current: true,
        retention_policy_allows: true,
        deletion_authority_present: true,
        retention_generation: 0,
    });
    assert_eq!(crossed, Err(ProllyServiceError::GcAdmissionDenied));
}

pub(super) fn merged_facts(profile: &ProllyProfile, snapshots: &[&MapSnapshot]) -> Vec<GraphFact> {
    snapshots
        .iter()
        .flat_map(|snapshot| facts_from_snapshot(profile, snapshot).expect("facts"))
        .map(|fact| (fact.node_ref.as_str().to_string(), fact))
        .collect::<BTreeMap<_, _>>()
        .into_values()
        .collect()
}
