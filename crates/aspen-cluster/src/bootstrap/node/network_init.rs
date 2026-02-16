//! Network initialization for cluster nodes.
//!
//! This module handles Iroh endpoint creation and peer address parsing.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use aspen_raft::types::NodeId;
use iroh::EndpointAddr;
use tracing::info;
use tracing::warn;

use crate::IrohEndpointConfig;
use crate::IrohEndpointManager;
use crate::config::NodeConfig;

/// Build IrohEndpointConfig from NodeConfig's Iroh settings.
///
/// Converts the application-level IrohConfig (with hex strings and optional fields)
/// into the transport-level IrohEndpointConfig (with parsed types).
///
/// # Errors
///
/// Returns an error if secret_key hex decoding fails.
pub(super) fn build_iroh_config_from_node_config(config: &NodeConfig) -> Result<IrohEndpointConfig> {
    let iroh_config = IrohEndpointConfig::default()
        .with_gossip(config.iroh.enable_gossip)
        .with_mdns(config.iroh.enable_mdns)
        .with_dns_discovery(config.iroh.enable_dns_discovery)
        .with_pkarr(config.iroh.enable_pkarr)
        .with_pkarr_dht(config.iroh.enable_pkarr_dht)
        .with_pkarr_relay(config.iroh.enable_pkarr_relay)
        .with_pkarr_direct_addresses(config.iroh.include_pkarr_direct_addresses)
        .with_pkarr_republish_delay_secs(config.iroh.pkarr_republish_delay_secs)
        .with_relay_mode(config.iroh.relay_mode.clone())
        .with_relay_urls(config.iroh.relay_urls.clone())
        .with_bind_port(config.iroh.bind_port);

    // Apply optional pkarr relay URL
    let iroh_config = match &config.iroh.pkarr_relay_url {
        Some(url) => iroh_config.with_pkarr_relay_url(url.clone()),
        None => iroh_config,
    };

    // Apply optional DNS discovery URL
    let iroh_config = match &config.iroh.dns_discovery_url {
        Some(url) => iroh_config.with_dns_discovery_url(url.clone()),
        None => iroh_config,
    };

    // Parse and apply optional secret key from config
    let iroh_config = match &config.iroh.secret_key {
        Some(secret_key_hex) => {
            let bytes = hex::decode(secret_key_hex).map_err(|e| anyhow::anyhow!("invalid secret key hex: {}", e))?;
            let secret_key = iroh::SecretKey::from_bytes(&bytes.try_into().map_err(|e: Vec<u8>| {
                anyhow::anyhow!("invalid secret key length: expected 32 bytes, got {}: {:?}", e.len(), e)
            })?);
            iroh_config.with_secret_key(secret_key)
        }
        None => iroh_config,
    };

    // Set secret key persistence path based on data_dir
    // This ensures stable node identity across restarts
    let iroh_config = match &config.data_dir {
        Some(data_dir) => {
            let key_path = data_dir.join("iroh_secret_key");
            info!(path = %key_path.display(), "configured secret key persistence path");
            iroh_config.with_secret_key_path(key_path)
        }
        None => {
            warn!("no data_dir configured - secret key will not persist across restarts");
            iroh_config
        }
    };

    Ok(iroh_config)
}

/// Initialize Iroh endpoint manager.
pub(super) async fn initialize_iroh_endpoint(config: &NodeConfig) -> Result<Arc<IrohEndpointManager>> {
    let iroh_config =
        build_iroh_config_from_node_config(config).context("failed to build Iroh config from NodeConfig")?;

    let iroh_manager =
        Arc::new(IrohEndpointManager::new(iroh_config).await.context("failed to create Iroh endpoint manager")?);
    info!(
        node_id = config.node_id,
        endpoint_id = %iroh_manager.node_addr().id,
        "created Iroh endpoint"
    );

    Ok(iroh_manager)
}

/// Parse peer addresses from CLI arguments.
pub fn parse_peer_addresses(peer_specs: &[String]) -> Result<HashMap<NodeId, EndpointAddr>> {
    let mut peers = HashMap::new();

    for spec in peer_specs {
        let parts: Vec<&str> = spec.split('@').collect();
        ensure!(parts.len() == 2, "invalid peer spec '{}', expected format: node_id@endpoint_id", spec);

        let node_id: u64 =
            parts[0].parse().map_err(|e| anyhow::anyhow!("invalid node_id in peer spec '{}': {}", spec, e))?;

        // Parse endpoint address (could be full JSON or just ID)
        let addr = if parts[1].starts_with('{') {
            serde_json::from_str(parts[1])
                .map_err(|e| anyhow::anyhow!("invalid JSON endpoint in peer spec '{}': {}", spec, e))?
        } else {
            // Parse as bare endpoint ID
            let endpoint_id = parts[1]
                .parse()
                .map_err(|e| anyhow::anyhow!("invalid endpoint_id in peer spec '{}': {}", spec, e))?;
            EndpointAddr::new(endpoint_id)
        };

        peers.insert(node_id.into(), addr);
    }

    Ok(peers)
}
