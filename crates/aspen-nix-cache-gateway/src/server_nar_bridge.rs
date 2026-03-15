//! Nix binary cache server backed by nar-bridge.
//!
//! Replaces the hand-rolled hyper HTTP server with nar-bridge's axum router,
//! backed by Aspen's `BlobService`/`DirectoryService`/`PathInfoService`
//! implementations.

use std::net::SocketAddr;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use snix_castore::blobservice::BlobService;
use snix_castore::directoryservice::DirectoryService;
use snix_store::pathinfoservice::PathInfoService;
use tracing::info;

use crate::GatewayConfig;

/// Cache priority for substituter ordering.
/// Lower = higher priority. cache.nixos.org defaults to 40.
/// We use 30 so the cluster cache is preferred when available.
const CACHE_PRIORITY: u64 = 30;

/// LRU cache capacity for root nodes during NAR upload.
const ROOT_NODE_CACHE_CAPACITY: usize = 1000;

/// Build snix services backed by a live Aspen cluster via ticket.
async fn build_cluster_services(
    ticket: &str,
    timeout_secs: u64,
) -> anyhow::Result<(Arc<dyn BlobService>, Arc<dyn DirectoryService>, Arc<dyn PathInfoService>)> {
    let timeout = Duration::from_secs(timeout_secs);
    let client = aspen_client::AspenClient::connect(ticket, timeout, None)
        .await
        .context("failed to connect to cluster")?;

    info!("connected to Aspen cluster");

    let blob_client = client.clone();
    let kv_client = client;

    // BlobService: RpcBlobStore wraps AspenClient for remote blob operations
    let rpc_blob_store = aspen_client::RpcBlobStore::new(blob_client);
    let blob_svc: Arc<dyn BlobService> = Arc::new(aspen_snix::IrohBlobService::new(rpc_blob_store));

    // DirectoryService + PathInfoService: ClientKvAdapter wraps AspenClient for KV
    let kv =
        Arc::new(crate::client_kv::ClientKvAdapter::new(Arc::new(kv_client))) as Arc<dyn aspen_traits::KeyValueStore>;
    let dir_svc: Arc<dyn DirectoryService> = Arc::new(aspen_snix::RaftDirectoryService::from_arc(kv.clone()));
    let pathinfo_svc: Arc<dyn PathInfoService> = Arc::new(aspen_snix::RaftPathInfoService::from_arc(kv));

    Ok((blob_svc, dir_svc, pathinfo_svc))
}

/// Run the HTTP server using nar-bridge's axum router.
pub async fn run(config: &GatewayConfig) -> anyhow::Result<()> {
    // Build snix services backed by the cluster
    let (blob_svc, dir_svc, pathinfo_svc) = build_cluster_services(&config.ticket, config.timeout_secs).await?;

    // Construct nar-bridge app state
    let cache_capacity = NonZeroUsize::new(ROOT_NODE_CACHE_CAPACITY).expect("ROOT_NODE_CACHE_CAPACITY is non-zero");
    let state = nar_bridge::AppState::new(blob_svc, dir_svc, pathinfo_svc, cache_capacity);

    // Build the nar-bridge router with Aspen's preferred cache priority
    let router = nar_bridge::gen_router(CACHE_PRIORITY).with_state(state);

    let addr: SocketAddr = format!("{}:{}", config.bind, config.port).parse().context("invalid bind address")?;

    info!(%addr, "nix cache gateway listening (nar-bridge)");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
            info!("shutting down");
        })
        .await?;

    Ok(())
}
