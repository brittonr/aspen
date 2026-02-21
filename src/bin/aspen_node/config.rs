//! Configuration loading and validation for aspen-node.
//!
//! Handles building cluster configuration from CLI arguments, loading TOML config
//! files, and providing actionable error messages for common misconfigurations.

use anyhow::Result;
use aspen::cluster::bootstrap::load_config;
use aspen::cluster::config::IrohConfig;
use aspen::cluster::config::NodeConfig;
use clap::Parser;
use tracing::info;

use crate::args::Args;

/// Initialize tracing subscriber with environment-based filtering.
///
/// Tiger Style: Focused initialization function.
pub fn init_tracing() {
    use tracing_subscriber::EnvFilter;

    // Suppress noisy warnings from network-related crates:
    // - netlink_packet_route: kernel has newer NLA attributes than crate expects
    // - quinn_udp: IPv6 unreachable errors when IPv6 is not available
    const NOISY_CRATES: &str = ",netlink_packet_route=error,quinn_udp=error,netlink_packet_core=error";

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(format!("info{NOISY_CRATES}")));
    tracing_subscriber::fmt().with_env_filter(filter).with_target(false).compact().init();
}

/// Build cluster configuration from CLI arguments.
///
/// Tiger Style: Focused function for config construction (single responsibility).
pub fn build_cluster_config(args: &Args) -> NodeConfig {
    let mut config = NodeConfig::from_env();
    config.node_id = args.node_id.unwrap_or(0);
    config.data_dir = args.data_dir.clone();
    config.storage_backend = args.storage_backend.as_deref().and_then(|s| s.parse().ok()).unwrap_or_default();
    // Prefer redb_log_path if provided, fall back to redb_sm_path for backwards compat
    config.redb_path = args.redb_log_path.clone().or_else(|| args.redb_sm_path.clone());
    config.host = args.host.clone().unwrap_or_else(|| "127.0.0.1".into());
    config.cookie = args.cookie.clone().unwrap_or_else(|| "aspen-cookie".into());
    config.control_backend = args.control_backend.unwrap_or_default();
    config.heartbeat_interval_ms = args.heartbeat_interval_ms.unwrap_or(500);
    config.election_timeout_min_ms = args.election_timeout_min_ms.unwrap_or(1500);
    config.election_timeout_max_ms = args.election_timeout_max_ms.unwrap_or(3000);
    // Parse relay mode from CLI string
    let relay_mode = args
        .relay_mode
        .as_deref()
        .map(|s| match s.to_lowercase().as_str() {
            "custom" => aspen::cluster::config::RelayMode::Custom,
            "disabled" => aspen::cluster::config::RelayMode::Disabled,
            _ => aspen::cluster::config::RelayMode::Default,
        })
        .unwrap_or_default();

    config.iroh = IrohConfig {
        secret_key: args.iroh_secret_key.clone(),
        enable_gossip: !args.disable_gossip,
        gossip_ticket: args.ticket.clone(),
        enable_mdns: !args.disable_mdns,
        enable_dns_discovery: args.enable_dns_discovery,
        dns_discovery_url: args.dns_discovery_url.clone(),
        enable_pkarr: args.enable_pkarr,
        enable_pkarr_dht: true,               // DHT enabled by default when pkarr is on
        enable_pkarr_relay: true,             // Relay enabled by default for fallback
        include_pkarr_direct_addresses: true, // Include direct IPs by default
        pkarr_republish_delay_secs: 600,      // 10 minutes default republish
        pkarr_relay_url: args.pkarr_relay_url.clone(),
        relay_mode,
        relay_urls: args.relay_url.clone(),
        enable_raft_auth: args.enable_raft_auth,
        bind_port: args.bind_port.unwrap_or(0),
    };
    config.peers = args.peers.clone();

    // Apply worker configuration from CLI flags
    if args.enable_workers {
        config.worker.is_enabled = true;
    }
    if let Some(count) = args.worker_count {
        // Tiger Style: Cap at 64 workers per node
        config.worker.worker_count = count.min(64) as u32;
    }
    if !args.worker_job_types.is_empty() {
        config.worker.job_types = args.worker_job_types.clone();
    }

    // Apply CI configuration from CLI flags
    if args.enable_ci {
        config.ci.is_enabled = true;
    }
    if args.ci_auto_trigger {
        config.ci.auto_trigger = true;
        // CI auto-trigger requires forge gossip to receive ref update announcements
        config.forge.enable_gossip = true;
    }

    // Apply proxy configuration from CLI flags
    if args.enable_proxy {
        config.proxy.is_enabled = true;
    }
    if let Some(max_conn) = args.proxy_max_connections {
        config.proxy.max_connections = max_conn;
    }

    // Apply security defaults (e.g., auto-enable raft_auth when pkarr is on)
    config.apply_security_defaults();

    config
}

