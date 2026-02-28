//! Multi-seeder sync orchestration with quorum and fork detection.
//!
//! Coordinates syncing a federated resource from multiple seeders,
//! verifying that enough trusted seeders agree on state (quorum) and
//! detecting when seeders disagree (fork detection) before fetching data.
//!
//! This is the high-level entry point for sync — it calls the client
//! functions (`get_remote_resource_state`, `sync_remote_objects`) and
//! uses the verified pure functions (`check_quorum`, `detect_ref_forks`)
//! to decide whether to accept the data.

use std::collections::HashMap;

use anyhow::Context;
use anyhow::Result;
use iroh::Endpoint;
use iroh::PublicKey;
use tracing::debug;
use tracing::info;
use tracing::warn;

use super::client::connect_to_cluster;
use super::client::get_remote_resource_state;
use super::client::sync_remote_objects;
use super::types::SyncObject;
use crate::identity::ClusterIdentity;
use crate::policy::ForkDetectionMode;
use crate::policy::ResourcePolicy;
use crate::trust::TrustLevel;
use crate::trust::TrustManager;
use crate::types::FederatedId;
use crate::verified::ForkInfo;
use crate::verified::SeederReport;
use crate::verified::calculate_seeder_quorum;
use crate::verified::check_quorum;
use crate::verified::detect_ref_forks;

// ============================================================================
// Constants (Tiger Style: Fixed limits)
// ============================================================================

/// Maximum number of seeders to query in parallel.
const MAX_PARALLEL_SEEDERS: usize = 8;

/// Timeout for querying a single seeder's state (seconds).
const SEEDER_QUERY_TIMEOUT_SECS: u64 = 10;

// ============================================================================
// Types
// ============================================================================

/// A seeder that can be queried for resource state.
///
/// The `cluster_key` is used for both Iroh connection (peer ID)
/// and trust lookups.
#[derive(Debug, Clone)]
pub struct SeederEndpoint {
    /// Public key of the seeding cluster (also the Iroh peer ID).
    pub cluster_key: PublicKey,
}

/// Configuration for an orchestrated sync operation.
pub struct SyncRequest<'a> {
    /// Our Iroh endpoint for connecting to seeders.
    pub endpoint: &'a Endpoint,
    /// The federated resource to sync.
    pub fed_id: &'a FederatedId,
    /// Our cluster identity for handshakes.
    pub our_identity: &'a ClusterIdentity,
    /// Known seeders for this resource.
    pub seeders: &'a [SeederEndpoint],
    /// Resource policy (quorum, fork detection, etc.).
    pub policy: &'a ResourcePolicy,
    /// Trust manager for seeder trust lookups.
    pub trust_manager: &'a TrustManager,
    /// Object types to request (e.g., "commit", "blob").
    pub want_types: Vec<String>,
    /// Hashes we already have (to avoid re-fetching).
    pub have_hashes: Vec<[u8; 32]>,
    /// Maximum objects to fetch per round.
    pub limit: u32,
    /// Delegate keys for signature verification (None to skip).
    pub delegates: Option<&'a [PublicKey]>,
}

impl std::fmt::Debug for SyncRequest<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyncRequest")
            .field("fed_id", &self.fed_id.short())
            .field("seeders", &self.seeders.len())
            .field("want_types", &self.want_types)
            .field("limit", &self.limit)
            .finish()
    }
}

/// Result of an orchestrated multi-seeder sync.
#[derive(Debug)]
pub struct OrchestratedSyncResult {
    /// Synced objects from the winning seeder.
    pub objects: Vec<SyncObject>,
    /// Whether the winning seeder has more objects.
    pub has_more: bool,
    /// Canonical ref heads (agreed upon by quorum).
    pub canonical_heads: HashMap<String, [u8; 32]>,
    /// Number of trusted seeders that participated.
    pub trusted_seeder_count: u32,
    /// Forks detected (may be non-empty even if sync succeeded, depending on policy).
    pub detected_forks: Vec<ForkInfo>,
}

