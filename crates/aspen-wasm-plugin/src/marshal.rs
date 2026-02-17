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
        assert_eq!(name, Some("Ping".to_string()));
    }

    #[test]
    fn extract_variant_name_from_read_key() {
        let request = ClientRpcRequest::ReadKey {
            key: "test".to_string(),
        };
        let name = extract_variant_name(&request);
        assert_eq!(name, Some("ReadKey".to_string()));
    }

    #[test]
    fn extract_variant_name_from_write_key() {
        let request = ClientRpcRequest::WriteKey {
            key: "test".to_string(),
            value: b"val".to_vec(),
        };
        let name = extract_variant_name(&request);
        assert_eq!(name, Some("WriteKey".to_string()));
    }

    #[test]
    fn serialize_request_produces_valid_json() {
        let request = ClientRpcRequest::ReadKey {
            key: "mykey".to_string(),
        };
        let bytes = serialize_request(&request).expect("serialization should succeed");
        let value: serde_json::Value = serde_json::from_slice(&bytes).expect("should be valid JSON");
        assert!(value.is_object(), "externally tagged enum serializes as object");
        assert!(value.get("ReadKey").is_some(), "should contain ReadKey variant key");
    }

    #[test]
    fn serialize_request_unit_variant() {
        let request = ClientRpcRequest::Ping;
        let bytes = serialize_request(&request).expect("serialization should succeed");
        let value: serde_json::Value = serde_json::from_slice(&bytes).expect("should be valid JSON");
        // Unit variants serialize as strings in serde's default external tagging
        assert!(value.is_string(), "unit variant serializes as string");
        assert_eq!(value.as_str(), Some("Ping"));
    }

    #[test]
    fn deserialize_response_roundtrip() {
        let response = ClientRpcResponse::Pong;
        let bytes = serde_json::to_vec(&response).expect("serialize");
        let roundtripped = deserialize_response(&bytes).expect("deserialize");
        assert!(matches!(roundtripped, ClientRpcResponse::Pong));
    }
}
