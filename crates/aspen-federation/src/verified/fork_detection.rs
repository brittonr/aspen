//! Fork detection for federated resources.
//!
//! Pure functions that detect when multiple seeders report conflicting
//! state for the same resource. A fork indicates either eventual consistency
//! lag, a split-brain scenario, or a malicious seeder.
//!
//! Formally verified — see `verus/fork_detection_spec.rs` for proofs (when added).

use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;

use super::quorum::SeederReport;
use crate::trust::TrustLevel;

// ============================================================================
// Types
// ============================================================================

/// Information about a detected fork for a single ref.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkInfo {
    /// The ref name that has conflicting state.
    pub ref_name: String,
    /// The competing branches (different hash values claimed).
    pub branches: Vec<ForkBranch>,
}

/// One branch of a fork — a hash value and the seeders supporting it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkBranch {
    /// The hash value claimed by this branch.
    pub head: [u8; 32],
    /// Cluster keys of seeders supporting this branch.
    pub supporters: Vec<[u8; 32]>,
    /// Most recent timestamp among supporters.
    pub latest_timestamp_us: u64,
    /// Number of trusted supporters.
    pub trusted_count: u32,
}

// ============================================================================
// Pure Functions
// ============================================================================

/// Detect forks across seeder reports.
///
/// A fork exists when two or more seeders report different hash values
/// for the same ref name. This function examines ALL seeders (trusted
/// and untrusted) to provide complete fork visibility.
///
/// # Arguments
///
/// * `reports` - State reports from discovered seeders
///
/// # Returns
///
/// List of `ForkInfo` for each ref that has disagreement.
/// Empty list means all seeders agree (no forks).
pub fn detect_ref_forks(reports: &[SeederReport]) -> Vec<ForkInfo> {
    if reports.len() < 2 {
        return Vec::new();
    }

    // Collect all ref names
    let mut all_refs: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for report in reports {
        for ref_name in report.heads.keys() {
            all_refs.insert(ref_name.as_str());
        }
    }

    let mut forks = Vec::new();

    for ref_name in &all_refs {
        // Group seeders by the hash they report for this ref
        let mut hash_groups: HashMap<[u8; 32], Vec<(&SeederReport, bool)>> = HashMap::new();

        for report in reports {
            if let Some(hash) = report.heads.get(*ref_name) {
                let is_trusted = report.trust_level == TrustLevel::Trusted;
                hash_groups.entry(*hash).or_default().push((report, is_trusted));
            }
        }

        // If more than one distinct hash, we have a fork
        if hash_groups.len() > 1 {
            let mut branches: Vec<ForkBranch> = hash_groups
                .into_iter()
                .map(|(hash, supporters)| {
                    let trusted_count = supporters.iter().filter(|(_, t)| *t).count() as u32;
                    let latest_timestamp_us = supporters.iter().map(|(r, _)| r.timestamp_us).max().unwrap_or(0);
                    let supporter_keys: Vec<[u8; 32]> = supporters.iter().map(|(r, _)| r.cluster_key).collect();

                    ForkBranch {
                        head: hash,
                        supporters: supporter_keys,
                        latest_timestamp_us,
                        trusted_count,
                    }
                })
                .collect();

            // Sort branches by trusted count descending, then by timestamp
            branches.sort_by(|a, b| {
                b.trusted_count.cmp(&a.trusted_count).then(b.latest_timestamp_us.cmp(&a.latest_timestamp_us))
            });

            forks.push(ForkInfo {
                ref_name: ref_name.to_string(),
                branches,
            });
        }
    }

    forks
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_report(key_byte: u8, heads: Vec<(&str, [u8; 32])>, trust: TrustLevel, ts: u64) -> SeederReport {
        SeederReport {
            cluster_key: [key_byte; 32],
            heads: heads.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
            timestamp_us: ts,
            trust_level: trust,
        }
    }

    #[test]
    fn test_no_forks_single_report() {
        let reports = vec![make_report(1, vec![("main", [0xaa; 32])], TrustLevel::Trusted, 100)];
        assert!(detect_ref_forks(&reports).is_empty());
    }

    #[test]
    fn test_no_forks_all_agree() {
        let hash = [0xaa; 32];
        let reports = vec![
            make_report(1, vec![("main", hash)], TrustLevel::Trusted, 100),
            make_report(2, vec![("main", hash)], TrustLevel::Trusted, 200),
            make_report(3, vec![("main", hash)], TrustLevel::Public, 300),
        ];
        assert!(detect_ref_forks(&reports).is_empty());
    }

    #[test]
    fn test_fork_detected() {
        let hash_a = [0xaa; 32];
        let hash_b = [0xbb; 32];
        let reports = vec![
            make_report(1, vec![("main", hash_a)], TrustLevel::Trusted, 100),
            make_report(2, vec![("main", hash_b)], TrustLevel::Trusted, 200),
        ];

        let forks = detect_ref_forks(&reports);
        assert_eq!(forks.len(), 1);
        assert_eq!(forks[0].ref_name, "main");
        assert_eq!(forks[0].branches.len(), 2);
    }

    #[test]
    fn test_fork_branches_sorted_by_trust() {
        let trusted_hash = [0xaa; 32];
        let untrusted_hash = [0xbb; 32];
        let reports = vec![
            make_report(1, vec![("main", trusted_hash)], TrustLevel::Trusted, 100),
            make_report(2, vec![("main", trusted_hash)], TrustLevel::Trusted, 200),
            make_report(3, vec![("main", untrusted_hash)], TrustLevel::Public, 300),
        ];

        let forks = detect_ref_forks(&reports);
        assert_eq!(forks.len(), 1);
        // First branch should be the trusted one
        assert_eq!(forks[0].branches[0].head, trusted_hash);
        assert_eq!(forks[0].branches[0].trusted_count, 2);
        assert_eq!(forks[0].branches[1].head, untrusted_hash);
        assert_eq!(forks[0].branches[1].trusted_count, 0);
    }

    #[test]
    fn test_fork_three_way() {
        let reports = vec![
            make_report(1, vec![("main", [0xaa; 32])], TrustLevel::Trusted, 100),
            make_report(2, vec![("main", [0xbb; 32])], TrustLevel::Trusted, 200),
            make_report(3, vec![("main", [0xcc; 32])], TrustLevel::Trusted, 300),
        ];

        let forks = detect_ref_forks(&reports);
        assert_eq!(forks.len(), 1);
        assert_eq!(forks[0].branches.len(), 3);
    }

    #[test]
    fn test_fork_only_on_disagreeing_refs() {
        let agreed_hash = [0xaa; 32];
        let reports = vec![
            make_report(1, vec![("main", agreed_hash), ("dev", [0x11; 32])], TrustLevel::Trusted, 100),
            make_report(2, vec![("main", agreed_hash), ("dev", [0x22; 32])], TrustLevel::Trusted, 200),
        ];

        let forks = detect_ref_forks(&reports);
        assert_eq!(forks.len(), 1);
        assert_eq!(forks[0].ref_name, "dev");
    }

    #[test]
    fn test_empty_reports() {
        assert!(detect_ref_forks(&[]).is_empty());
    }

    #[test]
    fn test_fork_supporter_tracking() {
        let hash_a = [0xaa; 32];
        let hash_b = [0xbb; 32];
        let reports = vec![
            make_report(1, vec![("main", hash_a)], TrustLevel::Trusted, 100),
            make_report(2, vec![("main", hash_a)], TrustLevel::Trusted, 200),
            make_report(3, vec![("main", hash_b)], TrustLevel::Trusted, 300),
        ];

        let forks = detect_ref_forks(&reports);
        let fork = &forks[0];

        // Branch with 2 supporters should be first (higher trusted_count)
        assert_eq!(fork.branches[0].supporters.len(), 2);
        assert_eq!(fork.branches[0].trusted_count, 2);
        assert_eq!(fork.branches[1].supporters.len(), 1);
        assert_eq!(fork.branches[1].trusted_count, 1);
    }
}
