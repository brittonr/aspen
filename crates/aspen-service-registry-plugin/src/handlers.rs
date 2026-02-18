//! Service registry request handlers.
//!
//! Each handler mirrors the behavior of the native `ServiceRegistryHandler`
//! in `aspen-service-registry-handler`, operating through the host KV store.

use std::collections::HashMap;

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::ServiceDeregisterResultResponse;
use aspen_client_api::ServiceDiscoverResultResponse;
use aspen_client_api::ServiceGetInstanceResultResponse;
use aspen_client_api::ServiceHeartbeatResultResponse;
use aspen_client_api::ServiceListResultResponse;
use aspen_client_api::ServiceRegisterResultResponse;
use aspen_client_api::ServiceUpdateHealthResultResponse;
use aspen_client_api::ServiceUpdateMetadataResultResponse;

use crate::kv;
use crate::types::HealthStatus;
use crate::types::ServiceInstance;
use crate::types::ServiceInstanceMetadata;

/// KV key prefix for service instances.
const SERVICE_PREFIX: &str = "__service:";

/// Default service TTL in milliseconds (30 seconds).
const DEFAULT_SERVICE_TTL_MS: u64 = 30_000;

/// Maximum service TTL in milliseconds (24 hours).
const MAX_SERVICE_TTL_MS: u64 = 86_400_000;

/// Maximum number of discovery results.
const MAX_DISCOVERY_RESULTS: u32 = 1000;

// ============================================================================
// Key helpers
// ============================================================================

fn instance_key(service_name: &str, instance_id: &str) -> String {
    format!("{SERVICE_PREFIX}{service_name}:{instance_id}")
}

fn service_prefix(service_name: &str) -> String {
    format!("{SERVICE_PREFIX}{service_name}:")
}

fn read_instance(key: &str) -> Option<ServiceInstance> {
    let bytes = kv::kv_get(key).ok()??;
    serde_json::from_slice(&bytes).ok()
}

fn write_instance(key: &str, instance: &ServiceInstance) -> Result<(), String> {
    let bytes = serde_json::to_vec(instance).map_err(|e| e.to_string())?;
    kv::kv_put(key, &bytes)
}

// ============================================================================
// Handlers
// ============================================================================

#[allow(clippy::too_many_arguments)]
pub fn handle_register(
    service_name: String,
    instance_id: String,
    address: String,
    version: String,
    tags: String,
    weight: u32,
    custom_metadata: String,
    ttl_ms: u64,
    lease_id: Option<u64>,
) -> ClientRpcResponse {
    let parsed_tags: Vec<String> = serde_json::from_str(&tags).unwrap_or_default();
    let custom: HashMap<String, String> = serde_json::from_str(&custom_metadata).unwrap_or_default();

    let now = kv::now_ms();
    let effective_ttl = if ttl_ms == 0 {
        DEFAULT_SERVICE_TTL_MS
    } else {
        ttl_ms.min(MAX_SERVICE_TTL_MS)
    };
    let is_lease_based = lease_id.is_some();
    let deadline_ms = if is_lease_based {
        0
    } else {
        now.saturating_add(effective_ttl)
    };

    let key = instance_key(&service_name, &instance_id);
    let existing = read_instance(&key);

    let (fencing_token, registered_at_ms) = match &existing {
        Some(inst) => (inst.fencing_token.saturating_add(1), inst.registered_at_ms),
        None => (1, now),
    };

    let instance = ServiceInstance {
        instance_id,
        service_name,
        address,
        health_status: HealthStatus::Healthy,
        metadata: ServiceInstanceMetadata {
            version,
            tags: parsed_tags,
            weight,
            custom,
        },
        registered_at_ms,
        last_heartbeat_ms: now,
        deadline_ms,
        ttl_ms: effective_ttl,
        lease_id,
        fencing_token,
    };

    match write_instance(&key, &instance) {
        Ok(()) => ClientRpcResponse::ServiceRegisterResult(ServiceRegisterResultResponse {
            is_success: true,
            fencing_token: Some(fencing_token),
            deadline_ms: Some(deadline_ms),
            error: None,
        }),
        Err(e) => ClientRpcResponse::ServiceRegisterResult(ServiceRegisterResultResponse {
            is_success: false,
            fencing_token: None,
            deadline_ms: None,
            error: Some(e),
        }),
    }
}