/// Error from orchestrated sync.
#[derive(Debug, thiserror::Error)]
pub enum OrchestratedSyncError {
    /// No seeders responded.
    #[error("no seeders responded for resource {fed_id}")]
    NoSeedersResponded { fed_id: String },

    /// Quorum not met — not enough trusted seeders agree.
    #[error("quorum not met for {fed_id}: {reason}")]
    QuorumNotMet { fed_id: String, reason: String },

    /// Fork detected and policy is Halt.
    #[error("fork detected for {fed_id}, halting sync: {fork_count} conflicting refs")]
    ForkHalted { fed_id: String, fork_count: usize },

    /// Connection or protocol error.
    #[error("sync error: {0}")]
    SyncError(#[from] anyhow::Error),
}

// ============================================================================
// Orchestrator
// ============================================================================

/// Orchestrate syncing a federated resource from multiple seeders.
///
/// Queries seeders for resource state, checks quorum agreement, detects
/// forks, and syncs objects from the best seeder per the resource policy.
///
/// # Flow
///
/// 1. Query seeders in parallel for resource state (ref heads)
/// 2. Build seeder reports with trust levels
/// 3. Detect forks (seeders disagreeing on ref values)
/// 4. Enforce fork detection policy (warn, halt, or ignore)
/// 5. Check quorum (enough trusted seeders agree?)
/// 6. Sync objects from the best responding seeder
pub async fn orchestrated_sync(req: &SyncRequest<'_>) -> Result<OrchestratedSyncResult, OrchestratedSyncError> {
    let seeder_count = req.seeders.len().min(MAX_PARALLEL_SEEDERS);
    if seeder_count == 0 {
        return Err(OrchestratedSyncError::NoSeedersResponded {
            fed_id: req.fed_id.short(),
        });
    }

    info!(fed_id = %req.fed_id.short(), seeder_count, "starting orchestrated sync");

    // Step 1: Query seeders for resource state
    let reports =
        query_seeders(req.endpoint, req.fed_id, req.our_identity, &req.seeders[..seeder_count], req.trust_manager)
            .await;

    if reports.is_empty() {
        return Err(OrchestratedSyncError::NoSeedersResponded {
            fed_id: req.fed_id.short(),
        });
    }

    // Step 2: Detect forks
    let detected_forks = detect_ref_forks(&reports);
    enforce_fork_policy(req.fed_id, &detected_forks, &req.policy.fork_detection)?;

    // Step 3: Check quorum
    let threshold = effective_threshold(req.policy, &reports);
    let quorum_result = check_quorum(&reports, threshold).map_err(|failure| OrchestratedSyncError::QuorumNotMet {
        fed_id: req.fed_id.short(),
        reason: format!("{:?}", failure),
    })?;

    info!(
        fed_id = %req.fed_id.short(),
        trusted_seeders = quorum_result.trusted_seeder_count,
        threshold,
        fork_count = detected_forks.len(),
        "quorum verified, syncing objects"
    );

    // Step 4: Pick best seeder and sync objects
    let best_seeder = pick_best_seeder(req.seeders, &reports, req.trust_manager);
    let (objects, has_more) = sync_from_best(req, best_seeder).await?;

    Ok(OrchestratedSyncResult {
        objects,
        has_more,
        canonical_heads: quorum_result.canonical_heads,
        trusted_seeder_count: quorum_result.trusted_seeder_count,
        detected_forks,
    })
}

/// Query multiple seeders for resource state in parallel.
///
/// Returns reports only for seeders that responded successfully.
async fn query_seeders(
    endpoint: &Endpoint,
    fed_id: &FederatedId,
    our_identity: &ClusterIdentity,
    seeders: &[SeederEndpoint],
    trust_manager: &TrustManager,
) -> Vec<SeederReport> {
    let mut tasks = Vec::with_capacity(seeders.len());

    for seeder in seeders {
        let ep = endpoint.clone();
        let fid = *fed_id;
        let ident = our_identity.clone();
        let cluster_key = seeder.cluster_key;

        tasks.push(tokio::spawn(async move {
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(SEEDER_QUERY_TIMEOUT_SECS),
                query_single_seeder(&ep, &fid, &ident, cluster_key),
            )
            .await;

            match result {
                Ok(Ok(heads)) => Some((cluster_key, heads)),
                Ok(Err(e)) => {
                    warn!(cluster = %cluster_key, error = %e, "seeder query failed");
                    None
                }
                Err(_) => {
                    warn!(cluster = %cluster_key, "seeder query timed out");
                    None
                }
            }
        }));
    }

