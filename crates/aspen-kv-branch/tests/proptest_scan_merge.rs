//! Property-based tests for the scan merge function.

#![allow(
    sentinel_fallback,
    reason = "proptest limit conversion fallback keeps generated fixture assertions total across target widths"
)]

use aspen_kv_branch::BranchEntry;
use aspen_kv_branch::verified::scan_merge::merge_scan;
use aspen_kv_types::KeyValueWithRevision;
use proptest::prelude::*;

fn arb_kv(max_keys: usize) -> impl Strategy<Value = Vec<KeyValueWithRevision>> {
    proptest::collection::btree_set("[a-z]{1,4}", 0..max_keys).prop_map(|keys| {
        keys.into_iter()
            .map(|key| KeyValueWithRevision {
                key,
                value: "parent".to_owned(),
                version: 1,
                create_revision: 1,
                mod_revision: 1,
            })
            .collect()
    })
}

fn arb_dirty(max_keys: usize) -> impl Strategy<Value = Vec<(String, BranchEntry)>> {
    proptest::collection::btree_set("[a-z]{1,4}", 0..max_keys).prop_flat_map(|keys| {
        let entries: Vec<_> = keys
            .into_iter()
            .map(|key| {
                prop_oneof![
                    Just(BranchEntry::Write {
                        value: "branch".to_owned(),
                    }),
                    Just(BranchEntry::Tombstone),
                ]
                .prop_map(move |entry| (key.clone(), entry))
            })
            .collect();
        entries
    })
}

proptest! {
    #[test]
    fn tombstoned_keys_never_appear(
        dirty in arb_dirty(20),
        parent in arb_kv(20),
        limit in 1..100_u32,
    ) {
        let result = merge_scan(&dirty, &parent, "", limit);

        let tombstones: Vec<&str> = dirty.iter()
            .filter(|(_, e)| e.is_tombstone())
            .map(|(k, _)| k.as_str())
            .collect();

        for entry in &result {
            prop_assert!(
                !tombstones.contains(&entry.key.as_str()),
                "tombstoned key {} appeared in output",
                entry.key
            );
        }
    }

    #[test]
    fn output_is_sorted(
        dirty in arb_dirty(20),
        parent in arb_kv(20),
        limit in 1..100_u32,
    ) {
        let result = merge_scan(&dirty, &parent, "", limit);

        for window in result.windows(2) {
            prop_assert!(
                window[0].key < window[1].key,
                "output not sorted: {} >= {}",
                window[0].key,
                window[1].key
            );
        }
    }

    #[test]
    fn branch_overrides_parent(
        dirty in arb_dirty(20),
        parent in arb_kv(20),
        limit in 1..100_u32,
    ) {
        let result = merge_scan(&dirty, &parent, "", limit);

        let branch_writes: std::collections::HashMap<&str, &str> = dirty.iter()
            .filter_map(|(k, e)| match e {
                BranchEntry::Write { value } => Some((k.as_str(), value.as_str())),
                _ => None,
            })
            .collect();

        for entry in &result {
            if let Some(branch_val) = branch_writes.get(entry.key.as_str()) {
                prop_assert_eq!(
                    entry.value.as_str(),
                    *branch_val,
                    "key {} should have branch value",
                    entry.key
                );
            }
        }
    }

    #[test]
    fn limit_is_respected(
        dirty in arb_dirty(20),
        parent in arb_kv(20),
        limit in 1..50_u32,
    ) {
        let result = merge_scan(&dirty, &parent, "", limit);
        prop_assert!(
            result.len() <= usize::try_from(limit).unwrap_or(usize::MAX),
            "result has {} entries, limit was {}",
            result.len(),
            limit
        );
    }
}
