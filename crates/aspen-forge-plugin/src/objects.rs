//! Git object handlers: blob, tree, commit, log.

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::ForgeBlobResultResponse;
use aspen_client_api::ForgeCommitInfo;
use aspen_client_api::ForgeCommitResultResponse;
use aspen_client_api::ForgeLogResultResponse;
use aspen_client_api::ForgeTreeEntry as ApiTreeEntry;
use aspen_client_api::ForgeTreeResultResponse;

use crate::kv;
use crate::signing;
use crate::types::Author;
use crate::types::BlobObject;
use crate::types::CommitObject;
use crate::types::GitObject;
use crate::types::SignedObject;
use crate::types::TreeEntry;
use crate::types::TreeObject;

/// Maximum commits returned by log.
const MAX_LOG_COMMITS: u32 = 500;

// ============================================================================
// KV key helpers (refs are used by log to find HEAD)
// ============================================================================

fn ref_key(repo_id_hex: &str, ref_name: &str) -> String {
    format!("forge:refs:{repo_id_hex}:{ref_name}")
}

// ============================================================================
// Blob handlers
// ============================================================================

pub fn handle_store_blob(_repo_id: String, content: Vec<u8>) -> ClientRpcResponse {
    let size = content.len() as u64;
    let signed = signing::sign_object(&GitObject::Blob(BlobObject { content }));

    let serialized = match serde_json::to_vec(&signed) {
        Ok(b) => b,
        Err(e) => {
            return ClientRpcResponse::ForgeBlobResult(ForgeBlobResultResponse {
                is_success: false,
                hash: None,
                content: None,
                size: None,
                error: Some(format!("failed to serialize blob: {e}")),
            });
        }
    };

    match kv::blob_put(&serialized) {
        Ok(hash) => ClientRpcResponse::ForgeBlobResult(ForgeBlobResultResponse {
            is_success: true,
            hash: Some(hash),
            content: None,
            size: Some(size),
            error: None,
        }),
        Err(e) => ClientRpcResponse::ForgeBlobResult(ForgeBlobResultResponse {
            is_success: false,
            hash: None,
            content: None,
            size: None,
            error: Some(format!("failed to store blob: {e}")),
        }),
    }
}

pub fn handle_get_blob(hash: String) -> ClientRpcResponse {
    let data = match kv::blob_get(&hash) {
        Some(d) => d,
        None => {
            return ClientRpcResponse::ForgeBlobResult(ForgeBlobResultResponse {
                is_success: true,
                hash: Some(hash),
                content: None,
                size: None,
                error: None,
            });
        }
    };

    let signed: SignedObject<GitObject> = match serde_json::from_slice(&data) {
        Ok(s) => s,
        Err(e) => {
            return ClientRpcResponse::ForgeBlobResult(ForgeBlobResultResponse {
                is_success: false,
                hash: Some(hash),
                content: None,
                size: None,
                error: Some(format!("failed to deserialize blob: {e}")),
            });
        }
    };

    match signed.payload {
        GitObject::Blob(blob) => {
            let size = blob.content.len() as u64;
            ClientRpcResponse::ForgeBlobResult(ForgeBlobResultResponse {
                is_success: true,
                hash: Some(hash),
                content: Some(blob.content),
                size: Some(size),
                error: None,
            })
        }
        _ => ClientRpcResponse::ForgeBlobResult(ForgeBlobResultResponse {
            is_success: false,
            hash: Some(hash),
            content: None,
            size: None,
            error: Some("object is not a blob".to_string()),
        }),
    }
}

// ============================================================================
// Tree handlers
// ============================================================================

pub fn handle_create_tree(_repo_id: String, entries_json: String) -> ClientRpcResponse {
    let api_entries: Vec<ApiTreeEntry> = match serde_json::from_str(&entries_json) {
        Ok(e) => e,
        Err(e) => {
            return ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
                is_success: false,
                hash: None,
                entries: None,
                error: Some(format!("failed to parse entries_json: {e}")),
            });
        }
    };

    // Convert API entries to internal format
    let entries: Vec<TreeEntry> = api_entries
        .iter()
        .map(|e| TreeEntry {
            mode: e.mode,
            name: e.name.clone(),
            hash: e.hash.clone(),
        })
        .collect();

    let tree = GitObject::Tree(TreeObject { entries });
    let signed = signing::sign_object(&tree);

    let serialized = match serde_json::to_vec(&signed) {
        Ok(b) => b,
        Err(e) => {
            return ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
                is_success: false,
                hash: None,
                entries: None,
                error: Some(format!("failed to serialize tree: {e}")),
            });
        }
    };

    match kv::blob_put(&serialized) {
        Ok(hash) => ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
            is_success: true,
            hash: Some(hash),
            entries: Some(api_entries),
            error: None,
        }),
        Err(e) => ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
            is_success: false,
            hash: None,
            entries: None,
            error: Some(format!("failed to store tree: {e}")),
        }),
    }
}

