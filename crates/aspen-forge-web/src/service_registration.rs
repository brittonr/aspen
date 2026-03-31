//! Service mesh registration for forge-web.
//!
//! When the `net` feature is enabled, registers `forge-web` as a named
//! service in aspen-net so it's discoverable at `forge-web.aspen`.

use anyhow::Context;
use anyhow::Result;
use aspen_client::AspenClient;
use aspen_client_api::messages::ClientRpcRequest;
use aspen_client_api::messages::ClientRpcResponse;
use tracing::info;
use tracing::warn;

/// Register forge-web as a service in the aspen-net service registry.
///
/// Fire-and-forget: logs a warning on failure but does not return an error.
pub async fn register_service(client: &AspenClient, endpoint_id: &str, tcp_port: u16) {
    let result = register_service_inner(client, endpoint_id, tcp_port).await;
    match result {
        Ok(()) => info!(service = "forge-web", port = tcp_port, "registered with service mesh"),
        Err(e) => warn!(error = %e, "failed to register forge-web service (non-fatal)"),
    }
}

async fn register_service_inner(client: &AspenClient, endpoint_id: &str, tcp_port: u16) -> Result<()> {
    let resp = client
        .send(ClientRpcRequest::NetPublish {
            name: "forge-web".to_string(),
            endpoint_id: endpoint_id.to_string(),
            port: tcp_port,
            proto: "http".to_string(),
            tags: vec!["forge".to_string(), "web".to_string()],
        })
        .await
        .context("net publish")?;
    match resp {
        ClientRpcResponse::NetPublishResult(r) if r.is_success => Ok(()),
        ClientRpcResponse::NetPublishResult(r) => {
            anyhow::bail!("publish failed: {}", r.error.unwrap_or_else(|| "unknown".into()))
        }
        ClientRpcResponse::CapabilityUnavailable(_) => {
            warn!("aspen-net not available on cluster — service registration skipped");
            Ok(())
        }
        _ => {
            warn!("unexpected response for NetPublish");
            Ok(())
        }
    }
}

/// Deregister forge-web from the service registry.
///
/// Best-effort: swallows errors since we're shutting down.
pub async fn deregister_service(client: &AspenClient) {
    let result = client
        .send(ClientRpcRequest::NetUnpublish {
            name: "forge-web".to_string(),
        })
        .await;
    match result {
        Ok(ClientRpcResponse::NetUnpublishResult(r)) if r.is_success => {
            info!("deregistered forge-web from service mesh");
        }
        Ok(_) => {}  // Don't log on shutdown — noisy
        Err(_) => {} // Shutdown, connection may already be gone
    }
}
