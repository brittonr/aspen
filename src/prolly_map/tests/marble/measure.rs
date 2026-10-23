// The measured comparison of the Redb and marble-plus-art Prolly stores.
//
// Run explicitly; the numbers are recorded in the spike decision record:
// `cargo test -p molten --lib --features marble-object-store
//  prolly_map::tests::marble::measure -- --ignored --nocapture`
// Timings are diagnostic observations of one host, never correctness.

#![allow(
    tigerstyle::ambient_clock,
    reason = "the measurement harness records wall-clock latency of one diagnostic run; no decision reads the clock"
)]

use molten_core::prolly_map::*;
use molten_node_host::node_state::DirectoryRoot;
use molten_node_host::node_state::NamespaceKind;

use super::super::*;
use super::marble_profile;

struct Workload {
    workload_ref: &'static str,
    entries: u32,
    rounds: u32,
    keys_per_round: u32,
}

const WORKLOADS: [Workload; 2] = [
    Workload {
        workload_ref: "prolly-64-entries-1-update",
        entries: 64,
        rounds: 1,
        keys_per_round: 1,
    },
    Workload {
        workload_ref: "prolly-4096-entries-32x16-updates",
        entries: 4_096,
        rounds: 32,
        keys_per_round: 16,
    },
];

fn entry(index: u32, round: u32) -> SemanticEntry {
    SemanticEntry {
        key: format!("key-{index:05}").into_bytes(),
        value: vec![b'a' + u8::try_from(round % 26).expect("round byte"); 64],
    }
}

/// The plans of one workload: the initial build, then each update round.
fn plans(workload: &Workload) -> Vec<EditPlan> {
    let profile = profile();
    let empty = build_map(&profile, &[]).expect("empty map");
    let initial = (0..workload.entries).map(|index| MapEdit::Insert(entry(index, 0))).collect::<Vec<_>>();
    let mut plans = vec![plan_edits(&profile, &empty.snapshot, &initial).expect("initial plan")];
    for round in 1..=workload.rounds {
        let prior = plans.last().expect("prior plan").next.snapshot.clone();
        let edits = (0..workload.keys_per_round)
            .map(|offset| MapEdit::Update(entry((round * 97 + offset * 131) % workload.entries, round)))
            .collect::<Vec<_>>();
        plans.push(plan_edits(&profile, &prior, &edits).expect("update plan"));
    }
    plans
}

struct Observed {
    batch_write_nanos: u64,
    point_read_nanos: u64,
    recovery_nanos: u64,
    space_permille: u64,
}

fn mean(total: u128, count: usize) -> u64 {
    u64::try_from(total / u128::try_from(count.max(1)).expect("count")).expect("mean")
}

fn measure<S: ProllyBlockStorePort + ProllyRetentionPort>(
    plans: &[EditPlan],
    open: &dyn Fn() -> S,
    disk_bytes: &dyn Fn(&mut S) -> u64,
) -> Observed {
    let profile = profile();
    let mut store = open();
    let mut expected = ExpectedProllyRoot {
        root_ref: None,
        generation: 0,
    };
    let mut writes = 0_u128;
    for plan in plans {
        let started = std::time::Instant::now();
        publish_prolly_edit(&mut store, MAP_ID, &expected, plan).expect("publication");
        writes += started.elapsed().as_nanos();
        let head = store.read_root(MAP_ID).expect("root").expect("published");
        expected = ExpectedProllyRoot {
            root_ref: Some(head.root.root_ref.clone()),
            generation: head.generation,
        };
    }
    let snapshots = plans.iter().map(|plan| &plan.next.snapshot).collect::<Vec<_>>();
    let last = *snapshots.last().expect("final snapshot");
    let all_nodes = snapshots.iter().flat_map(|snapshot| &snapshot.blocks).map(|block| block.node_ref.clone());
    let mut all_nodes = all_nodes.collect::<Vec<_>>();
    all_nodes.sort();
    all_nodes.dedup();
    let facts = super::super::local::merged_facts(&profile, &snapshots);
    let plan = plan_gc(&profile, &all_nodes, core::slice::from_ref(&last.root.top_node_ref), &[], &facts).expect("gc");
    let inventory = store.retention_inventory().expect("inventory");
    execute_prolly_gc(&mut store, &plan, &ProllyDeletionAdmission {
        roots: plan.roots.clone(),
        pins: plan.pins.clone(),
        candidate_unreachable: plan.candidate_unreachable.clone(),
        generation_current: true,
        retention_policy_allows: true,
        deletion_authority_present: true,
        retention_generation: inventory.retention_generation,
    })
    .expect("gc");
    let live_bytes = last.blocks.iter().map(|block| block.bytes.len() as u64).sum::<u64>();
    let space_permille = disk_bytes(&mut store) * 1_000 / live_bytes.max(1);
    let started = std::time::Instant::now();
    for _ in 0..2 {
        for block in &last.blocks {
            assert!(store.read_block(&block.node_ref).expect("read").is_some());
        }
    }
    let point_read_nanos = mean(started.elapsed().as_nanos(), last.blocks.len() * 2);
    drop(store);
    let started = std::time::Instant::now();
    let reopened = open();
    load_prolly_snapshot(&reopened, &profile, last.root.clone()).expect("recovered snapshot");
    let recovery_nanos = mean(started.elapsed().as_nanos(), 1);
    Observed {
        batch_write_nanos: mean(writes, plans.len()),
        point_read_nanos,
        recovery_nanos,
        space_permille,
    }
}

fn redb_file_bytes(root: &std::path::Path) -> u64 {
    let mut pending = vec![root.to_path_buf()];
    let mut total = 0;
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("directory") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "redb") {
                total += std::fs::metadata(&path).expect("metadata").len();
            }
        }
    }
    total
}

fn print(workload_ref: &str, backend: &str, observed: &Observed) {
    for (metric, unit, value) in [
        ("batch-write", "nanoseconds", observed.batch_write_nanos),
        ("point-read", "nanoseconds", observed.point_read_nanos),
        ("recovery", "nanoseconds", observed.recovery_nanos),
        ("space-amplification", "permille", observed.space_permille),
    ] {
        println!(
            "{{ workload_ref = \"{workload_ref}\", backend = \"{backend}\", metric = \"{metric}\", unit = \"{unit}\", value = {value} }},"
        );
    }
}

#[test]
#[ignore = "diagnostic measurement; run explicitly and record the output"]
fn measure_redb_and_marble_stores() {
    for workload in &WORKLOADS {
        let plans = plans(workload);
        let redb_root = crate::test_support::process_workspace("measure_redb").expect("redb root");
        let redb_path = redb_root.to_path_buf();
        let directory = cap_std::fs::Dir::open_ambient_dir(&redb_root, cap_std::ambient_authority()).expect("dir");
        let root = DirectoryRoot::from_dir(directory);
        root.create_layout().expect("node state layout");
        let storage = root.namespace(NamespaceKind::Storage).expect("storage namespace");
        let redb =
            measure(&plans, &|| LocalProllyBlockStore::open(&storage).expect("redb"), &|_| redb_file_bytes(&redb_path));
        print(workload.workload_ref, "redb-current", &redb);

        let heap = crate::test_support::process_workspace("measure_marble").expect("marble heap");
        let profile = marble_profile(1 << 32);
        let marble = measure(&plans, &|| MarbleArtBlockStore::open(&heap, &profile).expect("marble"), &|store| {
            store.maintain().expect("maintenance");
            store.statistics().expect("statistics").heap_bytes
        });
        print(workload.workload_ref, "marble-art", &marble);
    }
}