pub fn handle_get_tree(hash: String) -> ClientRpcResponse {
    let data = match kv::blob_get(&hash) {
        Some(d) => d,
        None => {
            return ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
                is_success: true,
                hash: Some(hash),
                entries: None,
                error: None,
            });
        }
    };

    let signed: SignedObject<GitObject> = match serde_json::from_slice(&data) {
        Ok(s) => s,
        Err(e) => {
            return ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
                is_success: false,
                hash: Some(hash),
                entries: None,
                error: Some(format!("failed to deserialize tree: {e}")),
            });
        }
    };

    match signed.payload {
        GitObject::Tree(tree) => {
            let api_entries: Vec<ApiTreeEntry> = tree
                .entries
                .iter()
                .map(|e| ApiTreeEntry {
                    mode: e.mode,
                    name: e.name.clone(),
                    hash: e.hash.clone(),
                })
                .collect();
            ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
                is_success: true,
                hash: Some(hash),
                entries: Some(api_entries),
                error: None,
            })
        }
        _ => ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
            is_success: false,
            hash: Some(hash),
            entries: None,
            error: Some("object is not a tree".to_string()),
        }),
    }
}

// ============================================================================
// Commit handlers
// ============================================================================

fn build_commit_info(hash: &str, signed: &SignedObject<GitObject>) -> Option<ForgeCommitInfo> {
    match &signed.payload {
        GitObject::Commit(commit) => Some(ForgeCommitInfo {
            hash: hash.to_string(),
            tree: commit.tree.clone(),
            parents: commit.parents.clone(),
            author_name: commit.author.name.clone(),
            author_email: Some(commit.author.email.clone()),
            author_key: commit.author.public_key.clone(),
            message: commit.message.clone(),
            timestamp_ms: commit.author.timestamp_ms,
        }),
        _ => None,
    }
}

pub fn handle_commit(_repo_id: String, tree: String, parents: Vec<String>, message: String) -> ClientRpcResponse {
    let author_hex = kv::public_key();
    let now = kv::hlc_now();

    let author = Author {
        name: author_hex.clone(),
        email: String::new(),
        public_key: Some(author_hex),
        timestamp_ms: now,
        timezone: "+0000".to_string(),
    };

    let commit_obj = GitObject::Commit(Box::new(CommitObject {
        tree,
        parents,
        author: author.clone(),
        committer: author,
        message,
    }));

    let signed = signing::sign_object(&commit_obj);

    let serialized = match serde_json::to_vec(&signed) {
        Ok(b) => b,
        Err(e) => {
            return ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
                is_success: false,
                commit: None,
                error: Some(format!("failed to serialize commit: {e}")),
            });
        }
    };

    match kv::blob_put(&serialized) {
        Ok(hash) => {
            let info = build_commit_info(&hash, &signed);
            ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
                is_success: true,
                commit: info,
                error: None,
            })
        }
        Err(e) => ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
            is_success: false,
            commit: None,
            error: Some(format!("failed to store commit: {e}")),
        }),
    }
}

pub fn handle_get_commit(hash: String) -> ClientRpcResponse {
    let data = match kv::blob_get(&hash) {
        Some(d) => d,
        None => {
            return ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
                is_success: true,
                commit: None,
                error: None,
            });
        }
    };

    let signed: SignedObject<GitObject> = match serde_json::from_slice(&data) {
        Ok(s) => s,
        Err(e) => {
            return ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
                is_success: false,
                commit: None,
                error: Some(format!("failed to deserialize commit: {e}")),
            });
        }
    };

    let info = build_commit_info(&hash, &signed);
    if info.is_some() {
        ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
            is_success: true,
            commit: info,
            error: None,
        })
    } else {
        ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
            is_success: false,
            commit: None,
            error: Some("object is not a commit".to_string()),
        })
    }
}

// ============================================================================
// Log handler
// ============================================================================

pub fn handle_log(repo_id: String, ref_name: Option<String>, limit: Option<u32>) -> ClientRpcResponse {
    let max_commits = limit.unwrap_or(MAX_LOG_COMMITS).min(MAX_LOG_COMMITS);
    let resolved_ref = ref_name.unwrap_or_else(|| "heads/main".to_string());

    // Resolve ref to commit hash
    let ref_key = ref_key(&repo_id, &resolved_ref);
    let start_hash = match kv::kv_get(&ref_key) {
        Some(bytes) => match String::from_utf8(bytes) {
            Ok(h) => h,
            Err(_) => {
                return ClientRpcResponse::ForgeLogResult(ForgeLogResultResponse {
                    is_success: false,
                    commits: Vec::new(),
                    count: 0,
                    error: Some("ref value is not valid UTF-8".to_string()),
                });
            }
        },
        None => {
            return ClientRpcResponse::ForgeLogResult(ForgeLogResultResponse {
                is_success: true,
                commits: Vec::new(),
                count: 0,
                error: None,
            });
        }
    };

    // Walk the parent chain
    let mut commits = Vec::new();
    let mut current_hash = start_hash;

    while commits.len() < max_commits as usize {
        let data = match kv::blob_get(&current_hash) {
            Some(d) => d,
            None => break,
        };

        let signed: SignedObject<GitObject> = match serde_json::from_slice(&data) {
            Ok(s) => s,
            Err(_) => break,
        };

        let info = match build_commit_info(&current_hash, &signed) {
            Some(i) => i,
            None => break,
        };

        // Get the first parent for linear traversal
        let next_parent = match &signed.payload {
            GitObject::Commit(c) => c.parents.first().cloned(),
            _ => None,
        };

        commits.push(info);

        match next_parent {
            Some(parent) => current_hash = parent,
            None => break,
        }
    }

    let count = commits.len() as u32;
    ClientRpcResponse::ForgeLogResult(ForgeLogResultResponse {
        is_success: true,
        commits,
        count,
        error: None,
    })
}
