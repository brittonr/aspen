// Blob operation response types.

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddBlobResultResponse {
    #[serde(rename = "success")]
    pub is_success: bool,
    pub hash: Option<String>,
    #[serde(rename = "size")]
    pub size_bytes: Option<u64>,
    pub was_new: Option<bool>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlobResultResponse {
    #[serde(rename = "found")]
    pub was_found: bool,
    pub data: Option<Vec<u8>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasBlobResultResponse {
    #[serde(rename = "exists")]
    pub does_exist: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlobTicketResultResponse {
    #[serde(rename = "success")]
    pub is_success: bool,
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
    #[serde(rename = "success")]
    pub is_success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnprotectBlobResultResponse {
    #[serde(rename = "success")]
    pub is_success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteBlobResultResponse {
    #[serde(rename = "success")]
    pub is_success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadBlobResultResponse {
    #[serde(rename = "success")]
    pub is_success: bool,
    pub hash: Option<String>,
    #[serde(rename = "size")]
    pub size_bytes: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlobStatusResultResponse {
    #[serde(rename = "found")]
    pub was_found: bool,
    pub hash: Option<String>,
    #[serde(rename = "size")]
    pub size_bytes: Option<u64>,
    #[serde(rename = "complete")]
    pub is_complete: Option<bool>,
    pub tags: Option<Vec<String>>,
    pub error: Option<String>,
}
