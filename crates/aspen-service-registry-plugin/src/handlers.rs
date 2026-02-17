//! Service registry request handlers.
//!
//! Each handler mirrors the behavior of the native `ServiceRegistryHandler`
//! in `aspen-service-registry-handler`, operating through the host KV store.

use std::collections::HashMap;

use serde_json::Value;
use serde_json::json;

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
    let bytes = kv::kv_get(key)?;
    serde_json::from_slice(&bytes).ok()
}

fn write_instance(key: &str, instance: &ServiceInstance) -> Result<(), String> {
    let bytes = serde_json::to_vec(instance).map_err(|e| e.to_string())?;
    kv::kv_put(key, &bytes)
}

// ============================================================================
// Handlers
// ============================================================================

pub fn handle_register(req: &Value) -> Value {
    let service_name = req["service_name"].as_str().unwrap_or_default();
    let instance_id = req["instance_id"].as_str().unwrap_or_default();
    let address = req["address"].as_str().unwrap_or_default();
    let version = req["version"].as_str().unwrap_or_default();
    let tags_json = req["tags"].as_str().unwrap_or("[]");
    let weight = req["weight"].as_u64().unwrap_or(100) as u32;
    let custom_json = req["custom_metadata"].as_str().unwrap_or("{}");
    let ttl_ms = req["ttl_ms"].as_u64().unwrap_or(0);
    let lease_id = req["lease_id"].as_u64();

    let tags: Vec<String> = serde_json::from_str(tags_json).unwrap_or_default();
    let custom: HashMap<String, String> = serde_json::from_str(custom_json).unwrap_or_default();

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

    let key = instance_key(service_name, instance_id);

    let existing = read_instance(&key);

    let (fencing_token, registered_at_ms) = match &existing {
        Some(inst) => (inst.fencing_token.saturating_add(1), inst.registered_at_ms),
        None => (1, now),
    };

    let instance = ServiceInstance {
        instance_id: instance_id.to_string(),
        service_name: service_name.to_string(),
        address: address.to_string(),
        health_status: HealthStatus::Healthy,
        metadata: ServiceInstanceMetadata {
            version: version.to_string(),
            tags,
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
        Ok(()) => json!({
            "ServiceRegisterResult": {
                "is_success": true,
                "fencing_token": fencing_token,
                "deadline_ms": deadline_ms,
                "error": null
            }
        }),
        Err(e) => json!({
            "ServiceRegisterResult": {
                "is_success": false,
                "fencing_token": null,
                "deadline_ms": null,
                "error": e
            }
        }),
    }
}

pub fn handle_deregister(req: &Value) -> Value {
    let service_name = req["service_name"].as_str().unwrap_or_default();
    let instance_id = req["instance_id"].as_str().unwrap_or_default();
    let fencing_token = req["fencing_token"].as_u64().unwrap_or(0);

    let key = instance_key(service_name, instance_id);

    match read_instance(&key) {
        None => json!({
            "ServiceDeregisterResult": {
                "is_success": true,
                "was_registered": false,
                "error": null
            }
        }),
        Some(inst) => {
            if inst.fencing_token != fencing_token {
                return json!({
                    "ServiceDeregisterResult": {
                        "is_success": false,
                        "was_registered": false,
                        "error": format!("fencing token mismatch: expected {}, got {}", inst.fencing_token, fencing_token)
                    }
                });
            }

            match kv::kv_delete(&key) {
                Ok(()) => json!({
                    "ServiceDeregisterResult": {
                        "is_success": true,
                        "was_registered": true,
                        "error": null
                    }
                }),
                Err(e) => json!({
                    "ServiceDeregisterResult": {
                        "is_success": false,
                        "was_registered": false,
                        "error": e
                    }
                }),
            }
        }
    }
}

pub fn handle_discover(req: &Value) -> Value {
    let service_name = req["service_name"].as_str().unwrap_or_default();
    let healthy_only = req["healthy_only"].as_bool().unwrap_or(false);
    let tags_json = req["tags"].as_str().unwrap_or("[]");
    let version_prefix = req["version_prefix"].as_str();
    let limit = req["limit"].as_u64().map(|l| l as u32);

    let required_tags: Vec<String> = serde_json::from_str(tags_json).unwrap_or_default();
    let scan_limit = limit.unwrap_or(MAX_DISCOVERY_RESULTS).min(MAX_DISCOVERY_RESULTS);

    let prefix = service_prefix(service_name);
    let entries = kv::kv_scan(&prefix, scan_limit);
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
        if let Some(vp) = version_prefix {
            if !inst.metadata.version.starts_with(vp) {
                continue;
            }
        }

        instances.push(inst.to_response());

        if instances.len() >= scan_limit as usize {
            break;
        }
    }

    let count = instances.len() as u32;
    json!({
        "ServiceDiscoverResult": {
            "is_success": true,
            "instances": instances,
            "count": count,
            "error": null
        }
    })
}

pub fn handle_list(req: &Value) -> Value {
    let prefix = req["prefix"].as_str().unwrap_or_default();
    let limit = req["limit"].as_u64().unwrap_or(100) as u32;
    let scan_limit = limit.min(MAX_DISCOVERY_RESULTS);

    let full_prefix = format!("{SERVICE_PREFIX}{prefix}");
    let entries = kv::kv_scan(&full_prefix, scan_limit);

    let mut services = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for (key, _) in &entries {
        // Key format: __service:{name}:{instance_id}
        if let Some(rest) = key.strip_prefix(SERVICE_PREFIX) {
            if let Some(colon_pos) = rest.find(':') {
                let svc_name = &rest[..colon_pos];
                if seen.insert(svc_name.to_string()) {
                    services.push(svc_name.to_string());
                }
            }
        }
    }

    let count = services.len() as u32;
    json!({
        "ServiceListResult": {
            "is_success": true,
            "services": services,
            "count": count,
            "error": null
        }
    })
}

