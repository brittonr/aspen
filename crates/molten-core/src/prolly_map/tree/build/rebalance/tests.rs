//! Deterministic cases for internal-group rebalancing under `max_node_bytes`.

use super::*;

const SMALL_KEY_BYTES: usize = 6;
const LARGE_KEY_BYTES: usize = 64;
const MAX_PREVIOUS_GROUP: usize = 8;
const MAX_LAST_GROUP: usize = 3;
const BUILD_ENTRY_COUNTS: usize = 96;
const BUILD_VALUE_BYTES: usize = 4;
const BUILD_MAX_NODE_BYTES: u32 = 1_100;

fn child(index: usize, key_bytes: usize) -> ChildRange {
    let mut key = format!("{index:05}").into_bytes();
    key.resize(key_bytes, b'k');
    ChildRange {
        min_key: key.clone(),
        max_key: key,
        node_ref: NodeRef::new(format!("blake3:{index:064x}")),
        encoded_len: 64,
    }
}

fn bounded_profile(max_node_bytes: u32, min_fanout: u16) -> ProllyProfile {
    let mut profile = standard_prolly_profile().expect("standard profile");
    profile.max_node_bytes = max_node_bytes;
    profile.min_fanout = min_fanout;
    profile
}

fn group_bytes(profile: &ProllyProfile, group: &[ChildRange]) -> u32 {
    internal_encoded_len(profile, group).expect("encoded group length")
}

fn assert_within_bounds(profile: &ProllyProfile, groups: &[Vec<ChildRange>], input: &[ChildRange]) {
    assert_eq!(groups.concat(), input, "rebalancing preserves child order and membership");
    for group in groups {
        assert!(!group.is_empty(), "no group is emptied");
        assert!(group.len() <= usize::from(profile.max_fanout), "fanout bound holds");
        assert!(group_bytes(profile, group) <= profile.max_node_bytes, "byte bound holds");
    }
}

// r[verify molten.prolly.rebalance_enforces_max_node_bytes]
#[test]
fn a_byte_split_last_group_is_never_overfilled_by_borrowing_or_merging() {
    let large = child(10, LARGE_KEY_BYTES);
    let smalls = (0..3).map(|index| child(index, SMALL_KEY_BYTES)).collect::<Vec<_>>();
    let standard = bounded_profile(MAX_NODE_BYTES, MIN_FANOUT);
    let byte_split_bound = group_bytes(&standard, &smalls).max(group_bytes(&standard, core::slice::from_ref(&large)));
    let profile = bounded_profile(byte_split_bound, MIN_FANOUT);
    assert!(group_bytes(&profile, &[smalls[2].clone(), large.clone()]) > profile.max_node_bytes);

    let borrow = vec![smalls.clone(), vec![large.clone()]];
    let mut groups = borrow.clone();
    rebalance_last_group(&profile, &mut groups).expect("rebalance");
    assert_eq!(groups, borrow, "a borrow that would exceed max_node_bytes is refused");

    let merge = vec![smalls[..2].to_vec(), vec![large.clone()]];
    let mut groups = merge.clone();
    rebalance_last_group(&profile, &mut groups).expect("rebalance");
    assert_eq!(groups, merge, "a merge that would exceed max_node_bytes is refused");
    for group in &groups {
        encode_internal(&profile, group).expect("every rebalanced group encodes");
    }
}

// r[verify molten.prolly.rebalance_enforces_max_node_bytes]
#[test]
fn rebalancing_still_borrows_and_merges_when_bytes_allow() {
    let profile = bounded_profile(MAX_NODE_BYTES, MIN_FANOUT);
    let children = (0..4).map(|index| child(index, SMALL_KEY_BYTES)).collect::<Vec<_>>();

    let mut groups = vec![children[..3].to_vec(), vec![children[3].clone()]];
    rebalance_last_group(&profile, &mut groups).expect("rebalance");
    assert_eq!(groups, vec![children[..2].to_vec(), children[2..].to_vec()], "borrow tops up the last group");

    let mut groups = vec![children[..2].to_vec(), vec![children[2].clone()]];
    rebalance_last_group(&profile, &mut groups).expect("rebalance");
    assert_eq!(groups, vec![children[..3].to_vec()], "merge absorbs an underfull last group");
}

