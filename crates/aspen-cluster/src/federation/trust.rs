//! Trust management for federation.
//!
//! This module manages trust relationships between federated clusters,
//! controlling which clusters can access federated resources and how
//! their data is verified.
//!
//! # Trust Levels
//!
//! Clusters can have different trust relationships:
//!
//! - **Trusted**: Explicitly trusted, full sync access
//! - **Public**: Can access public resources only
//! - **Blocked**: No access, connections rejected
//!
//! # Verification
//!
//! For federated resources, multiple verification layers apply:
//!
//! 1. **Cluster signature**: Announces are signed by cluster key
//! 2. **Delegate signature**: Canonical refs are signed by delegates
//! 3. **Content hash**: Objects are verified by BLAKE3 hash

use std::collections::HashMap;
use std::collections::HashSet;
use std::time::Instant;

use iroh::PublicKey;
use parking_lot::RwLock;
use serde::Deserialize;
use serde::Serialize;
use tracing::debug;
use tracing::info;
use tracing::warn;

use super::identity::SignedClusterIdentity;
use super::types::FederatedId;
use super::types::FederationMode;
use aspen_core::Signature;

// ============================================================================
// Constants (Tiger Style: Fixed limits)
// ============================================================================

/// Maximum trusted clusters.
pub const MAX_TRUSTED_CLUSTERS: usize = 256;

/// Maximum blocked clusters.
pub const MAX_BLOCKED_CLUSTERS: usize = 256;

/// Maximum pending trust requests.
pub const MAX_PENDING_REQUESTS: usize = 64;

/// Trust request expiry (1 hour).
pub const TRUST_REQUEST_EXPIRY_SECS: u64 = 3600;

// ============================================================================
// Trust Types
// ============================================================================

/// Trust level for a cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustLevel {
    /// Explicitly trusted - full access to federated resources.
    Trusted,
    /// Public access only - can access public resources.
    Public,
    /// Blocked - no access allowed.
    Blocked,
}

impl Default for TrustLevel {
    fn default() -> Self {
        Self::Public
    }
}

/// Information about a trusted cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedClusterInfo {
    /// Cluster public key.
    pub cluster_key: PublicKey,
    /// Cluster name (from identity).
    pub name: String,
    /// Trust level.
    pub level: TrustLevel,
    /// When trust was established.
    pub trusted_since_secs: u64,
    /// Optional notes.
    pub notes: Option<String>,
}

/// A pending trust request.
#[derive(Debug, Clone)]
pub struct TrustRequest {
    /// Requester's cluster identity.
    pub identity: SignedClusterIdentity,
    /// When the request was received.
    pub received_at: Instant,
    /// Optional message from requester.
    pub message: Option<String>,
}

// ============================================================================
// Trust Manager
// ============================================================================

/// Manages trust relationships for federation.
pub struct TrustManager {
    /// Trusted clusters (cluster_key -> info).
    trusted: RwLock<HashMap<PublicKey, TrustedClusterInfo>>,
    /// Blocked clusters.
    blocked: RwLock<HashSet<PublicKey>>,
    /// Pending trust requests.
    pending_requests: RwLock<HashMap<PublicKey, TrustRequest>>,
}

impl TrustManager {
    /// Create a new trust manager.
    pub fn new() -> Self {
        Self {
            trusted: RwLock::new(HashMap::new()),
            blocked: RwLock::new(HashSet::new()),
            pending_requests: RwLock::new(HashMap::new()),
        }
    }

    /// Create a trust manager with initial trusted clusters.
    pub fn with_trusted(clusters: Vec<PublicKey>) -> Self {
        let manager = Self::new();
        for key in clusters.into_iter().take(MAX_TRUSTED_CLUSTERS) {
            manager.add_trusted(key, "config".to_string(), None);
        }
        manager
    }

    /// Check if a cluster is trusted.
    pub fn is_trusted(&self, cluster_key: &PublicKey) -> bool {
        let trusted = self.trusted.read();
        if let Some(info) = trusted.get(cluster_key) {
            info.level == TrustLevel::Trusted
        } else {
            false
        }
    }

    /// Check if a cluster is blocked.
    pub fn is_blocked(&self, cluster_key: &PublicKey) -> bool {
        self.blocked.read().contains(cluster_key)
    }