/// Check if running in worker-only mode (via flag or environment variable).
pub fn is_worker_only_mode(args: &Args) -> bool {
    args.worker_only
        || std::env::var("ASPEN_MODE")
            .map(|v| v.to_lowercase() == "ci_worker" || v.to_lowercase() == "worker_only")
            .unwrap_or(false)
}

/// Initialize tracing and load configuration.
pub async fn initialize_and_load_config() -> Result<(Args, NodeConfig)> {
    init_tracing();

    let args = Args::parse();

    // Check for worker-only mode BEFORE loading config
    // Worker-only mode doesn't require node_id validation since it's ephemeral
    if is_worker_only_mode(&args) {
        // For worker-only mode, use minimal config from environment
        // Don't validate node_id/cookie since they aren't needed
        let config = NodeConfig::from_env();
        return Ok((args, config));
    }

    let cli_config = build_cluster_config(&args);

    // Load configuration with proper precedence (env < TOML < CLI)
    // Provide actionable error messages for common misconfigurations
    let config = match load_config(args.config.as_deref(), cli_config) {
        Ok(config) => config,
        Err(e) => {
            handle_config_error(e)?;
            unreachable!()
        }
    };

    Ok((args, config))
}

/// Handle configuration errors with actionable user guidance.
fn handle_config_error(e: anyhow::Error) -> Result<()> {
    let error_msg = e.to_string();

    if error_msg.contains("node_id must be non-zero") {
        eprintln!("Error: node_id must be non-zero");
        eprintln!();
        eprintln!("Required configuration missing. To start an Aspen node, you must provide:");
        eprintln!();
        eprintln!("  1. A unique node ID (positive integer, e.g., 1, 2, 3)");
        eprintln!("  2. A cluster cookie (shared secret for cluster authentication)");
        eprintln!();
        eprintln!("Examples:");
        eprintln!();
        eprintln!("  # Using command-line arguments:");
        eprintln!("  aspen-node --node-id 1 --cookie my-cluster-secret");
        eprintln!();
        eprintln!("  # Using environment variables:");
        eprintln!("  export ASPEN_NODE_ID=1");
        eprintln!("  export ASPEN_COOKIE=my-cluster-secret");
        eprintln!("  aspen-node");
        eprintln!();
        eprintln!("  # Using a config file:");
        eprintln!("  aspen-node --config /path/to/config.toml");
        eprintln!();
        eprintln!("  Example config.toml:");
        eprintln!("    node_id = 1");
        eprintln!("    cookie = \"my-cluster-secret\"");
        eprintln!("    data_dir = \"./data/node-1\"");
        eprintln!();
        eprintln!("For a 3-node cluster, use: nix run .#cluster");
        std::process::exit(1);
    } else if error_msg.contains("default cluster cookie") || error_msg.contains("UNSAFE-CHANGE-ME") {
        eprintln!("Error: using default cluster cookie is not allowed");
        eprintln!();
        eprintln!("Security: You must set a unique cluster cookie to prevent");
        eprintln!("accidental cluster merges. All nodes in a cluster share the");
        eprintln!("same gossip topic derived from the cookie.");
        eprintln!();
        eprintln!("Set a unique cookie via:");
        eprintln!("  --cookie my-cluster-secret");
        eprintln!("  ASPEN_COOKIE=my-cluster-secret");
        eprintln!("  cookie = \"my-cluster-secret\" in config.toml");
        std::process::exit(1);
    } else {
        // For other errors, return the original error
        Err(e)
    }
}

