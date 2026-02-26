//! Service registry client for service discovery and health management.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use anyhow::bail;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::warn;

use super::CoordinationRpc;

/// Client for service registry operations.
///
/// Provides a high-level API for service registration, discovery,
/// and health management.
pub struct ServiceClient<C: CoordinationRpc> {
    client: Arc<C>,
}

impl<C: CoordinationRpc> ServiceClient<C> {
    /// Create a new service registry client.
    pub fn new(client: Arc<C>) -> Self {
        Self { client }
    }

    /// Register a service instance.
    ///
    /// Returns a registration handle with fencing token and deadline.
    pub async fn register(
        &self,
        service_name: &str,
        instance_id: &str,
        address: &str,
        options: ServiceRegisterOptions,
    ) -> Result<ServiceRegistration<C>> {
        use aspen_client_api::ServiceRegisterResultResponse;

        let tags_json = serde_json::to_string(&options.tags).unwrap_or_else(|_| "[]".to_string());
        let custom_json = serde_json::to_string(&options.custom_metadata).unwrap_or_else(|_| "{}".to_string());

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::ServiceRegister {
                service_name: service_name.to_string(),
                instance_id: instance_id.to_string(),
                address: address.to_string(),
                version: options.version.clone(),
                tags: tags_json,
                weight: options.weight,
                custom_metadata: custom_json,
                ttl_ms: options.ttl.map(|d| d.as_millis() as u64).unwrap_or(0),
                lease_id: options.lease_id,
            })
            .await?;