    /// Get the trust level for a cluster.
    pub fn trust_level(&self, cluster_key: &PublicKey) -> TrustLevel {
        if self.is_blocked(cluster_key) {
            return TrustLevel::Blocked;
        }

        let trusted = self.trusted.read();
        trusted
            .get(cluster_key)
            .map(|info| info.level)
            .unwrap_or(TrustLevel::Public)
    }

    /// Check if a cluster can access a resource.
    pub fn can_access_resource(
        &self,
        cluster_key: &PublicKey,
        resource_mode: &FederationMode,
    ) -> bool {
        let level = self.trust_level(cluster_key);

        match level {
            TrustLevel::Blocked => false,
            TrustLevel::Trusted => true,
            TrustLevel::Public => matches!(resource_mode, FederationMode::Public),
        }
    }

    /// Add a trusted cluster.
    ///
    /// Tiger Style: Fails silently if at capacity.
    pub fn add_trusted(
        &self,
        cluster_key: PublicKey,
        name: String,
        notes: Option<String>,
    ) -> bool {
        // Remove from blocked if present
        self.blocked.write().remove(&cluster_key);

        let mut trusted = self.trusted.write();
        if trusted.len() >= MAX_TRUSTED_CLUSTERS && !trusted.contains_key(&cluster_key) {
            warn!(
                cluster = %cluster_key,
                "Cannot add trusted cluster: at capacity ({})",
                MAX_TRUSTED_CLUSTERS
            );
            return false;
        }

        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        trusted.insert(
            cluster_key,
            TrustedClusterInfo {
                cluster_key,
                name: name.clone(),
                level: TrustLevel::Trusted,
                trusted_since_secs: now_secs,
                notes,
            },
        );

        info!(cluster = %cluster_key, name = %name, "added trusted cluster");
        true
    }

    /// Remove trust from a cluster.
    pub fn remove_trusted(&self, cluster_key: &PublicKey) -> bool {
        let mut trusted = self.trusted.write();
        if trusted.remove(cluster_key).is_some() {
            info!(cluster = %cluster_key, "removed trusted cluster");
            true
        } else {
            false
        }
    }

    /// Block a cluster.
    ///
    /// Tiger Style: Fails silently if at capacity.
    pub fn block(&self, cluster_key: PublicKey) -> bool {
        // Remove from trusted if present
        self.trusted.write().remove(&cluster_key);

        let mut blocked = self.blocked.write();
        if blocked.len() >= MAX_BLOCKED_CLUSTERS && !blocked.contains(&cluster_key) {
            warn!(
                cluster = %cluster_key,
                "Cannot block cluster: at capacity ({})",
                MAX_BLOCKED_CLUSTERS
            );
            return false;
        }

        blocked.insert(cluster_key);
        info!(cluster = %cluster_key, "blocked cluster");
        true
    }

    /// Unblock a cluster.
    pub fn unblock(&self, cluster_key: &PublicKey) -> bool {
        let mut blocked = self.blocked.write();
        if blocked.remove(cluster_key) {
            info!(cluster = %cluster_key, "unblocked cluster");
            true
        } else {
            false
        }
    }

    /// List all trusted clusters.
    pub fn list_trusted(&self) -> Vec<TrustedClusterInfo> {
        self.trusted.read().values().cloned().collect()
    }

    /// List all blocked clusters.
    pub fn list_blocked(&self) -> Vec<PublicKey> {
        self.blocked.read().iter().copied().collect()
    }

    /// Add a pending trust request.
    pub fn add_trust_request(
        &self,
        identity: SignedClusterIdentity,
        message: Option<String>,
    ) -> bool {
        // Verify identity first
        if !identity.verify() {
            warn!("rejected trust request with invalid signature");
            return false;
        }

        let cluster_key = identity.public_key();

        // Don't accept requests from blocked clusters
        if self.is_blocked(&cluster_key) {
            debug!(cluster = %cluster_key, "rejected trust request from blocked cluster");
            return false;
        }

        // Already trusted?
        if self.is_trusted(&cluster_key) {
            debug!(cluster = %cluster_key, "trust request from already-trusted cluster");
            return true;
        }

        let mut pending = self.pending_requests.write();

        // Clean up expired requests
        let now = Instant::now();
        pending.retain(|_, req| {
            now.duration_since(req.received_at).as_secs() < TRUST_REQUEST_EXPIRY_SECS
        });

        // Check capacity
        if pending.len() >= MAX_PENDING_REQUESTS && !pending.contains_key(&cluster_key) {
            warn!("trust request queue full, rejecting");
            return false;
        }

        pending.insert(
            cluster_key,
            TrustRequest {
                identity,
                received_at: now,
                message,
            },
        );

        info!(cluster = %cluster_key, "added pending trust request");
        true
    }

