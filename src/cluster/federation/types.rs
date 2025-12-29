//! Core types for federation.
//!
//! This module defines the fundamental types used in cross-cluster federation:
//!
//! - `FederatedId`: Globally unique resource identifier (origin + local_id)
//! - `FederationMode`: Trust model for federated resources
//! - `FederationSettings`: Per-resource federation configuration

use std::fmt;
use std::str::FromStr;

use iroh::PublicKey;
use serde::Deserialize;
use serde::Serialize;

/// Tiger Style: Maximum number of allowed clusters in an allowlist.
pub const MAX_ALLOWED_CLUSTERS: usize = 256;

/// A globally unique identifier for a federated resource.
///
/// Combines the origin cluster's public key with a local identifier to create
/// a globally addressable resource ID. This allows resources to be discovered
/// and synchronized across cluster boundaries while maintaining clear authority.
///
/// # Format
///
/// The string representation is: `{origin_hex}:{local_id_hex}`
///
/// For example: `abc123...def:fedcba...987`
///
/// # Authority
///
/// The origin key provides an authority anchor:
/// - The origin cluster controls canonical refs (via delegate signatures)
/// - Other clusters can mirror content but cannot modify authority
/// - Verification chains back to the origin cluster's identity
///
/// # Example
///
/// ```ignore
/// use aspen::cluster::federation::FederatedId;
/// use iroh::PublicKey;
///
/// let origin = PublicKey::from_bytes(&[...]).unwrap();
/// let local_id = blake3::hash(b"my-repo").into();
///
/// let fed_id = FederatedId::new(origin, local_id);
/// println!("Federated ID: {}", fed_id);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FederatedId {
    /// Public key of the cluster that originally created this resource.
    ///
    /// This is the authority anchor - the origin cluster controls canonical state.
    pub origin: PublicKey,

    /// Local identifier within the origin cluster.
    ///
    /// For Forge repositories, this is the RepoId (BLAKE3 hash of RepoIdentity).
    /// For other resources, this is a 32-byte identifier specific to that type.
    pub local_id: [u8; 32],
}

impl FederatedId {
    /// Create a new federated identifier.
    ///
    /// # Arguments
    ///
    /// * `origin` - Public key of the cluster that created this resource
    /// * `local_id` - Local identifier within the origin cluster
    pub fn new(origin: PublicKey, local_id: [u8; 32]) -> Self {
        Self { origin, local_id }
    }

    /// Create a federated ID from a BLAKE3 hash (convenience for Forge).
    pub fn from_blake3(origin: PublicKey, hash: blake3::Hash) -> Self {
        Self {
            origin,
            local_id: *hash.as_bytes(),
        }
    }

    /// Compute the DHT infohash for global discovery.
    ///
    /// This creates a 20-byte key suitable for BitTorrent Mainline DHT lookups.
    /// The key incorporates both the origin and local_id to ensure uniqueness.
    pub fn to_dht_infohash(&self) -> [u8; 20] {
        use sha2::Digest;
        use sha2::Sha256;

        let mut hasher = Sha256::new();
        hasher.update(b"aspen:fed:v1:");
        hasher.update(self.origin.as_bytes());
        hasher.update(&self.local_id);
        let hash = hasher.finalize();

        let mut infohash = [0u8; 20];
        infohash.copy_from_slice(&hash[..20]);
        infohash
    }

    /// Compute a gossip topic ID for this federated resource.
    ///
    /// Creates a BLAKE3-based topic ID that can be used with iroh-gossip
    /// for cross-cluster announcements about this resource.
    pub fn to_gossip_topic(&self) -> iroh_gossip::proto::TopicId {
        let mut data = Vec::with_capacity(64);
        data.extend_from_slice(b"aspen:fed:topic:");
        data.extend_from_slice(self.origin.as_bytes());
        data.extend_from_slice(&self.local_id);
        let hash = blake3::hash(&data);
        iroh_gossip::proto::TopicId::from_bytes(*hash.as_bytes())
    }

    /// Get the origin cluster's public key.
    pub fn origin(&self) -> PublicKey {
        self.origin
    }

    /// Get the local identifier.
    pub fn local_id(&self) -> &[u8; 32] {
        &self.local_id
    }

    /// Get a short display string (first 8 chars of each component).
    pub fn short(&self) -> String {
        format!(
            "{}:{}",
            &self.origin.to_string()[..8],
            hex::encode(&self.local_id[..4])
        )
    }

    /// Convert to bytes for serialization.
    pub fn to_bytes(&self) -> [u8; 64] {
        let mut bytes = [0u8; 64];
        bytes[..32].copy_from_slice(self.origin.as_bytes());
        bytes[32..].copy_from_slice(&self.local_id);
        bytes
    }

