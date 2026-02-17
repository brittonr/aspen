//! JSON marshaling between ClientRpcRequest/Response and WASM guest bytes.
//!
//! The WASM guest receives a JSON-serialized `ClientRpcRequest` and returns
//! a JSON-serialized `ClientRpcResponse`. This module handles the
//! serialization and deserialization, plus variant name extraction for
//! request routing.

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;

/// Serialize a request to JSON bytes for the WASM guest.
pub fn serialize_request(request: &ClientRpcRequest) -> anyhow::Result<Vec<u8>> {
    serde_json::to_vec(request).map_err(|e| anyhow::anyhow!("failed to serialize request: {e}"))
}

/// Deserialize a response from JSON bytes returned by the WASM guest.
pub fn deserialize_response(bytes: &[u8]) -> anyhow::Result<ClientRpcResponse> {
    serde_json::from_slice(bytes).map_err(|e| anyhow::anyhow!("failed to deserialize response: {e}"))
}

/// Extract the serde variant name from a `ClientRpcRequest`.
///
/// `ClientRpcRequest` uses serde's default external tagging, so serializing
/// to a JSON value gives either `{"VariantName": {...}}` for struct variants
/// or `"VariantName"` for unit variants.
pub fn extract_variant_name(request: &ClientRpcRequest) -> Option<String> {
    match serde_json::to_value(request) {
        Ok(serde_json::Value::Object(map)) => map.keys().next().cloned(),
        Ok(serde_json::Value::String(s)) => Some(s),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_variant_name_from_ping() {
        let request = ClientRpcRequest::Ping;
        let name = extract_variant_name(&request);
        assert!(name.is_some(), "should extract variant name from Ping");
    }
}
