//! Verification policy for federated resources.
//!
//! Controls how data from remote seeders is validated before acceptance.

use serde::Deserialize;
use serde::Serialize;

use crate::discovery::DiscoveredSeeder;
use crate::trust::TrustLevel;
use crate::trust::TrustManager;
use crate::verified::QuorumCheckResult;
use crate::verified::QuorumFailure;
use crate::verified::SeederReport;
use crate::verified::calculate_seeder_quorum;
use crate::verified::check_quorum;

// ============================================================================
// Configuration
// ============================================================================

/// Verification requirements for a federated resource.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VerificationConfig {
    /// Minimum number of trusted seeders that must agree on resource state.
    ///
    /// - `0` = no quorum required (accept any single seeder's data)
    /// - `1` = at least one trusted seeder (basic trust check)
    /// - `N` = N trusted seeders must agree (Byzantine fault tolerance)
    ///
    /// Use [`VerificationConfig::dynamic_majority()`] to compute threshold
    /// dynamically from the number of available trusted seeders.
    pub quorum_threshold: u32,

    /// Whether to use dynamic majority instead of fixed threshold.
    ///
    /// When `true`, the effective threshold is:
    /// `max(quorum_threshold, calculate_seeder_quorum(trusted_seeder_count))`
    #[serde(default)]
    pub use_dynamic_majority: bool,

    /// Whether delegate signatures are required on updates.
    ///
    /// App-specific: Forge uses this for ref signing by authorized delegates.
    /// Other apps may not need signatures.
    #[serde(default)]
    pub require_delegate_signatures: bool,

    /// Maximum acceptable age of a seeder report (microseconds).
    ///
    /// Reports older than this are excluded from quorum calculation.
    /// `0` = no age limit.
    #[serde(default)]
    pub max_report_age_us: u64,
}

impl VerificationConfig {
    /// No verification — accept any seeder's data.
    ///
    /// Suitable for CRDTs and other conflict-free data types.
    pub fn none() -> Self {
        Self::default()
    }

    /// Origin authority — require at least one trusted seeder.
    pub fn origin_authority() -> Self {
        Self {
            quorum_threshold: 1,
            ..Self::default()
        }
    }

    /// Fixed quorum — require exactly N trusted seeders to agree.
    pub fn quorum(threshold: u32) -> Self {
        Self {
            quorum_threshold: threshold,
            ..Self::default()
        }
    }

    /// Dynamic majority — threshold adapts to available seeder count.
    ///
    /// Effective threshold = `max(min_threshold, (trusted_count / 2) + 1)`
    pub fn dynamic_majority(min_threshold: u32) -> Self {
        Self {
            quorum_threshold: min_threshold,
            use_dynamic_majority: true,
            ..Self::default()
        }
    }

    /// Require delegate signatures on updates.
    pub fn with_delegate_signatures(mut self) -> Self {
        self.require_delegate_signatures = true;
        self
    }

    /// Set maximum acceptable report age.
    pub fn with_max_report_age_us(mut self, max_age_us: u64) -> Self {
        self.max_report_age_us = max_age_us;
        self
    }

    /// Whether quorum verification is enabled.
    pub fn is_quorum_enabled(&self) -> bool {
        self.quorum_threshold > 0 || self.use_dynamic_majority
    }
}

// ============================================================================
// Seeder Quorum Verifier
// ============================================================================

/// Verifies quorum agreement among seeders before accepting data.
///
/// This is the integration layer between discovery (seeders), trust
/// (who counts), and the pure quorum/fork detection functions.
pub struct SeederQuorumVerifier<'a> {
    config: &'a VerificationConfig,
    trust_manager: &'a TrustManager,
}

impl<'a> SeederQuorumVerifier<'a> {
    /// Create a new verifier.
    pub fn new(config: &'a VerificationConfig, trust_manager: &'a TrustManager) -> Self {
        Self { config, trust_manager }
    }

    /// Verify quorum across discovered seeders.
    ///
    /// Converts `DiscoveredSeeder`s into `SeederReport`s (enriching with
    /// trust level), then delegates to the pure `check_quorum()` function.
    ///
    /// # Arguments
    ///
    /// * `seeders` - Discovered seeders for a resource
    /// * `now_us` - Current time (microseconds) for staleness filtering
    ///
    /// # Returns
    ///
    /// * `Ok(QuorumCheckResult)` if quorum is met
    /// * `Err(QuorumFailure)` if verification fails
    pub fn verify(&self, seeders: &[DiscoveredSeeder], now_us: u64) -> Result<QuorumCheckResult, QuorumFailure> {
        if !self.config.is_quorum_enabled() {
            // Quorum disabled — return all heads from first seeder (if any)
            if let Some(first) = seeders.first() {
                return Ok(QuorumCheckResult {
                    canonical_heads: first.ref_heads.clone(),
                    trusted_seeder_count: 0,
                    agreement_counts: std::collections::HashMap::new(),
                });
            }
            return Err(QuorumFailure::NoReports);
        }

        // Convert seeders to reports
        let reports = self.to_reports(seeders, now_us);

        // Calculate effective threshold
        let trusted_count = reports.iter().filter(|r| r.trust_level == TrustLevel::Trusted).count() as u32;

        let threshold = if self.config.use_dynamic_majority {
            self.config.quorum_threshold.max(calculate_seeder_quorum(trusted_count))
        } else {
            self.config.quorum_threshold
        };

        // Delegate to pure function
        check_quorum(&reports, threshold)
    }

