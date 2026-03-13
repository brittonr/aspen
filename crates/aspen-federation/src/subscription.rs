//! Federation subscription types for publish/subscribe over KV prefixes.
//!
//! Any KV prefix can be published for federation. Remote clusters subscribe
//! to specific prefixes. The federation layer syncs matching entries + blobs.
//! Apps don't implement federation logic — they store data in KV and declare
//! which prefixes are federable.

use iroh::PublicKey;
use serde::Deserialize;
use serde::Serialize;

use crate::policy::ResourcePolicy;

// ============================================================================
// Constants
// ============================================================================

/// System prefix for federation publication records.
pub const FED_PUB_PREFIX: &str = "_sys:fed:pub:";

/// System prefix for federation subscription records.
pub const FED_SUB_PREFIX: &str = "_sys:fed:sub:";

// ============================================================================
// Access Policy
// ============================================================================

/// Access policy for a published prefix.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AccessPolicy {
    /// Any cluster with a valid credential covering this prefix can subscribe.
    Public,
    /// Only clusters presenting a credential with explicit Read capability
    /// covering this prefix can subscribe.
    TokenRequired,
}

// ============================================================================
// Sync Mode
// ============================================================================

/// How a subscription triggers syncs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SyncMode {
    /// Poll on a fixed interval.
    Periodic {
        /// Interval in seconds between syncs.
        interval_secs: u32,
    },
    /// Sync when a gossip event for the prefix is received.
    OnGossip,
}

// ============================================================================
// Published Prefix
// ============================================================================

/// A KV prefix published for federation.
///
/// Stored in KV at `_sys:fed:pub:{prefix_hash}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishedPrefix {
    /// The KV prefix being published (e.g., `_sys:nix-cache:narinfo:`).
    pub prefix: String,
    /// Access policy for this prefix.
    pub access_policy: AccessPolicy,
    /// Optional resource policy (quorum, verification, fork detection).
    pub resource_policy: Option<ResourcePolicy>,
    /// HLC timestamp when this prefix was announced.
    pub announced_at_hlc: u64,
}

// ============================================================================
// Subscription
// ============================================================================

/// Subscription state for syncing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SubscriptionStatus {
    /// Actively syncing.
    Active,
    /// Credential expired or refresh failed — needs new credential.
    NeedsRefresh,
    /// Paused by operator.
    Paused,
}

/// A subscription to a remote cluster's KV prefix.
///
/// Stored in KV at `_sys:fed:sub:{source_key_hex[..16]}:{prefix_hash[..16]}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    /// Source cluster's public key.
    #[serde(with = "public_key_serde")]
    pub source: PublicKey,
    /// KV prefix being subscribed to.
    pub prefix: String,
    /// Credential for authenticating with the source (base64-encoded).
    /// Stored as string to avoid nested binary in JSON.
    pub credential_b64: Option<String>,
    /// How sync is triggered.
    pub sync_mode: SyncMode,
    /// HLC timestamp of last successful sync.
    pub last_sync_hlc: u64,
    /// Pagination cursor for incremental sync.
    pub cursor: Option<String>,
    /// Current subscription status.
    pub status: SubscriptionStatus,
}

// ============================================================================
// Key Derivation
// ============================================================================

/// Derive the KV key for a published prefix.
///
/// Uses BLAKE3 hash of the prefix (first 16 hex chars) for fixed-width keys.
pub fn pub_key(prefix: &str) -> String {
    let hash = blake3::hash(prefix.as_bytes());
    let hex = hex::encode(&hash.as_bytes()[..8]); // 8 bytes = 16 hex chars
    format!("{FED_PUB_PREFIX}{hex}")
}

/// Derive the KV key for a subscription.
///
/// Uses source key hex (first 16 chars) + BLAKE3 hash of prefix (first 16 hex chars).
pub fn sub_key(source: &PublicKey, prefix: &str) -> String {
    let source_hex = &hex::encode(source.as_bytes())[..16];
    let prefix_hash = blake3::hash(prefix.as_bytes());
    let prefix_hex = hex::encode(&prefix_hash.as_bytes()[..8]);
    format!("{FED_SUB_PREFIX}{source_hex}:{prefix_hex}")
}

// ============================================================================
// Serde helper for PublicKey
// ============================================================================

mod public_key_serde {
    use iroh::PublicKey;
    use serde::Deserialize;
    use serde::Deserializer;
    use serde::Serialize;
    use serde::Serializer;

