//! Federation initialization for cluster nodes.
//!
//! Creates federation services (identity, trust, protocol handler) when
//! a cluster secret key is configured. Returns `None` when federation
//! is not configured, allowing the node to operate without federation.

use std::collections::HashMap;
use std::sync::Arc;

use tracing::error;
use tracing::info;
use tracing::warn;

use crate::config::NodeConfig;

/// Result of federation initialization.
///
/// Contains the protocol handler for ALPN registration and the context
/// components needed by RPC handlers (identity, trust manager).
#[cfg(feature = "federation")]
pub struct FederationInitResult {
    /// Protocol handler to register on the iroh Router.
    pub handler: aspen_federation::FederationProtocolHandler,
    /// Cluster identity (needed by RPC handlers for federation status).
    pub identity: Arc<aspen_federation::ClusterIdentity>,
    /// Trust manager (needed by RPC handlers for trust operations).
    pub trust_manager: Arc<aspen_federation::TrustManager>,
}

/// Initialize federation services if configured.
///
/// Reads the federation section of `NodeConfig` and creates:
/// - `ClusterIdentity` from the configured secret key
/// - `TrustManager` pre-populated with trusted clusters
/// - `FederationProtocolHandler` ready for Router registration
///
/// Returns `None` when:
/// - No `cluster_key` is configured
/// - The configured key is invalid hex
/// - Federation is explicitly disabled
#[cfg(feature = "federation")]
pub fn setup_federation(
    config: &NodeConfig,
    endpoint: &iroh::Endpoint,
    hlc: &Arc<aspen_core::hlc::HLC>,
) -> Option<FederationInitResult> {
    use aspen_federation::ClusterIdentity;
    use aspen_federation::FederationProtocolHandler;
    use aspen_federation::FederationSettings;
    use aspen_federation::TrustManager;
    use aspen_federation::sync::FederationProtocolContext;
    use tokio::sync::RwLock;

    if !config.federation.is_enabled {
        info!(node_id = config.node_id, "federation disabled by configuration");
        return None;
    }

    // Build ClusterIdentity: from config key, key file, or generate new
    let identity = match resolve_cluster_key(config) {
        Some(hex) => match ClusterIdentity::from_hex_key(&hex, config.federation.cluster_name.clone()) {
            Ok(id) => {
                info!(
                    node_id = config.node_id,
                    cluster_name = %config.federation.cluster_name,
                    public_key = %id.public_key(),
                    "federation identity loaded from config"
                );
                Arc::new(id)
            }
            Err(err) => {
                error!(
                    node_id = config.node_id,
                    error = %err,
                    "invalid federation cluster key, skipping federation initialization"
                );
                return None;
            }
        },
        None => {
            // Generate a new ephemeral identity (not persisted here)
            let id = ClusterIdentity::generate(config.federation.cluster_name.clone());
            info!(
                node_id = config.node_id,
                cluster_name = %id.name(),
                public_key = %id.public_key(),
                "generated ephemeral federation cluster identity"
            );
            Arc::new(id)
        }
    };

    // Build TrustManager with pre-trusted clusters
    let trust_manager = Arc::new(TrustManager::new());
    let mut trusted_count: u32 = 0;
    for key_hex in &config.federation.trusted_clusters {
        match parse_public_key_hex(key_hex) {
            Ok(pk) => {
                trust_manager.add_trusted(pk, format!("config-trusted-{trusted_count}"), None);
                trusted_count = trusted_count.saturating_add(1);
            }
            Err(err) => {
                warn!(
                    key = %key_hex,
                    error = %err,
                    "skipping invalid trusted cluster key"
                );
            }
        }
    }
    if trusted_count > 0 {
        info!(trusted_count, "pre-populated trust manager from config");
    }

    // Build the protocol context
    let context = FederationProtocolContext {
        cluster_identity: (*identity).clone(),
        trust_manager: trust_manager.clone(),
        resource_settings: Arc::new(RwLock::new(HashMap::new())),
        endpoint: Arc::new(endpoint.clone()),
        hlc: hlc.clone(),
        resource_resolver: None, // Resolver wired later when forge is available
        session_credential: std::sync::Mutex::new(None),
    };

    let handler = FederationProtocolHandler::new(context);

    Some(FederationInitResult {
        handler,
        identity,
        trust_manager,
    })
}

