//! Quorum verification for seeder agreement.
//!
//! Pure functions that check whether enough trusted seeders agree on
//! resource state before accepting it as canonical.
//!
//! Formally verified — see `verus/quorum_spec.rs` for proofs (when added).

use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;

use crate::trust::TrustLevel;

// ============================================================================
// Types
// ============================================================================

/// A report from a single seeder about a resource's ref heads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeederReport {
    /// Public key of the reporting cluster (as bytes for purity).
    pub cluster_key: [u8; 32],
    /// Ref heads reported by this seeder (ref_name → BLAKE3 hash).
    pub heads: HashMap<String, [u8; 32]>,
    /// Timestamp of the report (microseconds since epoch).
    pub timestamp_us: u64,
    /// Trust level of this seeder.
    pub trust_level: TrustLevel,
}

/// Result of a successful quorum check.
#[derive(Debug, Clone, PartialEq)]
pub struct QuorumCheckResult {
    /// The canonical ref heads (agreed upon by quorum).
    pub canonical_heads: HashMap<String, [u8; 32]>,
    /// Number of trusted seeders that participated.
    pub trusted_seeder_count: u32,
    /// Number of seeders that agreed on each ref.
    pub agreement_counts: HashMap<String, u32>,
}

/// Reason a quorum check failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuorumFailure {
    /// Not enough trusted seeders available.
    InsufficientSeeders {
        /// Number of trusted seeders available.
        have: u32,
        /// Number required for quorum.
        need: u32,
    },
    /// Seeders disagree on one or more refs (fork detected).
    ConflictDetected {
        /// Refs with disagreement.
        conflicting_refs: Vec<String>,
    },
    /// No reports provided.
    NoReports,
}

// ============================================================================
// Pure Functions
// ============================================================================

/// Calculate the quorum size for a given number of trusted seeders.
///
/// Uses simple majority: `(n / 2) + 1`.
/// Same formula as Raft consensus (see `aspen-raft/src/verified/membership.rs`).
///
/// # Examples
///
/// - 1 seeder → quorum of 1 (no redundancy)
/// - 2 seeders → quorum of 2 (both must agree)
/// - 3 seeders → quorum of 2 (majority)
/// - 5 seeders → quorum of 3 (majority)
#[inline]
pub fn calculate_seeder_quorum(total_trusted_seeders: u32) -> u32 {
    if total_trusted_seeders == 0 {
        return 1; // Always require at least 1
    }
    (total_trusted_seeders / 2).saturating_add(1)
}