// r[verify molten.prolly.rebalance_enforces_max_node_bytes]
#[test]
fn exhaustive_small_group_shapes_respect_both_bounds_and_rebalance_maximally() {
    let shapes: [fn(usize, usize) -> usize; 4] = [
        |_, _| SMALL_KEY_BYTES,
        |_, _| LARGE_KEY_BYTES,
        |index, total| {
            if index + 1 == total {
                LARGE_KEY_BYTES
            } else {
                SMALL_KEY_BYTES
            }
        },
        |index, _| {
            if index % 2 == 0 {
                LARGE_KEY_BYTES
            } else {
                SMALL_KEY_BYTES
            }
        },
    ];
    let mut cases = 0_usize;
    for shape in shapes {
        for min_fanout in MIN_FANOUT..=TARGET_FANOUT {
            for previous_len in 1..=MAX_PREVIOUS_GROUP {
                for last_len in 1..=MAX_LAST_GROUP {
                    let total = previous_len + last_len;
                    let input = (0..total).map(|index| child(index, shape(index, total))).collect::<Vec<_>>();
                    let base = bounded_profile(MAX_NODE_BYTES, min_fanout);
                    let previous_bytes = group_bytes(&base, &input[..previous_len]);
                    let last_bytes = group_bytes(&base, &input[previous_len..]);
                    let floor = previous_bytes.max(last_bytes);
                    let small = group_bytes(&base, &input[..1]) - group_bytes(&base, &[]);
                    for slack in [0, 1, small - 1, small, 2 * small, 4 * small] {
                        let profile = bounded_profile(floor + slack, min_fanout);
                        let mut groups = vec![input[..previous_len].to_vec(), input[previous_len..].to_vec()];
                        rebalance_last_group(&profile, &mut groups).expect("rebalance");
                        assert_within_bounds(&profile, &groups, &input);
                        assert_maximal(&profile, &groups);
                        cases += 1;
                    }
                }
            }
        }
    }
    assert_eq!(cases, 4 * 3 * MAX_PREVIOUS_GROUP * MAX_LAST_GROUP * 6);
}

/// An underfull last group remains only when neither a merge nor another
/// borrow fits both bounds.
fn assert_maximal(profile: &ProllyProfile, groups: &[Vec<ChildRange>]) {
    let minimum = usize::from(profile.min_fanout);
    let [previous, last] = groups else {
        return;
    };
    if last.len() >= minimum {
        return;
    }
    let merged = [previous.as_slice(), last.as_slice()].concat();
    let can_merge =
        merged.len() <= usize::from(profile.max_fanout) && group_bytes(profile, &merged) <= profile.max_node_bytes;
    assert!(!can_merge, "a merge that fits both bounds is taken");
    if previous.len() > minimum {
        let borrowed = [&previous[previous.len() - 1..], last.as_slice()].concat();
        assert!(group_bytes(profile, &borrowed) > profile.max_node_bytes, "a borrow that fits is taken");
    }
}

// r[verify molten.prolly.rebalance_enforces_max_node_bytes]
#[test]
fn byte_bounded_builds_never_exceed_max_node_bytes() {
    let mut profile = standard_prolly_profile().expect("standard profile");
    profile.max_node_bytes = BUILD_MAX_NODE_BYTES;
    profile.profile_ref = derive_profile_ref(&profile).expect("profile ref");
    assert!(validate_profile(&profile).is_empty(), "the byte-bounded profile is admitted");
    for key_bytes in [8, 32, LARGE_KEY_BYTES] {
        for count in 1..=BUILD_ENTRY_COUNTS {
            let entries = (0..count)
                .map(|index| {
                    let mut key = format!("{index:05}").into_bytes();
                    key.resize(key_bytes, b'k');
                    SemanticEntry {
                        key,
                        value: vec![b'v'; BUILD_VALUE_BYTES],
                    }
                })
                .collect::<Vec<_>>();
            let built = build_map(&profile, &entries)
                .unwrap_or_else(|issues| panic!("{count} entries of {key_bytes}-byte keys: {issues:?}"));
            for block in &built.snapshot.blocks {
                let block_bytes = u32::try_from(block.bytes.len()).expect("block length");
                assert!(block_bytes <= BUILD_MAX_NODE_BYTES, "{count}x{key_bytes}");
            }
            let read = validate_snapshot(&profile, &built.snapshot).expect("built map reads back");
            assert_eq!(read.entries, entries);
        }
    }
}