/// Load Nix cache public key from Raft KV store for CI substituter.
#[cfg(feature = "ci")]
pub async fn read_nix_cache_public_key(kv_store: &std::sync::Arc<dyn aspen_core::KeyValueStore>) -> Option<String> {
    use aspen_core::ReadRequest;
    use tracing::debug;

    let read_request = ReadRequest::new("_system:nix-cache:public-key");
    match kv_store.read(read_request).await {
        Ok(read_result) => {
            if let Some(kv) = read_result.kv {
                debug!(
                    public_key = %kv.value,
                    "Retrieved Nix cache public key from KV store for CI substituter"
                );
                Some(kv.value)
            } else {
                debug!("No Nix cache public key found in KV store");
                None
            }
        }
        Err(e) => {
            // This is expected when cluster is not yet initialized or key not set
            debug!(error = %e, "Failed to read Nix cache public key from KV store");
            None
        }
    }
}

/// Load or generate cluster identity for federation.
///
/// Loads the cluster secret key from:
/// 1. `cluster_key` config field (hex-encoded 32 bytes)
/// 2. `cluster_key_path` file (hex-encoded 32 bytes)
/// 3. Generated and stored in `data_dir/federation/cluster_key`
#[cfg(feature = "forge")]
pub fn load_federation_identity(config: &NodeConfig) -> Result<aspen_cluster::federation::ClusterIdentity> {
    use anyhow::Context;
    use aspen_cluster::federation::ClusterIdentity;

    // Try loading from inline cluster_key config
    if let Some(ref hex_key) = config.federation.cluster_key {
        return ClusterIdentity::from_hex_key(hex_key, config.federation.cluster_name.clone())
            .map_err(|e| anyhow::anyhow!("invalid federation cluster_key: {}", e));
    }

    // Try loading from cluster_key_path file
    if let Some(ref key_path) = config.federation.cluster_key_path {
        let hex_key = std::fs::read_to_string(key_path)
            .with_context(|| format!("failed to read federation key from {}", key_path.display()))?;
        let hex_key = hex_key.trim();
        return ClusterIdentity::from_hex_key(hex_key, config.federation.cluster_name.clone())
            .map_err(|e| anyhow::anyhow!("invalid federation cluster_key in {}: {}", key_path.display(), e));
    }

    // Generate new key and store in data_dir/federation/cluster_key
    let data_dir = config
        .data_dir
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("federation enabled but no data_dir configured for key storage"))?;

    let federation_dir = data_dir.join("federation");
    let key_path = federation_dir.join("cluster_key");

    // Check if key already exists
    if key_path.exists() {
        let hex_key = std::fs::read_to_string(&key_path)
            .with_context(|| format!("failed to read federation key from {}", key_path.display()))?;
        let hex_key = hex_key.trim();
        return ClusterIdentity::from_hex_key(hex_key, config.federation.cluster_name.clone())
            .map_err(|e| anyhow::anyhow!("invalid stored federation cluster_key: {}", e));
    }

    // Generate new identity and persist the key
    info!("generating new federation cluster identity");
    let identity = ClusterIdentity::generate(config.federation.cluster_name.clone());

    // Create federation directory if needed
    std::fs::create_dir_all(&federation_dir)
        .with_context(|| format!("failed to create federation directory: {}", federation_dir.display()))?;

    // Write secret key as hex
    let hex_key = hex::encode(identity.secret_key().to_bytes());
    std::fs::write(&key_path, &hex_key)
        .with_context(|| format!("failed to write federation key to {}", key_path.display()))?;

    info!(
        key_path = %key_path.display(),
        public_key = %identity.public_key(),
        "generated and stored new federation cluster key"
    );

    Ok(identity)
}

/// Parse trusted cluster public keys from config strings.
#[cfg(feature = "forge")]
pub fn parse_trusted_cluster_keys(keys: &[String]) -> Result<Vec<iroh::PublicKey>> {
    use anyhow::Context;

    let mut parsed = Vec::with_capacity(keys.len());
    for key_str in keys {
        let key: iroh::PublicKey =
            key_str.parse().with_context(|| format!("invalid trusted cluster public key: {}", key_str))?;
        parsed.push(key);
    }
    Ok(parsed)
}