pub fn handle_deregister(service_name: String, instance_id: String, fencing_token: u64) -> ClientRpcResponse {
    let key = instance_key(&service_name, &instance_id);

    match read_instance(&key) {
        None => ClientRpcResponse::ServiceDeregisterResult(ServiceDeregisterResultResponse {
            is_success: true,
            was_registered: false,
            error: None,
        }),
        Some(inst) => {
            if inst.fencing_token != fencing_token {
                return ClientRpcResponse::ServiceDeregisterResult(ServiceDeregisterResultResponse {
                    is_success: false,
                    was_registered: false,
                    error: Some(format!(
                        "fencing token mismatch: expected {}, got {}",
                        inst.fencing_token, fencing_token
                    )),
                });
            }

            match kv::kv_delete(&key) {
                Ok(()) => ClientRpcResponse::ServiceDeregisterResult(ServiceDeregisterResultResponse {
                    is_success: true,
                    was_registered: true,
                    error: None,
                }),
                Err(e) => ClientRpcResponse::ServiceDeregisterResult(ServiceDeregisterResultResponse {
                    is_success: false,
                    was_registered: false,
                    error: Some(e),
                }),
            }
        }
    }
}

pub fn handle_discover(
    service_name: String,
    healthy_only: bool,
    tags: String,
    version_prefix: Option<String>,
    limit: Option<u32>,
) -> ClientRpcResponse {
    let required_tags: Vec<String> = serde_json::from_str(&tags).unwrap_or_default();
    let scan_limit = limit.unwrap_or(MAX_DISCOVERY_RESULTS).min(MAX_DISCOVERY_RESULTS);

    let prefix = service_prefix(&service_name);
    let entries = kv::kv_scan(&prefix, scan_limit).unwrap_or_default();
    let now = kv::now_ms();

    let mut instances = Vec::new();

    for (_key, value) in &entries {
        let inst: ServiceInstance = match serde_json::from_slice(value) {
            Ok(i) => i,
            Err(_) => continue,
        };

        // Skip expired
        if inst.deadline_ms > 0 && now > inst.deadline_ms {
            continue;
        }

        // Health filter
        if healthy_only && inst.health_status != HealthStatus::Healthy {
            continue;
        }

        // Tags filter
        if !required_tags.is_empty() {
            let has_all = required_tags.iter().all(|t| inst.metadata.tags.contains(t));
            if !has_all {
                continue;
            }
        }

        // Version prefix filter
        if let Some(ref vp) = version_prefix
            && !inst.metadata.version.starts_with(vp.as_str())
        {
            continue;
        }

        instances.push(inst.to_response());

        if instances.len() >= scan_limit as usize {
            break;
        }
    }

    let count = instances.len() as u32;
    ClientRpcResponse::ServiceDiscoverResult(ServiceDiscoverResultResponse {
        is_success: true,
        instances,
        count,
        error: None,
    })
}

pub fn handle_list(prefix: String, limit: u32) -> ClientRpcResponse {
    let scan_limit = limit.min(MAX_DISCOVERY_RESULTS);

    let full_prefix = format!("{SERVICE_PREFIX}{prefix}");
    let entries = kv::kv_scan(&full_prefix, scan_limit).unwrap_or_default();

    let mut services = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for (key, _) in &entries {
        // Key format: __service:{name}:{instance_id}
        if let Some(rest) = key.strip_prefix(SERVICE_PREFIX)
            && let Some(colon_pos) = rest.find(':')
        {
            let svc_name = &rest[..colon_pos];
            if seen.insert(svc_name.to_string()) {
                services.push(svc_name.to_string());
            }
        }
    }

    let count = services.len() as u32;
    ClientRpcResponse::ServiceListResult(ServiceListResultResponse {
        is_success: true,
        services,
        count,
        error: None,
    })
}

pub fn handle_get_instance(service_name: String, instance_id: String) -> ClientRpcResponse {
    let key = instance_key(&service_name, &instance_id);

    match read_instance(&key) {
        Some(inst) => {
            let now = kv::now_ms();
            if inst.deadline_ms > 0 && now > inst.deadline_ms {
                // Expired
                let _ = kv::kv_delete(&key);
                return ClientRpcResponse::ServiceGetInstanceResult(ServiceGetInstanceResultResponse {
                    is_success: true,
                    was_found: false,
                    instance: None,
                    error: None,
                });
            }

            ClientRpcResponse::ServiceGetInstanceResult(ServiceGetInstanceResultResponse {
                is_success: true,
                was_found: true,
                instance: Some(inst.to_response()),
                error: None,
            })
        }
        None => ClientRpcResponse::ServiceGetInstanceResult(ServiceGetInstanceResultResponse {
            is_success: true,
            was_found: false,
            instance: None,
            error: None,
        }),
    }
}