    /// Convert discovered seeders into seeder reports with trust enrichment.
    fn to_reports(&self, seeders: &[DiscoveredSeeder], _now_us: u64) -> Vec<SeederReport> {
        seeders
            .iter()
            .filter(|s| {
                // Filter out stale reports if max_report_age_us is set
                if self.config.max_report_age_us > 0 {
                    // Note: discovered_at is Instant, so we compute approximate age
                    let age_us = s.discovered_at.elapsed().as_micros() as u64;
                    age_us <= self.config.max_report_age_us
                } else {
                    true
                }
            })
            .map(|s| {
                let trust_level = self.trust_manager.trust_level(&s.cluster_key);
                SeederReport {
                    cluster_key: *s.cluster_key.as_bytes(),
                    heads: s.ref_heads.clone(),
                    timestamp_us: s.discovered_at.elapsed().as_micros() as u64,
                    trust_level,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::Instant;

    use iroh::PublicKey;

    use super::*;
    use crate::discovery::DiscoveredSeeder;
    use crate::types::FederatedId;

    fn test_key() -> PublicKey {
        iroh::SecretKey::generate(&mut rand::rng()).public()
    }

    fn test_fed_id() -> FederatedId {
        FederatedId::new(test_key(), [0xab; 32])
    }

    fn make_seeder(cluster_key: PublicKey, heads: Vec<(&str, [u8; 32])>) -> DiscoveredSeeder {
        DiscoveredSeeder {
            fed_id: test_fed_id(),
            cluster_key,
            node_keys: vec![],
            relay_urls: vec![],
            ref_heads: heads.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
            discovered_at: Instant::now(),
        }
    }

    #[test]
    fn test_verification_config_defaults() {
        let config = VerificationConfig::default();
        assert_eq!(config.quorum_threshold, 0);
        assert!(!config.use_dynamic_majority);
        assert!(!config.require_delegate_signatures);
        assert!(!config.is_quorum_enabled());
    }

    #[test]
    fn test_verification_config_quorum() {
        let config = VerificationConfig::quorum(3);
        assert_eq!(config.quorum_threshold, 3);
        assert!(config.is_quorum_enabled());
    }

    #[test]
    fn test_verification_config_dynamic() {
        let config = VerificationConfig::dynamic_majority(1);
        assert!(config.use_dynamic_majority);
        assert!(config.is_quorum_enabled());
    }

    #[test]
    fn test_verifier_quorum_disabled() {
        let config = VerificationConfig::none();
        let trust = TrustManager::new();
        let verifier = SeederQuorumVerifier::new(&config, &trust);

        let key = test_key();
        let seeders = vec![make_seeder(key, vec![("main", [0xaa; 32])])];

        let result = verifier.verify(&seeders, 0).unwrap();
        assert_eq!(result.canonical_heads.get("main"), Some(&[0xaa; 32]));
    }

    #[test]
    fn test_verifier_quorum_passes() {
        let config = VerificationConfig::quorum(2);
        let trust = TrustManager::new();

        let key1 = test_key();
        let key2 = test_key();
        trust.add_trusted(key1, "cluster1".to_string(), None);
        trust.add_trusted(key2, "cluster2".to_string(), None);

        let verifier = SeederQuorumVerifier::new(&config, &trust);

        let hash = [0xaa; 32];
        let seeders = vec![
            make_seeder(key1, vec![("main", hash)]),
            make_seeder(key2, vec![("main", hash)]),
        ];

        let result = verifier.verify(&seeders, 0).unwrap();
        assert_eq!(result.canonical_heads.get("main"), Some(&hash));
        assert_eq!(result.trusted_seeder_count, 2);
    }

    #[test]
    fn test_verifier_quorum_fails_insufficient() {
        let config = VerificationConfig::quorum(3);
        let trust = TrustManager::new();

        let key1 = test_key();
        trust.add_trusted(key1, "cluster1".to_string(), None);

        let verifier = SeederQuorumVerifier::new(&config, &trust);

        let seeders = vec![make_seeder(key1, vec![("main", [0xaa; 32])])];

        let result = verifier.verify(&seeders, 0);
        assert!(matches!(result, Err(QuorumFailure::InsufficientSeeders { have: 1, need: 3 })));
    }

    #[test]
    fn test_verifier_dynamic_majority() {
        let config = VerificationConfig::dynamic_majority(1);
        let trust = TrustManager::new();

        let key1 = test_key();
        let key2 = test_key();
        let key3 = test_key();
        trust.add_trusted(key1, "c1".to_string(), None);
        trust.add_trusted(key2, "c2".to_string(), None);
        trust.add_trusted(key3, "c3".to_string(), None);

        let verifier = SeederQuorumVerifier::new(&config, &trust);

        let hash = [0xaa; 32];
        // 3 trusted seeders → dynamic quorum = max(1, (3/2)+1) = 2
        let seeders = vec![
            make_seeder(key1, vec![("main", hash)]),
            make_seeder(key2, vec![("main", hash)]),
            make_seeder(key3, vec![("main", [0xbb; 32])]),
        ];

        let result = verifier.verify(&seeders, 0).unwrap();
        assert_eq!(result.canonical_heads.get("main"), Some(&hash));
    }

    #[test]
    fn test_verification_config_roundtrip() {
        let config = VerificationConfig::dynamic_majority(2);
        let bytes = postcard::to_allocvec(&config).unwrap();
        let parsed: VerificationConfig = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.quorum_threshold, 2);
        assert!(parsed.use_dynamic_majority);
    }
}
