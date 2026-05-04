//! ALPN router setup and protocol registration.
//!
//! Configures the Iroh Router with all protocol handlers including Raft, client,
//! gossip, blobs, docs sync, log subscriber, and Nix cache gateway.

use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::fs::PermissionsExt;
#[cfg(test)]
use std::os::unix::fs::symlink;
use std::sync::Arc;

use anyhow::Context;
use aspen::ClientProtocolHandler;
use aspen::api::KeyValueStore;
use aspen::cluster::config::NodeConfig;
use aspen_auth::Capability;
use aspen_auth::TokenBuilder;
use aspen_automerge::AUTOMERGE_SYNC_ALPN;
use aspen_automerge::AspenAutomergeStore;
use aspen_automerge::AutomergeSyncHandler;
use aspen_core::context::WatchRegistry;
use tracing::info;
use tracing::warn;

use crate::node_mode::NodeMode;

/// Setup the Iroh Router with all protocol handlers.
pub fn setup_router(
    config: &NodeConfig,
    node_mode: &NodeMode,
    client_handler: ClientProtocolHandler,
    watch_registry: Arc<dyn WatchRegistry>,
    kv_store: Arc<dyn KeyValueStore>,
    #[cfg(feature = "nix-cache-gateway")] nix_cache_signer: Option<
        Arc<dyn aspen_nix_cache_gateway::NarinfoSigningProvider>,
    >,
    #[cfg(feature = "forge")] dag_sync_handler: Option<impl iroh::protocol::ProtocolHandler>,
) -> iroh::protocol::Router {
    use aspen::CLIENT_ALPN;
    use aspen::LOG_SUBSCRIBER_ALPN;
    use aspen::LogSubscriberProtocolHandler;
    #[allow(deprecated)]
    use aspen::RAFT_ALPN;
    use aspen::RAFT_SHARDED_ALPN;
    use aspen::RaftProtocolHandler;
    #[cfg(feature = "trust")]
    use aspen::TRUST_ALPN;
    #[cfg(feature = "trust")]
    use aspen::TrustProtocolHandler;
    use iroh::protocol::Router;
    use iroh_gossip::ALPN as GOSSIP_ALPN;

    let mut builder = Router::builder(node_mode.iroh_manager().endpoint().clone());

    // Register Raft protocol handler(s) based on mode
    match &node_mode {
        NodeMode::Single(handle) => {
            // Legacy mode: single Raft handler
            let raft_handler = RaftProtocolHandler::new(handle.storage.raft_node.raft().as_ref().clone());
            #[allow(deprecated)]
            {
                builder = builder.accept(RAFT_ALPN, raft_handler);
            }
        }
        NodeMode::Sharded(handle) => {
            // Sharded mode: register sharded Raft handler for multi-shard routing
            builder = builder.accept(RAFT_SHARDED_ALPN, handle.sharding.sharded_handler.clone());

            // Also register legacy ALPN routing to shard 0 for backward compatibility
            if let Some(shard_0) = handle.primary_shard() {
                let legacy_handler = RaftProtocolHandler::new(shard_0.raft().as_ref().clone());
                #[allow(deprecated)]
                {
                    builder = builder.accept(RAFT_ALPN, legacy_handler);
                }
                info!("Legacy RAFT_ALPN routing to shard 0 for backward compatibility");
            }
        }
    }

    #[cfg(feature = "trust")]
    match &node_mode {
        NodeMode::Single(handle) => {
            let provider: Arc<dyn aspen_transport::TrustShareProvider> = handle.storage.raft_node.clone();
            builder = builder.accept(TRUST_ALPN, TrustProtocolHandler::new(provider));
            info!("Trust protocol handler registered (ALPN: /aspen/trust/1)");
        }
        NodeMode::Sharded(handle) => {
            if let Some(shard) = handle.primary_shard() {
                let provider: Arc<dyn aspen_transport::TrustShareProvider> = shard.clone();
                builder = builder.accept(TRUST_ALPN, TrustProtocolHandler::new(provider));
                info!("Trust protocol handler registered on primary shard (ALPN: /aspen/trust/1)");
            }
        }
    }

    builder = builder.accept(CLIENT_ALPN, client_handler);

    // Add gossip handler if enabled
    if let Some(gossip) = node_mode.iroh_manager().gossip() {
        builder = builder.accept(GOSSIP_ALPN, gossip.clone());
    }

    // Add blobs protocol handler if blob store is enabled
    if let Some(blob_store) = node_mode.blob_store() {
        let blobs_handler = blob_store.protocol_handler();
        builder = builder.accept(iroh_blobs::ALPN, blobs_handler);
        info!("Blobs protocol handler registered");

        // Add SNIX castore protocol handler (blob + directory services over irpc)
        // This enables snix tooling (e.g. IrpcBlobService/IrpcDirectoryService) to
        // use the cluster as a content-addressed store via QUIC.
        #[cfg(feature = "snix")]
        {
            use aspen_castore::CASTORE_ALPN;
            use aspen_castore::CastoreServer;
            use aspen_snix::IrohBlobService;
            use aspen_snix::RaftDirectoryService;

            let snix_blob = IrohBlobService::from_arc(blob_store.clone());
            let snix_dir = RaftDirectoryService::from_arc(kv_store.clone());
            let castore_server = CastoreServer::new(snix_blob, snix_dir);
            let castore_handler = castore_server.into_protocol_handler();
            builder = builder.accept(CASTORE_ALPN, castore_handler);
            info!("SNIX castore protocol handler registered (ALPN: aspen-castore/0)");
        }
    }

    // Add federation protocol handler for cross-cluster sync.
    // Accepts incoming federation handshake/sync requests from remote clusters.
    #[cfg(feature = "federation")]
    if config.federation.is_enabled {
        if let Ok(cluster_identity) = crate::config::load_federation_identity(config) {
            let hlc = Arc::new(aspen_core::hlc::create_hlc(&config.node_id.to_string()));
            let trusted_keys =
                crate::config::parse_trusted_cluster_keys(&config.federation.trusted_clusters).unwrap_or_default();
            let trust_manager = Arc::new(aspen_cluster::federation::TrustManager::with_trusted(trusted_keys));
            let resource_settings = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));

            // Wire Forge resource resolver for KV-backed resource listing and state queries.
            // ForgeResourceResolver requires Sized K, so we use the concrete RaftNode type
            // from node_mode rather than the Arc<dyn KeyValueStore>.
            let resolver: Option<Arc<dyn aspen_cluster::federation::FederationResourceResolver>> = match &node_mode {
                NodeMode::Single(handle) => {
                    let raft_node = handle.storage.raft_node.clone();
                    Some(Arc::new(aspen_forge::resolver::ForgeResourceResolver::new(raft_node)))
                }
                NodeMode::Sharded(handle) => {
                    if let Some(shard) = handle.primary_shard() {
                        Some(Arc::new(aspen_forge::resolver::ForgeResourceResolver::new(shard.clone())))
                    } else {
                        None
                    }
                }
            };

            let context = aspen_cluster::federation::sync::FederationProtocolContext {
                cluster_identity,
                trust_manager,
                resource_settings,
                endpoint: Arc::new(node_mode.iroh_manager().endpoint().clone()),
                hlc,
                resource_resolver: resolver,
                session_credential: std::sync::Mutex::new(None),
            };
            let federation_handler = aspen_cluster::federation::FederationProtocolHandler::new(context);
            builder = builder.accept(aspen_cluster::federation::FEDERATION_ALPN, federation_handler);
            info!("Federation protocol handler registered (ALPN: /aspen/federation/1)");
        } else {
            warn!("federation enabled but identity load failed, skipping ALPN registration");
        }
    }

    // Add DAG sync protocol handler for streaming content-addressed graph sync.
    // Serves incoming requests from peers doing forge repo sync or snix store closure sync.
    #[cfg(feature = "forge")]
    if let Some(handler) = dag_sync_handler {
        builder = builder.accept(aspen_forge::DAG_SYNC_ALPN, handler);
        info!("DAG sync protocol handler registered (ALPN: /aspen/dag-sync/1)");
    }

    // Add docs sync protocol handler if docs sync is enabled
    if let Some(docs_sync) = node_mode.docs_sync() {
        use aspen::docs::DOCS_SYNC_ALPN;
        let docs_handler = docs_sync.protocol_handler();
        builder = builder.accept(DOCS_SYNC_ALPN, docs_handler);
        info!(
            namespace = %docs_sync.namespace_id,
            "Docs sync protocol handler registered"
        );
    }

    // Add automerge sync protocol handler
    let am_store = Arc::new(AspenAutomergeStore::new(kv_store.clone()));
    let node_public_key = node_mode.iroh_manager().endpoint().id();
    let am_handler = Arc::new(AutomergeSyncHandler::with_capability_auth(am_store, node_public_key));
    builder = builder.accept(AUTOMERGE_SYNC_ALPN, am_handler);
    info!("Automerge sync protocol handler registered with capability auth");

    // Add log subscriber protocol handler if log broadcast is enabled
    if let Some(log_sender) = node_mode.log_broadcast() {
        use std::sync::atomic::AtomicU64;
        let committed_index = Arc::new(AtomicU64::new(0));
        let log_subscriber_handler = LogSubscriberProtocolHandler::with_sender(
            &config.cookie,
            config.node_id,
            log_sender.clone(),
            committed_index,
        )
        .with_watch_registry(watch_registry);
        builder = builder.accept(LOG_SUBSCRIBER_ALPN, log_subscriber_handler);
        info!("Log subscriber protocol handler registered (ALPN: aspen-logs)");
    }

    // Add Nix binary cache HTTP/3 gateway if enabled in config
    #[cfg(all(feature = "blob", feature = "nix-cache-gateway"))]
    if config.nix_cache.is_enabled {
        use aspen_cache::KvCacheIndex;
        use aspen_nix_cache_gateway::NIX_CACHE_H3_ALPN;
        use aspen_nix_cache_gateway::NixCacheGatewayConfig;
        use aspen_nix_cache_gateway::NixCacheProtocolHandler;

        if let Some(blob_store) = node_mode.blob_store() {
            let cache_index = Arc::new(KvCacheIndex::new(kv_store.clone()));

            let gateway_config = NixCacheGatewayConfig {
                store_dir: config.nix_cache.store_dir.to_string_lossy().to_string(),
                priority: config.nix_cache.priority,
                want_mass_query: config.nix_cache.want_mass_query,
                cache_name: config.nix_cache.cache_name.clone(),
                ..NixCacheGatewayConfig::default()
            };

            let handler =
                NixCacheProtocolHandler::new(gateway_config, cache_index, blob_store.clone(), nix_cache_signer.clone());
            builder = builder.accept(NIX_CACHE_H3_ALPN, handler);
            info!(
                priority = config.nix_cache.priority,
                want_mass_query = config.nix_cache.want_mass_query,
                cache_name = ?config.nix_cache.cache_name,
                "Nix cache gateway protocol handler registered (ALPN: {})",
                std::str::from_utf8(NIX_CACHE_H3_ALPN).unwrap_or("iroh+h3")
            );
        } else {
            warn!("nix_cache.enabled is true but blob store not available - cache gateway disabled");
        }
    }

    // Add HTTP proxy handler if enabled
    #[cfg(feature = "proxy")]
    if config.proxy.is_enabled {
        use aspen_proxy::AspenUpstreamProxy;

        let proxy = AspenUpstreamProxy::new(config.cookie.clone());
        match proxy.into_handler() {
            Ok(handler) => {
                use aspen_proxy::HTTP_PROXY_ALPN;
                builder = builder.accept(HTTP_PROXY_ALPN, handler);
                info!(
                    max_connections = config.proxy.max_connections,
                    "HTTP proxy protocol handler registered (ALPN: iroh-http-proxy/1)"
                );
            }
            Err(e) => {
                warn!(error = %e, "failed to create HTTP proxy handler — proxy disabled");
            }
        }
    }

    // Add net tunnel acceptor if enabled
    #[cfg(feature = "net")]
    {
        use aspen_net::tunnel::TunnelAcceptor;
        use aspen_transport::constants::NET_TUNNEL_ALPN;

        let tunnel_cancel = tokio_util::sync::CancellationToken::new();
        let tunnel_acceptor = TunnelAcceptor::new(tunnel_cancel);
        builder = builder.accept(NET_TUNNEL_ALPN, tunnel_acceptor);
        info!("Net tunnel acceptor registered (ALPN: /aspen/net-tunnel/0)");
    }

    builder.spawn()
}

