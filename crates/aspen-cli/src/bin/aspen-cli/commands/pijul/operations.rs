//! Remote operation handlers for Pijul commands.
//!
//! Repository, channel, change, sync, pull, push, archive, show, blame,
//! and diff operations that communicate with the cluster.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use aspen_blob::InMemoryBlobStore;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_forge::identity::RepoId;
use aspen_pijul::AspenChangeStore;
use aspen_pijul::ChangeDirectory;
use aspen_pijul::ChangeMetadata;
use aspen_pijul::ChangeRecorder;
use aspen_pijul::PijulAuthor;
use aspen_pijul::PristineManager;

use super::ApplyArgs;
use super::ArchiveArgs;
use super::BlameArgs;
use super::ChannelCreateArgs;
use super::ChannelDeleteArgs;
use super::ChannelForkArgs;
use super::ChannelInfoArgs;
use super::ChannelListArgs;
use super::CheckoutArgs;
use super::DiffArgs;
use super::LogArgs;
use super::PullArgs;
use super::PushArgs;
use super::RecordArgs;
use super::RepoInfoArgs;
use super::RepoInitArgs;
use super::RepoListArgs;
use super::ShowArgs;
use super::SyncArgs;
use super::UnrecordArgs;
use super::output::BlameEntry;
use super::output::DiffHunk;
use super::output::PijulApplyOutput;
use super::output::PijulArchiveOutput;
use super::output::PijulBlameOutput;
use super::output::PijulChannelListOutput;
use super::output::PijulChannelOutput;
use super::output::PijulCheckoutOutput;
use super::output::PijulDiffOutput;
use super::output::PijulLogEntry;
use super::output::PijulLogOutput;
use super::output::PijulPullOutput;
use super::output::PijulPushOutput;
use super::output::PijulRecordOutput;
use super::output::PijulRepoListOutput;
use super::output::PijulRepoOutput;
use super::output::PijulShowOutput;
use super::output::PijulSyncOutput;
use super::output::PijulUnrecordOutput;
use super::repo_cache_dir;
use crate::client::AspenClient;
use crate::output::print_output;
use crate::output::print_success;

// =============================================================================
// Repository Handlers
// =============================================================================

