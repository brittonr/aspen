//! Push operation handler.

use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use aspen_blob::InMemoryBlobStore;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_forge::identity::RepoId;
use aspen_pijul::AspenChangeStore;
use aspen_pijul::ChangeDirectory;
use aspen_pijul::PristineManager;

use super::super::PushArgs;
use super::super::output::PijulPushOutput;
use super::super::repo_cache_dir;
use crate::client::AspenClient;
use crate::output::print_output;

/// Push local changes to the cluster.
pub(in super::super) async fn pijul_push(client: &AspenClient, args: PushArgs, json: bool) -> Result<()> {
    use aspen_pijul::ChangeApplicator;
    use tracing::info;

    // Parse repo ID
    let repo_id = RepoId::from_hex(&args.repo_id).context("invalid repository ID format")?;

    // Determine cache directory
    let cache_dir = args.data_dir.unwrap_or_else(|| repo_cache_dir(&repo_id).expect("failed to get cache dir"));

    // Check if cache exists
    if !cache_dir.exists() {
        anyhow::bail!(
            "Local cache not found at {}. Run 'pijul sync {}' or 'pijul record' first.",
            cache_dir.display(),
            args.repo_id
        );
    }

    info!(cache_dir = %cache_dir.display(), channel = %args.channel, "pushing to cluster");

    // Open local pristine
    let pristine_mgr = PristineManager::new(&cache_dir);
    let pristine = pristine_mgr.open(&repo_id).context("failed to open local pristine - run 'pijul sync' first")?;

    // Create change directory
    let temp_blobs = Arc::new(InMemoryBlobStore::new());
    let change_store = Arc::new(AspenChangeStore::new(temp_blobs.clone()));
    let change_dir = ChangeDirectory::new(&cache_dir, repo_id, change_store.clone());

    // Get local changes from pristine for this channel
    let applicator = ChangeApplicator::new(pristine.clone(), change_dir.clone());
    let local_changes = applicator.list_channel_changes(&args.channel).context("failed to list local changes")?;

    if local_changes.is_empty() {
        let output = PijulPushOutput {
            repo_id: args.repo_id,
            channel: args.channel,
            changes_pushed: 0,
            already_up_to_date: true,
            cache_dir: cache_dir.display().to_string(),
        };
        print_output(&output, json);
        return Ok(());
    }

    // Get the cluster's change log to find what we need to push
    let log_response = client
        .send(ClientRpcRequest::PijulLog {
            repo_id: args.repo_id.clone(),
            channel: args.channel.clone(),
            limit: 10_000,
        })
        .await?;

    let cluster_hashes: std::collections::HashSet<String> = match log_response {
        ClientRpcResponse::PijulLogResult(result) => result.entries.into_iter().map(|e| e.change_hash).collect(),
        ClientRpcResponse::Error(e) => {
            // Channel might not exist on cluster yet, that's fine
            tracing::debug!(error = %e.message, "cluster log fetch failed, assuming empty");
            std::collections::HashSet::new()
        }
        _ => std::collections::HashSet::new(),
    };

    // Find changes in local that are not in cluster
    let mut changes_to_push = Vec::new();
    for hash in &local_changes {
        let hash_str = hash.to_string();
        if !cluster_hashes.contains(&hash_str) {
            changes_to_push.push(*hash);
        }
    }

    if changes_to_push.is_empty() {
        let output = PijulPushOutput {
            repo_id: args.repo_id,
            channel: args.channel,
            changes_pushed: 0,
            already_up_to_date: true,
            cache_dir: cache_dir.display().to_string(),
        };
        print_output(&output, json);
        return Ok(());
    }

    info!(count = changes_to_push.len(), "pushing changes to cluster");

    let mut changes_pushed = 0u32;

    for hash in &changes_to_push {
        // Read the change from local storage
        let change_path = change_dir.change_path(hash);
        if !change_path.exists() {
            tracing::warn!(hash = %hash, "change file not found locally, skipping");
            continue;
        }

        let change_bytes = std::fs::read(&change_path).context("failed to read local change file")?;

        // Upload change to cluster blob store
        let blob_response = client
            .send(ClientRpcRequest::AddBlob {
                data: change_bytes.clone(),
                tag: Some(format!("pijul:{}:{}", args.repo_id, hash)),
            })
            .await
            .context("failed to upload change to cluster")?;

        match blob_response {
            ClientRpcResponse::AddBlobResult(_) => {
                info!(hash = %hash, "uploaded change to cluster blob store");
            }
            ClientRpcResponse::Error(e) => {
                anyhow::bail!("failed to upload change: {}: {}", e.code, e.message);
            }
            _ => anyhow::bail!("unexpected response from blob add"),
        }

        // Apply the change to update channel head
        let apply_response = client
            .send(ClientRpcRequest::PijulApply {
                repo_id: args.repo_id.clone(),
                channel: args.channel.clone(),
                change_hash: hash.to_string(),
            })
            .await
            .context("failed to apply change to cluster")?;

        match apply_response {
            ClientRpcResponse::PijulApplyResult(_) => {
                info!(hash = %hash, channel = %args.channel, "applied change via Raft");
                changes_pushed += 1;
            }
            ClientRpcResponse::Error(e) => {
                anyhow::bail!("failed to apply change: {}: {}", e.code, e.message);
            }
            _ => anyhow::bail!("unexpected response from apply"),
        }
    }

    // Output result
    let output = PijulPushOutput {
        repo_id: args.repo_id,
        channel: args.channel,
        changes_pushed,
        already_up_to_date: false,
        cache_dir: cache_dir.display().to_string(),
    };
    print_output(&output, json);

    Ok(())
}
