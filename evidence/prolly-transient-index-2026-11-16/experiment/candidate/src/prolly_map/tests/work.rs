use super::*;

const SMALL_MIN_NODE_BYTES: u32 = 256;
const SMALL_TARGET_NODE_BYTES: u32 = 384;
const SMALL_MAX_NODE_BYTES: u32 = 640;
const SMALL_ENTRY_COUNT: usize = 96;
const SMALL_VALUE_BYTES: usize = 40;
const GENERATED_ROUNDS: usize = 160;
const GENERATOR_SEED: u64 = 0x5eed_0f9a;

/// A profile whose nodes hold a few entries, so edits shift content-defined
/// boundaries and the trees have several levels.
fn small_profile() -> ProllyProfile {
    let mut profile = standard_prolly_profile().expect("standard profile");
    profile.min_node_bytes = SMALL_MIN_NODE_BYTES;
    profile.target_node_bytes = SMALL_TARGET_NODE_BYTES;
    profile.max_node_bytes = SMALL_MAX_NODE_BYTES;
    profile.profile_ref = derive_profile_ref(&profile).expect("small profile identity");
    assert!(validate_profile(&profile).is_empty(), "the small profile is admitted");
    profile
}

fn small_entry(index: usize, fill: u8, length: usize) -> SemanticEntry {
    SemanticEntry {
        key: format!("k{index:05}").into_bytes(),
        value: vec![fill; length],
    }
}

fn small_build(profile: &ProllyProfile) -> MapBuild {
    let entries = (0..SMALL_ENTRY_COUNT)
        .map(|index| small_entry(index * 2, b'a', SMALL_VALUE_BYTES))
        .collect::<Vec<_>>();
    build_map(profile, &entries).expect("small build")
}

/// A deterministic generator, so a failing case reproduces exactly.
struct Generator(u64);

impl Generator {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(bound).expect("bound")).expect("index")
    }
}

fn generated_edits(generator: &mut Generator, entries: &[SemanticEntry]) -> Vec<MapEdit> {
    let mut edits = Vec::new();
    let mut touched = std::collections::BTreeSet::new();
    let count = 1 + generator.below(3);
    for _ in 0..count {
        let fill = b'b' + u8::try_from(generator.below(20)).expect("fill");
        let length = 1 + generator.below(SMALL_VALUE_BYTES * 2);
        match generator.below(3) {
            0 if !entries.is_empty() => {
                let entry = &entries[generator.below(entries.len())];
                if touched.insert(entry.key.clone()) {
                    edits.push(MapEdit::Update(SemanticEntry {
                        key: entry.key.clone(),
                        value: vec![fill; length],
                    }));
                }
            }
            1 if entries.len() > 2 => {
                let entry = &entries[generator.below(entries.len())];
                if touched.insert(entry.key.clone()) {
                    edits.push(MapEdit::Delete(entry.key.clone()));
                }
            }
            _ => {
                let candidate = small_entry(generator.below(SMALL_ENTRY_COUNT * 2) * 2 + 1, fill, length);
                let exists = entries.iter().any(|entry| entry.key == candidate.key);
                if !exists && touched.insert(candidate.key.clone()) {
                    edits.push(MapEdit::Insert(candidate));
                }
            }
        }
    }
    edits
}

// r[verify molten.prolly.incremental]
#[test]
fn incremental_edits_produce_the_reference_root_across_boundary_shifts() {
    let profile = small_profile();
    let mut snapshot = small_build(&profile).snapshot;
    let mut generator = Generator(GENERATOR_SEED);
    let mut incremental_rounds = 0_usize;
    let mut fewer_leaves = 0_usize;
    for round in 0..GENERATED_ROUNDS {
        let entries = validate_snapshot(&profile, &snapshot).expect("valid snapshot").entries;
        let edits = generated_edits(&mut generator, &entries);
        if edits.is_empty() {
            continue;
        }
        let incremental = plan_edits(&profile, &snapshot, &edits).expect("incremental plan");
        let reference = plan_edits_reference(&profile, &snapshot, &edits).expect("reference plan");
        assert_eq!(
            incremental.next.snapshot.root, reference.next.snapshot.root,
            "round {round}: the incremental root equals the rebuild root"
        );
        assert_eq!(
            incremental.next.snapshot.blocks, reference.next.snapshot.blocks,
            "round {round}: the incremental blocks equal the rebuild blocks"
        );
        assert_eq!(incremental.staged_blocks, reference.staged_blocks);
        assert_eq!(incremental.reused_block_count, reference.reused_block_count);
        if incremental.strategy == EditStrategy::Incremental {
            incremental_rounds += 1;
        }
        if incremental.leaves_encoded < reference.leaves_encoded {
            fewer_leaves += 1;
        }
        snapshot = incremental.next.snapshot;
    }
    assert!(incremental_rounds > GENERATED_ROUNDS / 2, "most rounds take the incremental path");
    assert!(fewer_leaves > GENERATED_ROUNDS / 2, "most rounds encode fewer leaves than the rebuild");
}