/// Load Nix cache signer from Transit secrets engine if configured.
///
/// When `nix_cache.cache_name` and `nix_cache.signing_key_name` are both set,
/// creates a [`TransitNarinfoSigner`] that uses the Transit secrets engine for
/// Ed25519 signing. The signing key is auto-created if it doesn't exist.
///
/// The public key is stored in KV at `_system:nix-cache:public-key` so CI workers
/// can add it to their `trusted-public-keys` for binary cache substitution.
#[cfg(all(feature = "secrets", feature = "nix-cache-gateway"))]
pub async fn load_nix_cache_signer(
    config: &NodeConfig,
    _secrets_manager: Option<&std::sync::Arc<aspen_secrets::SecretsManager>>,
    kv_store: &std::sync::Arc<dyn aspen_core::KeyValueStore>,
) -> Result<Option<std::sync::Arc<dyn aspen_nix_cache_gateway::NarinfoSigningProvider>>> {
    use aspen_nix_cache_gateway::NarinfoSigner;
    use aspen_nix_cache_gateway::NarinfoSigningProvider;
    use aspen_secrets::MountRegistry;
    use aspen_secrets::transit::CreateKeyRequest;
    use aspen_secrets::transit::KeyType;
    use tracing::debug;
    use tracing::info;
    use tracing::warn;

    let cache_name = match config.nix_cache.cache_name.as_ref() {
        Some(name) => name.clone(),
        None => {
            debug!("Nix cache signing disabled: no cache_name configured");
            return Ok(None);
        }
    };

    let key_name = match config.nix_cache.signing_key_name.as_ref() {
        Some(name) => name.clone(),
        None => {
            debug!("Nix cache signing disabled: no signing_key_name configured");
            return Ok(None);
        }
    };

    let transit_mount = &config.nix_cache.transit_mount;

    info!(
        cache_name = %cache_name,
        key_name = %key_name,
        transit_mount = %transit_mount,
        "initializing Nix cache narinfo signing via Transit secrets engine"
    );

    // Create a MountRegistry to access the Transit store.
    // This uses the same KV store as the secrets handler, so keys are shared.
    let mount_registry = MountRegistry::new(kv_store.clone() as std::sync::Arc<dyn aspen_core::KeyValueStore>);
    let transit_store = mount_registry
        .get_or_create_transit_store(transit_mount)
        .await
        .map_err(|e| anyhow::anyhow!("failed to get Transit store for mount '{}': {}", transit_mount, e))?;

    // Auto-create the Ed25519 signing key if it doesn't exist.
    match transit_store.read_key(&key_name).await {
        Ok(Some(key)) => {
            info!(
                key_name = %key_name,
                version = key.current_version,
                "using existing Transit signing key for narinfo"
            );
        }
        Ok(None) => {
            info!(key_name = %key_name, "creating Ed25519 Transit signing key for narinfo");
            let request = CreateKeyRequest::new(&key_name).with_type(KeyType::Ed25519);
            transit_store
                .create_key(request)
                .await
                .map_err(|e| anyhow::anyhow!("failed to create Transit signing key '{}': {}", key_name, e))?;
        }
        Err(e) => {
            warn!(key_name = %key_name, error = %e, "failed to check Transit key, attempting create");
            let request = CreateKeyRequest::new(&key_name).with_type(KeyType::Ed25519);
            transit_store
                .create_key(request)
                .await
                .map_err(|e| anyhow::anyhow!("failed to create Transit signing key '{}': {}", key_name, e))?;
        }
    }

    // Build the TransitNarinfoSigner (caches the public key during construction).
    let signer: aspen_nix_cache_gateway::TransitNarinfoSigner =
        NarinfoSigner::from_transit(cache_name.clone(), transit_store, key_name.clone())
            .await
            .map_err(|e| anyhow::anyhow!("failed to create Transit narinfo signer: {}", e))?;

    // Store the public key in KV for CI workers to discover.
    let public_key = NarinfoSigningProvider::public_key(&signer)
        .await
        .map_err(|e| anyhow::anyhow!("failed to get narinfo signing public key: {}", e))?;

    info!(
        public_key = %public_key,
        "Nix cache narinfo signing enabled"
    );

    // Best-effort write: don't fail startup if KV write fails (cluster may not be initialized yet).
    {
        use aspen_core::WriteRequest;
        let write_req = WriteRequest::set("_system:nix-cache:public-key", &public_key);
        match kv_store.write(write_req).await {
            Ok(_) => {
                debug!(
                    key = "_system:nix-cache:public-key",
                    "stored narinfo public key in KV for CI substituter discovery"
                );
            }
            Err(e) => {
                // Expected on first boot before cluster init — the key will be written
                // when the cluster is initialized and the node restarts or reloads.
                warn!(
                    error = %e,
                    "failed to store narinfo public key in KV (cluster may not be initialized yet)"
                );
            }
        }
    }

    Ok(Some(std::sync::Arc::new(signer)))
}