/// Check if enough trusted seeders agree on resource state.
///
/// This is the core quorum verification function. It:
/// 1. Filters to trusted seeders only
/// 2. For each ref, counts how many trusted seeders agree on each hash
/// 3. Requires `threshold` seeders to agree for each ref
/// 4. Returns canonical heads or failure reason
///
/// # Arguments
///
/// * `reports` - State reports from discovered seeders
/// * `threshold` - Minimum number of trusted seeders that must agree (use
///   `calculate_seeder_quorum()` for dynamic majority, or a fixed value)
///
/// # Returns
///
/// * `Ok(QuorumCheckResult)` - Quorum met, with canonical heads
/// * `Err(QuorumFailure)` - Not enough agreement
pub fn check_quorum(reports: &[SeederReport], threshold: u32) -> Result<QuorumCheckResult, QuorumFailure> {
    if reports.is_empty() {
        return Err(QuorumFailure::NoReports);
    }

    // Step 1: Filter to trusted seeders only
    let trusted_reports: Vec<&SeederReport> = reports.iter().filter(|r| r.trust_level == TrustLevel::Trusted).collect();

    let trusted_count = trusted_reports.len() as u32;
    if trusted_count < threshold {
        return Err(QuorumFailure::InsufficientSeeders {
            have: trusted_count,
            need: threshold,
        });
    }

    // Step 2: Collect all ref names across all trusted reports
    let mut all_refs: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for report in &trusted_reports {
        for ref_name in report.heads.keys() {
            all_refs.insert(ref_name.as_str());
        }
    }

    // Step 3: For each ref, tally votes per hash value
    let mut canonical_heads: HashMap<String, [u8; 32]> = HashMap::new();
    let mut agreement_counts: HashMap<String, u32> = HashMap::new();
    let mut conflicting_refs: Vec<String> = Vec::new();

    for ref_name in &all_refs {
        // Count votes for each hash value of this ref
        let mut hash_votes: HashMap<[u8; 32], u32> = HashMap::new();
        for report in &trusted_reports {
            if let Some(hash) = report.heads.get(*ref_name) {
                *hash_votes.entry(*hash).or_insert(0) += 1;
            }
        }

        // Find the hash with the most votes
        if let Some((best_hash, best_count)) = hash_votes.iter().max_by_key(|(_, count)| **count) {
            if *best_count >= threshold {
                canonical_heads.insert(ref_name.to_string(), *best_hash);
                agreement_counts.insert(ref_name.to_string(), *best_count);
            } else {
                // No hash has enough votes — conflict
                conflicting_refs.push(ref_name.to_string());
            }
        }
    }

    if !conflicting_refs.is_empty() {
        return Err(QuorumFailure::ConflictDetected { conflicting_refs });
    }

    Ok(QuorumCheckResult {
        canonical_heads,
        trusted_seeder_count: trusted_count,
        agreement_counts,
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_report(key_byte: u8, heads: Vec<(&str, [u8; 32])>, trust: TrustLevel) -> SeederReport {
        SeederReport {
            cluster_key: [key_byte; 32],
            heads: heads.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
            timestamp_us: 1_000_000,
            trust_level: trust,
        }
    }

    #[test]
    fn test_calculate_seeder_quorum() {
        assert_eq!(calculate_seeder_quorum(0), 1);
        assert_eq!(calculate_seeder_quorum(1), 1);
        assert_eq!(calculate_seeder_quorum(2), 2);
        assert_eq!(calculate_seeder_quorum(3), 2);
        assert_eq!(calculate_seeder_quorum(4), 3);
        assert_eq!(calculate_seeder_quorum(5), 3);
        assert_eq!(calculate_seeder_quorum(6), 4);
        assert_eq!(calculate_seeder_quorum(7), 4);
    }

    #[test]
    fn test_quorum_no_reports() {
        let result = check_quorum(&[], 1);
        assert_eq!(result, Err(QuorumFailure::NoReports));
    }

    #[test]
    fn test_quorum_insufficient_trusted() {
        let reports = vec![
            make_report(1, vec![("main", [0xaa; 32])], TrustLevel::Public),
            make_report(2, vec![("main", [0xaa; 32])], TrustLevel::Public),
        ];
        let result = check_quorum(&reports, 2);
        assert_eq!(result, Err(QuorumFailure::InsufficientSeeders { have: 0, need: 2 }));
    }

    #[test]
    fn test_quorum_all_agree() {
        let hash = [0xaa; 32];
        let reports = vec![
            make_report(1, vec![("main", hash)], TrustLevel::Trusted),
            make_report(2, vec![("main", hash)], TrustLevel::Trusted),
            make_report(3, vec![("main", hash)], TrustLevel::Trusted),
        ];

        let result = check_quorum(&reports, 2).unwrap();
        assert_eq!(result.canonical_heads.get("main"), Some(&hash));
        assert_eq!(result.trusted_seeder_count, 3);
        assert_eq!(result.agreement_counts.get("main"), Some(&3));
    }

    #[test]
    fn test_quorum_majority_agrees() {
        let good_hash = [0xaa; 32];
        let bad_hash = [0xbb; 32];
        let reports = vec![
            make_report(1, vec![("main", good_hash)], TrustLevel::Trusted),
            make_report(2, vec![("main", good_hash)], TrustLevel::Trusted),
            make_report(3, vec![("main", bad_hash)], TrustLevel::Trusted),
        ];

        let result = check_quorum(&reports, 2).unwrap();
        assert_eq!(result.canonical_heads.get("main"), Some(&good_hash));
        assert_eq!(result.agreement_counts.get("main"), Some(&2));
    }

    #[test]
    fn test_quorum_conflict_no_majority() {
        let hash_a = [0xaa; 32];
        let hash_b = [0xbb; 32];
        let hash_c = [0xcc; 32];
        let reports = vec![
            make_report(1, vec![("main", hash_a)], TrustLevel::Trusted),
            make_report(2, vec![("main", hash_b)], TrustLevel::Trusted),
            make_report(3, vec![("main", hash_c)], TrustLevel::Trusted),
        ];

        let result = check_quorum(&reports, 2);
        match result {
            Err(QuorumFailure::ConflictDetected { conflicting_refs }) => {
                assert!(conflicting_refs.contains(&"main".to_string()));
            }
            other => panic!("expected ConflictDetected, got {:?}", other),
        }
    }

    #[test]
    fn test_quorum_ignores_untrusted() {
        let good_hash = [0xaa; 32];
        let evil_hash = [0xff; 32];
        let reports = vec![
            make_report(1, vec![("main", good_hash)], TrustLevel::Trusted),
            make_report(2, vec![("main", good_hash)], TrustLevel::Trusted),
            // Untrusted seeder disagrees — should be ignored
            make_report(3, vec![("main", evil_hash)], TrustLevel::Public),
            make_report(4, vec![("main", evil_hash)], TrustLevel::Blocked),
        ];

        let result = check_quorum(&reports, 2).unwrap();
        assert_eq!(result.canonical_heads.get("main"), Some(&good_hash));
        assert_eq!(result.trusted_seeder_count, 2);
    }

    #[test]
    fn test_quorum_multiple_refs() {
        let main_hash = [0xaa; 32];
        let dev_hash = [0xbb; 32];
        let reports = vec![
            make_report(1, vec![("main", main_hash), ("dev", dev_hash)], TrustLevel::Trusted),
            make_report(2, vec![("main", main_hash), ("dev", dev_hash)], TrustLevel::Trusted),
        ];

        let result = check_quorum(&reports, 2).unwrap();
        assert_eq!(result.canonical_heads.get("main"), Some(&main_hash));
        assert_eq!(result.canonical_heads.get("dev"), Some(&dev_hash));
    }

    #[test]
    fn test_quorum_partial_conflict() {
        // main agrees, dev conflicts
        let main_hash = [0xaa; 32];
        let reports = vec![
            make_report(1, vec![("main", main_hash), ("dev", [0x11; 32])], TrustLevel::Trusted),
            make_report(2, vec![("main", main_hash), ("dev", [0x22; 32])], TrustLevel::Trusted),
        ];

        // With threshold=2, dev can't reach quorum
        let result = check_quorum(&reports, 2);
        match result {
            Err(QuorumFailure::ConflictDetected { conflicting_refs }) => {
                assert!(conflicting_refs.contains(&"dev".to_string()));
            }
            other => panic!("expected ConflictDetected, got {:?}", other),
        }
    }

    #[test]
    fn test_quorum_single_seeder_threshold_one() {
        let hash = [0xaa; 32];
        let reports = vec![make_report(1, vec![("main", hash)], TrustLevel::Trusted)];

        let result = check_quorum(&reports, 1).unwrap();
        assert_eq!(result.canonical_heads.get("main"), Some(&hash));
        assert_eq!(result.trusted_seeder_count, 1);
    }

    #[test]
    fn test_quorum_seeder_missing_ref() {
        // Seeder 1 has main+dev, seeder 2 only has main (dev not yet propagated)
        let main_hash = [0xaa; 32];
        let dev_hash = [0xbb; 32];
        let reports = vec![
            make_report(1, vec![("main", main_hash), ("dev", dev_hash)], TrustLevel::Trusted),
            make_report(2, vec![("main", main_hash)], TrustLevel::Trusted),
        ];

        // With threshold=2: main passes (2/2), dev fails (1/2)
        let result = check_quorum(&reports, 2);
        match result {
            Err(QuorumFailure::ConflictDetected { conflicting_refs }) => {
                assert!(conflicting_refs.contains(&"dev".to_string()));
            }
            other => panic!("expected ConflictDetected for dev, got {:?}", other),
        }
    }

    #[test]
    fn test_quorum_overflow_safety() {
        // Ensure calculate_seeder_quorum handles edge cases
        assert_eq!(calculate_seeder_quorum(u32::MAX), u32::MAX / 2 + 1);
    }
}