// r[verify molten.prolly.incremental]
#[test]
fn the_work_bound_falls_back_to_the_rebuild_and_keeps_the_exact_root() {
    let profile = small_profile();
    let prior = small_build(&profile);
    let edits = (10..30)
        .map(|index| MapEdit::Update(small_entry(index * 2, b'z', SMALL_VALUE_BYTES * 2)))
        .collect::<Vec<_>>();
    let bounded = plan_edits_with_bounds(&profile, &prior.snapshot, &edits, IncrementalEditBounds {
        max_rechunked_entries: 4,
    })
    .expect("bounded plan");
    let reference = plan_edits_reference(&profile, &prior.snapshot, &edits).expect("reference plan");
    assert_eq!(bounded.strategy, EditStrategy::RebuildFallback);
    assert_eq!(bounded.next.snapshot.root, reference.next.snapshot.root);
    assert_eq!(bounded.next.snapshot.blocks, reference.next.snapshot.blocks);
}

// r[verify molten.prolly.incremental]
#[test]
fn an_edit_that_empties_or_fills_the_map_still_matches_the_reference() {
    let profile = small_profile();
    let empty = build_map(&profile, &[]).expect("empty build");
    let insert = [MapEdit::Insert(small_entry(1, b'a', SMALL_VALUE_BYTES))];
    let filled = plan_edits(&profile, &empty.snapshot, &insert).expect("fill the empty map");
    let reference = plan_edits_reference(&profile, &empty.snapshot, &insert).expect("reference fill");
    assert_eq!(filled.next.snapshot.root, reference.next.snapshot.root);

    let delete = [MapEdit::Delete(small_entry(1, b'a', SMALL_VALUE_BYTES).key)];
    let emptied = plan_edits(&profile, &filled.next.snapshot, &delete).expect("empty the map");
    assert_eq!(emptied.next.snapshot.root, empty.snapshot.root);
}

// r[verify molten.prolly.diff]
// r[verify molten.prolly.metrics]
#[test]
fn the_demand_driven_diff_equals_the_reference_and_skips_shared_subtrees() {
    let profile = small_profile();
    let prior = small_build(&profile);
    let middle = SMALL_ENTRY_COUNT;
    let cases = [
        vec![MapEdit::Update(small_entry(middle, b'z', SMALL_VALUE_BYTES))],
        vec![MapEdit::Insert(small_entry(1, b'y', 3))],
        vec![MapEdit::Delete(
            small_entry((SMALL_ENTRY_COUNT - 1) * 2, b'a', SMALL_VALUE_BYTES).key,
        )],
        vec![
            MapEdit::Update(small_entry(4, b'q', 70)),
            MapEdit::Insert(small_entry(77, b'r', 9)),
            MapEdit::Delete(small_entry(150, b'a', SMALL_VALUE_BYTES).key),
        ],
    ];
    for edits in cases {
        let next = plan_edits_reference(&profile, &prior.snapshot, &edits).expect("edit").next;
        for (left, right) in [(&prior.snapshot, &next.snapshot), (&next.snapshot, &prior.snapshot)] {
            let diff = diff_maps(&profile, left, right).expect("diff");
            let reference = diff_maps_reference(&profile, left, right).expect("reference diff");
            assert_eq!(diff.records, reference.records, "the records equal the reference records exactly");
            assert_eq!(diff.shared_node_count, reference.shared_node_count);
            assert!(diff.comparison.subtrees_not_traversed > 0, "the comparison skips shared subtrees");
            assert!(
                diff.comparison.nodes_read < reference.comparison.nodes_read,
                "the comparison reads fewer nodes than the reference"
            );
            assert_eq!(diff.comparison.nodes_decoded, 0, "the comparison reuses the validation decodes");
            assert!(
                diff.validation.nodes_decoded < reference.validation.nodes_decoded,
                "validation decodes each shared node once"
            );
        }
    }
}

// r[verify molten.prolly.diff]
#[test]
fn an_equal_root_diff_still_validates_and_skips_the_whole_tree() {
    let profile = small_profile();
    let prior = small_build(&profile);
    let diff = diff_maps(&profile, &prior.snapshot, &prior.snapshot).expect("equal diff");
    assert!(diff.records.is_empty());
    assert_eq!(diff.comparison.subtrees_not_traversed, 1);
    assert_eq!(diff.comparison.nodes_read, 0);
    assert!(diff.validation.nodes_read > 0, "both snapshots are still validated");
}

