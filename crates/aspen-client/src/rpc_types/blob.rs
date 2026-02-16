// Blob operation response types.

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddBlobResultResponse {
    pub success: bool,
    pub hash: Option<String>,
    #[serde(rename = "size")]
    pub size_bytes: Option<u64>,
    pub was_new: Option<bool>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlobResultResponse {
    pub found: bool,
    pub data: Option<Vec<u8>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasBlobResultResponse {
    pub exists: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlobTicketResultResponse {
    pub success: bool,
    pub ticket: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobListEntry {
    pub hash: String,
    #[serde(rename = "size")]
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListBlobsResultResponse {
    pub blobs: Vec<BlobListEntry>,
    pub count: u32,
    pub has_more: bool,
    pub continuation_token: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectBlobResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnprotectBlobResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteBlobResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadBlobResultResponse {
    pub success: bool,
    pub hash: Option<String>,
    #[serde(rename = "size")]
    pub size_bytes: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlobStatusResultResponse {
    pub found: bool,
    pub hash: Option<String>,
    #[serde(rename = "size")]
    pub size_bytes: Option<u64>,
    pub complete: Option<bool>,
    pub tags: Option<Vec<String>>,
    pub error: Option<String>,
}
