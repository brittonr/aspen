//! Service registry request handler.
//!
//! Handles: ServiceRegister, ServiceDeregister, ServiceDiscover, ServiceList,
//! ServiceGetInstance, ServiceHeartbeat, ServiceUpdateHealth, ServiceUpdateMetadata.

use std::collections::HashMap;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::ServiceDeregisterResultResponse;
use aspen_client_api::ServiceDiscoverResultResponse;
use aspen_client_api::ServiceGetInstanceResultResponse;
use aspen_client_api::ServiceHeartbeatResultResponse;
use aspen_client_api::ServiceInstanceResponse;
use aspen_client_api::ServiceListResultResponse;
use aspen_client_api::ServiceRegisterResultResponse;
use aspen_client_api::ServiceUpdateHealthResultResponse;
use aspen_client_api::ServiceUpdateMetadataResultResponse;
use aspen_coordination::DiscoveryFilter;
use aspen_coordination::HealthStatus;
use aspen_coordination::RegisterOptions;
use aspen_coordination::ServiceInstanceMetadata;
use aspen_coordination::ServiceRegistry;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;

/// Handler for service registry operations.
pub struct ServiceRegistryHandler;

#[async_trait::async_trait]
impl RequestHandler for ServiceRegistryHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::ServiceRegister { .. }
                | ClientRpcRequest::ServiceDeregister { .. }
                | ClientRpcRequest::ServiceDiscover { .. }
                | ClientRpcRequest::ServiceList { .. }
                | ClientRpcRequest::ServiceGetInstance { .. }
                | ClientRpcRequest::ServiceHeartbeat { .. }
                | ClientRpcRequest::ServiceUpdateHealth { .. }
                | ClientRpcRequest::ServiceUpdateMetadata { .. }
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::ServiceRegister {
                service_name,
                instance_id,
                address,
                version,
                tags,
                weight,
                custom_metadata,
                ttl_ms,
                lease_id,
            } => {
                handle_service_register(
                    ctx,
                    service_name,
                    instance_id,
                    address,
                    version,
                    tags,
                    weight,
                    custom_metadata,
                    ttl_ms,
                    lease_id,
                )
                .await
            }

            ClientRpcRequest::ServiceDeregister {
                service_name,
                instance_id,
                fencing_token,
            } => handle_service_deregister(ctx, service_name, instance_id, fencing_token).await,

            ClientRpcRequest::ServiceDiscover {
                service_name,
                healthy_only,
                tags,
                version_prefix,
                limit,
            } => handle_service_discover(ctx, service_name, healthy_only, tags, version_prefix, limit).await,

            ClientRpcRequest::ServiceList { prefix, limit } => handle_service_list(ctx, prefix, limit).await,

            ClientRpcRequest::ServiceGetInstance {
                service_name,
                instance_id,
            } => handle_service_get_instance(ctx, service_name, instance_id).await,

            ClientRpcRequest::ServiceHeartbeat {
                service_name,
                instance_id,
                fencing_token,
            } => handle_service_heartbeat(ctx, service_name, instance_id, fencing_token).await,

            ClientRpcRequest::ServiceUpdateHealth {
                service_name,
                instance_id,
                fencing_token,
                status,
            } => handle_service_update_health(ctx, service_name, instance_id, fencing_token, status).await,

            ClientRpcRequest::ServiceUpdateMetadata {
                service_name,
                instance_id,
                fencing_token,
                version,
                tags,
                weight,
                custom_metadata,
            } => {
                handle_service_update_metadata(
                    ctx,
                    service_name,
                    instance_id,
                    fencing_token,
                    version,
                    tags,
                    weight,
                    custom_metadata,
                )
                .await
            }

            _ => Err(anyhow::anyhow!("request not handled by ServiceRegistryHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "ServiceRegistryHandler"
    }
}

// ============================================================================
// Service Registry Operation Handlers
// ============================================================================