    /// List pending trust requests.
    pub fn list_pending_requests(&self) -> Vec<TrustRequest> {
        let mut pending = self.pending_requests.write();

        // Clean up expired
        let now = Instant::now();
        pending.retain(|_, req| {
            now.duration_since(req.received_at).as_secs() < TRUST_REQUEST_EXPIRY_SECS
        });

        pending.values().cloned().collect()
    }

    /// Accept a pending trust request.
    pub fn accept_trust_request(&self, cluster_key: &PublicKey) -> bool {
        let mut pending = self.pending_requests.write();
        if let Some(request) = pending.remove(cluster_key) {
            drop(pending); // Release lock before calling add_trusted
            self.add_trusted(
                *cluster_key,
                request.identity.name().to_string(),
                request.message,
            )
        } else {
            false
        }
    }

    /// Reject a pending trust request.
    pub fn reject_trust_request(&self, cluster_key: &PublicKey) -> bool {
        self.pending_requests.write().remove(cluster_key).is_some()
    }

    /// Get info about a specific trusted cluster.
    pub fn get_trusted_info(&self, cluster_key: &PublicKey) -> Option<TrustedClusterInfo> {
        self.trusted.read().get(cluster_key).cloned()
    }
}

impl Default for TrustManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Signature Verification
// ============================================================================

/// Verify a delegate signature on a ref update.
///
/// This checks that the signature was made by a valid delegate for the resource.
pub fn verify_delegate_signature(
    fed_id: &FederatedId,
    ref_name: &str,
    new_hash: &[u8; 32],
    timestamp_ms: u64,
    signature: &Signature,
    signer: &PublicKey,
    valid_delegates: &[PublicKey],
) -> bool {
    // Check signer is a valid delegate
    if !valid_delegates.contains(signer) {
        debug!(
            signer = %signer,
            "signer is not a valid delegate"
        );
        return false;
    }

    // Build the signed message (same format as used in Forge)
    let mut message = Vec::new();
    message.extend_from_slice(fed_id.origin().as_bytes());
    message.extend_from_slice(fed_id.local_id());
    message.extend_from_slice(ref_name.as_bytes());
    message.extend_from_slice(new_hash);
    message.extend_from_slice(&timestamp_ms.to_le_bytes());

    // Verify signature
    let sig_bytes: [u8; 64] = match signature.0.clone().try_into() {
        Ok(bytes) => bytes,
        Err(_) => {
            debug!("invalid signature format");
            return false;
        }
    };
    let sig = iroh::Signature::from_bytes(&sig_bytes);

    match signer.verify(&message, &sig) {
        Ok(()) => true,
        Err(e) => {
            debug!(error = %e, "delegate signature verification failed");
            false
        }
    }
}

/// Verify a cluster signature on an announcement.
pub fn verify_cluster_signature(
    message: &[u8],
    signature: &Signature,
    cluster_key: &PublicKey,
) -> bool {
    let sig_bytes: [u8; 64] = match signature.0.clone().try_into() {
        Ok(bytes) => bytes,
        Err(_) => {
            debug!("invalid signature format");
            return false;
        }
    };
    let sig = iroh::Signature::from_bytes(&sig_bytes);

    match cluster_key.verify(message, &sig) {
        Ok(()) => true,
        Err(e) => {
            debug!(error = %e, "cluster signature verification failed");
            false
        }
    }
}

