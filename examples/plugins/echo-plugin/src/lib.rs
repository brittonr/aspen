//! Echo plugin -- minimal Aspen WASM handler plugin example.
//!
//! Demonstrates the guest SDK by handling two request types:
//! - `Ping` -> `Pong`
//! - `ReadKey` -> reads from host KV store and returns the result

use aspen_wasm_guest_sdk::AspenPlugin;
use aspen_wasm_guest_sdk::ClientRpcRequest;
use aspen_wasm_guest_sdk::ClientRpcResponse;
use aspen_wasm_guest_sdk::PluginInfo;
use aspen_wasm_guest_sdk::ReadResultResponse;
use aspen_wasm_guest_sdk::register_plugin;

struct EchoPlugin;

impl AspenPlugin for EchoPlugin {
    fn info() -> PluginInfo {
        PluginInfo {
            name: "echo-plugin".to_string(),
            version: "0.1.0".to_string(),
            handles: vec!["Ping".to_string(), "ReadKey".to_string()],
            priority: 950,
        }
    }

    fn handle(request: ClientRpcRequest) -> ClientRpcResponse {
        match request {
            ClientRpcRequest::Ping => ClientRpcResponse::Pong,

            ClientRpcRequest::ReadKey { ref key } => {
                let value = aspen_wasm_guest_sdk::host::kv_get_value(key);
                let was_found = value.is_some();
                ClientRpcResponse::ReadResult(ReadResultResponse {
                    value,
                    was_found,
                    error: None,
                })
            }

            _ => ClientRpcResponse::Error(aspen_wasm_guest_sdk::response::error_response(
                "UNHANDLED",
                "echo-plugin does not handle this request type",
            )),
        }
    }
}

register_plugin!(EchoPlugin);