        match response {
            ClientRpcResponse::ServiceRegisterResult(ServiceRegisterResultResponse {
                is_success: true,
                fencing_token: Some(token),
                deadline_ms: Some(deadline),
                ..
            }) => Ok(ServiceRegistration {
                client: self.client.clone(),
                service_name: service_name.to_string(),
                instance_id: instance_id.to_string(),
                fencing_token: token,
                deadline_ms: deadline,
            }),
            ClientRpcResponse::ServiceRegisterResult(result) => {
                bail!("service register failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
            _ => bail!("unexpected response type for ServiceRegister"),
        }
    }

    /// Discover service instances.
    ///
    /// Returns a list of matching instances.
    pub async fn discover(
        &self,
        service_name: &str,
        filter: Option<ServiceDiscoveryFilter>,
    ) -> Result<Vec<ServiceInstanceInfo>> {
        use aspen_client_api::ServiceDiscoverResultResponse;

        let filter = filter.unwrap_or_default();
        let tags_json = serde_json::to_string(&filter.tags.unwrap_or_default()).unwrap_or_else(|_| "[]".to_string());

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::ServiceDiscover {
                service_name: service_name.to_string(),
                healthy_only: filter.healthy_only,
                tags: tags_json,
                version_prefix: filter.version_prefix,
                limit: filter.limit,
            })
            .await?;

        match response {
            ClientRpcResponse::ServiceDiscoverResult(ServiceDiscoverResultResponse {
                is_success: true,
                instances,
                ..
            }) => Ok(instances
                .into_iter()
                .map(|inst| ServiceInstanceInfo {
                    instance_id: inst.instance_id,
                    service_name: inst.service_name,
                    address: inst.address,
                    health_status: inst.health_status,
                    version: inst.version,
                    tags: inst.tags,
                    weight: inst.weight,
                    custom_metadata: serde_json::from_str(&inst.custom_metadata).unwrap_or_default(),
                    registered_at: Duration::from_millis(inst.registered_at_ms),
                    last_heartbeat: Duration::from_millis(inst.last_heartbeat_ms),
                    deadline: Duration::from_millis(inst.deadline_ms),
                    lease_id: inst.lease_id,
                    fencing_token: inst.fencing_token,
                })
                .collect()),
            ClientRpcResponse::ServiceDiscoverResult(result) => {
                bail!("service discover failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
            _ => bail!("unexpected response type for ServiceDiscover"),
        }
    }

    /// List service names by prefix.
    pub async fn list_services(&self, prefix: &str, limit: Option<u32>) -> Result<Vec<String>> {
        use aspen_client_api::ServiceListResultResponse;

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::ServiceList {
                prefix: prefix.to_string(),
                limit: limit.unwrap_or(100),
            })
            .await?;

        match response {
            ClientRpcResponse::ServiceListResult(ServiceListResultResponse {
                is_success: true,
                services,
                ..
            }) => Ok(services),
            ClientRpcResponse::ServiceListResult(result) => {
                bail!("service list failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
            _ => bail!("unexpected response type for ServiceList"),
        }
    }

    /// Get a specific service instance.
    pub async fn get_instance(&self, service_name: &str, instance_id: &str) -> Result<Option<ServiceInstanceInfo>> {
        use aspen_client_api::ServiceGetInstanceResultResponse;

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::ServiceGetInstance {
                service_name: service_name.to_string(),
                instance_id: instance_id.to_string(),
            })
            .await?;

        match response {
            ClientRpcResponse::ServiceGetInstanceResult(ServiceGetInstanceResultResponse {
                is_success: true,
                was_found: true,
                instance: Some(inst),
                ..
            }) => Ok(Some(ServiceInstanceInfo {
                instance_id: inst.instance_id,
                service_name: inst.service_name,
                address: inst.address,
                health_status: inst.health_status,
                version: inst.version,
                tags: inst.tags,
                weight: inst.weight,
                custom_metadata: serde_json::from_str(&inst.custom_metadata).unwrap_or_default(),
                registered_at: Duration::from_millis(inst.registered_at_ms),
                last_heartbeat: Duration::from_millis(inst.last_heartbeat_ms),
                deadline: Duration::from_millis(inst.deadline_ms),
                lease_id: inst.lease_id,
                fencing_token: inst.fencing_token,
            })),
            ClientRpcResponse::ServiceGetInstanceResult(ServiceGetInstanceResultResponse {
                is_success: true,
                was_found: false,
                ..
            }) => Ok(None),
            ClientRpcResponse::ServiceGetInstanceResult(result) => {
                bail!("service get instance failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
            _ => bail!("unexpected response type for ServiceGetInstance"),
        }
    }
}

/// Handle for a registered service instance.
///
/// Provides methods for heartbeat, health updates, and deregistration.
pub struct ServiceRegistration<C: CoordinationRpc> {
    client: Arc<C>,
    service_name: String,
    instance_id: String,
    fencing_token: u64,
    deadline_ms: u64,
}

impl<C: CoordinationRpc + 'static> ServiceRegistration<C> {
    /// Get the fencing token for this registration.
    pub fn fencing_token(&self) -> u64 {
        self.fencing_token
    }

    /// Get the TTL deadline in milliseconds since Unix epoch.
    pub fn deadline_ms(&self) -> u64 {
        self.deadline_ms
    }

    /// Get the service name.
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// Get the instance ID.
    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    /// Send a heartbeat to renew the TTL.
    ///
    /// Returns the new deadline and current health status.
    pub async fn heartbeat(&mut self) -> Result<(u64, String)> {
        use aspen_client_api::ServiceHeartbeatResultResponse;

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::ServiceHeartbeat {
                service_name: self.service_name.clone(),
                instance_id: self.instance_id.clone(),
                fencing_token: self.fencing_token,
            })
            .await?;

        match response {
            ClientRpcResponse::ServiceHeartbeatResult(ServiceHeartbeatResultResponse {
                is_success: true,
                new_deadline_ms: Some(deadline),
                health_status: Some(status),
                ..
            }) => {
                self.deadline_ms = deadline;
                Ok((deadline, status))
            }
            ClientRpcResponse::ServiceHeartbeatResult(result) => {
                bail!("service heartbeat failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
            _ => bail!("unexpected response type for ServiceHeartbeat"),
        }
    }

    /// Update the health status.
    pub async fn set_health(&self, status: &str) -> Result<()> {
        use aspen_client_api::ServiceUpdateHealthResultResponse;

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::ServiceUpdateHealth {
                service_name: self.service_name.clone(),
                instance_id: self.instance_id.clone(),
                fencing_token: self.fencing_token,
                status: status.to_string(),
            })
            .await?;

        match response {
            ClientRpcResponse::ServiceUpdateHealthResult(ServiceUpdateHealthResultResponse {
                is_success: true,
                ..
            }) => Ok(()),
            ClientRpcResponse::ServiceUpdateHealthResult(result) => {
                bail!("service update health failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
            _ => bail!("unexpected response type for ServiceUpdateHealth"),
        }
    }

    /// Update instance metadata.
    pub async fn update_metadata(&self, updates: ServiceMetadataUpdate) -> Result<()> {
        use aspen_client_api::ServiceUpdateMetadataResultResponse;

        let tags_json = updates.tags.map(|t| serde_json::to_string(&t).unwrap_or_else(|_| "[]".to_string()));
        let custom_json =
            updates.custom_metadata.map(|c| serde_json::to_string(&c).unwrap_or_else(|_| "{}".to_string()));

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::ServiceUpdateMetadata {
                service_name: self.service_name.clone(),
                instance_id: self.instance_id.clone(),
                fencing_token: self.fencing_token,
                version: updates.version,
                tags: tags_json,
                weight: updates.weight,
                custom_metadata: custom_json,
            })
            .await?;

        match response {
            ClientRpcResponse::ServiceUpdateMetadataResult(ServiceUpdateMetadataResultResponse {
                is_success: true,
                ..
            }) => Ok(()),
            ClientRpcResponse::ServiceUpdateMetadataResult(result) => {
                bail!("service update metadata failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
            _ => bail!("unexpected response type for ServiceUpdateMetadata"),
        }
    }

    /// Deregister this instance.
    ///
    /// Returns true if the instance was registered.
    pub async fn deregister(self) -> Result<bool> {
        use aspen_client_api::ServiceDeregisterResultResponse;

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::ServiceDeregister {
                service_name: self.service_name.clone(),
                instance_id: self.instance_id.clone(),
                fencing_token: self.fencing_token,
            })
            .await?;

        match response {
            ClientRpcResponse::ServiceDeregisterResult(ServiceDeregisterResultResponse {
                is_success: true,
                was_registered,
                ..
            }) => Ok(was_registered),
            ClientRpcResponse::ServiceDeregisterResult(result) => {
                bail!("service deregister failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
            _ => bail!("unexpected response type for ServiceDeregister"),
        }
    }

    /// Start a background task that sends heartbeats automatically.
    ///
    /// Returns a handle that can be used to stop the heartbeat task.
    pub fn start_heartbeat_task(self, interval: Duration) -> ServiceHeartbeatHandle<C> {
        let cancel_token = CancellationToken::new();
        let running_token = cancel_token.clone();

        let registration = Arc::new(tokio::sync::Mutex::new(self));
        let reg_clone = registration.clone();

        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = running_token.cancelled() => {
                        debug!("service heartbeat task cancelled");
                        break;
                    }
                    _ = tokio::time::sleep(interval) => {
                        let mut reg = reg_clone.lock().await;
                        match reg.heartbeat().await {
                            Ok((deadline, _status)) => {
                                debug!(
                                    service = %reg.service_name,
                                    instance = %reg.instance_id,
                                    new_deadline = deadline,
                                    "service heartbeat sent"
                                );
                            }
                            Err(e) => {
                                warn!(
                                    service = %reg.service_name,
                                    instance = %reg.instance_id,
                                    error = %e,
                                    "service heartbeat failed"
                                );
                            }
                        }
                    }
                }
            }
        });

        ServiceHeartbeatHandle {
            cancel_token,
            task: Some(task),
            registration,
        }
    }
}

/// Handle for a background service heartbeat task.
pub struct ServiceHeartbeatHandle<C: CoordinationRpc> {
    cancel_token: CancellationToken,
    task: Option<tokio::task::JoinHandle<()>>,
    registration: Arc<tokio::sync::Mutex<ServiceRegistration<C>>>,
}

impl<C: CoordinationRpc + 'static> ServiceHeartbeatHandle<C> {
    /// Check if the heartbeat task is still running.
    pub fn is_running(&self) -> bool {
        self.task.as_ref().map(|t| !t.is_finished()).unwrap_or(false)
    }

    /// Stop the heartbeat task.
    pub fn stop(mut self) {
        self.cancel_token.cancel();
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }

    /// Stop heartbeats and deregister the instance.
    pub async fn stop_and_deregister(mut self) -> Result<bool> {
        self.cancel_token.cancel();
        if let Some(task) = self.task.take() {
            task.abort();
        }

        // Take ownership of the registration
        let reg = Arc::try_unwrap(self.registration)
            .map_err(|_| anyhow::anyhow!("registration still in use"))?
            .into_inner();
        reg.deregister().await
    }

    /// Get the service name.
    pub async fn service_name(&self) -> String {
        self.registration.lock().await.service_name.clone()
    }

    /// Get the instance ID.
    pub async fn instance_id(&self) -> String {
        self.registration.lock().await.instance_id.clone()
    }

    /// Get the fencing token.
    pub async fn fencing_token(&self) -> u64 {
        self.registration.lock().await.fencing_token
    }
}

/// Options for service registration.
#[derive(Debug, Clone, Default)]
pub struct ServiceRegisterOptions {
    /// Version string.
    pub version: String,
    /// Tags for filtering.
    pub tags: Vec<String>,
    /// Load balancing weight.
    pub weight: u32,
    /// Custom metadata.
    pub custom_metadata: std::collections::HashMap<String, String>,
    /// TTL for the registration (None = default).
    pub ttl: Option<Duration>,
    /// Optional lease ID to attach to.
    pub lease_id: Option<u64>,
}

/// Filter for service discovery.
#[derive(Debug, Clone, Default)]
pub struct ServiceDiscoveryFilter {
    /// Only return healthy instances.
    pub healthy_only: bool,
    /// Filter by tags (all must match).
    pub tags: Option<Vec<String>>,
    /// Filter by version prefix.
    pub version_prefix: Option<String>,
    /// Maximum instances to return.
    pub limit: Option<u32>,
}

/// Information about a discovered service instance.
#[derive(Debug, Clone)]
pub struct ServiceInstanceInfo {
    /// Unique instance identifier.
    pub instance_id: String,
    /// Service name.
    pub service_name: String,
    /// Network address.
    pub address: String,
    /// Health status.
    pub health_status: String,
    /// Version string.
    pub version: String,
    /// Tags.
    pub tags: Vec<String>,
    /// Load balancing weight.
    pub weight: u32,
    /// Custom metadata.
    pub custom_metadata: std::collections::HashMap<String, String>,
    /// Registration time (as Duration from Unix epoch).
    pub registered_at: Duration,
    /// Last heartbeat time (as Duration from Unix epoch).
    pub last_heartbeat: Duration,
    /// TTL deadline (as Duration from Unix epoch).
    pub deadline: Duration,
    /// Associated lease ID.
    pub lease_id: Option<u64>,
    /// Fencing token.
    pub fencing_token: u64,
}

/// Updates for service instance metadata.
#[derive(Debug, Clone, Default)]
pub struct ServiceMetadataUpdate {
    /// New version (None = keep current).
    pub version: Option<String>,
    /// New tags (None = keep current).
    pub tags: Option<Vec<String>>,
    /// New weight (None = keep current).
    pub weight: Option<u32>,
    /// New custom metadata (None = keep current).
    pub custom_metadata: Option<std::collections::HashMap<String, String>>,
}