#[allow(clippy::too_many_arguments)]
async fn handle_service_register(
    ctx: &ClientProtocolContext,
    service_name: String,
    instance_id: String,
    address: String,
    version: String,
    tags: String,
    weight: u32,
    custom_metadata: String,
    ttl_ms: u64,
    lease_id: Option<u64>,
) -> anyhow::Result<ClientRpcResponse> {
    let registry = ServiceRegistry::new(ctx.kv_store.clone());

    // Parse tags from JSON array (Tiger Style: log parse failures)
    let tags_vec: Vec<String> = match serde_json::from_str(&tags) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, tags = %tags, "failed to parse service tags JSON, using empty");
            Vec::new()
        }
    };

    // Parse custom metadata from JSON object (Tiger Style: log parse failures)
    let custom_map: HashMap<String, String> = match serde_json::from_str(&custom_metadata) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, metadata = %custom_metadata, "failed to parse service custom metadata JSON, using empty");
            HashMap::new()
        }
    };

    let metadata = ServiceInstanceMetadata {
        version,
        tags: tags_vec,
        weight,
        custom: custom_map,
    };

    let options = RegisterOptions {
        ttl_ms: if ttl_ms == 0 { None } else { Some(ttl_ms) },
        initial_status: Some(HealthStatus::Healthy),
        lease_id,
    };

    match registry.register(&service_name, &instance_id, &address, metadata, options).await {
        Ok((fencing_token, deadline_ms)) => {
            Ok(ClientRpcResponse::ServiceRegisterResult(ServiceRegisterResultResponse {
                success: true,
                fencing_token: Some(fencing_token),
                deadline_ms: Some(deadline_ms),
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ServiceRegisterResult(ServiceRegisterResultResponse {
            success: false,
            fencing_token: None,
            deadline_ms: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_service_deregister(
    ctx: &ClientProtocolContext,
    service_name: String,
    instance_id: String,
    fencing_token: u64,
) -> anyhow::Result<ClientRpcResponse> {
    let registry = ServiceRegistry::new(ctx.kv_store.clone());

    match registry.deregister(&service_name, &instance_id, fencing_token).await {
        Ok(was_registered) => Ok(ClientRpcResponse::ServiceDeregisterResult(ServiceDeregisterResultResponse {
            success: true,
            was_registered,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::ServiceDeregisterResult(ServiceDeregisterResultResponse {
            success: false,
            was_registered: false,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_service_discover(
    ctx: &ClientProtocolContext,
    service_name: String,
    healthy_only: bool,
    tags: String,
    version_prefix: Option<String>,
    limit: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    let registry = ServiceRegistry::new(ctx.kv_store.clone());

    // Parse tags from JSON array (Tiger Style: log parse failures)
    let tags_vec: Vec<String> = match serde_json::from_str(&tags) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, tags = %tags, "failed to parse discovery tags JSON, using empty");
            Vec::new()
        }
    };

    let filter = DiscoveryFilter {
        healthy_only,
        tags: tags_vec,
        version_prefix,
        limit,
    };

    match registry.discover(&service_name, filter).await {
        Ok(instances) => {
            let response_instances: Vec<ServiceInstanceResponse> =
                instances.into_iter().map(convert_instance_to_response).collect();
            let count = response_instances.len() as u32;
            Ok(ClientRpcResponse::ServiceDiscoverResult(ServiceDiscoverResultResponse {
                success: true,
                instances: response_instances,
                count,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ServiceDiscoverResult(ServiceDiscoverResultResponse {
            success: false,
            instances: vec![],
            count: 0,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_service_list(
    ctx: &ClientProtocolContext,
    prefix: String,
    limit: u32,
) -> anyhow::Result<ClientRpcResponse> {
    let registry = ServiceRegistry::new(ctx.kv_store.clone());

    match registry.discover_services(&prefix, limit).await {
        Ok(services) => {
            let count = services.len() as u32;
            Ok(ClientRpcResponse::ServiceListResult(ServiceListResultResponse {
                success: true,
                services,
                count,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ServiceListResult(ServiceListResultResponse {
            success: false,
            services: vec![],
            count: 0,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_service_get_instance(
    ctx: &ClientProtocolContext,
    service_name: String,
    instance_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    let registry = ServiceRegistry::new(ctx.kv_store.clone());

    match registry.get_instance(&service_name, &instance_id).await {
        Ok(Some(inst)) => {
            let response_instance = convert_instance_to_response(inst);
            Ok(ClientRpcResponse::ServiceGetInstanceResult(ServiceGetInstanceResultResponse {
                success: true,
                found: true,
                instance: Some(response_instance),
                error: None,
            }))
        }
        Ok(None) => Ok(ClientRpcResponse::ServiceGetInstanceResult(ServiceGetInstanceResultResponse {
            success: true,
            found: false,
            instance: None,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::ServiceGetInstanceResult(ServiceGetInstanceResultResponse {
            success: false,
            found: false,
            instance: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_service_heartbeat(
    ctx: &ClientProtocolContext,
    service_name: String,
    instance_id: String,
    fencing_token: u64,
) -> anyhow::Result<ClientRpcResponse> {
    let registry = ServiceRegistry::new(ctx.kv_store.clone());

    match registry.heartbeat(&service_name, &instance_id, fencing_token).await {
        Ok((new_deadline, health_status)) => {
            let status_str = match health_status {
                HealthStatus::Healthy => "healthy",
                HealthStatus::Unhealthy => "unhealthy",
                HealthStatus::Unknown => "unknown",
            };
            Ok(ClientRpcResponse::ServiceHeartbeatResult(ServiceHeartbeatResultResponse {
                success: true,
                new_deadline_ms: Some(new_deadline),
                health_status: Some(status_str.to_string()),
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ServiceHeartbeatResult(ServiceHeartbeatResultResponse {
            success: false,
            new_deadline_ms: None,
            health_status: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_service_update_health(
    ctx: &ClientProtocolContext,
    service_name: String,
    instance_id: String,
    fencing_token: u64,
    status: String,
) -> anyhow::Result<ClientRpcResponse> {
    let registry = ServiceRegistry::new(ctx.kv_store.clone());

    let health_status = match status.to_lowercase().as_str() {
        "healthy" => HealthStatus::Healthy,
        "unhealthy" => HealthStatus::Unhealthy,
        _ => HealthStatus::Unknown,
    };

    match registry.update_health(&service_name, &instance_id, fencing_token, health_status).await {
        Ok(()) => Ok(ClientRpcResponse::ServiceUpdateHealthResult(ServiceUpdateHealthResultResponse {
            success: true,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::ServiceUpdateHealthResult(ServiceUpdateHealthResultResponse {
            success: false,
            error: Some(e.to_string()),
        })),
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_service_update_metadata(
    ctx: &ClientProtocolContext,
    service_name: String,
    instance_id: String,
    fencing_token: u64,
    version: Option<String>,
    tags: Option<String>,
    weight: Option<u32>,
    custom_metadata: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    let registry = ServiceRegistry::new(ctx.kv_store.clone());

    // We need to get existing instance first to merge metadata
    match registry.get_instance(&service_name, &instance_id).await {
        Ok(Some(mut instance)) => {
            // Verify fencing token
            if instance.fencing_token != fencing_token {
                return Ok(ClientRpcResponse::ServiceUpdateMetadataResult(ServiceUpdateMetadataResultResponse {
                    success: false,
                    error: Some("fencing token mismatch".to_string()),
                }));
            }

            // Update fields that were provided
            if let Some(v) = version {
                instance.metadata.version = v;
            }
            if let Some(t) = tags
                && let Ok(parsed_tags) = serde_json::from_str::<Vec<String>>(&t)
            {
                instance.metadata.tags = parsed_tags;
            }
            if let Some(w) = weight {
                instance.metadata.weight = w;
            }
            if let Some(c) = custom_metadata
                && let Ok(parsed_custom) = serde_json::from_str::<HashMap<String, String>>(&c)
            {
                instance.metadata.custom = parsed_custom;
            }

            match registry.update_metadata(&service_name, &instance_id, fencing_token, instance.metadata).await {
                Ok(()) => Ok(ClientRpcResponse::ServiceUpdateMetadataResult(ServiceUpdateMetadataResultResponse {
                    success: true,
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::ServiceUpdateMetadataResult(ServiceUpdateMetadataResultResponse {
                    success: false,
                    error: Some(e.to_string()),
                })),
            }
        }
        Ok(None) => Ok(ClientRpcResponse::ServiceUpdateMetadataResult(ServiceUpdateMetadataResultResponse {
            success: false,
            error: Some("instance not found".to_string()),
        })),
        Err(e) => Ok(ClientRpcResponse::ServiceUpdateMetadataResult(ServiceUpdateMetadataResultResponse {
            success: false,
            error: Some(e.to_string()),
        })),
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert internal ServiceInstance to client RPC response format.
fn convert_instance_to_response(inst: aspen_coordination::ServiceInstance) -> ServiceInstanceResponse {
    // Serialize custom metadata (Tiger Style: log failures, should never fail for HashMap<String,
    // String>)
    let custom_metadata = match serde_json::to_string(&inst.metadata.custom) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(error = %e, instance_id = %inst.instance_id, "failed to serialize custom metadata");
            "{}".to_string()
        }
    };

    ServiceInstanceResponse {
        instance_id: inst.instance_id,
        service_name: inst.service_name,
        address: inst.address,
        health_status: match inst.health_status {
            HealthStatus::Healthy => "healthy".to_string(),
            HealthStatus::Unhealthy => "unhealthy".to_string(),
            HealthStatus::Unknown => "unknown".to_string(),
        },
        version: inst.metadata.version,
        tags: inst.metadata.tags,
        weight: inst.metadata.weight,
        custom_metadata,
        registered_at_ms: inst.registered_at_ms,
        last_heartbeat_ms: inst.last_heartbeat_ms,
        deadline_ms: inst.deadline_ms,
        lease_id: inst.lease_id,
        fencing_token: inst.fencing_token,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_handle_service_register() {
        let handler = ServiceRegistryHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ServiceRegister {
            service_name: "test-service".to_string(),
            instance_id: "instance-1".to_string(),
            address: "127.0.0.1:8080".to_string(),
            version: "1.0.0".to_string(),
            tags: "[]".to_string(),
            weight: 100,
            custom_metadata: "{}".to_string(),
            ttl_ms: 30000,
            lease_id: None,
        }));
    }

    #[test]
    fn test_can_handle_service_deregister() {
        let handler = ServiceRegistryHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ServiceDeregister {
            service_name: "test-service".to_string(),
            instance_id: "instance-1".to_string(),
            fencing_token: 1,
        }));
    }

    #[test]
    fn test_can_handle_service_discover() {
        let handler = ServiceRegistryHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ServiceDiscover {
            service_name: "test-service".to_string(),
            healthy_only: true,
            tags: "[]".to_string(),
            version_prefix: None,
            limit: Some(10),
        }));
    }

    #[test]
    fn test_can_handle_service_list() {
        let handler = ServiceRegistryHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ServiceList {
            prefix: "".to_string(),
            limit: 100,
        }));
    }

    #[test]
    fn test_can_handle_service_get_instance() {
        let handler = ServiceRegistryHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ServiceGetInstance {
            service_name: "test-service".to_string(),
            instance_id: "instance-1".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_service_heartbeat() {
        let handler = ServiceRegistryHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ServiceHeartbeat {
            service_name: "test-service".to_string(),
            instance_id: "instance-1".to_string(),
            fencing_token: 1,
        }));
    }

    #[test]
    fn test_can_handle_service_update_health() {
        let handler = ServiceRegistryHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ServiceUpdateHealth {
            service_name: "test-service".to_string(),
            instance_id: "instance-1".to_string(),
            fencing_token: 1,
            status: "healthy".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_service_update_metadata() {
        let handler = ServiceRegistryHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ServiceUpdateMetadata {
            service_name: "test-service".to_string(),
            instance_id: "instance-1".to_string(),
            fencing_token: 1,
            version: Some("2.0.0".to_string()),
            tags: None,
            weight: None,
            custom_metadata: None,
        }));
    }

    #[test]
    fn test_rejects_unrelated_requests() {
        let handler = ServiceRegistryHandler;

        // KV requests
        assert!(!handler.can_handle(&ClientRpcRequest::ReadKey {
            key: "test".to_string(),
        }));

        // Core requests
        assert!(!handler.can_handle(&ClientRpcRequest::Ping));

        // Lease requests
        assert!(!handler.can_handle(&ClientRpcRequest::LeaseGrant {
            ttl_seconds: 60,
            lease_id: None,
        }));
    }

    #[test]
    fn test_handler_name() {
        let handler = ServiceRegistryHandler;
        assert_eq!(handler.name(), "ServiceRegistryHandler");
    }
}