fn validate_sensitive_file(file: &fs::File) -> anyhow::Result<()> {
    let metadata = file.metadata().context("failed to inspect sensitive file")?;
    anyhow::ensure!(metadata.file_type().is_file(), "sensitive path is not a regular file");
    let euid = unsafe { libc::geteuid() };
    anyhow::ensure!(metadata.uid() == euid, "sensitive file is not owned by the current user");
    Ok(())
}

fn write_sensitive_file(path: &std::path::Path, content: &str) -> anyhow::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .context("failed to open sensitive file")?;
    validate_sensitive_file(&file).context("refusing unsafe sensitive file")?;
    file.set_permissions(fs::Permissions::from_mode(0o600))
        .context("failed to restrict sensitive file permissions before writing")?;
    file.set_len(0).context("failed to truncate sensitive file after validation")?;
    file.write_all(content.as_bytes()).context("failed to write sensitive file")?;
    file.set_permissions(fs::Permissions::from_mode(0o600))
        .context("failed to restrict sensitive file permissions")?;
    file.sync_all().context("failed to sync sensitive file")
}

fn shell_quote_path(path: &std::path::Path) -> String {
    let path = path.display().to_string();
    format!("'{}'", path.replace('\'', "'\\''"))
}

/// Generate and print cluster ticket for TUI connection.
pub fn print_cluster_ticket(
    config: &NodeConfig,
    endpoint_addr: &iroh::EndpointAddr,
    secret_key: &iroh_base::SecretKey,
) -> anyhow::Result<()> {
    use aspen::cluster::ticket::AspenClusterTicket;
    use iroh_gossip::proto::TopicId;

    let hash = blake3::hash(config.cookie.as_bytes());
    let topic_id = TopicId::from_bytes(*hash.as_bytes());

    let cluster_ticket = AspenClusterTicket::with_bootstrap_addr(topic_id, config.cookie.clone(), endpoint_addr);

    let ticket_str = cluster_ticket.serialize();
    let direct_addrs: Vec<_> = endpoint_addr
        .addrs
        .iter()
        .filter_map(|a| match a {
            iroh::TransportAddr::Ip(addr) => Some(addr),
            _ => None,
        })
        .collect();
    info!(
        endpoint_id = %endpoint_addr.id,
        direct_addrs = ?direct_addrs,
        ticket_bytes = ticket_str.len(),
        "cluster ticket generated with direct addresses"
    );

    // Write ticket to a well-known file location for automated scripts
    let ticket_file = if let Some(ref data_dir) = config.data_dir {
        data_dir.join("cluster-ticket.txt")
    } else {
        std::path::PathBuf::from(format!("/tmp/aspen-node-{}-ticket.txt", config.node_id))
    };

    write_sensitive_file(&ticket_file, &ticket_str)
        .with_context(|| format!("failed to write cluster ticket to {}", ticket_file.display()))?;
    info!(path = %ticket_file.display(), "cluster ticket written to file");

    // Generate wildcard capability token for automerge sync.
    // Full capability grants read+write on all automerge:* keys.
    let token = TokenBuilder::new(secret_key.clone())
        .with_capability(Capability::Full {
            prefix: "automerge:".into(),
        })
        .build()
        .expect("token building should not fail");

    // Bundle endpoint address + capability into a single sync ticket
    let sync_ticket = aspen_automerge::AutomergeSyncTicket::new(endpoint_addr.clone(), &token)
        .map_err(|e| anyhow::anyhow!("capability token encoding failed: {e}"))?;
    let sync_ticket_str =
        sync_ticket.serialize().map_err(|e| anyhow::anyhow!("sync ticket serialization failed: {e}"))?;

    info!(sync_ticket_bytes = sync_ticket_str.len(), "automerge sync ticket generated");

    let sync_ticket_file = ticket_file.with_file_name("automerge-sync-ticket.txt");
    write_sensitive_file(&sync_ticket_file, &sync_ticket_str)
        .with_context(|| format!("failed to write automerge sync ticket to {}", sync_ticket_file.display()))?;
    info!(path = %sync_ticket_file.display(), "automerge sync ticket written to file");

    let ticket_file_shell = shell_quote_path(&ticket_file);
    let sync_ticket_file_shell = shell_quote_path(&sync_ticket_file);

    println!();
    println!("Connect with TUI:");
    println!("  aspen-tui --ticket \"$(cat -- {ticket_file_shell})\"");
    println!();
    println!("Automerge sync (one string, includes auth):");
    println!("  irohscii --cluster \"$(cat -- {sync_ticket_file_shell})\"");
    println!();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cluster_ticket_files_are_owner_only() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("cluster-ticket.txt");

        write_sensitive_file(&path, "sensitive-ticket").expect("write sensitive ticket");

        let mode = fs::metadata(&path).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn cluster_ticket_files_restrict_existing_files_before_rewrite() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("cluster-ticket.txt");
        fs::write(&path, "old").expect("seed sensitive ticket");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).expect("set permissive mode");

        write_sensitive_file(&path, "sensitive-ticket").expect("rewrite sensitive ticket");

        let mode = fs::metadata(&path).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        assert_eq!(fs::read_to_string(&path).expect("read sensitive ticket"), "sensitive-ticket");
    }

    #[test]
    fn cluster_ticket_files_reject_symlink_paths() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target_path = dir.path().join("target-ticket.txt");
        let link_path = dir.path().join("cluster-ticket-link.txt");
        fs::write(&target_path, "old-target").expect("seed symlink target");
        symlink(&target_path, &link_path).expect("create symlink");

        assert!(write_sensitive_file(&link_path, "sensitive-ticket").is_err());
        assert_eq!(fs::read_to_string(&target_path).expect("read symlink target"), "old-target");
    }

    #[test]
    fn cluster_ticket_files_reject_non_regular_paths() {
        let dir = tempfile::tempdir().expect("tempdir");
        let fifo_path = dir.path().join("cluster-ticket-fifo");
        let fifo_c = std::ffi::CString::new(fifo_path.as_os_str().as_encoded_bytes()).expect("fifo path");
        let rc = unsafe { libc::mkfifo(fifo_c.as_ptr(), 0o600) };
        assert_eq!(rc, 0, "mkfifo failed");

        assert!(write_sensitive_file(&fifo_path, "sensitive-ticket").is_err());
    }

    #[test]
    fn shell_quote_path_handles_spaces_and_quotes() {
        let path = std::path::Path::new("/tmp/aspen dogfood/o'clock/cluster-ticket.txt");
        assert_eq!(shell_quote_path(path), "'/tmp/aspen dogfood/o'\\''clock/cluster-ticket.txt'");
    }

    #[test]
    fn ticket_strings_are_not_logged_or_printed_directly() {
        let source = include_str!("router.rs");
        assert!(!source.contains(&format!("ticket = %{}", "ticket_str")));
        assert!(!source.contains(&format!("sync_ticket = %{}", "sync_ticket_str")));
        assert!(!source.contains(&format!("aspen-tui --ticket {brace}", brace = "{}")));
        assert!(!source.contains(&format!("irohscii --cluster {brace}", brace = "{}")));
        assert!(source.contains("aspen-tui --ticket \\\"$(cat -- {ticket_file_shell})\\\""));
        assert!(source.contains("irohscii --cluster \\\"$(cat -- {sync_ticket_file_shell})\\\""));
    }

    #[test]
    fn ticket_commands_are_printed_only_after_successful_file_writes() {
        let source = include_str!("router.rs");
        let cluster_write =
            source.find("write_sensitive_file(&ticket_file, &ticket_str)").expect("cluster ticket write");
        let sync_write =
            source.find("write_sensitive_file(&sync_ticket_file, &sync_ticket_str)").expect("sync ticket write");
        let tui_print = source.find("aspen-tui --ticket").expect("tui print");
        let sync_print = source.find("irohscii --cluster").expect("sync print");

        assert!(cluster_write < tui_print);
        assert!(sync_write < sync_print);
        assert!(source[cluster_write..tui_print].contains("?;"));
        assert!(source[sync_write..sync_print].contains("?;"));
    }
}
