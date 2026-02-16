//! History (log) operations.

use aspen_client_api::ClientRpcResponse;

use super::ForgeNodeRef;

pub(crate) async fn handle_log(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    ref_name: Option<String>,
    limit: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeCommitInfo;
    use aspen_client_api::ForgeLogResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeLogResult(ForgeLogResultResponse {
                is_success: false,
                commits: vec![],
                count: 0,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    let ref_name = ref_name.unwrap_or_else(|| "heads/main".to_string());
    let limit = limit.unwrap_or(50).min(1000);

    // Get the ref
    let start_hash = match forge_node.refs.get(&repo_id, &ref_name).await {
        Ok(Some(h)) => h,
        Ok(None) => {
            return Ok(ClientRpcResponse::ForgeLogResult(ForgeLogResultResponse {
                is_success: true,
                commits: vec![],
                count: 0,
                error: None,
            }));
        }
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeLogResult(ForgeLogResultResponse {
                is_success: false,
                commits: vec![],
                count: 0,
                error: Some(e.to_string()),
            }));
        }
    };

    // Walk commits
    let mut commits = Vec::new();
    let mut queue = vec![start_hash];
    let mut seen = std::collections::HashSet::new();

    while let Some(hash) = queue.pop() {
        if commits.len() >= limit as usize {
            break;
        }
        if seen.contains(&hash) {
            continue;
        }
        seen.insert(hash);

        match forge_node.git.get_commit(&hash).await {
            Ok(commit) => {
                commits.push(ForgeCommitInfo {
                    hash: hash.to_hex().to_string(),
                    tree: blake3::Hash::from_bytes(commit.tree).to_hex().to_string(),
                    parents: commit.parents.iter().map(|p| blake3::Hash::from_bytes(*p).to_hex().to_string()).collect(),
                    author_name: commit.author.name.clone(),
                    author_email: Some(commit.author.email.clone()),
                    author_key: commit.author.public_key.map(|k| hex::encode(k.as_bytes())),
                    message: commit.message.clone(),
                    timestamp_ms: commit.author.timestamp_ms,
                });

                // Add parents to queue
                for parent in &commit.parents {
                    queue.push(blake3::Hash::from_bytes(*parent));
                }
            }
            Err(_) => break,
        }
    }

    let count = commits.len() as u32;
    Ok(ClientRpcResponse::ForgeLogResult(ForgeLogResultResponse {
        is_success: true,
        commits,
        count,
        error: None,
    }))
}
