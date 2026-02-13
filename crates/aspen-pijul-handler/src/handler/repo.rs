//! Repository operations: repo_init, repo_list, repo_info.

use aspen_client_api::ClientRpcResponse;

use super::PijulStoreRef;

/// Handle PijulRepoInit request.
pub(crate) async fn handle_repo_init(
    pijul_store: &PijulStoreRef,
    name: String,
    description: Option<String>,
    default_channel: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ErrorResponse;
    use aspen_client_api::PijulRepoResponse;
    use aspen_pijul::types::PijulRepoIdentity;

    // Create identity with no delegates (self-signed)
    let mut identity = PijulRepoIdentity::new(&name, vec![]).with_default_channel(&default_channel);

    if let Some(desc) = description {
        identity = identity.with_description(desc);
    }

    match pijul_store.create_repo(identity.clone()).await {
        Ok(repo_id) => {
            // Count channels (should be 1 for the default channel)
            let channel_count = pijul_store.list_channels(&repo_id).await.map(|c| c.len() as u32).unwrap_or(1);

            Ok(ClientRpcResponse::PijulRepoResult(PijulRepoResponse {
                id: repo_id.to_hex(),
                name: identity.name,
                description: identity.description,
                default_channel: identity.default_channel,
                channel_count,
                created_at_ms: identity.created_at_ms,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "REPO_CREATE_FAILED".to_string(),
            message: format!("{}", e),
        })),
    }
}

/// Handle PijulRepoList request.
pub(crate) async fn handle_repo_list(pijul_store: &PijulStoreRef, limit: u32) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ErrorResponse;
    use aspen_client_api::PijulRepoListResponse;
    use aspen_client_api::PijulRepoResponse;

    // Tiger Style: cap limit to 1000
    let limit = limit.min(1000);

    match pijul_store.list_repos(limit).await {
        Ok(repos) => {
            let count = repos.len() as u32;
            let mut response_repos = Vec::with_capacity(repos.len());

            for (repo_id, identity) in repos {
                let channel_count = pijul_store.list_channels(&repo_id).await.map(|c| c.len() as u32).unwrap_or(0);

                response_repos.push(PijulRepoResponse {
                    id: repo_id.to_hex(),
                    name: identity.name,
                    description: identity.description,
                    default_channel: identity.default_channel,
                    channel_count,
                    created_at_ms: identity.created_at_ms,
                });
            }

            Ok(ClientRpcResponse::PijulRepoListResult(PijulRepoListResponse {
                repos: response_repos,
                count,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "REPO_LIST_FAILED".to_string(),
            message: format!("{}", e),
        })),
    }
}

/// Handle PijulRepoInfo request.
pub(crate) async fn handle_repo_info(
    pijul_store: &PijulStoreRef,
    repo_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ErrorResponse;
    use aspen_client_api::PijulRepoResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "INVALID_REPO_ID".to_string(),
                message: format!("Invalid repo ID: {}", e),
            }));
        }
    };

    match pijul_store.get_repo(&repo_id).await {
        Ok(Some(identity)) => {
            let channel_count = pijul_store.list_channels(&repo_id).await.map(|c| c.len() as u32).unwrap_or(0);

            Ok(ClientRpcResponse::PijulRepoResult(PijulRepoResponse {
                id: repo_id.to_hex(),
                name: identity.name,
                description: identity.description,
                default_channel: identity.default_channel,
                channel_count,
                created_at_ms: identity.created_at_ms,
            }))
        }
        Ok(None) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "REPO_NOT_FOUND".to_string(),
            message: format!("Repository not found: {}", repo_id),
        })),
        Err(e) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "REPO_INFO_FAILED".to_string(),
            message: format!("{}", e),
        })),
    }
}
