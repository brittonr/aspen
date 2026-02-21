//! WASM guest plugin for the Aspen secrets engine.
//!
//! Handles KV (versioned secrets) and Transit (encryption-as-a-service)
//! operations using the host-provided KV store, blob store, and crypto bindings.
//!
//! ## Scope
//!
//! - **KV**: Versioned key-value secrets with soft/hard delete, CAS, metadata
//! - **Transit**: Symmetric encryption (XChaCha20-Poly1305 via host random bytes), Ed25519 signing
//!   (via host crypto), key rotation, data key generation
//!
//! PKI and Nix Cache operations remain in the native handler (complex crypto
//! dependencies that don't suit the WASM sandbox).

mod kv;
mod secrets_kv;
mod transit;
mod types;

use aspen_wasm_guest_sdk::AspenPlugin;
use aspen_wasm_guest_sdk::ClientRpcRequest;
use aspen_wasm_guest_sdk::ClientRpcResponse;
use aspen_wasm_guest_sdk::PluginInfo;
use aspen_wasm_guest_sdk::PluginPermissions;
use aspen_wasm_guest_sdk::register_plugin;

struct SecretsPlugin;

impl AspenPlugin for SecretsPlugin {
    fn info() -> PluginInfo {
        PluginInfo {
            name: "secrets".to_string(),
            version: "0.1.0".to_string(),
            handles: vec![
                // KV
                "SecretsKvRead".to_string(),
                "SecretsKvWrite".to_string(),
                "SecretsKvDelete".to_string(),
                "SecretsKvDestroy".to_string(),
                "SecretsKvUndelete".to_string(),
                "SecretsKvList".to_string(),
                "SecretsKvMetadata".to_string(),
                "SecretsKvUpdateMetadata".to_string(),
                "SecretsKvDeleteMetadata".to_string(),
                // Transit
                "SecretsTransitCreateKey".to_string(),
                "SecretsTransitEncrypt".to_string(),
                "SecretsTransitDecrypt".to_string(),
                "SecretsTransitSign".to_string(),
                "SecretsTransitVerify".to_string(),
                "SecretsTransitRotateKey".to_string(),
                "SecretsTransitListKeys".to_string(),
                "SecretsTransitRewrap".to_string(),
                "SecretsTransitDatakey".to_string(),
            ],
            priority: 940,
            app_id: Some("secrets".to_string()),
            kv_prefixes: vec!["__secrets:".to_string()],
            permissions: PluginPermissions {
                kv_read: true,
                kv_write: true,
                blob_read: true,
                blob_write: true,
                randomness: true,
                signing: true,
                ..PluginPermissions::default()
            },
        }
    }

    fn handle(request: ClientRpcRequest) -> ClientRpcResponse {
        match request {
            // KV secrets
            ClientRpcRequest::SecretsKvRead { mount, path, version } => {
                secrets_kv::handle_kv_read(mount, path, version)
            }

            ClientRpcRequest::SecretsKvWrite { mount, path, data, cas } => {
                secrets_kv::handle_kv_write(mount, path, data, cas)
            }

            ClientRpcRequest::SecretsKvDelete { mount, path, versions } => {
                secrets_kv::handle_kv_delete(mount, path, versions)
            }

            ClientRpcRequest::SecretsKvDestroy { mount, path, versions } => {
                secrets_kv::handle_kv_destroy(mount, path, versions)
            }

            ClientRpcRequest::SecretsKvUndelete { mount, path, versions } => {
                secrets_kv::handle_kv_undelete(mount, path, versions)
            }

            ClientRpcRequest::SecretsKvList { mount, path } => secrets_kv::handle_kv_list(mount, path),

            ClientRpcRequest::SecretsKvMetadata { mount, path } => secrets_kv::handle_kv_metadata(mount, path),

            ClientRpcRequest::SecretsKvUpdateMetadata {
                mount,
                path,
                max_versions,
                cas_required,
                custom_metadata,
            } => secrets_kv::handle_kv_update_metadata(mount, path, max_versions, cas_required, custom_metadata),

            ClientRpcRequest::SecretsKvDeleteMetadata { mount, path } => {
                secrets_kv::handle_kv_delete_metadata(mount, path)
            }

            // Transit
            ClientRpcRequest::SecretsTransitCreateKey { mount, name, key_type } => {
                transit::handle_create_key(mount, name, key_type)
            }

            ClientRpcRequest::SecretsTransitEncrypt {
                mount,
                name,
                plaintext,
                context,
            } => transit::handle_encrypt(mount, name, plaintext, context),

            ClientRpcRequest::SecretsTransitDecrypt {
                mount,
                name,
                ciphertext,
                context,
            } => transit::handle_decrypt(mount, name, ciphertext, context),

            ClientRpcRequest::SecretsTransitSign { mount, name, data } => transit::handle_sign(mount, name, data),

            ClientRpcRequest::SecretsTransitVerify {
                mount,
                name,
                data,
                signature,
            } => transit::handle_verify(mount, name, data, signature),

            ClientRpcRequest::SecretsTransitRotateKey { mount, name } => transit::handle_rotate_key(mount, name),

            ClientRpcRequest::SecretsTransitListKeys { mount } => transit::handle_list_keys(mount),

            ClientRpcRequest::SecretsTransitRewrap {
                mount,
                name,
                ciphertext,
                context,
            } => transit::handle_rewrap(mount, name, ciphertext, context),

            ClientRpcRequest::SecretsTransitDatakey { mount, name, key_type } => {
                transit::handle_datakey(mount, name, key_type)
            }

            _ => ClientRpcResponse::Error(aspen_wasm_guest_sdk::response::error_response(
                "UNHANDLED",
                "secrets plugin does not handle this request type",
            )),
        }
    }
}

register_plugin!(SecretsPlugin);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_info_matches_manifest() {
        let manifest_bytes = include_bytes!("../plugin.json");
        let manifest: PluginInfo = serde_json::from_slice(manifest_bytes).expect("plugin.json should be valid");
        let info = SecretsPlugin::info();
        assert_eq!(info.name, manifest.name, "name mismatch between code and plugin.json");
        assert_eq!(info.handles, manifest.handles, "handles mismatch between code and plugin.json");
        assert_eq!(info.priority, manifest.priority, "priority mismatch between code and plugin.json");
        assert_eq!(info.version, manifest.version, "version mismatch between code and plugin.json");
        assert_eq!(info.app_id, manifest.app_id, "app_id mismatch between code and plugin.json");
        assert_eq!(info.kv_prefixes, manifest.kv_prefixes, "kv_prefixes mismatch between code and plugin.json");
    }
}