    pub fn serialize<S: Serializer>(key: &PublicKey, s: S) -> Result<S::Ok, S::Error> {
        hex::encode(key.as_bytes()).serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<PublicKey, D::Error> {
        let hex_str: String = Deserialize::deserialize(d)?;
        let bytes = hex::decode(&hex_str).map_err(serde::de::Error::custom)?;
        if bytes.len() != 32 {
            return Err(serde::de::Error::custom(format!("expected 32 bytes, got {}", bytes.len())));
        }
        PublicKey::try_from(&bytes[..]).map_err(serde::de::Error::custom)
    }
}

// ============================================================================
// Store/Load helpers
// ============================================================================

/// Serialize a `PublishedPrefix` to JSON for KV storage.
pub fn serialize_published_prefix(p: &PublishedPrefix) -> Result<String, serde_json::Error> {
    serde_json::to_string(p)
}

/// Deserialize a `PublishedPrefix` from JSON.
pub fn deserialize_published_prefix(s: &str) -> Result<PublishedPrefix, serde_json::Error> {
    serde_json::from_str(s)
}

/// Serialize a `Subscription` to JSON for KV storage.
pub fn serialize_subscription(s: &Subscription) -> Result<String, serde_json::Error> {
    serde_json::to_string(s)
}

/// Deserialize a `Subscription` from JSON.
pub fn deserialize_subscription(s: &str) -> Result<Subscription, serde_json::Error> {
    serde_json::from_str(s)
}

// ============================================================================
// Operations (async, require KV store)
// ============================================================================

/// Publish a KV prefix for federation.
///
/// Validates the prefix, stores a `PublishedPrefix` in KV, and returns the
/// KV key used. Gossip emission should be handled by the caller.
pub async fn publish_prefix<K: aspen_core::KeyValueStore>(
    kv: &K,
    prefix: &str,
    access_policy: AccessPolicy,
    resource_policy: Option<crate::policy::ResourcePolicy>,
    announced_at_hlc: u64,
) -> Result<String, anyhow::Error> {
    let pp = PublishedPrefix {
        prefix: prefix.to_string(),
        access_policy,
        resource_policy,
        announced_at_hlc,
    };

    let key = pub_key(prefix);
    let value = serialize_published_prefix(&pp)?;
    kv.write(aspen_core::WriteRequest {
        command: aspen_core::WriteCommand::Set {
            key: key.clone(),
            value,
        },
    })
    .await?;

    Ok(key)
}

/// Unpublish a KV prefix. Removes the publication record from KV.
/// Gossip emission should be handled by the caller.
pub async fn unpublish_prefix<K: aspen_core::KeyValueStore>(kv: &K, prefix: &str) -> Result<(), anyhow::Error> {
    let key = pub_key(prefix);
    kv.write(aspen_core::WriteRequest {
        command: aspen_core::WriteCommand::Delete { key },
    })
    .await?;
    Ok(())
}

/// Create a subscription to a remote cluster's KV prefix.
///
/// Validates the credential authorizes the prefix, stores the subscription
/// in KV, and returns the KV key used. The caller is responsible for starting
/// the sync loop.
pub async fn subscribe<K: aspen_core::KeyValueStore>(
    kv: &K,
    source: iroh::PublicKey,
    prefix: &str,
    credential: Option<&aspen_auth::Credential>,
    sync_mode: SyncMode,
) -> Result<String, anyhow::Error> {
    // If credential provided, validate it authorizes read for this prefix
    if let Some(cred) = credential {
        let authorized = cred.token.capabilities.iter().any(|cap| {
            cap.authorizes(&aspen_auth::Operation::Read {
                key: prefix.to_string(),
            })
        });
        if !authorized {
            anyhow::bail!("credential does not authorize Read for prefix: {prefix}");
        }
    }

    let credential_b64 = credential
        .map(|c| c.to_base64())
        .transpose()
        .map_err(|e| anyhow::anyhow!("failed to encode credential: {e}"))?;

    let sub = Subscription {
        source,
        prefix: prefix.to_string(),
        credential_b64,
        sync_mode,
        last_sync_hlc: 0,
        cursor: None,
        status: SubscriptionStatus::Active,
    };

    let key = sub_key(&source, prefix);
    let value = serialize_subscription(&sub)?;
    kv.write(aspen_core::WriteRequest {
        command: aspen_core::WriteCommand::Set {
            key: key.clone(),
            value,
        },
    })
    .await?;

    Ok(key)
}

/// Remove a subscription. Stops sync and removes from KV.
/// Previously synced data remains in the local KV store.
pub async fn unsubscribe<K: aspen_core::KeyValueStore>(
    kv: &K,
    source: &iroh::PublicKey,
    prefix: &str,
) -> Result<(), anyhow::Error> {
    let key = sub_key(source, prefix);
    kv.write(aspen_core::WriteRequest {
        command: aspen_core::WriteCommand::Delete { key },
    })
    .await?;
    Ok(())
}

/// Resume subscriptions after restart.
///
/// Scans `_sys:fed:sub:*` from KV, re-validates credentials (check expiry),
/// and returns subscriptions categorized by status.
pub async fn resume_subscriptions<K: aspen_core::KeyValueStore>(kv: &K) -> Result<Vec<Subscription>, anyhow::Error> {
    let scan_result = kv
        .scan(aspen_core::ScanRequest {
            prefix: FED_SUB_PREFIX.to_string(),
            limit_results: Some(1000),
            continuation_token: None,
        })
        .await?;

    let now_secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);