    /// Create from bytes.
    ///
    /// # Errors
    ///
    /// Returns `None` if the bytes don't represent a valid public key.
    pub fn from_bytes(bytes: &[u8; 64]) -> Option<Self> {
        let origin_bytes: [u8; 32] = bytes[..32].try_into().ok()?;
        let origin = PublicKey::from_bytes(&origin_bytes).ok()?;
        let mut local_id = [0u8; 32];
        local_id.copy_from_slice(&bytes[32..]);
        Some(Self { origin, local_id })
    }
}

impl fmt::Display for FederatedId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.origin, hex::encode(self.local_id))
    }
}

impl FromStr for FederatedId {
    type Err = FederatedIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(FederatedIdParseError::InvalidFormat);
        }

        let origin = parts[0]
            .parse::<PublicKey>()
            .map_err(|_| FederatedIdParseError::InvalidOrigin)?;

        let local_bytes =
            hex::decode(parts[1]).map_err(|_| FederatedIdParseError::InvalidLocalId)?;

        if local_bytes.len() != 32 {
            return Err(FederatedIdParseError::InvalidLocalId);
        }

        let mut local_id = [0u8; 32];
        local_id.copy_from_slice(&local_bytes);

        Ok(Self { origin, local_id })
    }
}

/// Error parsing a federated ID from string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FederatedIdParseError {
    /// Invalid format (expected "origin:local_id").
    InvalidFormat,
    /// Invalid origin public key.
    InvalidOrigin,
    /// Invalid local ID (not valid hex or wrong length).
    InvalidLocalId,
}

impl fmt::Display for FederatedIdParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat => write!(f, "invalid format, expected 'origin:local_id'"),
            Self::InvalidOrigin => write!(f, "invalid origin public key"),
            Self::InvalidLocalId => write!(f, "invalid local ID (must be 64 hex chars)"),
        }
    }
}

impl std::error::Error for FederatedIdParseError {}

/// Federation mode for a resource.
///
/// Controls who can discover and sync this resource across clusters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FederationMode {
    /// Federation is disabled for this resource.
    ///
    /// The resource is only accessible within the local cluster.
    #[default]
    Disabled,

    /// Public federation - anyone can discover and sync.
    ///
    /// The resource is announced to the DHT and any cluster can sync it.
    /// Requires robust spam protection and rate limiting.
    Public,

    /// Allowlist-based federation.
    ///
    /// Only clusters in the allowlist can discover and sync this resource.
    /// Provides more control but requires manual cluster onboarding.
    AllowList,
}

impl FromStr for FederationMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "disabled" => Ok(Self::Disabled),
            "public" => Ok(Self::Public),
            "allowlist" | "allow_list" | "allow-list" => Ok(Self::AllowList),
            _ => Err(format!(
                "invalid federation mode: {s}, expected: disabled, public, allowlist"
            )),
        }
    }
}

impl fmt::Display for FederationMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled => write!(f, "disabled"),
            Self::Public => write!(f, "public"),
            Self::AllowList => write!(f, "allowlist"),
        }
    }
}

/// Per-resource federation settings.
///
/// These settings control how a specific resource (e.g., a Forge repository)
/// participates in federation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FederationSettings {
    /// Federation mode for this resource.
    #[serde(default)]
    pub mode: FederationMode,

    /// Clusters allowed to sync this resource (when mode is AllowList).
    ///
    /// Each entry is the public key of a trusted cluster.
    /// Tiger Style: Bounded to MAX_ALLOWED_CLUSTERS entries.
    #[serde(default)]
    pub allowed_clusters: Vec<PublicKey>,
}

impl FederationSettings {
    /// Create disabled federation settings.
    pub fn disabled() -> Self {
        Self {
            mode: FederationMode::Disabled,
            allowed_clusters: Vec::new(),
        }
    }

    /// Create public federation settings.
    pub fn public() -> Self {
        Self {
            mode: FederationMode::Public,
            allowed_clusters: Vec::new(),
        }
    }

    /// Create allowlist federation settings.
    ///
    /// Tiger Style: Truncates to MAX_ALLOWED_CLUSTERS entries.
    pub fn allowlist(clusters: Vec<PublicKey>) -> Self {
        let allowed_clusters = clusters
            .into_iter()
            .take(MAX_ALLOWED_CLUSTERS)
            .collect();
        Self {
            mode: FederationMode::AllowList,
            allowed_clusters,
        }
    }