// r[verify molten.prolly.diff]
#[test]
fn a_matching_root_string_does_not_admit_an_untrusted_snapshot() {
    let profile = small_profile();
    let prior = small_build(&profile);
    let mut untrusted = prior.snapshot.clone();
    let tampered = untrusted.blocks.iter_mut().last().expect("a block");
    let last = tampered.bytes.len() - 1;
    tampered.bytes[last] ^= 0x01;
    assert_eq!(untrusted.root, prior.snapshot.root, "the untrusted snapshot carries the valid root");
    assert!(diff_maps(&profile, &prior.snapshot, &untrusted).is_err());
    assert!(diff_maps(&profile, &untrusted, &prior.snapshot).is_err());
    assert!(diff_maps_reference(&profile, &prior.snapshot, &untrusted).is_err());
}

// r[verify molten.prolly.cache]
#[test]
fn cache_retention_never_changes_a_validation_outcome() {
    let profile = small_profile();
    let prior = small_build(&profile);
    let next = plan_edits_reference(&profile, &prior.snapshot, &[MapEdit::Update(small_entry(
        SMALL_ENTRY_COUNT,
        b'z',
        SMALL_VALUE_BYTES,
    ))])
    .expect("edit")
    .next;
    let baseline = [
        validate_snapshot(&profile, &prior.snapshot).expect("prior"),
        validate_snapshot(&profile, &next.snapshot).expect("next"),
    ];
    for (probation, protected) in [(0, 0), (1, 0), (1, 1), (2, 1), (4, 4), (512, 512)] {
        let mut cache = DecodedNodeCache::new(probation, protected);
        for (snapshot, expected) in [&prior.snapshot, &next.snapshot].into_iter().zip(&baseline) {
            let read = validate_snapshot_cached(&profile, snapshot, &mut cache).expect("cached validation");
            assert_eq!(read.root_ref, expected.root_ref);
            assert_eq!(read.entries, expected.entries);
            assert_eq!(read.closure, expected.closure);
            assert_eq!(read.graph_facts, expected.graph_facts);
            assert_eq!(read.leaf_spans, expected.leaf_spans);
            assert_eq!(read.visited_nodes, expected.visited_nodes);
        }
    }
}

// r[verify molten.prolly.cache]
#[test]
fn a_hot_working_set_survives_a_scan_and_eviction_stays_reachable() {
    let profile = small_profile();
    let blocks = small_build(&profile).snapshot.blocks;
    assert!(blocks.len() > 8, "the scan has more nodes than the cache holds");
    let mut cache = DecodedNodeCache::new(2, 2);
    for hot in &blocks[..2] {
        decode_block_cached(&profile, hot, &mut cache).expect("first decode");
        let (_, is_decoded) = decode_block_cached(&profile, hot, &mut cache).expect("second decode");
        assert!(!is_decoded, "the second decode is a hit");
    }
    for scanned in &blocks[2..] {
        decode_block_cached(&profile, scanned, &mut cache).expect("scan decode");
    }
    for hot in &blocks[..2] {
        let key = DecodedNodeKey::for_block(&profile, &hot.node_ref);
        assert!(cache.is_protected(&key), "the hot node survives the scan");
    }
    assert!(cache.stats().evictions > 0, "the scan exercises eviction");
    assert!(cache.len() <= 4, "the cache stays inside its bounds");
}

// r[verify molten.prolly.cache]
#[test]
fn a_cache_hit_requires_the_same_identity_and_bytes_and_never_replaces_an_entry() {
    let profile = small_profile();
    let blocks = small_build(&profile).snapshot.blocks;
    let block = &blocks[0];
    let mut cache = DecodedNodeCache::new(8, 8);
    let (node, _) = decode_block_cached(&profile, block, &mut cache).expect("decode");

    let mut tampered = block.clone();
    let last = tampered.bytes.len() - 1;
    tampered.bytes[last] ^= 0x01;
    assert!(
        decode_block_cached(&profile, &tampered, &mut cache).is_err(),
        "tampered bytes under a cached reference are decoded and rejected"
    );

    let mut other_codec = DecodedNodeKey::for_block(&profile, &block.node_ref);
    other_codec.format_version += 1;
    assert!(cache.get(&other_codec, &block.bytes).is_none(), "another codec version misses");

    let key = DecodedNodeKey::for_block(&profile, &block.node_ref);
    let other_node = decode_block(&profile, &blocks[1]).expect("other node");
    cache.insert(key.clone(), block.bytes.clone(), other_node);
    assert_eq!(cache.get(&key, &block.bytes), Some(node), "an existing entry is never replaced");
}
