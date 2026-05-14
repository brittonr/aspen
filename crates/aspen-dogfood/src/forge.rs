//! Forge operations — create repo and push source via git-remote-aspen.

use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use aspen_client::AspenClient;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use tokio::time::timeout;
use tracing::info;

use crate::RunConfig;
use crate::error::DogfoodResult;
use crate::error::ForgeSnafu;
use crate::error::GitPushSnafu;

/// Find a Forge repo by name.
pub async fn lookup_repo_id(ticket: &str, repo_name: &str) -> DogfoodResult<Option<String>> {
    let client = connect(ticket).await?;
    let result = lookup_repo_id_with_client(&client, repo_name, ticket).await;
    client.shutdown().await;
    result
}

/// Find a Forge repo by name using an existing client.
pub(crate) async fn lookup_repo_id_with_client(
    client: &AspenClient,
    repo_name: &str,
    ticket: &str,
) -> DogfoodResult<Option<String>> {
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

    match resp {
        ClientRpcResponse::ForgeRepoListResult(list) => {
            Ok(list.repos.iter().find(|repo| repo.name == repo_name).map(|repo| repo.id.clone()))
        }
        other => ForgeSnafu {
            operation: "list repos",
            reason: format!("unexpected response: {other:?}"),
        }
        .fail(),
    }
}

/// Ensure a Forge repository exists, creating it if needed.
/// Returns the hex-encoded repo ID (needed for the aspen:// URL).
pub async fn ensure_repo_exists(ticket: &str, repo_name: &str) -> DogfoodResult<String> {
    let client = connect(ticket).await?;
    let result = async {
        if let Some(repo_id) = lookup_repo_id_with_client(&client, repo_name, ticket).await? {
            info!("  repo '{repo_name}' already exists (id: {})", &repo_id[..repo_id.len().min(16)]);
            return Ok(repo_id);
        }

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

/// Push a bounded current-source snapshot to the Forge repo via `git push` with git-remote-aspen.
pub async fn git_push(config: &RunConfig, ticket: &str, repo_id: &str) -> DogfoodResult<()> {
    let remote_url = format!("aspen://{ticket}/{repo_id}");
    let push_workspace = prepare_push_workspace(config).await?;

    // Configure the remote (idempotent)
    let _ = tokio::process::Command::new("git")
        .args(["remote", "remove", "aspen-dogfood"])
        .current_dir(&push_workspace)
        .output()
        .await;

    let add_output = tokio::process::Command::new("git")
        .args(["remote", "add", "aspen-dogfood", &remote_url])
        .current_dir(&push_workspace)
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
            .current_dir(&push_workspace)
            .output()
            .await;
    }

    // Push to the forge remote. Dogfood must exercise Aspen's Forge/CI boundary,
    // not arbitrary developer workstation pre-push hooks or full-history transfer;
    // those can obscure the current-source product boundary being proven.
    info!("  git push aspen-dogfood main from bounded source snapshot (--no-verify)...");
    let push_output = timeout(
        Duration::from_secs(config.git_push_timeout_secs),
        tokio::process::Command::new("git")
            .args(git_push_args())
            .current_dir(&push_workspace)
            .env("PATH", augmented_path(&config.git_remote_aspen_bin))
            .env("ASPEN_RELAY_DISABLED", "1")
            .env("ASPEN_DISCOVERY_DISABLED", "1")
            .env("GIT_TRANSPORT_HELPER_DEBUG", "1")
            .env("GIT_TRACE", "1")
            .kill_on_drop(true)
            .output(),
    )
    .await
    .map_err(|_| crate::error::DogfoodError::Timeout {
        operation: "git push aspen-dogfood".to_string(),
        timeout_secs: config.git_push_timeout_secs,
    })?
    .map_err(|e| crate::error::DogfoodError::ProcessSpawn {
        binary: "git push".to_string(),
        source: e,
    })?;

    if !push_output.status.success() {
        return GitPushSnafu {
            exit_code: push_output.status.code().unwrap_or(-1),
            stderr: process_output_detail(&push_output),
        }
        .fail();
    }

    Ok(())
}

async fn prepare_push_workspace(config: &RunConfig) -> DogfoodResult<PathBuf> {
    let snapshot_dir = push_snapshot_dir(config);
    let archive_path = push_snapshot_archive_path(config);

    reset_dir(&snapshot_dir).await?;
    if let Some(parent) = archive_path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|source| crate::error::DogfoodError::ProcessSpawn {
            binary: format!("mkdir -p {}", parent.display()),
            source,
        })?;
    }

    let source_commit = git_stdout(&config.project_dir, &["rev-parse", "HEAD"], "git rev-parse HEAD").await?;
    run_process(
        "git",
        &[
            "archive",
            "HEAD",
            "--format=tar",
            "--output",
            &archive_path.display().to_string(),
        ],
        &config.project_dir,
        "git archive HEAD",
    )
    .await?;

    tokio::fs::create_dir_all(&snapshot_dir)
        .await
        .map_err(|source| crate::error::DogfoodError::ProcessSpawn {
            binary: format!("mkdir -p {}", snapshot_dir.display()),
            source,
        })?;
    run_process(
        "tar",
        &[
            "-xf",
            &archive_path.display().to_string(),
            "-C",
            &snapshot_dir.display().to_string(),
        ],
        ".",
        "tar extract dogfood source snapshot",
    )
    .await?;
    let _ = tokio::fs::remove_file(&archive_path).await;

    run_process("git", &["init", "-b", "main"], &snapshot_dir, "git init snapshot").await?;
    run_process("git", &["config", "user.name", "Aspen Dogfood"], &snapshot_dir, "git config snapshot user.name")
        .await?;
    run_process(
        "git",
        &["config", "user.email", "dogfood@aspen.local"],
        &snapshot_dir,
        "git config snapshot user.email",
    )
    .await?;
    run_process("git", &["add", "-f", "-A"], &snapshot_dir, "git add snapshot").await?;
    run_process(
        "git",
        &[
            "commit",
            "-m",
            "Dogfood source snapshot",
            "-m",
            &format!("Source-Commit: {source_commit}"),
        ],
        &snapshot_dir,
        "git commit snapshot",
    )
    .await?;

    info!("  prepared bounded source snapshot at {}", snapshot_dir.display());
    Ok(snapshot_dir)
}

