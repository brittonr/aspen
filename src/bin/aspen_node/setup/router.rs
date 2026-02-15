//! ALPN router setup and protocol registration.
//!
//! Configures the Iroh Router with all protocol handlers including Raft, client,
//! gossip, blobs, docs sync, log subscriber, and Nix cache gateway.

use std::sync::Arc;

use aspen::ClientProtocolHandler;
use aspen::api::KeyValueStore;
use aspen::cluster::config::NodeConfig;
use aspen_core::context::WatchRegistry;
use tracing::info;
use tracing::warn;

use crate::node_mode::NodeMode;

/// Setup the Iroh Router with all protocol handlers.
#[allow(unused_variables)] // kv_store used only with nix-cache-gateway feature
pub fn setup_router(
    config: &NodeConfig,
    node_mode: &NodeMode,
    client_handler: ClientProtocolHandler,
    watch_registry: Arc<dyn WatchRegistry>,
    kv_store: Arc<dyn KeyValueStore>,
) -> iroh::protocol::Router {
    use aspen::CLIENT_ALPN;
    use aspen::LOG_SUBSCRIBER_ALPN;
    use aspen::LogSubscriberProtocolHandler;
    #[allow(deprecated)]
    use aspen::RAFT_ALPN;
    use aspen::RAFT_SHARDED_ALPN;
    use aspen::RaftProtocolHandler;
    use iroh::protocol::Router;
    use iroh_gossip::ALPN as GOSSIP_ALPN;

    let mut builder = Router::builder(node_mode.iroh_manager().endpoint().clone());

    // Register Raft protocol handler(s) based on mode
    match &node_mode {
        NodeMode::Single(handle) => {
            // Legacy mode: single Raft handler
            // SAFETY: aspen_raft::types::AppTypeConfig and aspen_transport::rpc::AppTypeConfig are
            // structurally identical (both use the same types from aspen-raft-types: AppRequest,
            // AppResponse, NodeId, RaftMemberInfo). This transmute is verified safe at compile time
            // by static_assertions in aspen_raft::types::_transmute_safety_static_checks.
            let transport_raft: openraft::Raft<aspen_transport::rpc::AppTypeConfig> =
                unsafe { std::mem::transmute(handle.storage.raft_node.raft().as_ref().clone()) };
            let raft_handler = RaftProtocolHandler::new(transport_raft);
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
                // SAFETY: Same transmute safety as above.
                let transport_raft: openraft::Raft<aspen_transport::rpc::AppTypeConfig> =
                    unsafe { std::mem::transmute(shard_0.raft().as_ref().clone()) };
                let legacy_handler = RaftProtocolHandler::new(transport_raft);
                #[allow(deprecated)]
                {
                    builder = builder.accept(RAFT_ALPN, legacy_handler);
                }
                info!("Legacy RAFT_ALPN routing to shard 0 for backward compatibility");
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

            let handler = NixCacheProtocolHandler::new(
                gateway_config,
                cache_index,
                blob_store.clone(),
                None, // TODO: Add narinfo signing support
            );
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

    builder.spawn()
}

/// Start the DNS protocol server if enabled.
pub async fn start_dns_server(_config: &NodeConfig) {
    #[cfg(feature = "dns")]
    let config = _config;
    #[cfg(feature = "dns")]
    if config.dns_server.is_enabled {
        use aspen::dns::AspenDnsClient;
        use aspen::dns::DnsProtocolServer;
        use tracing::error;

        let dns_client = Arc::new(AspenDnsClient::new());
        let dns_config = config.dns_server.clone();

        info!("DNS cache sync disabled - docs_sync feature not yet implemented");

        let dns_server_config = aspen::dns::DnsServerConfig {
            is_enabled: dns_config.is_enabled,
            bind_addr: dns_config.bind_addr,
            zones: dns_config.zones.clone(),
            upstreams: dns_config.upstreams.clone(),
            forwarding_enabled: dns_config.forwarding_enabled,
        };
        match DnsProtocolServer::new(dns_server_config, Arc::clone(&dns_client)).await {
            Ok(dns_server) => {
                info!(
                    bind_addr = %dns_config.bind_addr,
                    zones = ?dns_config.zones,
                    forwarding = dns_config.forwarding_enabled,
                    "DNS protocol server starting"
                );

                tokio::spawn(async move {
                    if let Err(e) = dns_server.run().await {
                        error!(error = %e, "DNS protocol server error");
                    }
                });
            }
            Err(e) => {
                warn!(error = %e, "Failed to create DNS protocol server");
            }
        }
    }
}

/// Generate and print cluster ticket for TUI connection.
pub fn print_cluster_ticket(config: &NodeConfig, endpoint_addr: &iroh::EndpointAddr) {
    use std::io::Write;

    use aspen::cluster::ticket::AspenClusterTicketV2;
    use iroh_gossip::proto::TopicId;

    let hash = blake3::hash(config.cookie.as_bytes());
    let topic_id = TopicId::from_bytes(*hash.as_bytes());

    let cluster_ticket = AspenClusterTicketV2::with_bootstrap_addr(topic_id, config.cookie.clone(), endpoint_addr);

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
        ticket = %ticket_str,
        endpoint_id = %endpoint_addr.id,
        direct_addrs = ?direct_addrs,
        "cluster ticket generated (V2 with direct addresses)"
    );

    // Write ticket to a well-known file location for automated scripts
    let ticket_file = if let Some(ref data_dir) = config.data_dir {
        data_dir.join("cluster-ticket.txt")
    } else {
        std::path::PathBuf::from(format!("/tmp/aspen-node-{}-ticket.txt", config.node_id))
    };

    match std::fs::File::create(&ticket_file).and_then(|mut f| {
        f.write_all(ticket_str.as_bytes())?;
        f.sync_all()
    }) {
        Ok(()) => info!(path = %ticket_file.display(), "cluster ticket written to file"),
        Err(e) => warn!(
            path = %ticket_file.display(),
            error = %e,
            "failed to write cluster ticket to file"
        ),
    }

    println!();
    println!("Connect with TUI:");
    println!("  aspen-tui --ticket {}", ticket_str);
    println!();
}