    let mut reports = Vec::with_capacity(tasks.len());
    for task in tasks {
        if let Ok(Some((cluster_key, heads))) = task.await {
            let trust_level = trust_manager.trust_level(&cluster_key);
            reports.push(SeederReport {
                cluster_key: *cluster_key.as_bytes(),
                heads,
                timestamp_us: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_micros() as u64,
                trust_level,
            });
        }
    }

    debug!(fed_id = %fed_id.short(), responding = reports.len(), "seeder queries complete");
    reports
}

/// Query a single seeder for resource state.
async fn query_single_seeder(
    endpoint: &Endpoint,
    fed_id: &FederatedId,
    our_identity: &ClusterIdentity,
    peer_key: PublicKey,
) -> Result<HashMap<String, [u8; 32]>> {
    let (conn, _peer_id) = connect_to_cluster(endpoint, our_identity, peer_key).await.context("handshake failed")?;

    let (was_found, heads, _metadata) =
        get_remote_resource_state(&conn, fed_id).await.context("get resource state failed")?;

    if !was_found {
        anyhow::bail!("resource not found on seeder");
    }

    Ok(heads)
}

/// Enforce fork detection policy.
fn enforce_fork_policy(
    fed_id: &FederatedId,
    forks: &[ForkInfo],
    mode: &ForkDetectionMode,
) -> Result<(), OrchestratedSyncError> {
    if forks.is_empty() {
        return Ok(());
    }

    match mode {
        ForkDetectionMode::Disabled => Ok(()),
        ForkDetectionMode::Warn => {
            for fork in forks {
                warn!(
                    fed_id = %fed_id.short(),
                    ref_name = %fork.ref_name,
                    branches = fork.branches.len(),
                    "fork detected (warn mode, continuing sync)"
                );
            }
            Ok(())
        }
        ForkDetectionMode::Halt => Err(OrchestratedSyncError::ForkHalted {
            fed_id: fed_id.short(),
            fork_count: forks.len(),
        }),
    }
}

/// Calculate the effective quorum threshold from policy and seeder count.
fn effective_threshold(policy: &ResourcePolicy, reports: &[SeederReport]) -> u32 {
    let trusted_count = reports.iter().filter(|r| r.trust_level == TrustLevel::Trusted).count() as u32;

    if policy.verification.use_dynamic_majority {
        policy.verification.quorum_threshold.max(calculate_seeder_quorum(trusted_count))
    } else {
        policy.verification.quorum_threshold
    }
}

/// Pick the best seeder based on trust level.
///
/// Prefers trusted seeders over unknown ones.
fn pick_best_seeder<'a>(
    seeders: &'a [SeederEndpoint],
    reports: &[SeederReport],
    trust_manager: &TrustManager,
) -> &'a SeederEndpoint {
    // Find first trusted seeder that responded
    for seeder in seeders {
        let trust = trust_manager.trust_level(&seeder.cluster_key);
        let responded = reports.iter().any(|r| r.cluster_key == *seeder.cluster_key.as_bytes());
        if trust == TrustLevel::Trusted && responded {
            return seeder;
        }
    }

    // Fall back to first seeder that responded
    for seeder in seeders {
        let responded = reports.iter().any(|r| r.cluster_key == *seeder.cluster_key.as_bytes());
        if responded {
            return seeder;
        }
    }

    // Should not happen (we checked reports is non-empty), but be safe
    &seeders[0]
}

