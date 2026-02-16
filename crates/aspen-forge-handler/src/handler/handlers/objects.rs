//! Git object operations (blob, tree, commit).

use aspen_client_api::ClientRpcResponse;

use super::ForgeNodeRef;

pub(crate) async fn handle_store_blob(
    forge_node: &ForgeNodeRef,
    _repo_id: String,
    content: Vec<u8>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeBlobResultResponse;

    let size = content.len() as u64;
    match forge_node.git.store_blob(content).await {
        Ok(hash) => Ok(ClientRpcResponse::ForgeBlobResult(ForgeBlobResultResponse {
            is_success: true,
            hash: Some(hash.to_hex().to_string()),
            content: None,
            size: Some(size),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::ForgeBlobResult(ForgeBlobResultResponse {
            is_success: false,
            hash: None,
            content: None,
            size: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_get_blob(forge_node: &ForgeNodeRef, hash: String) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeBlobResultResponse;

    let hash = match blake3::Hash::from_hex(&hash) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeBlobResult(ForgeBlobResultResponse {
                is_success: false,
                hash: None,
                content: None,
                size: None,
                error: Some(format!("Invalid hash: {}", e)),
            }));
        }
    };

    match forge_node.git.get_blob(&hash).await {
        Ok(content) => {
            let size = content.len() as u64;
            Ok(ClientRpcResponse::ForgeBlobResult(ForgeBlobResultResponse {
                is_success: true,
                hash: Some(hash.to_hex().to_string()),
                content: Some(content),
                size: Some(size),
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ForgeBlobResult(ForgeBlobResultResponse {
            is_success: false,
            hash: None,
            content: None,
            size: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_create_tree(
    forge_node: &ForgeNodeRef,
    _repo_id: String,
    entries_json: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeTreeEntry as RpcTreeEntry;
    use aspen_client_api::ForgeTreeResultResponse;
    use aspen_forge::TreeEntry;

    // Parse entries from JSON
    let parsed: Vec<RpcTreeEntry> = match serde_json::from_str(&entries_json) {
        Ok(e) => e,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
                is_success: false,
                hash: None,
                entries: None,
                error: Some(format!("Invalid entries JSON: {}", e)),
            }));
        }
    };

    // Convert to internal TreeEntry format
    let entries: Result<Vec<TreeEntry>, blake3::HexError> = parsed
        .iter()
        .map(|e| {
            let hash = blake3::Hash::from_hex(&e.hash)?;
            Ok(TreeEntry {
                mode: e.mode,
                name: e.name.clone(),
                hash: *hash.as_bytes(),
            })
        })
        .collect();

    let entries = match entries {
        Ok(e) => e,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
                is_success: false,
                hash: None,
                entries: None,
                error: Some(format!("Invalid entry hash: {}", e)),
            }));
        }
    };

    match forge_node.git.create_tree(&entries).await {
        Ok(hash) => Ok(ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
            is_success: true,
            hash: Some(hash.to_hex().to_string()),
            entries: Some(parsed),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
            is_success: false,
            hash: None,
            entries: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_get_tree(forge_node: &ForgeNodeRef, hash: String) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeTreeEntry as RpcTreeEntry;
    use aspen_client_api::ForgeTreeResultResponse;

    let hash = match blake3::Hash::from_hex(&hash) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
                is_success: false,
                hash: None,
                entries: None,
                error: Some(format!("Invalid hash: {}", e)),
            }));
        }
    };

    match forge_node.git.get_tree(&hash).await {
        Ok(tree) => {
            let entries: Vec<RpcTreeEntry> = tree
                .entries
                .iter()
                .map(|e| RpcTreeEntry {
                    mode: e.mode,
                    name: e.name.clone(),
                    hash: blake3::Hash::from_bytes(e.hash).to_hex().to_string(),
                })
                .collect();

            Ok(ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
                is_success: true,
                hash: Some(hash.to_hex().to_string()),
                entries: Some(entries),
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
            is_success: false,
            hash: None,
            entries: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_commit(
    forge_node: &ForgeNodeRef,
    _repo_id: String,
    tree: String,
    parents: Vec<String>,
    message: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeCommitInfo;
    use aspen_client_api::ForgeCommitResultResponse;

    let tree_hash = match blake3::Hash::from_hex(&tree) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
                is_success: false,
                commit: None,
                error: Some(format!("Invalid tree hash: {}", e)),
            }));
        }
    };

    let parent_hashes: Result<Vec<blake3::Hash>, _> = parents.iter().map(blake3::Hash::from_hex).collect();
    let parent_hashes = match parent_hashes {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
                is_success: false,
                commit: None,
                error: Some(format!("Invalid parent hash: {}", e)),
            }));
        }
    };

    match forge_node.git.commit(tree_hash, parent_hashes.clone(), &message).await {
        Ok(hash) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            Ok(ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
                is_success: true,
                commit: Some(ForgeCommitInfo {
                    hash: hash.to_hex().to_string(),
                    tree,
                    parents,
                    author_name: "anonymous".to_string(),
                    author_email: None,
                    author_key: Some(forge_node.public_key().to_string()),
                    message,
                    timestamp_ms: now,
                }),
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
            is_success: false,
            commit: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_get_commit(forge_node: &ForgeNodeRef, hash: String) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeCommitInfo;
    use aspen_client_api::ForgeCommitResultResponse;

    let hash = match blake3::Hash::from_hex(&hash) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
                is_success: false,
                commit: None,
                error: Some(format!("Invalid hash: {}", e)),
            }));
        }
    };

    match forge_node.git.get_commit(&hash).await {
        Ok(commit) => Ok(ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
            is_success: true,
            commit: Some(ForgeCommitInfo {
                hash: hash.to_hex().to_string(),
                tree: blake3::Hash::from_bytes(commit.tree).to_hex().to_string(),
                parents: commit.parents.iter().map(|p| blake3::Hash::from_bytes(*p).to_hex().to_string()).collect(),
                author_name: commit.author.name.clone(),
                author_email: Some(commit.author.email.clone()),
                author_key: commit.author.public_key.map(|k| hex::encode(k.as_bytes())),
                message: commit.message.clone(),
                timestamp_ms: commit.author.timestamp_ms,
            }),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
            is_success: false,
            commit: None,
            error: Some(e.to_string()),
        })),
    }
}
