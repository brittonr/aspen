// Pijul response types (feature-gated).

use serde::Deserialize;
use serde::Serialize;

#[cfg(feature = "pijul")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulRepoResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub default_channel: String,
    pub channel_count: u32,
    pub created_at_ms: u64,
}

#[cfg(feature = "pijul")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulRepoListResponse {
    pub repos: Vec<PijulRepoResponse>,
    pub count: u32,
}

#[cfg(feature = "pijul")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulChannelResponse {
    pub name: String,
    pub head: Option<String>,
    pub updated_at_ms: u64,
}

#[cfg(feature = "pijul")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulChannelListResponse {
    pub channels: Vec<PijulChannelResponse>,
    pub count: u32,
}

#[cfg(feature = "pijul")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulRecordedChange {
    pub hash: String,
    pub message: String,
    pub hunks: u32,
    pub size_bytes: u64,
}

#[cfg(feature = "pijul")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulRecordResponse {
    pub change: Option<PijulRecordedChange>,
}

#[cfg(feature = "pijul")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulApplyResponse {
    pub operations: u64,
}

#[cfg(feature = "pijul")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulUnrecordResponse {
    #[serde(rename = "unrecorded")]
    pub was_unrecorded: bool,
}

#[cfg(feature = "pijul")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulLogEntry {
    pub change_hash: String,
    pub message: String,
    pub author: Option<String>,
    pub timestamp_ms: u64,
}

#[cfg(feature = "pijul")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulLogResponse {
    pub entries: Vec<PijulLogEntry>,
    pub count: u32,
}

#[cfg(feature = "pijul")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulCheckoutResponse {
    pub files_written: u32,
    pub conflicts: u32,
}