    /// Check if a cluster is allowed to sync this resource.
    pub fn is_cluster_allowed(&self, cluster_key: &PublicKey) -> bool {
        match self.mode {
            FederationMode::Disabled => false,
            FederationMode::Public => true,
            FederationMode::AllowList => self.allowed_clusters.contains(cluster_key),
        }
    }

    /// Add a cluster to the allowlist.
    ///
    /// Tiger Style: Fails silently if at MAX_ALLOWED_CLUSTERS.
    pub fn add_allowed_cluster(&mut self, cluster_key: PublicKey) {
        if self.allowed_clusters.len() < MAX_ALLOWED_CLUSTERS
            && !self.allowed_clusters.contains(&cluster_key)
        {
            self.allowed_clusters.push(cluster_key);
        }
    }

    /// Remove a cluster from the allowlist.
    pub fn remove_allowed_cluster(&mut self, cluster_key: &PublicKey) {
        self.allowed_clusters.retain(|k| k != cluster_key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_origin() -> PublicKey {
        let secret = iroh::SecretKey::generate(&mut rand::rng());
        secret.public()
    }

    #[test]
    fn test_federated_id_creation() {
        let origin = test_origin();
        let local_id = [42u8; 32];

        let fed_id = FederatedId::new(origin, local_id);

        assert_eq!(fed_id.origin(), origin);
        assert_eq!(fed_id.local_id(), &local_id);
    }

    #[test]
    fn test_federated_id_from_blake3() {
        let origin = test_origin();
        let hash = blake3::hash(b"test data");

        let fed_id = FederatedId::from_blake3(origin, hash);

        assert_eq!(fed_id.local_id(), hash.as_bytes());
    }

    #[test]
    fn test_federated_id_display_parse() {
        let origin = test_origin();
        let local_id = [0xab; 32];

        let fed_id = FederatedId::new(origin, local_id);
        let display = fed_id.to_string();

        let parsed: FederatedId = display.parse().expect("should parse");

        assert_eq!(parsed, fed_id);
    }

    #[test]
    fn test_federated_id_bytes_roundtrip() {
        let origin = test_origin();
        let local_id = [0xcd; 32];

        let fed_id = FederatedId::new(origin, local_id);
        let bytes = fed_id.to_bytes();

        let restored = FederatedId::from_bytes(&bytes).expect("should parse");

        assert_eq!(restored, fed_id);
    }

    #[test]
    fn test_federated_id_dht_infohash() {
        let origin = test_origin();
        let local_id = [1u8; 32];

        let fed_id = FederatedId::new(origin, local_id);
        let infohash = fed_id.to_dht_infohash();

        assert_eq!(infohash.len(), 20);

        // Same input should produce same hash
        let infohash2 = fed_id.to_dht_infohash();
        assert_eq!(infohash, infohash2);
    }

    #[test]
    fn test_federation_mode_parsing() {
        assert_eq!(
            "disabled".parse::<FederationMode>().unwrap(),
            FederationMode::Disabled
        );
        assert_eq!(
            "public".parse::<FederationMode>().unwrap(),
            FederationMode::Public
        );
        assert_eq!(
            "allowlist".parse::<FederationMode>().unwrap(),
            FederationMode::AllowList
        );
        assert_eq!(
            "allow-list".parse::<FederationMode>().unwrap(),
            FederationMode::AllowList
        );
    }

    #[test]
    fn test_federation_settings_disabled() {
        let settings = FederationSettings::disabled();
        let cluster = test_origin();

        assert!(!settings.is_cluster_allowed(&cluster));
    }

    #[test]
    fn test_federation_settings_public() {
        let settings = FederationSettings::public();
        let cluster = test_origin();

        assert!(settings.is_cluster_allowed(&cluster));
    }

    #[test]
    fn test_federation_settings_allowlist() {
        let cluster1 = test_origin();
        let cluster2 = test_origin();
        let cluster3 = test_origin();

        let settings = FederationSettings::allowlist(vec![cluster1, cluster2]);

        assert!(settings.is_cluster_allowed(&cluster1));
        assert!(settings.is_cluster_allowed(&cluster2));
        assert!(!settings.is_cluster_allowed(&cluster3));
    }

    #[test]
    fn test_federation_settings_add_remove() {
        let cluster1 = test_origin();
        let cluster2 = test_origin();

        let mut settings = FederationSettings::allowlist(vec![cluster1]);

        assert!(settings.is_cluster_allowed(&cluster1));
        assert!(!settings.is_cluster_allowed(&cluster2));

        settings.add_allowed_cluster(cluster2);
        assert!(settings.is_cluster_allowed(&cluster2));

        settings.remove_allowed_cluster(&cluster1);
        assert!(!settings.is_cluster_allowed(&cluster1));
    }
}
