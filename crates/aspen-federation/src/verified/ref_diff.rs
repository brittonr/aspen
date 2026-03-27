//! Pure ref-diff computation for bidirectional federation sync.
//!
//! Compares local and remote ref heads to determine what needs to be
//! pulled, pushed, or is already in sync. Divergent refs (same name,
//! different hash) are reported as conflicts.
//!
//! Formally verified — see `verus/` for proofs (when added).

use std::collections::HashMap;

/// Result of comparing local and remote ref heads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefDiff {
    /// Refs where the remote has them and local does not → pull.
    pub to_pull: Vec<String>,
    /// Refs where local has them and remote does not → push.
    pub to_push: Vec<String>,
    /// Refs with matching hashes on both sides.
    pub in_sync: Vec<String>,
    /// Refs that exist on both sides with different hashes.
    /// No ancestry info available, so direction is ambiguous.
    pub conflicts: Vec<String>,
}

/// Compare local and remote ref heads to produce a categorized diff.
///
/// Pure function: no I/O, no async, deterministic output for given inputs.
/// Results are sorted alphabetically within each category for determinism.
///
/// - `to_pull`: ref exists only on remote
/// - `to_push`: ref exists only on local
/// - `in_sync`: ref exists on both with same hash
/// - `conflicts`: ref exists on both with different hashes
#[inline]
pub fn compute_ref_diff(local_heads: &HashMap<String, [u8; 32]>, remote_heads: &HashMap<String, [u8; 32]>) -> RefDiff {
    let mut to_pull = Vec::new();
    let mut to_push = Vec::new();
    let mut in_sync = Vec::new();
    let mut conflicts = Vec::new();

    // Check all remote refs against local
    for (ref_name, remote_hash) in remote_heads {
        match local_heads.get(ref_name) {
            Some(local_hash) if local_hash == remote_hash => {
                in_sync.push(ref_name.clone());
            }
            Some(_) => {
                // Different hashes — conflict
                conflicts.push(ref_name.clone());
            }
            None => {
                // Remote-only → pull
                to_pull.push(ref_name.clone());
            }
        }
    }

    // Check for local-only refs (not in remote)
    for ref_name in local_heads.keys() {
        if !remote_heads.contains_key(ref_name) {
            to_push.push(ref_name.clone());
        }
    }

    // Sort for deterministic output
    to_pull.sort();
    to_push.sort();
    in_sync.sort();
    conflicts.sort();

    RefDiff {
        to_pull,
        to_push,
        in_sync,
        conflicts,
    }
}

/// Resolve conflicts by moving them into the pull or push list.
///
/// - `pull_wins = true`: conflicts go to `to_pull` (remote wins, default)
/// - `pull_wins = false`: conflicts go to `to_push` (local wins, `--push-wins`)
#[inline]
pub fn resolve_conflicts(diff: &mut RefDiff, pull_wins: bool) {
    if diff.conflicts.is_empty() {
        return;
    }

    let resolved = std::mem::take(&mut diff.conflicts);
    if pull_wins {
        diff.to_pull.extend(resolved);
        diff.to_pull.sort();
    } else {
        diff.to_push.extend(resolved);
        diff.to_push.sort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    #[test]
    fn test_empty_inputs() {
        let diff = compute_ref_diff(&HashMap::new(), &HashMap::new());
        assert!(diff.to_pull.is_empty());
        assert!(diff.to_push.is_empty());
        assert!(diff.in_sync.is_empty());
        assert!(diff.conflicts.is_empty());
    }

    #[test]
    fn test_all_in_sync() {
        let mut local = HashMap::new();
        local.insert("heads/main".to_string(), h(0xaa));
        local.insert("heads/dev".to_string(), h(0xbb));

        let remote = local.clone();
        let diff = compute_ref_diff(&local, &remote);

        assert!(diff.to_pull.is_empty());
        assert!(diff.to_push.is_empty());
        assert_eq!(diff.in_sync, vec!["heads/dev", "heads/main"]);
        assert!(diff.conflicts.is_empty());
    }

    #[test]
    fn test_pull_only() {
        let local = HashMap::new();
        let mut remote = HashMap::new();
        remote.insert("heads/main".to_string(), h(0xaa));
        remote.insert("heads/dev".to_string(), h(0xbb));

        let diff = compute_ref_diff(&local, &remote);

        assert_eq!(diff.to_pull, vec!["heads/dev", "heads/main"]);
        assert!(diff.to_push.is_empty());
        assert!(diff.in_sync.is_empty());
        assert!(diff.conflicts.is_empty());
    }

    #[test]
    fn test_push_only() {
        let mut local = HashMap::new();
        local.insert("heads/feature".to_string(), h(0xcc));
        let remote = HashMap::new();

        let diff = compute_ref_diff(&local, &remote);

        assert!(diff.to_pull.is_empty());
        assert_eq!(diff.to_push, vec!["heads/feature"]);
        assert!(diff.in_sync.is_empty());
        assert!(diff.conflicts.is_empty());
    }

    #[test]
    fn test_mixed() {
        let mut local = HashMap::new();
        local.insert("heads/main".to_string(), h(0xaa)); // in sync
        local.insert("heads/feature".to_string(), h(0xbb)); // local-only

        let mut remote = HashMap::new();
        remote.insert("heads/main".to_string(), h(0xaa)); // in sync
        remote.insert("heads/experiment".to_string(), h(0xcc)); // remote-only

        let diff = compute_ref_diff(&local, &remote);

        assert_eq!(diff.to_pull, vec!["heads/experiment"]);
        assert_eq!(diff.to_push, vec!["heads/feature"]);
        assert_eq!(diff.in_sync, vec!["heads/main"]);
        assert!(diff.conflicts.is_empty());
    }

    #[test]
    fn test_divergent_conflicts() {
        let mut local = HashMap::new();
        local.insert("heads/main".to_string(), h(0xaa));

        let mut remote = HashMap::new();
        remote.insert("heads/main".to_string(), h(0xff));

        let diff = compute_ref_diff(&local, &remote);

        assert!(diff.to_pull.is_empty());
        assert!(diff.to_push.is_empty());
        assert!(diff.in_sync.is_empty());
        assert_eq!(diff.conflicts, vec!["heads/main"]);
    }

    #[test]
    fn test_resolve_conflicts_pull_wins() {
        let mut diff = RefDiff {
            to_pull: vec!["heads/new".to_string()],
            to_push: vec![],
            in_sync: vec![],
            conflicts: vec!["heads/main".to_string()],
        };

        resolve_conflicts(&mut diff, true);

        assert_eq!(diff.to_pull, vec!["heads/main", "heads/new"]);
        assert!(diff.conflicts.is_empty());
    }

    #[test]
    fn test_resolve_conflicts_push_wins() {
        let mut diff = RefDiff {
            to_pull: vec![],
            to_push: vec!["heads/local".to_string()],
            in_sync: vec![],
            conflicts: vec!["heads/main".to_string()],
        };

        resolve_conflicts(&mut diff, false);

        assert_eq!(diff.to_push, vec!["heads/local", "heads/main"]);
        assert!(diff.conflicts.is_empty());
    }

    #[test]
    fn test_resolve_empty_conflicts_is_noop() {
        let mut diff = RefDiff {
            to_pull: vec!["a".to_string()],
            to_push: vec!["b".to_string()],
            in_sync: vec!["c".to_string()],
            conflicts: vec![],
        };
        let before = diff.clone();

        resolve_conflicts(&mut diff, true);

        assert_eq!(diff, before);
    }
}