pub fn handle_heartbeat(service_name: String, instance_id: String, fencing_token: u64) -> ClientRpcResponse {
    let key = instance_key(&service_name, &instance_id);

    match read_instance(&key) {
        None => ClientRpcResponse::ServiceHeartbeatResult(ServiceHeartbeatResultResponse {
            is_success: false,
            new_deadline_ms: None,
            health_status: None,
            error: Some(format!("instance not found: {}:{}", service_name, instance_id)),
        }),
        Some(mut inst) => {
            if inst.fencing_token != fencing_token {
                return ClientRpcResponse::ServiceHeartbeatResult(ServiceHeartbeatResultResponse {
                    is_success: false,
                    new_deadline_ms: None,
                    health_status: None,
                    error: Some(format!(
                        "fencing token mismatch: expected {}, got {}",
                        inst.fencing_token, fencing_token
                    )),
                });
            }

            let now = kv::now_ms();
            inst.last_heartbeat_ms = now;
            inst.deadline_ms = if inst.lease_id.is_some() {
                0
            } else {
                now.saturating_add(inst.ttl_ms)
            };

            match write_instance(&key, &inst) {
                Ok(()) => ClientRpcResponse::ServiceHeartbeatResult(ServiceHeartbeatResultResponse {
                    is_success: true,
                    new_deadline_ms: Some(inst.deadline_ms),
                    health_status: Some(inst.health_status.as_str().to_string()),
                    error: None,
                }),
                Err(e) => ClientRpcResponse::ServiceHeartbeatResult(ServiceHeartbeatResultResponse {
                    is_success: false,
                    new_deadline_ms: None,
                    health_status: None,
                    error: Some(e),
                }),
            }
        }
    }
}

pub fn handle_update_health(
    service_name: String,
    instance_id: String,
    fencing_token: u64,
    status: String,
) -> ClientRpcResponse {
    let key = instance_key(&service_name, &instance_id);

    match read_instance(&key) {
        None => ClientRpcResponse::ServiceUpdateHealthResult(ServiceUpdateHealthResultResponse {
            is_success: false,
            error: Some(format!("instance not found: {}:{}", service_name, instance_id)),
        }),
        Some(mut inst) => {
            if inst.fencing_token != fencing_token {
                return ClientRpcResponse::ServiceUpdateHealthResult(ServiceUpdateHealthResultResponse {
                    is_success: false,
                    error: Some(format!(
                        "fencing token mismatch: expected {}, got {}",
                        inst.fencing_token, fencing_token
                    )),
                });
            }

            inst.health_status = HealthStatus::parse(&status);

            match write_instance(&key, &inst) {
                Ok(()) => ClientRpcResponse::ServiceUpdateHealthResult(ServiceUpdateHealthResultResponse {
                    is_success: true,
                    error: None,
                }),
                Err(e) => ClientRpcResponse::ServiceUpdateHealthResult(ServiceUpdateHealthResultResponse {
                    is_success: false,
                    error: Some(e),
                }),
            }
        }
    }
}

pub fn handle_update_metadata(
    service_name: String,
    instance_id: String,
    fencing_token: u64,
    version: Option<String>,
    tags: Option<String>,
    weight: Option<u32>,
    custom_metadata: Option<String>,
) -> ClientRpcResponse {
    let key = instance_key(&service_name, &instance_id);

    match read_instance(&key) {
        None => ClientRpcResponse::ServiceUpdateMetadataResult(ServiceUpdateMetadataResultResponse {
            is_success: false,
            error: Some("instance not found".to_string()),
        }),
        Some(mut inst) => {
            if inst.fencing_token != fencing_token {
                return ClientRpcResponse::ServiceUpdateMetadataResult(ServiceUpdateMetadataResultResponse {
                    is_success: false,
                    error: Some("fencing token mismatch".to_string()),
                });
            }

            if let Some(v) = version {
                inst.metadata.version = v;
            }
            if let Some(t) = tags
                && let Ok(parsed) = serde_json::from_str::<Vec<String>>(&t)
            {
                inst.metadata.tags = parsed;
            }
            if let Some(w) = weight {
                inst.metadata.weight = w;
            }
            if let Some(c) = custom_metadata
                && let Ok(parsed) = serde_json::from_str::<HashMap<String, String>>(&c)
            {
                inst.metadata.custom = parsed;
            }

            match write_instance(&key, &inst) {
                Ok(()) => ClientRpcResponse::ServiceUpdateMetadataResult(ServiceUpdateMetadataResultResponse {
                    is_success: true,
                    error: None,
                }),
                Err(e) => ClientRpcResponse::ServiceUpdateMetadataResult(ServiceUpdateMetadataResultResponse {
                    is_success: false,
                    error: Some(e),
                }),
            }
        }
    }
}
