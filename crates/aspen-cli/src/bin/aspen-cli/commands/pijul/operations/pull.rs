//! Pull operation handler.

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

use super::super::PullArgs;
use super::super::output::PijulPullOutput;
use super::super::repo_cache_dir;
use crate::client::AspenClient;
use crate::output::print_output;
use crate::output::print_success;

/// Pull changes from the cluster to local pristine.
pub(in super::super) async fn pijul_pull(client: &AspenClient, args: PullArgs, json: bool) -> Result<()> {
    use tracing::info;

    // Parse repo ID
    let repo_id = RepoId::from_hex(&args.repo_id).context("invalid repository ID format")?;

    // Determine cache directory
    let cache_dir = match args.data_dir {
        Some(dir) => dir,
        None => repo_cache_dir(&repo_id).context("failed to determine cache directory")?,
    };

    // Create cache directory if it doesn't exist
    std::fs::create_dir_all(&cache_dir).context("failed to create cache directory")?;

    info!(cache_dir = %cache_dir.display(), "using cache directory");

    // Get channels to pull
    let channels = if let Some(ref channel) = args.channel {
        vec![channel.clone()]
    } else {
        // Fetch all channels from the cluster
        let response = client
            .send(ClientRpcRequest::PijulChannelList {
                repo_id: args.repo_id.clone(),
            })
            .await?;

        match response {
            ClientRpcResponse::PijulChannelListResult(result) => result.channels.into_iter().map(|c| c.name).collect(),
            ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
            _ => anyhow::bail!("unexpected response type"),
        }
    };

    if channels.is_empty() {
        print_success("No channels to pull", json);
        return Ok(());
    }

    // Create local pristine manager
    let pristine_mgr = PristineManager::new(&cache_dir);
    let pristine = pristine_mgr.open_or_create(&repo_id).context("failed to open/create local pristine")?;

    // Create temporary blob store for fetching changes
    let temp_blobs = Arc::new(InMemoryBlobStore::new());
    let change_store = Arc::new(AspenChangeStore::new(temp_blobs.clone()));
    let change_dir = ChangeDirectory::new(&cache_dir, repo_id, change_store.clone());

    // Ensure change directory exists
    change_dir.ensure_dir().context("failed to create change directory")?;

    let mut total_fetched = 0u32;
    let mut total_applied = 0u32;
    let mut total_conflicts = 0u32;
    let mut all_up_to_date = true;

    // Create the applicator once for all channels
    use aspen_pijul::ChangeApplicator;
    let applicator = ChangeApplicator::new(pristine.clone(), change_dir.clone());

    // Pull each channel
    for channel in &channels {
        info!(channel = %channel, "pulling channel");

        // Get the cluster's change log
        let log_response = client
            .send(ClientRpcRequest::PijulLog {
                repo_id: args.repo_id.clone(),
                channel: channel.clone(),
                limit: 10_000,
            })
            .await?;

        let cluster_log = match log_response {
            ClientRpcResponse::PijulLogResult(result) => result.entries,
            ClientRpcResponse::Error(e) => {
                tracing::warn!(channel = %channel, error = %e.message, "failed to fetch log");
                continue;
            }
            _ => continue,
        };

        if cluster_log.is_empty() {
            info!(channel = %channel, "channel is empty");
            continue;
        }

        // For each change in the log, fetch if missing
        for entry in &cluster_log {
            let change_hash = aspen_pijul::ChangeHash::from_hex(&entry.change_hash).context("invalid change hash")?;

            // Check if we already have this change locally
            let change_path = change_dir.change_path(&change_hash);
            if change_path.exists() {
                continue;
            }

            // Fetch change from cluster blob store
            let blob_response = client
                .send(ClientRpcRequest::GetBlob {
                    hash: entry.change_hash.clone(),
                })
                .await;

            match blob_response {
                Ok(ClientRpcResponse::GetBlobResult(blob_result)) => {
                    if let Some(data) = blob_result.data {
                        // Store locally
                        change_store.store_change(&data).await.context("failed to store change locally")?;

                        // Write to change directory for libpijul
                        change_dir.ensure_dir()?;
                        std::fs::write(&change_path, &data).context("failed to write change file")?;

                        total_fetched += 1;
                        all_up_to_date = false;
                        info!(hash = %change_hash, "fetched change");
                    }
                }
                Ok(ClientRpcResponse::Error(e)) => {
                    tracing::warn!(hash = %change_hash, error = %e.message, "failed to fetch change");
                }
                _ => {}
            }
        }

        // Apply changes to pristine
        for entry in &cluster_log {
            let change_hash = aspen_pijul::ChangeHash::from_hex(&entry.change_hash)?;
            let change_path = change_dir.change_path(&change_hash);

            if !change_path.exists() {
                continue;
            }

            match applicator.apply_local(channel, &change_hash) {
                Ok(_) => {
                    total_applied += 1;
                    all_up_to_date = false;
                }
                Err(e) => {
                    // Change might already be applied, or there's a conflict
                    tracing::debug!(hash = %change_hash, error = %e, "apply result");
                }
            }
        }

        // Check for conflicts after applying all changes to this channel
        match applicator.check_conflicts(channel) {
            Ok(conflicts) => {
                total_conflicts += conflicts;
                if conflicts > 0 {
                    tracing::warn!(channel = %channel, conflicts = conflicts, "conflicts detected");
                }
            }
            Err(e) => {
                tracing::debug!(channel = %channel, error = %e, "failed to check conflicts");
            }
        }

        info!(channel = %channel, "channel pull complete");
    }

    // Output result
    let output = PijulPullOutput {
        repo_id: args.repo_id,
        channel: args.channel,
        changes_fetched: total_fetched,
        changes_applied: total_applied,
        already_up_to_date: all_up_to_date && total_fetched == 0,
        conflicts: total_conflicts,
        cache_dir: cache_dir.display().to_string(),
    };
    print_output(&output, json);

    Ok(())
}