pub fn handle_get_instance(req: &Value) -> Value {
    let service_name = req["service_name"].as_str().unwrap_or_default();
    let instance_id = req["instance_id"].as_str().unwrap_or_default();

    let key = instance_key(service_name, instance_id);

    match read_instance(&key) {
        Some(inst) => {
            let now = kv::now_ms();
            if inst.deadline_ms > 0 && now > inst.deadline_ms {
                // Expired
                let _ = kv::kv_delete(&key);
                return json!({
                    "ServiceGetInstanceResult": {
                        "is_success": true,
                        "was_found": false,
                        "instance": null,
                        "error": null
                    }
                });
            }

            json!({
                "ServiceGetInstanceResult": {
                    "is_success": true,
                    "was_found": true,
                    "instance": inst.to_response(),
                    "error": null
                }
            })
        }
        None => json!({
            "ServiceGetInstanceResult": {
                "is_success": true,
                "was_found": false,
                "instance": null,
                "error": null
            }
        }),
    }
}

pub fn handle_heartbeat(req: &Value) -> Value {
    let service_name = req["service_name"].as_str().unwrap_or_default();
    let instance_id = req["instance_id"].as_str().unwrap_or_default();
    let fencing_token = req["fencing_token"].as_u64().unwrap_or(0);

    let key = instance_key(service_name, instance_id);

    match read_instance(&key) {
        None => json!({
            "ServiceHeartbeatResult": {
                "is_success": false,
                "new_deadline_ms": null,
                "health_status": null,
                "error": format!("instance not found: {}:{}", service_name, instance_id)
            }
        }),
        Some(mut inst) => {
            if inst.fencing_token != fencing_token {
                return json!({
                    "ServiceHeartbeatResult": {
                        "is_success": false,
                        "new_deadline_ms": null,
                        "health_status": null,
                        "error": format!("fencing token mismatch: expected {}, got {}", inst.fencing_token, fencing_token)
                    }
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
                Ok(()) => json!({
                    "ServiceHeartbeatResult": {
                        "is_success": true,
                        "new_deadline_ms": inst.deadline_ms,
                        "health_status": inst.health_status.as_str(),
                        "error": null
                    }
                }),
                Err(e) => json!({
                    "ServiceHeartbeatResult": {
                        "is_success": false,
                        "new_deadline_ms": null,
                        "health_status": null,
                        "error": e
                    }
                }),
            }
        }
    }
}

pub fn handle_update_health(req: &Value) -> Value {
    let service_name = req["service_name"].as_str().unwrap_or_default();
    let instance_id = req["instance_id"].as_str().unwrap_or_default();
    let fencing_token = req["fencing_token"].as_u64().unwrap_or(0);
    let status = req["status"].as_str().unwrap_or("unknown");

    let key = instance_key(service_name, instance_id);

    match read_instance(&key) {
        None => json!({
            "ServiceUpdateHealthResult": {
                "is_success": false,
                "error": format!("instance not found: {}:{}", service_name, instance_id)
            }
        }),
        Some(mut inst) => {
            if inst.fencing_token != fencing_token {
                return json!({
                    "ServiceUpdateHealthResult": {
                        "is_success": false,
                        "error": format!("fencing token mismatch: expected {}, got {}", inst.fencing_token, fencing_token)
                    }
                });
            }

            inst.health_status = HealthStatus::parse(status);

            match write_instance(&key, &inst) {
                Ok(()) => json!({
                    "ServiceUpdateHealthResult": {
                        "is_success": true,
                        "error": null
                    }
                }),
                Err(e) => json!({
                    "ServiceUpdateHealthResult": {
                        "is_success": false,
                        "error": e
                    }
                }),
            }
        }
    }
}

pub fn handle_update_metadata(req: &Value) -> Value {
    let service_name = req["service_name"].as_str().unwrap_or_default();
    let instance_id = req["instance_id"].as_str().unwrap_or_default();
    let fencing_token = req["fencing_token"].as_u64().unwrap_or(0);

    let key = instance_key(service_name, instance_id);

    match read_instance(&key) {
        None => json!({
            "ServiceUpdateMetadataResult": {
                "is_success": false,
                "error": "instance not found"
            }
        }),
        Some(mut inst) => {
            if inst.fencing_token != fencing_token {
                return json!({
                    "ServiceUpdateMetadataResult": {
                        "is_success": false,
                        "error": "fencing token mismatch"
                    }
                });
            }

            if let Some(v) = req["version"].as_str() {
                inst.metadata.version = v.to_string();
            }
            if let Some(t) = req["tags"].as_str() {
                if let Ok(parsed) = serde_json::from_str::<Vec<String>>(t) {
                    inst.metadata.tags = parsed;
                }
            }
            if let Some(w) = req["weight"].as_u64() {
                inst.metadata.weight = w as u32;
            }
            if let Some(c) = req["custom_metadata"].as_str() {
                if let Ok(parsed) = serde_json::from_str::<HashMap<String, String>>(c) {
                    inst.metadata.custom = parsed;
                }
            }

            match write_instance(&key, &inst) {
                Ok(()) => json!({
                    "ServiceUpdateMetadataResult": {
                        "is_success": true,
                        "error": null
                    }
                }),
                Err(e) => json!({
                    "ServiceUpdateMetadataResult": {
                        "is_success": false,
                        "error": e
                    }
                }),
            }
        }
    }
}
