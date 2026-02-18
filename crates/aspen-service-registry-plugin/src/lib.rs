//! WASM guest plugin for the Aspen service registry handler.
//!
//! This crate compiles to `wasm32-wasip2` and exports `handle_request`
//! and `plugin_info` for the Aspen plugin runtime. It reimplements the
//! native `ServiceRegistryHandler` using host-provided KV operations
//! through the `aspen-wasm-guest-sdk`.

mod handlers;
mod kv;
mod types;

use aspen_wasm_guest_sdk::AspenPlugin;
use aspen_wasm_guest_sdk::ClientRpcRequest;
use aspen_wasm_guest_sdk::ClientRpcResponse;
use aspen_wasm_guest_sdk::PluginInfo;
use aspen_wasm_guest_sdk::PluginPermissions;
use aspen_wasm_guest_sdk::register_plugin;

struct ServiceRegistryPlugin;

impl AspenPlugin for ServiceRegistryPlugin {
    fn info() -> PluginInfo {
        PluginInfo {
            name: "service-registry".to_string(),
            version: "0.1.0".to_string(),
            handles: vec![
                "ServiceRegister".to_string(),
                "ServiceDeregister".to_string(),
                "ServiceDiscover".to_string(),
                "ServiceList".to_string(),
                "ServiceGetInstance".to_string(),
                "ServiceHeartbeat".to_string(),
                "ServiceUpdateHealth".to_string(),
                "ServiceUpdateMetadata".to_string(),
            ],
            priority: 950,
            app_id: Some("service-registry".to_string()),
            kv_prefixes: vec!["__service:".to_string()],
            permissions: PluginPermissions {
                kv_read: true,
                kv_write: true,
                ..PluginPermissions::default()
            },
        }
    }

    fn handle(request: ClientRpcRequest) -> ClientRpcResponse {
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
            } => handlers::handle_register(
                service_name,
                instance_id,
                address,
                version,
                tags,
                weight,
                custom_metadata,
                ttl_ms,
                lease_id,
            ),

            ClientRpcRequest::ServiceDeregister {
                service_name,
                instance_id,
                fencing_token,
            } => handlers::handle_deregister(service_name, instance_id, fencing_token),

            ClientRpcRequest::ServiceDiscover {
                service_name,
                healthy_only,
                tags,
                version_prefix,
                limit,
            } => handlers::handle_discover(service_name, healthy_only, tags, version_prefix, limit),

            ClientRpcRequest::ServiceList { prefix, limit } => handlers::handle_list(prefix, limit),

            ClientRpcRequest::ServiceGetInstance {
                service_name,
                instance_id,
            } => handlers::handle_get_instance(service_name, instance_id),

            ClientRpcRequest::ServiceHeartbeat {
                service_name,
                instance_id,
                fencing_token,
            } => handlers::handle_heartbeat(service_name, instance_id, fencing_token),

            ClientRpcRequest::ServiceUpdateHealth {
                service_name,
                instance_id,
                fencing_token,
                status,
            } => handlers::handle_update_health(service_name, instance_id, fencing_token, status),

            ClientRpcRequest::ServiceUpdateMetadata {
                service_name,
                instance_id,
                fencing_token,
                version,
                tags,
                weight,
                custom_metadata,
            } => handlers::handle_update_metadata(
                service_name,
                instance_id,
                fencing_token,
                version,
                tags,
                weight,
                custom_metadata,
            ),

            _ => ClientRpcResponse::Error(aspen_wasm_guest_sdk::response::error_response(
                "UNHANDLED",
                "service-registry plugin does not handle this request type",
            )),
        }
    }
}

register_plugin!(ServiceRegistryPlugin);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_info_matches_manifest() {
        let manifest_bytes = include_bytes!("../plugin.json");
        let manifest: PluginInfo = serde_json::from_slice(manifest_bytes).expect("plugin.json should be valid");
        let info = ServiceRegistryPlugin::info();
        assert_eq!(info.name, manifest.name, "name mismatch between code and plugin.json");
        assert_eq!(info.handles, manifest.handles, "handles mismatch between code and plugin.json");
        assert_eq!(info.priority, manifest.priority, "priority mismatch between code and plugin.json");
        assert_eq!(info.version, manifest.version, "version mismatch between code and plugin.json");
        assert_eq!(info.app_id, manifest.app_id, "app_id mismatch between code and plugin.json");
        assert_eq!(info.kv_prefixes, manifest.kv_prefixes, "kv_prefixes mismatch between code and plugin.json");
    }
}