/// Resolve the cluster secret key from config (inline or file path).
#[cfg(feature = "federation")]
fn resolve_cluster_key(config: &NodeConfig) -> Option<String> {
    // Try inline key first
    if let Some(ref key) = config.federation.cluster_key {
        if !key.is_empty() {
            return Some(key.clone());
        }
    }

    // Try key file
    if let Some(ref path) = config.federation.cluster_key_path {
        match std::fs::read_to_string(path) {
            Ok(contents) => {
                let trimmed = contents.trim().to_string();
                if trimmed.is_empty() {
                    warn!(path = %path.display(), "cluster key file is empty");
                    return None;
                }
                return Some(trimmed);
            }
            Err(err) => {
                error!(
                    path = %path.display(),
                    error = %err,
                    "failed to read cluster key file"
                );
                return None;
            }
        }
    }

    None
}

/// Parse a hex-encoded Ed25519 public key (64 hex chars = 32 bytes).
#[cfg(feature = "federation")]
fn parse_public_key_hex(hex: &str) -> Result<iroh::PublicKey, String> {
    if hex.len() != 64 {
        return Err(format!("expected 64 hex chars, got {}", hex.len()));
    }

    let mut bytes = [0u8; 32];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let s = std::str::from_utf8(chunk).map_err(|_| "invalid UTF-8 in hex")?;
        bytes[i] = u8::from_str_radix(s, 16).map_err(|e| format!("invalid hex: {e}"))?;
    }

    iroh::PublicKey::from_bytes(&bytes).map_err(|e| format!("invalid Ed25519 public key: {e}"))
}

#[cfg(all(test, feature = "federation"))]
mod tests {
    use super::*;
    use crate::config::NodeConfig;

    #[test]
    fn test_setup_federation_disabled_by_default() {
        let config = NodeConfig::default();
        let endpoint = make_test_endpoint();
        let hlc = Arc::new(aspen_core::hlc::HLC::new());

        // Default config has federation disabled
        assert!(!config.federation.is_enabled);
        let result = setup_federation(&config, &endpoint, &hlc);
        assert!(result.is_none());
    }

    #[test]
    fn test_setup_federation_no_key_generates_identity() {
        let mut config = NodeConfig::default();
        config.federation.is_enabled = true;
        // No key configured — should auto-generate
        let endpoint = make_test_endpoint();
        let hlc = Arc::new(aspen_core::hlc::HLC::new());

        let result = setup_federation(&config, &endpoint, &hlc);
        assert!(result.is_some(), "should auto-generate identity when no key configured");
    }

    #[test]
    fn test_setup_federation_valid_key() {
        let mut config = NodeConfig::default();
        config.federation.is_enabled = true;
        config.federation.cluster_key =
            Some("a11ce00000000001a11ce00000000001a11ce00000000001a11ce00000000001".to_string());
        config.federation.cluster_name = "test-cluster".to_string();

        let endpoint = make_test_endpoint();
        let hlc = Arc::new(aspen_core::hlc::HLC::new());

        let result = setup_federation(&config, &endpoint, &hlc);
        assert!(result.is_some());

        let r = result.unwrap();
        assert_eq!(r.identity.name(), "test-cluster");
    }

    #[test]
    fn test_setup_federation_invalid_key() {
        let mut config = NodeConfig::default();
        config.federation.is_enabled = true;
        config.federation.cluster_key = Some("not-valid-hex".to_string());

        let endpoint = make_test_endpoint();
        let hlc = Arc::new(aspen_core::hlc::HLC::new());

        let result = setup_federation(&config, &endpoint, &hlc);
        assert!(result.is_none());
    }

    #[test]
    fn test_setup_federation_with_trusted_clusters() {
        let mut config = NodeConfig::default();
        config.federation.is_enabled = true;
        config.federation.cluster_key =
            Some("a11ce00000000001a11ce00000000001a11ce00000000001a11ce00000000001".to_string());

        // Add a valid trusted cluster key (must be a valid Ed25519 point)
        // Use a known valid key from ClusterIdentity::generate
        let trusted_identity = aspen_federation::ClusterIdentity::generate("trusted-peer".to_string());
        config.federation.trusted_clusters = vec![format!("{}", trusted_identity.public_key())];

        let endpoint = make_test_endpoint();
        let hlc = Arc::new(aspen_core::hlc::HLC::new());

        let result = setup_federation(&config, &endpoint, &hlc);
        assert!(result.is_some());

        let r = result.unwrap();
        let trust_level = r.trust_manager.trust_level(&trusted_identity.public_key());
        assert!(matches!(trust_level, aspen_federation::TrustLevel::Trusted));
    }

    #[test]
    fn test_parse_public_key_hex_invalid_length() {
        let result = parse_public_key_hex("abcd");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_public_key_hex_invalid_chars() {
        let result = parse_public_key_hex("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz");
        assert!(result.is_err());
    }

    /// Create a minimal iroh endpoint for testing.
    fn make_test_endpoint() -> iroh::Endpoint {
        // Use tokio runtime to create endpoint synchronously in test
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async { iroh::Endpoint::builder().bind().await.unwrap() })
    }
}
