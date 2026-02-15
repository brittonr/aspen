// Docs operation response types.

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsSetResultResponse {
    pub success: bool,
    pub key: Option<String>,
    pub size: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsGetResultResponse {
    pub found: bool,
    pub value: Option<Vec<u8>>,
    pub size: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsDeleteResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsListEntry {
    pub key: String,
    pub size: u64,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsListResultResponse {
    pub entries: Vec<DocsListEntry>,
    pub count: u32,
    pub has_more: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsStatusResultResponse {
    #[serde(rename = "enabled")]
    pub is_enabled: bool,
    pub namespace_id: Option<String>,
    pub author_id: Option<String>,
    pub entry_count: Option<u64>,
    pub replica_open: Option<bool>,
    pub error: Option<String>,
}
