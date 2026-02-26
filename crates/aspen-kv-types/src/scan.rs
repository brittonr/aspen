//! Scan operation types for prefix-based key enumeration.

use serde::Deserialize;
use serde::Serialize;

use crate::KeyValueWithRevision;

/// Request to scan keys with a given prefix.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanRequest {
    pub prefix: String,
    #[serde(rename = "limit")]
    pub limit_results: Option<u32>,
    pub continuation_token: Option<String>,
}

/// Response from a scan operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanResult {
    pub entries: Vec<KeyValueWithRevision>,
    #[serde(rename = "count")]
    pub result_count: u32,
    pub is_truncated: bool,
    pub continuation_token: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_request_serialization_roundtrip() {
        let req = ScanRequest {
            prefix: "prefix".into(),
            limit_results: Some(100),
            continuation_token: Some("token".into()),
        };
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: ScanRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, deserialized);
    }

    #[test]
    fn scan_result_serialization_roundtrip() {
        let result = ScanResult {
            entries: vec![KeyValueWithRevision {
                key: "k".into(),
                value: "v".into(),
                version: 1,
                create_revision: 1,
                mod_revision: 1,
            }],
            result_count: 1,
            is_truncated: false,
            continuation_token: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ScanResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result, deserialized);
    }
}
