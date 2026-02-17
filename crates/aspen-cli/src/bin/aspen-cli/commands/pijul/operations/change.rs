//! Change operation handlers (record, apply, unrecord, log, checkout).

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

use super::super::ApplyArgs;
use super::super::CheckoutArgs;
use super::super::LogArgs;
use super::super::RecordArgs;
use super::super::UnrecordArgs;
use super::super::output::PijulApplyOutput;
use super::super::output::PijulCheckoutOutput;
use super::super::output::PijulLogEntry;
use super::super::output::PijulLogOutput;
use super::super::output::PijulRecordOutput;
use super::super::output::PijulUnrecordOutput;
use super::super::repo_cache_dir;
use crate::client::AspenClient;
use crate::output::print_output;
use crate::output::print_success;

pub(in super::super) async fn pijul_record(client: &AspenClient, args: RecordArgs, json: bool) -> Result<()> {
    // Parse repo ID first to determine cache directory
    let repo_id = RepoId::from_hex(&args.repo_id).context("invalid repository ID format")?;

    // Use provided data-dir or auto-use the cache directory
    let data_dir = match args.data_dir {
        Some(dir) => dir,
        None => repo_cache_dir(&repo_id).context("failed to determine cache directory")?,
    };

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
        ChangeRecorder::new(pristine.clone(), change_dir, PathBuf::from(&working_dir)).with_threads(threads as u32);

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

pub(in super::super) async fn pijul_apply(client: &AspenClient, args: ApplyArgs, json: bool) -> Result<()> {
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

pub(in super::super) async fn pijul_unrecord(client: &AspenClient, args: UnrecordArgs, json: bool) -> Result<()> {
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

pub(in super::super) async fn pijul_log(client: &AspenClient, args: LogArgs, json: bool) -> Result<()> {
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

pub(in super::super) async fn pijul_checkout(client: &AspenClient, args: CheckoutArgs, json: bool) -> Result<()> {
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
    let cache_dir = match args.data_dir {
        Some(dir) => dir,
        None => repo_cache_dir(&repo_id).context("failed to determine cache directory")?,
    };

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
