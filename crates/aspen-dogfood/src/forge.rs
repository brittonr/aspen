//! Forge operations — create repo and push source via git-remote-aspen.

use std::time::Duration;

use aspen_client::AspenClient;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use tracing::info;

use crate::RunConfig;
use crate::error::DogfoodResult;
use crate::error::ForgeSnafu;
use crate::error::GitPushSnafu;

/// Ensure a Forge repository exists, creating it if needed.
/// Returns the hex-encoded repo ID (needed for the aspen:// URL).
pub async fn ensure_repo_exists(ticket: &str, repo_name: &str) -> DogfoodResult<String> {
    let client = connect(ticket).await?;
    let result = async {
        // Check if repo already exists by listing repos
        let resp = client
            .send(ClientRpcRequest::ForgeListRepos {
                limit: Some(100),
                offset: None,
            })
            .await
            .map_err(|e| crate::error::DogfoodError::ClientRpc {
                operation: "ForgeListRepos".to_string(),
                target: crate::cluster::ticket_preview(ticket),
                source: e,
            })?;

        let already_exists = match &resp {
            ClientRpcResponse::ForgeRepoListResult(list) => list.repos.iter().any(|r| r.name == repo_name),
            _ => false,
        };

        if already_exists {
            // Extract existing repo ID
            if let ClientRpcResponse::ForgeRepoListResult(list) = &resp
                && let Some(repo) = list.repos.iter().find(|r| r.name == repo_name)
            {
                info!("  repo '{repo_name}' already exists (id: {})", &repo.id[..16]);
                return Ok(repo.id.clone());
            }
            // Shouldn't happen, but fall through to create
        }

        // Create the repo
        info!("  creating repo '{repo_name}'...");
        let create_resp = client
            .send(ClientRpcRequest::ForgeCreateRepo {
                name: repo_name.to_string(),
                description: Some("Aspen self-hosted source".to_string()),
                default_branch: Some("main".to_string()),
            })
            .await
            .map_err(|e| crate::error::DogfoodError::ClientRpc {
                operation: "ForgeCreateRepo".to_string(),
                target: crate::cluster::ticket_preview(ticket),
                source: e,
            })?;

        match &create_resp {
            ClientRpcResponse::ForgeRepoResult(r) if r.is_success => {
                let repo_id = r.repo.as_ref().map(|r| r.id.clone()).unwrap_or_default();
                info!("  repo created (id: {})", &repo_id[..repo_id.len().min(16)]);
                Ok(repo_id)
            }
            ClientRpcResponse::ForgeRepoResult(r) => ForgeSnafu {
                operation: "create repo",
                reason: r.error.clone().unwrap_or_else(|| "unknown error".to_string()),
            }
            .fail(),
            other => ForgeSnafu {
                operation: "create repo",
                reason: format!("unexpected response: {other:?}"),
            }
            .fail(),
        }
    }
    .await;
    client.shutdown().await;
    result
}

/// Register `CiWatchRepo` so auto-triggered CI fires on push.
///
/// The old dogfood-local.sh did this (`cli ci watch $repo_id`) before
/// every `git push`. Without it, the push-triggered CI path is not
/// exercised and the orchestrator falls back to manual trigger after
/// a 120s wait.
pub async fn watch_repo(ticket: &str, repo_id: &str) -> DogfoodResult<()> {
    let client = connect(ticket).await?;
    let result = async {
        let resp = client
            .send(ClientRpcRequest::CiWatchRepo {
                repo_id: repo_id.to_string(),
            })
            .await
            .map_err(|e| crate::error::DogfoodError::ClientRpc {
                operation: "CiWatchRepo".to_string(),
                target: crate::cluster::ticket_preview(ticket),
                source: e,
            })?;

        match resp {
            ClientRpcResponse::CiWatchRepoResult(r) if r.is_success => {
                info!("  CI watch registered for repo {}", &repo_id[..repo_id.len().min(16)]);
            }
            ClientRpcResponse::CiWatchRepoResult(r) => {
                // Non-fatal: watch may already be active, or CI may not be enabled.
                tracing::warn!(
                    "  CI watch returned error (continuing): {}",
                    r.error.unwrap_or_else(|| "unknown".to_string())
                );
            }
            _ => {
                tracing::warn!("  unexpected CiWatchRepo response (continuing)");
            }
        }

        Ok(())
    }
    .await;
    client.shutdown().await;
    result
}

/// Push workspace source to the Forge repo via `git push` with git-remote-aspen.
pub async fn git_push(config: &RunConfig, ticket: &str, repo_id: &str) -> DogfoodResult<()> {
    let remote_url = format!("aspen://{ticket}/{repo_id}");

    // Configure the remote (idempotent)
    let _ = tokio::process::Command::new("git")
        .args(["remote", "remove", "aspen-dogfood"])
        .current_dir(&config.project_dir)
        .output()
        .await;

    let add_output = tokio::process::Command::new("git")
        .args(["remote", "add", "aspen-dogfood", &remote_url])
        .current_dir(&config.project_dir)
        .output()
        .await
        .map_err(|e| crate::error::DogfoodError::ProcessSpawn {
            binary: "git remote add".to_string(),
            source: e,
        })?;

    if !add_output.status.success() {
        // Remote may already exist, try set-url
        let _ = tokio::process::Command::new("git")
            .args(["remote", "set-url", "aspen-dogfood", &remote_url])
            .current_dir(&config.project_dir)
            .output()
            .await;
    }

    // Push to the forge remote
    info!("  git push aspen-dogfood main...");
    let push_output = tokio::process::Command::new("git")
        .args(["push", "aspen-dogfood", "HEAD:refs/heads/main", "--force"])
        .current_dir(&config.project_dir)
        .env("PATH", augmented_path(&config.git_remote_aspen_bin))
        .output()
        .await
        .map_err(|e| crate::error::DogfoodError::ProcessSpawn {
            binary: "git push".to_string(),
            source: e,
        })?;

    if !push_output.status.success() {
        let stderr = String::from_utf8_lossy(&push_output.stderr).to_string();
        return GitPushSnafu {
            exit_code: push_output.status.code().unwrap_or(-1),
            stderr,
        }
        .fail();
    }

    Ok(())
}

/// Connect an `AspenClient` from a ticket string.
async fn connect(ticket: &str) -> DogfoodResult<AspenClient> {
    AspenClient::connect(ticket, Duration::from_secs(10), None).await.map_err(|e| {
        crate::error::DogfoodError::ClientRpc {
            operation: "connect".to_string(),
            target: crate::cluster::ticket_preview(ticket),
            source: e,
        }
    })
}

/// Build a PATH that includes the directory containing git-remote-aspen.
fn augmented_path(git_remote_bin: &str) -> String {
    let base = std::env::var("PATH").unwrap_or_default();
    if let Some(parent) = std::path::Path::new(git_remote_bin).parent() {
        format!("{}:{base}", parent.display())
    } else {
        base
    }
}
