// Forge response types (decentralized git).

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRepoInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub default_branch: String,
    pub delegates: Vec<String>,
    pub threshold: u32,
    pub created_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRepoResultResponse {
    pub success: bool,
    pub repo: Option<ForgeRepoInfo>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRepoListResultResponse {
    pub success: bool,
    pub repos: Vec<ForgeRepoInfo>,
    pub count: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeBlobResultResponse {
    pub success: bool,
    pub hash: Option<String>,
    pub content: Option<Vec<u8>>,
    pub size: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeTreeEntry {
    pub mode: u32,
    pub name: String,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeTreeResultResponse {
    pub success: bool,
    pub hash: Option<String>,
    pub entries: Option<Vec<ForgeTreeEntry>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeCommitInfo {
    pub hash: String,
    pub tree: String,
    pub parents: Vec<String>,
    pub author_name: String,
    pub author_email: Option<String>,
    pub author_key: Option<String>,
    pub message: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeCommitResultResponse {
    pub success: bool,
    pub commit: Option<ForgeCommitInfo>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeLogResultResponse {
    pub success: bool,
    pub commits: Vec<ForgeCommitInfo>,
    pub count: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRefInfo {
    pub name: String,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRefResultResponse {
    pub success: bool,
    pub found: bool,
    pub ref_info: Option<ForgeRefInfo>,
    pub previous_hash: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRefListResultResponse {
    pub success: bool,
    pub refs: Vec<ForgeRefInfo>,
    pub count: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeCommentInfo {
    pub hash: String,
    pub author: String,
    pub body: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeIssueInfo {
    pub id: String,
    pub title: String,
    pub body: String,
    pub state: String,
    pub labels: Vec<String>,
    pub comment_count: u32,
    pub assignees: Vec<String>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeIssueResultResponse {
    pub success: bool,
    pub issue: Option<ForgeIssueInfo>,
    pub comments: Option<Vec<ForgeCommentInfo>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeIssueListResultResponse {
    pub success: bool,
    pub issues: Vec<ForgeIssueInfo>,
    pub count: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgePatchRevision {
    pub hash: String,
    pub head: String,
    pub message: Option<String>,
    pub author: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgePatchApproval {
    pub author: String,
    pub commit: String,
    pub message: Option<String>,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgePatchInfo {
    pub id: String,
    pub title: String,
    pub description: String,
    pub state: String,
    pub base: String,
    pub head: String,
    pub labels: Vec<String>,
    pub revision_count: u32,
    pub approval_count: u32,
    pub assignees: Vec<String>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgePatchResultResponse {
    pub success: bool,
    pub patch: Option<ForgePatchInfo>,
    pub comments: Option<Vec<ForgeCommentInfo>>,
    pub revisions: Option<Vec<ForgePatchRevision>>,
    pub approvals: Option<Vec<ForgePatchApproval>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgePatchListResultResponse {
    pub success: bool,
    pub patches: Vec<ForgePatchInfo>,
    pub count: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeOperationResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeKeyResultResponse {
    pub success: bool,
    pub public_key: Option<String>,
    pub secret_key: Option<String>,
    pub error: Option<String>,
}

// Git Bridge types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgeObject {
    pub sha1: String,
    pub object_type: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgeRefUpdate {
    pub ref_name: String,
    pub old_sha1: String,
    pub new_sha1: String,
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgeRefInfo {
    pub ref_name: String,
    pub sha1: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgeListRefsResponse {
    pub success: bool,
    pub refs: Vec<GitBridgeRefInfo>,
    pub head: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgeFetchResponse {
    pub success: bool,
    pub objects: Vec<GitBridgeObject>,
    pub skipped: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgePushResponse {
    pub success: bool,
    pub objects_imported: usize,
    pub objects_skipped: usize,
    pub ref_results: Vec<GitBridgeRefResult>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgeRefResult {
    pub ref_name: String,
    pub success: bool,
    pub error: Option<String>,
}