async fn reset_dir(path: &Path) -> DogfoodResult<()> {
    match tokio::fs::remove_dir_all(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(crate::error::DogfoodError::ProcessSpawn {
            binary: format!("rm -rf {}", path.display()),
            source,
        }),
    }
}

async fn git_stdout(cwd: impl AsRef<Path>, args: &[&str], operation: &str) -> DogfoodResult<String> {
    let output = run_process_output("git", args, cwd, operation).await?;
    if !output.status.success() {
        return GitPushSnafu {
            exit_code: output.status.code().unwrap_or(-1),
            stderr: process_output_detail(&output),
        }
        .fail();
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

async fn run_process(binary: &str, args: &[&str], cwd: impl AsRef<Path>, operation: &str) -> DogfoodResult<()> {
    let output = run_process_output(binary, args, cwd, operation).await?;
    if output.status.success() {
        return Ok(());
    }
    GitPushSnafu {
        exit_code: output.status.code().unwrap_or(-1),
        stderr: process_output_detail(&output),
    }
    .fail()
}

async fn run_process_output(
    binary: &str,
    args: &[&str],
    cwd: impl AsRef<Path>,
    operation: &str,
) -> DogfoodResult<std::process::Output> {
    tokio::process::Command::new(binary).args(args).current_dir(cwd).output().await.map_err(|source| {
        crate::error::DogfoodError::ProcessSpawn {
            binary: operation.to_string(),
            source,
        }
    })
}

fn process_output_detail(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    crate::error::redact_credential_fragments(&format!("stderr:\n{stderr}\nstdout:\n{stdout}"))
}

pub(crate) fn push_snapshot_dir(config: &RunConfig) -> PathBuf {
    Path::new(&config.cluster_dir).join("source-snapshot")
}

pub(crate) fn push_snapshot_archive_path(config: &RunConfig) -> PathBuf {
    Path::new(&config.cluster_dir).join("source-snapshot.tar")
}

pub(crate) fn git_push_args() -> [&'static str; 5] {
    [
        "push",
        "--no-verify",
        "aspen-dogfood",
        "HEAD:refs/heads/main",
        "--force",
    ]
}

/// Connect an `AspenClient` from a ticket string.
async fn connect(ticket: &str) -> DogfoodResult<AspenClient> {
    AspenClient::connect_direct(ticket, Duration::from_secs(10), None).await.map_err(|e| {
        crate::error::DogfoodError::ClientRpc {
            operation: "connect".to_string(),
            target: crate::cluster::ticket_preview(ticket),
            source: e,
        }
    })
}

/// Build a PATH that includes the directory containing git-remote-aspen.
pub(crate) fn augmented_path(git_remote_bin: &str) -> String {
    let base = std::env::var("PATH").unwrap_or_default();
    if let Some(parent) = std::path::Path::new(git_remote_bin).parent() {
        format!("{}:{base}", parent.display())
    } else {
        base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dogfood_push_bypasses_local_git_hooks() {
        let args = git_push_args();

        assert_eq!(args[0], "push");
        assert!(args.contains(&"--no-verify"));
        assert!(args.contains(&"aspen-dogfood"));
        assert!(args.contains(&"HEAD:refs/heads/main"));
    }

    #[test]
    fn dogfood_push_uses_cluster_local_source_snapshot() {
        let config = test_config("/repo", "/tmp/aspen-dogfood-test");

        assert_eq!(push_snapshot_dir(&config), Path::new("/tmp/aspen-dogfood-test/source-snapshot"));
        assert_eq!(push_snapshot_archive_path(&config), Path::new("/tmp/aspen-dogfood-test/source-snapshot.tar"));
    }

    #[tokio::test]
    async fn prepare_push_workspace_snapshots_committed_tree_without_history_or_untracked_files() {
        let source = tempfile::tempdir().unwrap();
        git(source.path(), &["init", "-b", "main"]);
        git(source.path(), &["config", "user.name", "Test"]);
        git(source.path(), &["config", "user.email", "test@example.invalid"]);
        std::fs::write(source.path().join("tracked.txt"), "v1\n").unwrap();
        git(source.path(), &["add", "tracked.txt"]);
        git(source.path(), &["commit", "-m", "first"]);
        std::fs::write(source.path().join("tracked.txt"), "v2\n").unwrap();
        git(source.path(), &["commit", "-am", "second"]);
        std::fs::write(source.path().join("untracked.txt"), "do not include\n").unwrap();

        let cluster = tempfile::tempdir().unwrap();
        let config = test_config(source.path().to_str().unwrap(), cluster.path().to_str().unwrap());

        let snapshot = prepare_push_workspace(&config).await.unwrap();

        assert_eq!(std::fs::read_to_string(snapshot.join("tracked.txt")).unwrap(), "v2\n");
        assert!(!snapshot.join("untracked.txt").exists());
        assert_eq!(git_stdout_sync(&snapshot, &["rev-list", "--count", "HEAD"]), "1");
        assert_eq!(git_stdout_sync(&snapshot, &["status", "--short"]), "");
    }

    fn test_config(project_dir: &str, cluster_dir: &str) -> RunConfig {
        RunConfig {
            cluster_dir: cluster_dir.to_string(),
            federation: false,
            vm_ci: false,
            aspen_node_bin: "aspen-node".to_string(),
            git_remote_aspen_bin: "git-remote-aspen".to_string(),
            project_dir: project_dir.to_string(),
            nix_cache_gateway_bin: None,
            ci_timeout_secs: 60,
            git_push_timeout_secs: 60,
        }
    }

    fn git(cwd: &Path, args: &[&str]) {
        let output = std::process::Command::new("git").args(args).current_dir(cwd).output().unwrap();
        assert!(output.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&output.stderr));
    }

    fn git_stdout_sync(cwd: &Path, args: &[&str]) -> String {
        let output = std::process::Command::new("git").args(args).current_dir(cwd).output().unwrap();
        assert!(output.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&output.stderr));
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }
}
