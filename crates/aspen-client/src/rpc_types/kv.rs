// Key-value operation response types.

use serde::Deserialize;
use serde::Serialize;

/// Read key result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResultResponse {
    pub value: Option<Vec<u8>>,
    pub found: bool,
    pub error: Option<String>,
}

/// Write key result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

/// Compare-and-swap result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareAndSwapResultResponse {
    pub success: bool,
    pub actual_value: Option<Vec<u8>>,
    pub error: Option<String>,
}

/// Delete key result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteResultResponse {
    pub key: String,
    pub deleted: bool,
    pub error: Option<String>,
}

/// Scan keys result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResultResponse {
    pub entries: Vec<ScanEntry>,
    pub count: u32,
    pub is_truncated: bool,
    pub continuation_token: Option<String>,
    pub error: Option<String>,
}

/// Single entry from scan operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanEntry {
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub version: u64,
    #[serde(default)]
    pub create_revision: u64,
    #[serde(default)]
    pub mod_revision: u64,
}