/// Sync objects from the best seeder.
async fn sync_from_best(
    req: &SyncRequest<'_>,
    seeder: &SeederEndpoint,
) -> Result<(Vec<SyncObject>, bool), OrchestratedSyncError> {
    let (conn, _peer_id) = connect_to_cluster(req.endpoint, req.our_identity, seeder.cluster_key)
        .await
        .context("handshake with best seeder failed")?;

    let (objects, has_more) = sync_remote_objects(
        &conn,
        req.fed_id,
        req.want_types.clone(),
        req.have_hashes.clone(),
        req.limit,
        req.delegates,
    )
    .await
    .context("sync objects from best seeder failed")?;

    info!(
        fed_id = %req.fed_id.short(),
        seeder = %seeder.cluster_key,
        object_count = objects.len(),
        has_more,
        "synced objects from seeder"
    );

    Ok((objects, has_more))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enforce_fork_policy_disabled_ignores_forks() {
        let fed_id = FederatedId::new(iroh::SecretKey::generate(&mut rand::rng()).public(), [0xaa; 32]);
        let forks = vec![ForkInfo {
            ref_name: "refs/heads/main".to_string(),
            branches: vec![],
        }];
        assert!(enforce_fork_policy(&fed_id, &forks, &ForkDetectionMode::Disabled).is_ok());
    }

    #[test]
    fn test_enforce_fork_policy_warn_allows_forks() {
        let fed_id = FederatedId::new(iroh::SecretKey::generate(&mut rand::rng()).public(), [0xaa; 32]);
        let forks = vec![ForkInfo {
            ref_name: "refs/heads/main".to_string(),
            branches: vec![],
        }];
        assert!(enforce_fork_policy(&fed_id, &forks, &ForkDetectionMode::Warn).is_ok());
    }

    #[test]
    fn test_enforce_fork_policy_halt_blocks_forks() {
        let fed_id = FederatedId::new(iroh::SecretKey::generate(&mut rand::rng()).public(), [0xaa; 32]);
        let forks = vec![ForkInfo {
            ref_name: "refs/heads/main".to_string(),
            branches: vec![],
        }];
        let result = enforce_fork_policy(&fed_id, &forks, &ForkDetectionMode::Halt);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), OrchestratedSyncError::ForkHalted { .. }));
    }

    #[test]
    fn test_enforce_fork_policy_no_forks_always_ok() {
        let fed_id = FederatedId::new(iroh::SecretKey::generate(&mut rand::rng()).public(), [0xaa; 32]);
        assert!(enforce_fork_policy(&fed_id, &[], &ForkDetectionMode::Halt).is_ok());
        assert!(enforce_fork_policy(&fed_id, &[], &ForkDetectionMode::Warn).is_ok());
        assert!(enforce_fork_policy(&fed_id, &[], &ForkDetectionMode::Disabled).is_ok());
    }

    #[test]
    fn test_effective_threshold_fixed() {
        let policy = ResourcePolicy::new("test").with_verification(crate::policy::VerificationConfig::quorum(3));
        let reports = vec![];
        assert_eq!(effective_threshold(&policy, &reports), 3);
    }

    #[test]
    fn test_effective_threshold_dynamic_majority() {
        let policy =
            ResourcePolicy::new("test").with_verification(crate::policy::VerificationConfig::dynamic_majority(1));
        let reports = vec![
            SeederReport {
                cluster_key: [0x01; 32],
                heads: HashMap::new(),
                timestamp_us: 0,
                trust_level: TrustLevel::Trusted,
            },
            SeederReport {
                cluster_key: [0x02; 32],
                heads: HashMap::new(),
                timestamp_us: 0,
                trust_level: TrustLevel::Trusted,
            },
            SeederReport {
                cluster_key: [0x03; 32],
                heads: HashMap::new(),
                timestamp_us: 0,
                trust_level: TrustLevel::Trusted,
            },
        ];
        // 3 trusted seeders → dynamic quorum = (3/2)+1 = 2, max(1, 2) = 2
        assert_eq!(effective_threshold(&policy, &reports), 2);
    }
}