    let mut subs = Vec::new();
    for entry in scan_result.entries {
        let mut sub: Subscription = match deserialize_subscription(&entry.value) {
            Ok(s) => s,
            Err(_) => continue,
        };

        // Check credential expiry if present
        if let Some(ref b64) = sub.credential_b64 {
            if let Ok(cred) = aspen_auth::Credential::from_base64(b64) {
                if cred.token.expires_at < now_secs {
                    sub.status = SubscriptionStatus::NeedsRefresh;
                }
            }
        }

        subs.push(sub);
    }

    Ok(subs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> PublicKey {
        iroh::SecretKey::generate(&mut rand::rng()).public()
    }

    #[test]
    fn test_pub_key_determinism() {
        let k1 = pub_key("_sys:nix-cache:narinfo:");
        let k2 = pub_key("_sys:nix-cache:narinfo:");
        assert_eq!(k1, k2);
        assert!(k1.starts_with(FED_PUB_PREFIX));
    }

    #[test]
    fn test_pub_key_different_prefixes() {
        let k1 = pub_key("_sys:nix-cache:");
        let k2 = pub_key("_forge:repos:");
        assert_ne!(k1, k2);
    }

    #[test]
    fn test_sub_key_determinism() {
        let source = test_key();
        let k1 = sub_key(&source, "_sys:nix-cache:");
        let k2 = sub_key(&source, "_sys:nix-cache:");
        assert_eq!(k1, k2);
        assert!(k1.starts_with(FED_SUB_PREFIX));
    }

    #[test]
    fn test_sub_key_different_sources() {
        let s1 = test_key();
        let s2 = test_key();
        let k1 = sub_key(&s1, "_sys:nix-cache:");
        let k2 = sub_key(&s2, "_sys:nix-cache:");
        assert_ne!(k1, k2);
    }

    #[test]
    fn test_sub_key_different_prefixes() {
        let source = test_key();
        let k1 = sub_key(&source, "_sys:nix-cache:");
        let k2 = sub_key(&source, "_forge:repos:");
        assert_ne!(k1, k2);
    }

    #[test]
    fn test_published_prefix_roundtrip() {
        let pp = PublishedPrefix {
            prefix: "_sys:nix-cache:narinfo:".into(),
            access_policy: AccessPolicy::TokenRequired,
            resource_policy: None,
            announced_at_hlc: 12345,
        };

        let json = serialize_published_prefix(&pp).unwrap();
        let decoded = deserialize_published_prefix(&json).unwrap();

        assert_eq!(decoded.prefix, pp.prefix);
        assert_eq!(decoded.access_policy, pp.access_policy);
        assert_eq!(decoded.announced_at_hlc, pp.announced_at_hlc);
    }

    #[test]
    fn test_subscription_roundtrip() {
        let source = test_key();
        let sub = Subscription {
            source,
            prefix: "_forge:repos:".into(),
            credential_b64: Some("test-credential-b64".into()),
            sync_mode: SyncMode::Periodic { interval_secs: 60 },
            last_sync_hlc: 99999,
            cursor: Some("page2".into()),
            status: SubscriptionStatus::Active,
        };

        let json = serialize_subscription(&sub).unwrap();
        let decoded = deserialize_subscription(&json).unwrap();

        assert_eq!(decoded.source, source);
        assert_eq!(decoded.prefix, sub.prefix);
        assert_eq!(decoded.sync_mode, sub.sync_mode);
        assert_eq!(decoded.last_sync_hlc, sub.last_sync_hlc);
        assert_eq!(decoded.cursor, sub.cursor);
        assert_eq!(decoded.status, sub.status);
    }

    #[test]
    fn test_subscription_gossip_mode_roundtrip() {
        let source = test_key();
        let sub = Subscription {
            source,
            prefix: "_sys:nix-cache:".into(),
            credential_b64: None,
            sync_mode: SyncMode::OnGossip,
            last_sync_hlc: 0,
            cursor: None,
            status: SubscriptionStatus::NeedsRefresh,
        };

        let json = serialize_subscription(&sub).unwrap();
        let decoded = deserialize_subscription(&json).unwrap();

        assert_eq!(decoded.sync_mode, SyncMode::OnGossip);
        assert_eq!(decoded.status, SubscriptionStatus::NeedsRefresh);
        assert!(decoded.credential_b64.is_none());
    }
}
