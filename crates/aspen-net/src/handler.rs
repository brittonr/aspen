//! RPC handler for net service mesh operations.
//!
//! Implements `RequestHandler` to dispatch NetPublish, NetUnpublish,
//! NetLookup, and NetList requests through the `ServiceRegistry`.

use std::sync::Arc;

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::messages::net::NetListResponse;
use aspen_client_api::messages::net::NetLookupResponse;
use aspen_client_api::messages::net::NetPublishResponse;
use aspen_client_api::messages::net::NetServiceInfo;
use aspen_client_api::messages::net::NetUnpublishResponse;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;
use aspen_traits::KeyValueStore;
use async_trait::async_trait;
use tracing::debug;

use crate::registry::ServiceRegistry;
use crate::types::ServiceEntry;

/// Handler for net service mesh RPC requests.
pub struct NetHandler<S: KeyValueStore + ?Sized> {
    registry: Arc<ServiceRegistry<S>>,
}

impl<S: KeyValueStore + ?Sized> NetHandler<S> {
    /// Create a new handler wrapping the given registry.
    pub fn new(registry: Arc<ServiceRegistry<S>>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl<S: KeyValueStore + ?Sized + 'static> RequestHandler for NetHandler<S> {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::NetPublish { .. }
                | ClientRpcRequest::NetUnpublish { .. }
                | ClientRpcRequest::NetLookup { .. }
                | ClientRpcRequest::NetList { .. }
        )
    }

    async fn handle(&self, request: ClientRpcRequest, _ctx: &ClientProtocolContext) -> Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::NetPublish {
                name,
                endpoint_id,
                port,
                proto,
                tags,
            } => {
                debug!("NetPublish: {name}");
                let entry = ServiceEntry {
                    name: name.clone(),
                    endpoint_id,
                    port,
                    proto,
                    tags,
                    hostname: None,
                    published_at_ms: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                };
                match self.registry.publish(entry).await {
                    Ok(()) => Ok(ClientRpcResponse::NetPublishResult(NetPublishResponse {
                        is_success: true,
                        error: None,
                    })),
                    Err(e) => Ok(ClientRpcResponse::NetPublishResult(NetPublishResponse {
                        is_success: false,
                        error: Some(e.to_string()),
                    })),
                }
            }
            ClientRpcRequest::NetUnpublish { name } => {
                debug!("NetUnpublish: {name}");
                match self.registry.unpublish(&name).await {
                    Ok(()) => Ok(ClientRpcResponse::NetUnpublishResult(NetUnpublishResponse {
                        is_success: true,
                        error: None,
                    })),
                    Err(e) => Ok(ClientRpcResponse::NetUnpublishResult(NetUnpublishResponse {
                        is_success: false,
                        error: Some(e.to_string()),
                    })),
                }
            }
            ClientRpcRequest::NetLookup { name } => {
                debug!("NetLookup: {name}");
                match self.registry.lookup(&name).await {
                    Ok(Some(entry)) => Ok(ClientRpcResponse::NetLookupResult(NetLookupResponse {
                        entry: Some(service_to_info(&entry)),
                        error: None,
                    })),
                    Ok(None) => Ok(ClientRpcResponse::NetLookupResult(NetLookupResponse {
                        entry: None,
                        error: None,
                    })),
                    Err(e) => Ok(ClientRpcResponse::NetLookupResult(NetLookupResponse {
                        entry: None,
                        error: Some(e.to_string()),
                    })),
                }
            }
            ClientRpcRequest::NetList { tag_filter } => {
                debug!("NetList (tag={tag_filter:?})");
                match self.registry.list(tag_filter.as_deref()).await {
                    Ok(entries) => Ok(ClientRpcResponse::NetListResult(NetListResponse {
                        services: entries.iter().map(service_to_info).collect(),
                        error: None,
                    })),
                    Err(e) => Ok(ClientRpcResponse::NetListResult(NetListResponse {
                        services: vec![],
                        error: Some(e.to_string()),
                    })),
                }
            }
            _ => Ok(ClientRpcResponse::error("INVALID_REQUEST", "request not handled by net handler")),
        }
    }

    fn name(&self) -> &'static str {
        "NetHandler"
    }
}

/// Convert a `ServiceEntry` to a `NetServiceInfo` for RPC responses.
fn service_to_info(entry: &ServiceEntry) -> NetServiceInfo {
    NetServiceInfo {
        name: entry.name.clone(),
        endpoint_id: entry.endpoint_id.clone(),
        port: entry.port,
        proto: entry.proto.clone(),
        tags: entry.tags.clone(),
        hostname: entry.hostname.clone(),
        published_at_ms: entry.published_at_ms,
    }
}