/// Verify a content hash.
pub fn verify_content_hash(data: &[u8], expected_hash: &[u8; 32]) -> bool {
    let actual_hash = blake3::hash(data);
    actual_hash.as_bytes() == expected_hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> PublicKey {
        iroh::SecretKey::generate(&mut rand::rng()).public()
    }

    fn test_identity(name: &str) -> SignedClusterIdentity {
        use super::super::identity::ClusterIdentity;
        ClusterIdentity::generate(name.to_string()).to_signed()
    }

    #[test]
    fn test_trust_manager_basic() {
        let manager = TrustManager::new();
        let key = test_key();

        // Default is public
        assert_eq!(manager.trust_level(&key), TrustLevel::Public);
        assert!(!manager.is_trusted(&key));
        assert!(!manager.is_blocked(&key));
    }

    #[test]
    fn test_add_trusted() {
        let manager = TrustManager::new();
        let key = test_key();

        assert!(manager.add_trusted(key, "test-cluster".to_string(), None));
        assert!(manager.is_trusted(&key));
        assert_eq!(manager.trust_level(&key), TrustLevel::Trusted);
    }

    #[test]
    fn test_remove_trusted() {
        let manager = TrustManager::new();
        let key = test_key();

        manager.add_trusted(key, "test-cluster".to_string(), None);
        assert!(manager.is_trusted(&key));

        assert!(manager.remove_trusted(&key));
        assert!(!manager.is_trusted(&key));
    }

    #[test]
    fn test_block_cluster() {
        let manager = TrustManager::new();
        let key = test_key();

        assert!(manager.block(key));
        assert!(manager.is_blocked(&key));
        assert_eq!(manager.trust_level(&key), TrustLevel::Blocked);
    }

    #[test]
    fn test_block_removes_trust() {
        let manager = TrustManager::new();
        let key = test_key();

        manager.add_trusted(key, "test-cluster".to_string(), None);
        assert!(manager.is_trusted(&key));

        manager.block(key);
        assert!(!manager.is_trusted(&key));
        assert!(manager.is_blocked(&key));
    }

    #[test]
    fn test_trust_removes_block() {
        let manager = TrustManager::new();
        let key = test_key();

        manager.block(key);
        assert!(manager.is_blocked(&key));

        manager.add_trusted(key, "test-cluster".to_string(), None);
        assert!(!manager.is_blocked(&key));
        assert!(manager.is_trusted(&key));
    }

    #[test]
    fn test_can_access_resource() {
        let manager = TrustManager::new();
        let trusted_key = test_key();
        let public_key = test_key();
        let blocked_key = test_key();

        manager.add_trusted(trusted_key, "trusted".to_string(), None);
        manager.block(blocked_key);

        // Trusted can access anything
        assert!(manager.can_access_resource(&trusted_key, &FederationMode::Public));
        assert!(manager.can_access_resource(&trusted_key, &FederationMode::AllowList));
        assert!(manager.can_access_resource(&trusted_key, &FederationMode::Disabled));

        // Public can only access public
        assert!(manager.can_access_resource(&public_key, &FederationMode::Public));
        assert!(!manager.can_access_resource(&public_key, &FederationMode::AllowList));
        assert!(!manager.can_access_resource(&public_key, &FederationMode::Disabled));

        // Blocked can't access anything
        assert!(!manager.can_access_resource(&blocked_key, &FederationMode::Public));
        assert!(!manager.can_access_resource(&blocked_key, &FederationMode::AllowList));
    }

    #[test]
    fn test_pending_trust_request() {
        let manager = TrustManager::new();
        let identity = test_identity("requester");

        assert!(manager.add_trust_request(identity.clone(), Some("please trust me".to_string())));

        let pending = manager.list_pending_requests();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].message, Some("please trust me".to_string()));
    }

    #[test]
    fn test_accept_trust_request() {
        let manager = TrustManager::new();
        let identity = test_identity("requester");
        let key = identity.public_key();

        manager.add_trust_request(identity, None);
        assert!(!manager.is_trusted(&key));

        assert!(manager.accept_trust_request(&key));
        assert!(manager.is_trusted(&key));
        assert!(manager.list_pending_requests().is_empty());
    }

    #[test]
    fn test_reject_trust_request() {
        let manager = TrustManager::new();
        let identity = test_identity("requester");
        let key = identity.public_key();

        manager.add_trust_request(identity, None);
        assert!(manager.reject_trust_request(&key));
        assert!(!manager.is_trusted(&key));
        assert!(manager.list_pending_requests().is_empty());
    }

    #[test]
    fn test_content_hash_verification() {
        let data = b"hello world";
        let hash = *blake3::hash(data).as_bytes();

        assert!(verify_content_hash(data, &hash));
        assert!(!verify_content_hash(b"different", &hash));
    }

    #[test]
    fn test_list_trusted_and_blocked() {
        let manager = TrustManager::new();
        let key1 = test_key();
        let key2 = test_key();
        let key3 = test_key();

        manager.add_trusted(key1, "cluster1".to_string(), None);
        manager.add_trusted(key2, "cluster2".to_string(), None);
        manager.block(key3);

        let trusted = manager.list_trusted();
        assert_eq!(trusted.len(), 2);

        let blocked = manager.list_blocked();
        assert_eq!(blocked.len(), 1);
        assert!(blocked.contains(&key3));
    }
}
