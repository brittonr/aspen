//! Deterministic sorted merge of branch dirty entries with parent scan results.
//!
//! Formally verified — see `verus/scan_merge_spec.rs` for proofs.
//!
//! # Properties verified
//!
//! 1. **Tombstone exclusion**: No tombstoned key appears in the output.
//! 2. **Sort order**: Output is lexicographically sorted by key.
//! 3. **Branch precedence**: If a key exists in both branch and parent, the branch value wins.

use aspen_kv_types::KeyValueWithRevision;

use crate::entry::BranchEntry;

/// Merge branch dirty entries with parent scan results.
///
/// Both inputs must be sorted by key. The output is a sorted, deduplicated
/// merge where:
/// - Branch writes override parent values for the same key
/// - Tombstoned keys are excluded
/// - Only branch entries matching `prefix` are included
/// - Output is truncated to `limit` entries
///
/// This is a pure function with no I/O, no async, and no time dependency.
#[inline]
pub fn merge_scan(
    dirty_sorted: &[(String, BranchEntry)],
    parent_entries: &[KeyValueWithRevision],
    prefix: &str,
    limit_entries: u32,
) -> Vec<KeyValueWithRevision> {
    let limit = usize::try_from(limit_entries).unwrap_or(usize::MAX);
    let total_inputs = dirty_sorted.len().saturating_add(parent_entries.len());
    let mut result = Vec::with_capacity(limit.min(total_inputs));

    // Collect tombstoned keys for quick lookup during parent iteration.
    // Also collect branch writes that match the prefix.
    let mut branch_writes: Vec<(&str, &str)> = Vec::with_capacity(dirty_sorted.len());
    let mut tombstones: Vec<&str> = Vec::with_capacity(dirty_sorted.len());

    for (key, entry) in dirty_sorted {
        if !key.starts_with(prefix) {
            continue;
        }
        match entry {
            BranchEntry::Write { value } => {
                branch_writes.push((key.as_str(), value.as_str()));
            }
            BranchEntry::Tombstone => {
                tombstones.push(key.as_str());
            }
        }
    }

    // Two-pointer merge: branch_writes and parent_entries are both sorted.
    let mut bi = 0; // branch index
    let mut pi = 0; // parent index

    while result.len() < limit && (bi < branch_writes.len() || pi < parent_entries.len()) {
        let take_branch = match (branch_writes.get(bi), parent_entries.get(pi)) {
            (Some((bk, _)), Some(pe)) => {
                if *bk == pe.key.as_str() {
                    // Branch takes precedence on duplicate keys.
                    pi += 1; // Skip parent entry.
                    true
                } else {
                    *bk < pe.key.as_str()
                }
            }
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => break,
        };

        if take_branch {
            let (key, value) = branch_writes[bi];
            result.push(KeyValueWithRevision {
                key: key.to_owned(),
                value: value.to_owned(),
                version: 0,
                create_revision: 0,
                mod_revision: 0,
            });
            bi += 1;
        } else {
            let pe = &parent_entries[pi];
            pi += 1;

            // Skip tombstoned keys.
            if is_tombstoned(&pe.key, &tombstones) {
                continue;
            }

            result.push(pe.clone());
        }
    }

    result
}

/// Check if a key is in the tombstone list.
/// The tombstones slice is sorted (inherits sort from dirty_sorted).
#[inline]
fn is_tombstoned(key: &str, tombstones: &[&str]) -> bool {
    tombstones.binary_search(&key).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kv(key: &str, value: &str) -> KeyValueWithRevision {
        KeyValueWithRevision {
            key: key.to_owned(),
            value: value.to_owned(),
            version: 1,
            create_revision: 1,
            mod_revision: 1,
        }
    }

    #[test]
    fn empty_inputs() {
        let result = merge_scan(&[], &[], "", 100);
        assert!(result.is_empty());
    }

    #[test]
    fn branch_only() {
        let dirty = vec![
            ("a".into(), BranchEntry::Write { value: "1".into() }),
            ("b".into(), BranchEntry::Write { value: "2".into() }),
        ];
        let result = merge_scan(&dirty, &[], "", 100);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].key, "a");
        assert_eq!(result[1].key, "b");
    }

    #[test]
    fn parent_only() {
        let parent = vec![kv("a", "1"), kv("b", "2")];
        let result = merge_scan(&[], &parent, "", 100);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].key, "a");
        assert_eq!(result[1].key, "b");
    }

    #[test]
    fn branch_overrides_parent() {
        let dirty = vec![("a".into(), BranchEntry::Write { value: "branch".into() })];
        let parent = vec![kv("a", "parent")];
        let result = merge_scan(&dirty, &parent, "", 100);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].key, "a");
        assert_eq!(result[0].value, "branch");
    }

    #[test]
    fn tombstone_removes_parent() {
        let dirty = vec![("b".into(), BranchEntry::Tombstone)];
        let parent = vec![kv("a", "1"), kv("b", "2"), kv("c", "3")];
        let result = merge_scan(&dirty, &parent, "", 100);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].key, "a");
        assert_eq!(result[1].key, "c");
    }

    #[test]
    fn sorted_merge() {
        let dirty = vec![
            ("b".into(), BranchEntry::Write { value: "b".into() }),
            ("e".into(), BranchEntry::Write { value: "e".into() }),
        ];
        let parent = vec![kv("a", "a"), kv("d", "d"), kv("f", "f")];
        let result = merge_scan(&dirty, &parent, "", 100);
        assert_eq!(result.len(), 5);
        let keys: Vec<&str> = result.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(keys, vec!["a", "b", "d", "e", "f"]);
    }

    #[test]
    fn limit_respected() {
        let dirty = vec![
            ("a".into(), BranchEntry::Write { value: "1".into() }),
            ("b".into(), BranchEntry::Write { value: "2".into() }),
            ("c".into(), BranchEntry::Write { value: "3".into() }),
        ];
        let result = merge_scan(&dirty, &[], "", 2);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].key, "a");
        assert_eq!(result[1].key, "b");
    }

    #[test]
    fn prefix_filters_branch_entries() {
        let dirty = vec![
            ("config/db".into(), BranchEntry::Write { value: "val".into() }),
            ("users/admin".into(), BranchEntry::Write { value: "val".into() }),
        ];
        let result = merge_scan(&dirty, &[], "config/", 100);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].key, "config/db");
    }

    #[test]
    fn tombstone_outside_prefix_ignored() {
        let dirty = vec![("users/admin".into(), BranchEntry::Tombstone)];
        let parent = vec![kv("config/a", "1")];
        let result = merge_scan(&dirty, &parent, "config/", 100);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].key, "config/a");
    }
}
