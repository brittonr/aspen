//! Pure diff function comparing two commits' mutation snapshots.
//!
//! Uses a two-pointer merge over sorted mutation lists (same pattern as
//! `merge_scan` in `aspen-kv-branch`).
//!
//! Formally verified — see `verus/diff_spec.rs` for proofs.

use crate::types::Commit;
use crate::types::DiffEntry;

/// Compare two commits' mutation snapshots and return a list of differences.
///
/// Both commits' mutations must be sorted by key (they always are —
/// `BranchOverlay.commit()` sorts them). The output is also sorted by key.
///
/// - `Added`: key in `b` but not in `a`
/// - `Removed`: key in `a` but not in `b`
/// - `Changed`: key in both but with different `MutationType`
pub fn diff(a: &Commit, b: &Commit) -> Vec<DiffEntry> {
    debug_assert!(
        a.mutations.windows(2).all(|w| w[0].0 <= w[1].0),
        "a.mutations must be sorted by key"
    );
    debug_assert!(
        b.mutations.windows(2).all(|w| w[0].0 <= w[1].0),
        "b.mutations must be sorted by key"
    );
    let max_entries = a.mutations.len().saturating_add(b.mutations.len());
    let mut result = Vec::with_capacity(max_entries);
    let mut i = 0usize;
    let mut j = 0usize;

    let a_muts = &a.mutations;
    let b_muts = &b.mutations;

    while i < a_muts.len() && j < b_muts.len() {
        let (ref a_key, ref a_val) = a_muts[i];
        let (ref b_key, ref b_val) = b_muts[j];

        match a_key.cmp(b_key) {
            std::cmp::Ordering::Less => {
                // Key only in a → removed in b
                result.push(DiffEntry::Removed { key: a_key.clone() });
                i += 1;
            }
            std::cmp::Ordering::Greater => {
                // Key only in b → added in b
                result.push(DiffEntry::Added {
                    key: b_key.clone(),
                    value: b_val.clone(),
                });
                j += 1;
            }
            std::cmp::Ordering::Equal => {
                // Key in both — check if mutation differs
                if a_val != b_val {
                    result.push(DiffEntry::Changed {
                        key: a_key.clone(),
                        old: a_val.clone(),
                        new: b_val.clone(),
                    });
                }
                i += 1;
                j += 1;
            }
        }
    }

    // Remaining in a → removed in b
    while i < a_muts.len() {
        result.push(DiffEntry::Removed {
            key: a_muts[i].0.clone(),
        });
        i += 1;
    }

    // Remaining in b → added in b
    while j < b_muts.len() {
        result.push(DiffEntry::Added {
            key: b_muts[j].0.clone(),
            value: b_muts[j].1.clone(),
        });
        j += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MutationType;
    use crate::verified::commit_hash::compute_commit_id;
    use crate::verified::commit_hash::compute_mutations_hash;

    fn make_commit(branch_id: &str, mutations: Vec<(String, MutationType)>) -> Commit {
        let mutations_hash = compute_mutations_hash(&mutations);
        let id = compute_commit_id(&None, &mutations_hash, branch_id, 1, 1000);
        Commit {
            id,
            parent: None,
            branch_id: branch_id.to_string(),
            mutations,
            mutations_hash,
            raft_revision: 1,
            chain_hash_at_commit: [0u8; 32],
            timestamp_ms: 1000,
        }
    }

    #[test]
    fn identical_commits_empty_diff() {
        let c = make_commit("b", vec![("a".into(), MutationType::Set("1".into()))]);
        assert!(diff(&c, &c).is_empty());
    }

    #[test]
    fn added_keys() {
        let a = make_commit("b", vec![("a".into(), MutationType::Set("1".into()))]);
        let b = make_commit("b", vec![
            ("a".into(), MutationType::Set("1".into())),
            ("b".into(), MutationType::Set("2".into())),
        ]);

        let d = diff(&a, &b);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0], DiffEntry::Added {
            key: "b".into(),
            value: MutationType::Set("2".into()),
        });
    }

    #[test]
    fn removed_keys() {
        let a = make_commit("b", vec![
            ("a".into(), MutationType::Set("1".into())),
            ("b".into(), MutationType::Set("2".into())),
        ]);
        let b = make_commit("b", vec![("a".into(), MutationType::Set("1".into()))]);

        let d = diff(&a, &b);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0], DiffEntry::Removed { key: "b".into() });
    }

    #[test]
    fn changed_values() {
        let a = make_commit("b", vec![("a".into(), MutationType::Set("1".into()))]);
        let b = make_commit("b", vec![("a".into(), MutationType::Set("2".into()))]);

        let d = diff(&a, &b);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0], DiffEntry::Changed {
            key: "a".into(),
            old: MutationType::Set("1".into()),
            new: MutationType::Set("2".into()),
        });
    }

    #[test]
    fn tombstone_transitions() {
        let a = make_commit("b", vec![("a".into(), MutationType::Set("1".into()))]);
        let b = make_commit("b", vec![("a".into(), MutationType::Delete)]);

        let d = diff(&a, &b);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0], DiffEntry::Changed {
            key: "a".into(),
            old: MutationType::Set("1".into()),
            new: MutationType::Delete,
        });
    }

    #[test]
    fn mixed_changes() {
        let a = make_commit("b", vec![
            ("a".into(), MutationType::Set("1".into())),
            ("b".into(), MutationType::Set("2".into())),
            ("c".into(), MutationType::Delete),
        ]);
        let b = make_commit("b", vec![
            ("a".into(), MutationType::Set("changed".into())),
            ("c".into(), MutationType::Set("restored".into())),
            ("d".into(), MutationType::Set("new".into())),
        ]);

        let d = diff(&a, &b);
        assert_eq!(d.len(), 4);
        assert_eq!(d[0], DiffEntry::Changed {
            key: "a".into(),
            old: MutationType::Set("1".into()),
            new: MutationType::Set("changed".into()),
        });
        assert_eq!(d[1], DiffEntry::Removed { key: "b".into() });
        assert_eq!(d[2], DiffEntry::Changed {
            key: "c".into(),
            old: MutationType::Delete,
            new: MutationType::Set("restored".into()),
        });
        assert_eq!(d[3], DiffEntry::Added {
            key: "d".into(),
            value: MutationType::Set("new".into()),
        });
    }

    #[test]
    fn both_empty() {
        let a = make_commit("b", vec![]);
        let b = make_commit("b", vec![]);
        assert!(diff(&a, &b).is_empty());
    }
}