pub(super) async fn repo_init(client: &AspenClient, args: RepoInitArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::PijulRepoInit {
            name: args.name.clone(),
            description: args.description,
            default_channel: args.default_channel,
        })
        .await?;

    match response {
        ClientRpcResponse::PijulRepoResult(result) => {
            let output = PijulRepoOutput {
                id: result.id,
                name: result.name,
                description: result.description,
                default_channel: result.default_channel,
                channel_count: result.channel_count,
                created_at_ms: result.created_at_ms,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

pub(super) async fn repo_list(client: &AspenClient, args: RepoListArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::PijulRepoList { limit: args.limit }).await?;

    match response {
        ClientRpcResponse::PijulRepoListResult(result) => {
            let output = PijulRepoListOutput {
                repos: result
                    .repos
                    .into_iter()
                    .map(|r| PijulRepoOutput {
                        id: r.id,
                        name: r.name,
                        description: r.description,
                        default_channel: r.default_channel,
                        channel_count: r.channel_count,
                        created_at_ms: r.created_at_ms,
                    })
                    .collect(),
                count: result.count,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

pub(super) async fn repo_info(client: &AspenClient, args: RepoInfoArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::PijulRepoInfo { repo_id: args.repo_id }).await?;

    match response {
        ClientRpcResponse::PijulRepoResult(result) => {
            let output = PijulRepoOutput {
                id: result.id,
                name: result.name,
                description: result.description,
                default_channel: result.default_channel,
                channel_count: result.channel_count,
                created_at_ms: result.created_at_ms,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

// =============================================================================
// Channel Handlers
// =============================================================================

pub(super) async fn channel_list(client: &AspenClient, args: ChannelListArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::PijulChannelList { repo_id: args.repo_id }).await?;

    match response {
        ClientRpcResponse::PijulChannelListResult(result) => {
            let output = PijulChannelListOutput {
                channels: result
                    .channels
                    .into_iter()
                    .map(|c| PijulChannelOutput {
                        name: c.name,
                        head: c.head,
                        updated_at_ms: c.updated_at_ms,
                    })
                    .collect(),
                count: result.count,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

pub(super) async fn channel_create(client: &AspenClient, args: ChannelCreateArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::PijulChannelCreate {
            repo_id: args.repo_id,
            name: args.name.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::PijulChannelResult(result) => {
            let output = PijulChannelOutput {
                name: result.name,
                head: result.head,
                updated_at_ms: result.updated_at_ms,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

pub(super) async fn channel_delete(client: &AspenClient, args: ChannelDeleteArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::PijulChannelDelete {
            repo_id: args.repo_id,
            name: args.name.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::PijulSuccess => {
            print_success(&format!("Deleted channel '{}'", args.name), json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

pub(super) async fn channel_fork(client: &AspenClient, args: ChannelForkArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::PijulChannelFork {
            repo_id: args.repo_id,
            source: args.source,
            target: args.target.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::PijulChannelResult(result) => {
            let output = PijulChannelOutput {
                name: result.name,
                head: result.head,
                updated_at_ms: result.updated_at_ms,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

pub(super) async fn channel_info(client: &AspenClient, args: ChannelInfoArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::PijulChannelInfo {
            repo_id: args.repo_id,
            name: args.name,
        })
        .await?;

    match response {
        ClientRpcResponse::PijulChannelResult(result) => {
            let output = PijulChannelOutput {
                name: result.name,
                head: result.head,
                updated_at_ms: result.updated_at_ms,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

// =============================================================================
// Change Handlers
// =============================================================================

pub(super) async fn pijul_record(client: &AspenClient, args: RecordArgs, json: bool) -> Result<()> {
    // Parse repo ID first to determine cache directory
    let repo_id = RepoId::from_hex(&args.repo_id).context("invalid repository ID format")?;

    // Use provided data-dir or auto-use the cache directory
    let data_dir = args.data_dir.unwrap_or_else(|| repo_cache_dir(&repo_id).expect("failed to get cache dir"));

    // Always use local mode with the cache directory
    // Recording requires a local pristine - changes are then uploaded to the cluster
    pijul_record_local(
        client,
        args.repo_id,
        args.channel,
        args.working_dir,
        args.message,
        args.author,
        args.email,
        data_dir,
        args.threads,
        args.prefix,
        json,
    )
    .await
}

/// Record changes in local mode.
///
/// This uses a local pristine database while syncing changes to the cluster.
async fn pijul_record_local(
    client: &AspenClient,
    repo_id_str: String,
    channel: String,
    working_dir: String,
    message: String,
    author: Option<String>,
    email: Option<String>,
    data_dir: PathBuf,
    threads: usize,
    prefix: Option<String>,
    json: bool,
) -> Result<()> {
    use tracing::info;

    // Parse repo ID
    let repo_id = RepoId::from_hex(&repo_id_str).context("invalid repository ID format")?;

    // Create local pristine manager
    let pristine_mgr = PristineManager::new(&data_dir);
    let pristine = pristine_mgr.open_or_create(&repo_id).context("failed to open/create local pristine")?;

    // Create temporary in-memory blob store for recording
    let temp_blobs = Arc::new(InMemoryBlobStore::new());
    let change_store = Arc::new(AspenChangeStore::new(temp_blobs.clone()));
    let change_dir = ChangeDirectory::new(&data_dir, repo_id, change_store.clone());

    // Create author string
    let author_str = match (author, email) {
        (Some(name), Some(email)) => format!("{} <{}>", name, email),
        (Some(name), None) => name,
        (None, Some(email)) => format!("<{}>", email),
        (None, None) => "Unknown Author <unknown@local>".to_string(),
    };

    // Record changes with performance options
    let mut recorder =
        ChangeRecorder::new(pristine.clone(), change_dir, PathBuf::from(&working_dir)).with_threads(threads);

    if let Some(ref p) = prefix {
        recorder = recorder.with_prefix(p);
    }

    let result = recorder.record(&channel, &message, &author_str).await.context("failed to record changes")?;

    match result {
        Some(record_result) => {
            info!(
                hash = %record_result.hash,
                hunks = record_result.num_hunks,
                bytes = record_result.size_bytes,
                "recorded change locally"
            );

            // Get the change bytes from local store
            let change_bytes = change_store
                .get_change(&record_result.hash)
                .await
                .context("failed to get change from local store")?
                .context("change not found in local store after recording")?;

            // Upload change to cluster blob store
            let blob_response = client
                .send(ClientRpcRequest::AddBlob {
                    data: change_bytes.clone(),
                    tag: Some(format!("pijul:{}:{}", repo_id_str, record_result.hash)),
                })
                .await
                .context("failed to upload change to cluster")?;

            match blob_response {
                ClientRpcResponse::AddBlobResult(_) => {
                    info!(hash = %record_result.hash, "uploaded change to cluster blob store");
                }
                ClientRpcResponse::Error(e) => {
                    anyhow::bail!("failed to upload change: {}: {}", e.code, e.message);
                }
                _ => anyhow::bail!("unexpected response from blob add"),
            }

            // Apply the change to update channel head
            let apply_response = client
                .send(ClientRpcRequest::PijulApply {
                    repo_id: repo_id_str.clone(),
                    channel: channel.clone(),
                    change_hash: record_result.hash.to_string(),
                })
                .await
                .context("failed to apply change to cluster")?;

            match apply_response {
                ClientRpcResponse::PijulApplyResult(_) => {
                    info!(channel = %channel, "updated channel head via Raft");
                }
                ClientRpcResponse::Error(e) => {
                    anyhow::bail!("failed to apply change: {}: {}", e.code, e.message);
                }
                _ => anyhow::bail!("unexpected response from apply"),
            }

            // Store change metadata so pijul log works
            // Parse author string - expected format "Name <email>" or just "Name"
            let (author_name, author_email) = if let Some(start) = author_str.find('<') {
                if let Some(end) = author_str.find('>') {
                    let name = author_str[..start].trim().to_string();
                    let email = author_str[start + 1..end].to_string();
                    (name, email)
                } else {
                    (author_str.clone(), String::new())
                }
            } else {
                (author_str.clone(), String::new())
            };

            let metadata = ChangeMetadata {
                hash: record_result.hash,
                repo_id,
                channel: channel.clone(),
                message: message.clone(),
                authors: vec![PijulAuthor::from_name_email(author_name, author_email)],
                dependencies: record_result.dependencies.clone(),
                size_bytes: record_result.size_bytes as u64,
                recorded_at_ms: chrono::Utc::now().timestamp_millis() as u64,
            };

            let meta_bytes = postcard::to_allocvec(&metadata).context("failed to serialize change metadata")?;
            // Base64 encode for storage - get_change_metadata expects base64
            let meta_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &meta_bytes);

            let meta_key = format!("pijul:change:meta:{}:{}", repo_id_str, record_result.hash);

            let meta_response = client
                .send(ClientRpcRequest::WriteKey {
                    key: meta_key,
                    value: meta_b64.into_bytes(),
                })
                .await
                .context("failed to store change metadata")?;

            match meta_response {
                ClientRpcResponse::WriteResult(_) => {
                    info!(hash = %record_result.hash, "stored change metadata");
                }
                ClientRpcResponse::Error(e) => {
                    // Warn but don't fail - the change was applied successfully
                    tracing::warn!("failed to store metadata: {}: {}", e.code, e.message);
                }
                _ => {
                    tracing::warn!("unexpected response from metadata store");
                }
            }

            // Output result
            let output = PijulRecordOutput {
                change_hash: record_result.hash.to_string(),
                message,
                hunks: record_result.num_hunks as u32,
                size_bytes: record_result.size_bytes as u64,
            };
            print_output(&output, json);
            Ok(())
        }
        None => {
            print_success("No changes to record", json);
            Ok(())
        }
    }
}

pub(super) async fn pijul_apply(client: &AspenClient, args: ApplyArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::PijulApply {
            repo_id: args.repo_id,
            channel: args.channel.clone(),
            change_hash: args.change_hash.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::PijulApplyResult(result) => {
            let output = PijulApplyOutput {
                change_hash: args.change_hash,
                channel: args.channel,
                operations: result.operations,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

pub(super) async fn pijul_unrecord(client: &AspenClient, args: UnrecordArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::PijulUnrecord {
            repo_id: args.repo_id,
            channel: args.channel.clone(),
            change_hash: args.change_hash.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::PijulUnrecordResult(result) => {
            let output = PijulUnrecordOutput {
                change_hash: args.change_hash,
                channel: args.channel,
                unrecorded: result.unrecorded,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

pub(super) async fn pijul_log(client: &AspenClient, args: LogArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::PijulLog {
            repo_id: args.repo_id,
            channel: args.channel,
            limit: args.limit,
        })
        .await?;

    match response {
        ClientRpcResponse::PijulLogResult(result) => {
            let output = PijulLogOutput {
                entries: result
                    .entries
                    .into_iter()
                    .map(|e| PijulLogEntry {
                        change_hash: e.change_hash,
                        message: e.message,
                        author: e.author,
                        timestamp_ms: e.timestamp_ms,
                    })
                    .collect(),
                count: result.count,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

pub(super) async fn pijul_checkout(client: &AspenClient, args: CheckoutArgs, json: bool) -> Result<()> {
    // Check if we're using local mode
    if args.local {
        return pijul_checkout_local(args, json).await;
    }

    // Remote mode - request from server
    let response = client
        .send(ClientRpcRequest::PijulCheckout {
            repo_id: args.repo_id,
            channel: args.channel.clone(),
            output_dir: args.output_dir.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::PijulCheckoutResult(result) => {
            let output = PijulCheckoutOutput {
                channel: args.channel,
                output_dir: args.output_dir,
                files_written: result.files_written,
                conflicts: result.conflicts,
            };
            print_output(&output, json);
            if result.conflicts > 0 {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

/// Checkout using local pristine cache.
async fn pijul_checkout_local(args: CheckoutArgs, json: bool) -> Result<()> {
    use aspen_pijul::WorkingDirOutput;
    use tracing::info;

    // Parse repo ID
    let repo_id = RepoId::from_hex(&args.repo_id).context("invalid repository ID format")?;

    // Determine cache directory
    let cache_dir = args.data_dir.unwrap_or_else(|| repo_cache_dir(&repo_id).expect("failed to get cache dir"));

    // Check if cache exists
    if !cache_dir.exists() {
        anyhow::bail!("Local cache not found at {}. Run 'pijul sync {}' first.", cache_dir.display(), args.repo_id);
    }

    info!(cache_dir = %cache_dir.display(), "using local cache");

    // Create local pristine manager
    let pristine_mgr = PristineManager::new(&cache_dir);
    let pristine = pristine_mgr.open(&repo_id).context("failed to open local pristine - run 'pijul sync' first")?;

    // Create change directory
    let temp_blobs = Arc::new(InMemoryBlobStore::new());
    let change_store = Arc::new(AspenChangeStore::new(temp_blobs));
    let change_dir = ChangeDirectory::new(&cache_dir, repo_id, change_store);

    // Create output directory
    let output_path = PathBuf::from(&args.output_dir);
    std::fs::create_dir_all(&output_path).context("failed to create output directory")?;

    // Output to working directory
    let outputter = WorkingDirOutput::new(pristine, change_dir, output_path);
    let result = outputter.output(&args.channel).context("failed to output to working directory")?;

    let conflicts = result.conflict_count() as u32;

    let output = PijulCheckoutOutput {
        channel: args.channel,
        output_dir: args.output_dir,
        files_written: 0, // WorkingDirOutput doesn't track this currently
        conflicts,
    };
    print_output(&output, json);

    if conflicts > 0 {
        std::process::exit(1);
    }
    Ok(())
}

// =============================================================================
// Sync Handler
// =============================================================================

/// Sync local pristine with cluster state.
pub(super) async fn pijul_sync(client: &AspenClient, args: SyncArgs, json: bool) -> Result<()> {
    use tracing::info;

    // Parse repo ID
    let repo_id = RepoId::from_hex(&args.repo_id).context("invalid repository ID format")?;

    // Determine cache directory
    let cache_dir = args.data_dir.unwrap_or_else(|| repo_cache_dir(&repo_id).expect("failed to get cache dir"));

    // Create cache directory if it doesn't exist
    std::fs::create_dir_all(&cache_dir).context("failed to create cache directory")?;

    info!(cache_dir = %cache_dir.display(), "using cache directory");

    // Get channels to sync
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
        print_success("No channels to sync", json);
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
    let mut all_synced = true;

    // Create the applicator once for all channels
    use aspen_pijul::ChangeApplicator;
    let applicator = ChangeApplicator::new(pristine.clone(), change_dir.clone());

    // Sync each channel
    for channel in &channels {
        info!(channel = %channel, "syncing channel");

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

        // For each change in the log, fetch and apply if missing
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
                        all_synced = false;
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
                    all_synced = false;
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

        info!(channel = %channel, "channel sync complete");
    }

    // Output result
    let output = PijulSyncOutput {
        repo_id: args.repo_id,
        channel: args.channel,
        changes_fetched: total_fetched,
        changes_applied: total_applied,
        already_synced: all_synced && total_fetched == 0,
        conflicts: total_conflicts,
        cache_dir: cache_dir.display().to_string(),
    };
    print_output(&output, json);

    Ok(())
}

// =============================================================================
// Archive Handler
// =============================================================================

/// Export repository state as archive (directory or tarball).
///
/// This command outputs the pristine state to a directory, then optionally
/// packages it as a tarball. It uses the local pristine cache, so you must
/// run `pijul sync` first if you want the latest cluster state.
pub(super) async fn pijul_archive(args: ArchiveArgs, json: bool) -> Result<()> {
    use std::fs::File;

    use aspen_pijul::WorkingDirOutput;
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use tracing::info;

    // Parse repo ID
    let repo_id = RepoId::from_hex(&args.repo_id).context("invalid repository ID format")?;

    // Determine cache directory
    let cache_dir = args.data_dir.unwrap_or_else(|| repo_cache_dir(&repo_id).expect("failed to get cache dir"));

    // Check if cache exists
    if !cache_dir.exists() {
        anyhow::bail!("Local cache not found at {}. Run 'pijul sync {}' first.", cache_dir.display(), args.repo_id);
    }

    info!(cache_dir = %cache_dir.display(), "using local cache");

    // Create local pristine manager
    let pristine_mgr = PristineManager::new(&cache_dir);
    let pristine = pristine_mgr.open(&repo_id).context("failed to open local pristine - run 'pijul sync' first")?;

    // Create change directory
    let temp_blobs = Arc::new(InMemoryBlobStore::new());
    let change_store = Arc::new(AspenChangeStore::new(temp_blobs));
    let change_dir = ChangeDirectory::new(&cache_dir, repo_id, change_store);

    // Determine output format
    let output_path = PathBuf::from(&args.output_path);
    let is_tar_gz = args.output_path.ends_with(".tar.gz") || args.output_path.ends_with(".tgz");
    let is_tar = args.output_path.ends_with(".tar") || is_tar_gz;
    let format = if is_tar_gz {
        "tar.gz"
    } else if is_tar {
        "tar"
    } else {
        "directory"
    };

    // For tarballs, we need a temp directory first
    let (work_dir, cleanup_temp) = if is_tar {
        let temp_dir = std::env::temp_dir().join(format!(
            "aspen-archive-{}-{}",
            repo_id.to_hex(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).context("failed to create temp directory")?;
        (temp_dir, true)
    } else {
        std::fs::create_dir_all(&output_path).context("failed to create output directory")?;
        (output_path.clone(), false)
    };

    // Output to working directory (or temp dir for tarball)
    let outputter = WorkingDirOutput::new(pristine, change_dir, work_dir.clone());

    let result = if let Some(ref prefix) = args.prefix {
        outputter.output_prefix(&args.channel, prefix).context("failed to output repository state")?
    } else {
        outputter.output(&args.channel).context("failed to output repository state")?
    };

    let conflicts = result.conflict_count() as u32;

    // If tarball requested, create it
    let size_bytes = if is_tar {
        // Create the tarball
        let tar_file = File::create(&output_path).context("failed to create archive file")?;

        let size = if is_tar_gz {
            let encoder = GzEncoder::new(tar_file, Compression::default());
            let mut tar_builder = tar::Builder::new(encoder);
            tar_builder.append_dir_all(".", &work_dir).context("failed to add files to archive")?;
            let encoder = tar_builder.into_inner().context("failed to finish tar archive")?;
            let tar_file = encoder.finish().context("failed to finish gzip compression")?;
            tar_file.metadata().map(|m| m.len()).unwrap_or(0)
        } else {
            let mut tar_builder = tar::Builder::new(tar_file);
            tar_builder.append_dir_all(".", &work_dir).context("failed to add files to archive")?;
            let tar_file = tar_builder.into_inner().context("failed to finish tar archive")?;
            tar_file.metadata().map(|m| m.len()).unwrap_or(0)
        };

        // Clean up temp directory
        if cleanup_temp && let Err(e) = std::fs::remove_dir_all(&work_dir) {
            tracing::warn!(path = %work_dir.display(), error = %e, "failed to clean up temp dir");
        }

        size
    } else {
        // Calculate directory size
        calculate_dir_size(&output_path).unwrap_or(0)
    };

    info!(
        output = %args.output_path,
        format = format,
        size = size_bytes,
        conflicts = conflicts,
        "archive complete"
    );

    let output = PijulArchiveOutput {
        repo_id: args.repo_id,
        channel: args.channel,
        output_path: args.output_path,
        format: format.to_string(),
        size_bytes,
        conflicts,
    };
    print_output(&output, json);

    if conflicts > 0 {
        std::process::exit(1);
    }

    Ok(())
}

// =============================================================================
// Show Handler
// =============================================================================

/// Show details of a specific change.
///
/// Queries the cluster for change metadata and displays it in a human-readable
/// or JSON format.
pub(super) async fn pijul_show(client: &AspenClient, args: ShowArgs, json: bool) -> Result<()> {
    // Send request to cluster
    let response = client
        .send(ClientRpcRequest::PijulShow {
            repo_id: args.repo_id.clone(),
            change_hash: args.change_hash.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::PijulShowResult(result) => {
            let output = PijulShowOutput {
                change_hash: result.change_hash,
                repo_id: result.repo_id,
                channel: result.channel,
                message: result.message,
                authors: result.authors.into_iter().map(|a| (a.name, a.email)).collect(),
                dependencies: result.dependencies,
                size_bytes: result.size_bytes,
                recorded_at_ms: result.recorded_at_ms,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message);
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

// =============================================================================
// Blame Handler
// =============================================================================

/// Show change attribution for a file.
///
/// Queries the cluster for blame information and displays it in a human-readable
/// or JSON format.
pub(super) async fn pijul_blame(client: &AspenClient, args: BlameArgs, json: bool) -> Result<()> {
    // Send request to cluster
    let response = client
        .send(ClientRpcRequest::PijulBlame {
            repo_id: args.repo_id.clone(),
            channel: args.channel.clone(),
            path: args.path.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::PijulBlameResult(result) => {
            let output = PijulBlameOutput {
                path: result.path,
                channel: result.channel,
                repo_id: result.repo_id,
                attributions: result
                    .attributions
                    .into_iter()
                    .map(|a| BlameEntry {
                        change_hash: a.change_hash,
                        author: a.author,
                        author_email: a.author_email,
                        message: a.message,
                        recorded_at_ms: a.recorded_at_ms,
                        change_type: a.change_type,
                    })
                    .collect(),
                file_exists: result.file_exists,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message);
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

/// Calculate the total size of a directory recursively.
fn calculate_dir_size(path: &PathBuf) -> Result<u64> {
    let mut total = 0u64;

    if path.is_file() {
        return Ok(std::fs::metadata(path)?.len());
    }

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            total += calculate_dir_size(&path)?;
        } else {
            total += std::fs::metadata(&path)?.len();
        }
    }

    Ok(total)
}

// =============================================================================
// Pull Handler
// =============================================================================

/// Pull changes from the cluster to local pristine.
pub(super) async fn pijul_pull(client: &AspenClient, args: PullArgs, json: bool) -> Result<()> {
    use tracing::info;

    // Parse repo ID
    let repo_id = RepoId::from_hex(&args.repo_id).context("invalid repository ID format")?;

    // Determine cache directory
    let cache_dir = args.data_dir.unwrap_or_else(|| repo_cache_dir(&repo_id).expect("failed to get cache dir"));

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

// =============================================================================
// Push Handler
// =============================================================================

/// Push local changes to the cluster.
pub(super) async fn pijul_push(client: &AspenClient, args: PushArgs, json: bool) -> Result<()> {
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

// =============================================================================
// Diff Handler
// =============================================================================

/// Show differences between working directory and pristine state.
///
/// This uses the same recording mechanism as `pijul record` but only shows
/// what would be recorded without actually creating a change.
pub(super) async fn pijul_diff(args: DiffArgs, json: bool) -> Result<()> {
    use aspen_pijul::ChangeRecorder;
    use tracing::info;

    // Parse repo ID
    let repo_id = RepoId::from_hex(&args.repo_id).context("invalid repository ID format")?;

    // Determine cache directory
    let cache_dir = args.data_dir.unwrap_or_else(|| repo_cache_dir(&repo_id).expect("failed to get cache dir"));

    // Check for channel-to-channel diff
    if let Some(ref channel2) = args.channel2 {
        // Channel-to-channel diff: show changes in channel2 not in channel1
        return pijul_diff_channels(&args.repo_id, &args.channel, channel2, &cache_dir, json).await;
    }

    // Working directory diff - require working_dir
    let working_dir = args
        .working_dir
        .context("--working-dir is required when diffing working directory against a channel")?;

    // Check if cache exists
    if !cache_dir.exists() {
        anyhow::bail!("Local cache not found at {}. Run 'pijul sync {}' first.", cache_dir.display(), args.repo_id);
    }

    info!(cache_dir = %cache_dir.display(), "using local cache for diff");

    // Create local pristine manager
    let pristine_mgr = PristineManager::new(&cache_dir);
    let pristine = pristine_mgr.open(&repo_id).context("failed to open local pristine - run 'pijul sync' first")?;

    // Create temporary in-memory blob store
    let temp_blobs = Arc::new(InMemoryBlobStore::new());
    let change_store = Arc::new(AspenChangeStore::new(temp_blobs));
    let change_dir = ChangeDirectory::new(&cache_dir, repo_id, change_store);

    // Create recorder with performance options to get diff info
    let mut recorder =
        ChangeRecorder::new(pristine, change_dir, PathBuf::from(&working_dir)).with_threads(args.threads);

    if let Some(ref prefix) = args.prefix {
        recorder = recorder.with_prefix(prefix);
    }

    // Get the diff (record with dry-run equivalent - we use the hunks info)
    let diff_result = recorder.diff(&args.channel).await.context("failed to compute diff")?;

    // Convert to output format
    let mut hunks = Vec::new();
    let mut files_added = 0u32;
    let mut files_deleted = 0u32;
    let mut files_modified = 0u32;
    let mut lines_added = 0u32;
    let mut lines_deleted = 0u32;

    for hunk_info in &diff_result.hunks {
        let change_type = match hunk_info.kind.as_str() {
            "add" | "new" => {
                files_added += 1;
                "add"
            }
            "delete" | "remove" => {
                files_deleted += 1;
                "delete"
            }
            "modify" | "edit" => {
                files_modified += 1;
                "modify"
            }
            "rename" => {
                files_modified += 1;
                "rename"
            }
            "permission" | "perm" => {
                files_modified += 1;
                "permission"
            }
            _ => {
                files_modified += 1;
                "modify"
            }
        };

        lines_added += hunk_info.additions;
        lines_deleted += hunk_info.deletions;

        hunks.push(DiffHunk {
            path: hunk_info.path.clone(),
            change_type: change_type.to_string(),
            additions: hunk_info.additions,
            deletions: hunk_info.deletions,
        });
    }

    let no_changes = hunks.is_empty();

    let output = PijulDiffOutput {
        repo_id: args.repo_id,
        channel: args.channel,
        channel2: None,
        working_dir: Some(working_dir),
        hunks,
        files_added,
        files_deleted,
        files_modified,
        lines_added,
        lines_deleted,
        no_changes,
    };

    print_output(&output, json);
    Ok(())
}

/// Diff between two channels.
///
/// Shows changes that are in channel2 but not in channel1.
async fn pijul_diff_channels(
    repo_id_str: &str,
    channel1: &str,
    channel2: &str,
    cache_dir: &PathBuf,
    json: bool,
) -> Result<()> {
    use tracing::info;

    // Parse repo ID
    let repo_id = RepoId::from_hex(repo_id_str).context("invalid repository ID format")?;

    // Check if cache exists
    if !cache_dir.exists() {
        anyhow::bail!("Local cache not found at {}. Run 'pijul sync {}' first.", cache_dir.display(), repo_id_str);
    }

    info!(
        cache_dir = %cache_dir.display(),
        channel1 = %channel1,
        channel2 = %channel2,
        "comparing channels"
    );

    // Create local pristine manager
    let pristine_mgr = PristineManager::new(cache_dir);
    let pristine = pristine_mgr.open(&repo_id).context("failed to open local pristine - run 'pijul sync' first")?;

    // Get changes from both channels and compute the difference
    // This shows changes in channel2 that are not in channel1
    let diff_result = pristine.diff_channels(channel1, channel2).context("failed to compute channel diff")?;

    // Convert to output format
    let mut hunks = Vec::new();
    let mut files_added = 0u32;
    let mut files_deleted = 0u32;
    let mut files_modified = 0u32;
    let mut lines_added = 0u32;
    let mut lines_deleted = 0u32;

    for hunk_info in &diff_result.hunks {
        let change_type = match hunk_info.kind.as_str() {
            "add" | "new" => {
                files_added += 1;
                "add"
            }
            "delete" | "remove" => {
                files_deleted += 1;
                "delete"
            }
            _ => {
                files_modified += 1;
                "modify"
            }
        };

        lines_added += hunk_info.additions;
        lines_deleted += hunk_info.deletions;

        hunks.push(DiffHunk {
            path: hunk_info.path.clone(),
            change_type: change_type.to_string(),
            additions: hunk_info.additions,
            deletions: hunk_info.deletions,
        });
    }

    let no_changes = hunks.is_empty();

    let output = PijulDiffOutput {
        repo_id: repo_id_str.to_string(),
        channel: channel1.to_string(),
        channel2: Some(channel2.to_string()),
        working_dir: None,
        hunks,
        files_added,
        files_deleted,
        files_modified,
        lines_added,
        lines_deleted,
        no_changes,
    };

    print_output(&output, json);
    Ok(())
}
