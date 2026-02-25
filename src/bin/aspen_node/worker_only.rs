//! Worker-only mode implementation for ephemeral CI workers.
//!
//! In this mode, the node connects to an existing cluster via ticket,
//! does NOT participate in Raft consensus, and processes CI jobs using
//! LocalExecutorWorker. Designed for CI VMs that need full SNIX access
//! but should not be consensus participants.

#[cfg(feature = "ci")]
use std::sync::Arc;

#[cfg(feature = "ci")]
use anyhow::Context;
#[cfg(feature = "ci")]
use anyhow::Result;
#[cfg(feature = "ci")]
use aspen::cluster::config::NodeConfig;
#[cfg(feature = "ci")]
use tracing::debug;
#[cfg(feature = "ci")]
use tracing::error;
#[cfg(feature = "ci")]
use tracing::info;
#[cfg(feature = "ci")]
use tracing::warn;

#[cfg(feature = "ci")]
use crate::args::Args;
#[cfg(feature = "ci")]
use crate::signals::shutdown_signal;

#[cfg(feature = "ci")]
pub async fn run_worker_only_mode(args: Args, config: NodeConfig) -> Result<()> {
    use std::time::Duration;

    use aspen::cluster::ticket::parse_ticket_to_addrs;
    use aspen_ci::LocalExecutorWorker;
    use aspen_ci::LocalExecutorWorkerConfig;
    use aspen_client::AspenClient;
    use aspen_client::AspenClusterTicket;
    use aspen_client::BootstrapPeer;
    use aspen_client::RpcBlobStore;
    use aspen_client_api::ClientRpcRequest;
    use aspen_client_api::ClientRpcResponse;
    use aspen_jobs::DependencyFailurePolicy;
    use aspen_jobs::DependencyState;
    use aspen_jobs::Worker;
    use iroh::EndpointAddr;

    // Get ticket from command line, environment variable, or file.
    // Priority: --ticket > ASPEN_GOSSIP_TICKET > ASPEN_CLUSTER_TICKET_FILE
    let ticket_str = if let Some(ticket) = args.ticket.as_ref() {
        ticket.clone()
    } else if let Some(ticket) = config.iroh.gossip_ticket.as_ref() {
        ticket.clone()
    } else if let Ok(ticket_file) = std::env::var("ASPEN_CLUSTER_TICKET_FILE") {
        let ticket_path = std::path::Path::new(&ticket_file);
        if ticket_path.exists() {
            info!(ticket_file = %ticket_file, "reading cluster ticket from file");
            std::fs::read_to_string(ticket_path)
                .context("failed to read cluster ticket from ASPEN_CLUSTER_TICKET_FILE")?
                .trim()
                .to_string()
        } else {
            return Err(anyhow::anyhow!(
                "ASPEN_CLUSTER_TICKET_FILE is set to '{}' but file does not exist",
                ticket_file
            ));
        }
    } else {
        return Err(anyhow::anyhow!(
            "worker-only mode requires a cluster ticket. \
             Provide via --ticket, ASPEN_GOSSIP_TICKET, or ASPEN_CLUSTER_TICKET_FILE."
        ));
    };

    // Parse the ticket to get connection info
    let (topic_id, cluster_id, bootstrap_addrs): (_, _, Vec<EndpointAddr>) =
        parse_ticket_to_addrs(&ticket_str).context("failed to parse cluster ticket")?;

    let bootstrap_count = bootstrap_addrs.len();
    info!(
        cluster_id = %cluster_id,
        bootstrap_peers = bootstrap_count,
        "connecting to cluster as ephemeral CI worker"
    );

    // Create ephemeral Iroh endpoint (no persistent identity)
    let endpoint_config = aspen::cluster::IrohEndpointConfig::new()
        .with_gossip(true)
        .with_bind_port(args.bind_port.unwrap_or(0));

    let iroh_manager = aspen::cluster::IrohEndpointManager::new(endpoint_config)
        .await
        .context("failed to create Iroh endpoint")?;

    let endpoint = iroh_manager.endpoint();
    let endpoint_id = endpoint.id();

    info!(
        endpoint_id = %endpoint_id.fmt_short(),
        "ephemeral Iroh endpoint created"
    );

    // Log bootstrap peers (addresses are discovered via gossip subscription)
    for addr in &bootstrap_addrs {
        debug!(peer_id = %addr.id.fmt_short(), "bootstrap peer from ticket");
    }

    // Join the gossip topic for cluster discovery
    if let Some(gossip) = iroh_manager.gossip() {
        let topic_hex = hex::encode(topic_id.as_bytes());
        info!(topic = %topic_hex, "joining cluster gossip topic");
        let bootstrap_ids: Vec<_> = bootstrap_addrs.iter().map(|a| a.id).collect();
        let _subscription =
            gossip.subscribe(topic_id, bootstrap_ids).await.context("failed to subscribe to gossip topic")?;
    }

    // Select a gateway node by probing bootstrap peers for liveness.
    // Sends a Ping RPC to each peer concurrently, picks the first responder.
    let gateway_node = {
        let probe_timeout = Duration::from_secs(10);
        let mut gateway = None;

        // Build a temporary client for probing (uses all bootstrap peers)
        let probe_peers: Vec<BootstrapPeer> = bootstrap_addrs.iter().map(BootstrapPeer::from_endpoint_addr).collect();
        let probe_ticket = AspenClusterTicket {
            topic_id,
            bootstrap: probe_peers,
            cluster_id: cluster_id.clone(),
        };
        let probe_client = AspenClient::with_endpoint(endpoint.clone(), probe_ticket, probe_timeout, None);

        // Probe with a Ping to discover a live node
        match probe_client.send(ClientRpcRequest::Ping).await {
            Ok(ClientRpcResponse::Pong) => {
                // The client rotates through bootstrap peers automatically;
                // after a successful Ping the last-used peer is live.
                // Use the first bootstrap peer as gateway since the client
                // confirmed at least one peer is reachable.
                if let Some(addr) = bootstrap_addrs.first() {
                    gateway = Some(addr.id);
                    info!(
                        gateway = %addr.id.fmt_short(),
                        "selected gateway via Ping probe"
                    );
                }
            }
            Ok(resp) => {
                warn!(?resp, "unexpected Ping response, falling back to first peer");
            }
            Err(e) => {
                warn!(error = %e, "Ping probe failed, falling back to first peer");
            }
        }

        // Fallback: first bootstrap peer
        gateway
            .or_else(|| bootstrap_addrs.first().map(|a| a.id))
            .ok_or_else(|| anyhow::anyhow!("no bootstrap peers in ticket"))?
    };

    info!(
        gateway = %gateway_node.fmt_short(),
        "using peer as gateway for cache and RPC"
    );

    // Get workspace directory from environment
    let workspace_dir = std::env::var("ASPEN_CI_WORKSPACE_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/workspace"));

    // Generate unique worker ID for this VM instance
    // SAFETY: SystemTime::now() returns time after UNIX_EPOCH on all supported platforms.
    // The only failure case is if the system clock is set before 1970, which is an invalid
    // system configuration that would break many other things. Using unwrap_or(0) as fallback.
    let worker_id = format!(
        "vm-worker-{}-{}",
        endpoint_id.fmt_short(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
    );

    info!(worker_id, "registering ephemeral worker with cluster");

    // Create AspenClient using the existing endpoint
    let bootstrap_peers: Vec<BootstrapPeer> = bootstrap_addrs.iter().map(BootstrapPeer::from_endpoint_addr).collect();

    let client_ticket = AspenClusterTicket {
        topic_id,
        bootstrap: bootstrap_peers,
        cluster_id: cluster_id.clone(),
    };

    let rpc_client = AspenClient::with_endpoint(
        endpoint.clone(),
        client_ticket.clone(),
        Duration::from_secs(30),
        None, // No auth token for now
    );

    // Create RpcBlobStore for workspace seeding from source archives
    let blob_store_client = AspenClient::with_endpoint(
        endpoint.clone(),
        client_ticket.clone(),
        Duration::from_secs(60), // Longer timeout for blob downloads
        None,
    );
    let rpc_blob_store: Arc<dyn aspen_blob::BlobStore> = Arc::new(RpcBlobStore::new(blob_store_client));

    info!("RpcBlobStore created for workspace seeding via RPC");

    // Create RPC-backed cache index for narinfo lookups via the cluster
    let cache_index_client =
        AspenClient::with_endpoint(endpoint.clone(), client_ticket.clone(), Duration::from_secs(30), None);
    let rpc_cache_index: Arc<dyn aspen_cache::CacheIndex> =
        Arc::new(aspen_client::RpcCacheIndex::new(cache_index_client));

    info!("RpcCacheIndex created for Nix binary cache lookups via RPC");

    // Fetch the cache signing public key from the cluster.
    // The cache name can be overridden via ASPEN_CACHE_NAME (default: "aspen-cache").
    let cache_name = std::env::var("ASPEN_CACHE_NAME").unwrap_or_else(|_| "aspen-cache".to_string());
    let transit_mount = std::env::var("ASPEN_TRANSIT_MOUNT").unwrap_or_else(|_| "transit".to_string());

    let cache_public_key = {
        let pk_request = ClientRpcRequest::SecretsNixCacheGetPublicKey {
            mount: transit_mount,
            cache_name: cache_name.clone(),
        };
        match rpc_client.send(pk_request).await {
            Ok(ClientRpcResponse::SecretsNixCacheKeyResult(result)) if result.is_success => {
                let pk = result.public_key.clone();
                info!(cache_name = %cache_name, public_key = ?pk, "fetched cache signing public key");
                pk
            }
            Ok(ClientRpcResponse::SecretsNixCacheKeyResult(result)) => {
                warn!(
                    cache_name = %cache_name,
                    error = ?result.error,
                    "cache public key fetch failed; cache substituter disabled"
                );
                None
            }
            Ok(resp) => {
                warn!(?resp, "unexpected response to SecretsNixCacheGetPublicKey");
                None
            }
            Err(e) => {
                warn!(error = %e, "failed to fetch cache public key; cache substituter disabled");
                None
            }
        }
    };

    // Enable cluster cache only when we have all required pieces
    let use_cluster_cache = cache_public_key.is_some();
    if use_cluster_cache {
        info!("cluster Nix binary cache enabled as substituter");
    } else {
        info!("cluster Nix binary cache disabled (missing signing key)");
    }

    // Create LocalExecutorWorker config
    let worker_config = LocalExecutorWorkerConfig {
        workspace_dir: workspace_dir.clone(),
        should_cleanup_workspaces: true,
        cache_index: Some(rpc_cache_index),
        kv_store: None, // No local KV store in worker-only mode
        use_cluster_cache,
        iroh_endpoint: Some(Arc::new(endpoint.clone())),
        gateway_node: Some(gateway_node),
        cache_public_key,
    };

    // Create worker with RPC blob store for workspace seeding
    let _worker = LocalExecutorWorker::with_blob_store(worker_config, rpc_blob_store);

    info!(
        workspace_dir = %workspace_dir.display(),
        "LocalExecutorWorker created for CI job execution (with RPC blob store)"
    );

    // Register worker with the cluster
    let register_request = ClientRpcRequest::WorkerRegister {
        worker_id: worker_id.clone(),
        capabilities: vec!["ci_vm".to_string()],
        capacity_jobs: 1,
    };

    match rpc_client.send(register_request).await {
        Ok(ClientRpcResponse::WorkerRegisterResult(result)) => {
            if result.is_success {
                info!(worker_id, "worker registered with cluster");
            } else {
                warn!(worker_id, error = ?result.error, "worker registration failed");
            }
        }
        Ok(resp) => {
            warn!(worker_id, response = ?resp, "unexpected response to worker registration");
        }
        Err(e) => {
            warn!(worker_id, error = %e, "failed to send worker registration");
        }
    }

    // Start job polling, execution, and heartbeat tasks
    let worker_id_clone = worker_id.clone();
    let worker_for_executor = _worker;
    let rpc_client_clone = Arc::new(rpc_client);
    let rpc_for_poll = rpc_client_clone.clone();

    let polling_task = tokio::spawn(async move {
        let mut poll_interval = tokio::time::interval(Duration::from_secs(5));
        let mut heartbeat_interval = tokio::time::interval(Duration::from_secs(30));
        let mut active_jobs: Vec<String> = Vec::new();
        let mut total_processed: u64 = 0;
        let mut total_failed: u64 = 0;

        loop {
            tokio::select! {
                _ = poll_interval.tick() => {
                    // Poll for jobs
                    debug!(worker_id = %worker_id_clone, "polling for jobs");

                    let poll_request = ClientRpcRequest::WorkerPollJobs {
                        worker_id: worker_id_clone.clone(),
                        job_types: vec!["ci_vm".to_string()],
                        max_jobs: 1,
                        visibility_timeout_secs: 300,
                    };

                    match rpc_for_poll.send(poll_request).await {
                        Ok(ClientRpcResponse::WorkerPollJobsResult(result)) => {
                            debug!(
                                worker_id = %worker_id_clone,
                                is_success = result.is_success,
                                jobs_count = result.jobs.len(),
                                error = ?result.error,
                                "poll result received"
                            );

                            if !result.is_success {
                                warn!(
                                    worker_id = %worker_id_clone,
                                    error = ?result.error,
                                    "job polling failed"
                                );
                                continue;
                            }

                            for job_info in result.jobs {
                                info!(
                                    worker_id = %worker_id_clone,
                                    job_id = %job_info.job_id,
                                    job_type = %job_info.job_type,
                                    "received job from cluster"
                                );

                                active_jobs.push(job_info.job_id.clone());

                                let job_spec: aspen_jobs::JobSpec = match serde_json::from_str(&job_info.job_spec_json) {
                                    Ok(spec) => spec,
                                    Err(e) => {
                                        error!(
                                            job_id = %job_info.job_id,
                                            error = %e,
                                            "failed to parse job spec"
                                        );
                                        let complete_request = ClientRpcRequest::WorkerCompleteJob {
                                            worker_id: worker_id_clone.clone(),
                                            job_id: job_info.job_id.clone(),
                                            receipt_handle: job_info.receipt_handle.clone(),
                                            execution_token: job_info.execution_token.clone(),
                                            is_success: false,
                                            error_message: Some(format!("Failed to parse job spec: {}", e)),
                                            output_data: None,
                                            processing_time_ms: 0,
                                        };
                                        let _ = rpc_for_poll.send(complete_request).await;
                                        active_jobs.retain(|id| id != &job_info.job_id);
                                        total_failed += 1;
                                        continue;
                                    }
                                };

                                let job = aspen_jobs::Job {
                                    id: aspen_jobs::JobId::parse(&job_info.job_id).unwrap_or_else(|_| aspen_jobs::JobId::new()),
                                    spec: job_spec,
                                    status: aspen_jobs::JobStatus::Running,
                                    attempts: 1,
                                    last_error: None,
                                    result: None,
                                    created_at: chrono::Utc::now(),
                                    updated_at: chrono::Utc::now(),
                                    scheduled_at: None,
                                    started_at: Some(chrono::Utc::now()),
                                    completed_at: None,
                                    worker_id: Some(worker_id_clone.clone()),
                                    next_retry_at: None,
                                    execution_token: Some(job_info.execution_token.clone()),
                                    progress: Some(0),
                                    progress_message: None,
                                    version: 1,
                                    dlq_metadata: None,
                                    dependency_state: DependencyState::Ready,
                                    blocked_by: Vec::new(),
                                    blocking: Vec::new(),
                                    dependency_failure_policy: DependencyFailurePolicy::default(),
                                };

                                let start_time = std::time::Instant::now();
                                let job_result = worker_for_executor.execute(job).await;
                                let processing_time_ms = start_time.elapsed().as_millis() as u64;

                                let (is_success, error_message, output_data) = match &job_result {
                                    aspen_jobs::JobResult::Success(output) => {
                                        let data = serde_json::to_vec(&output.data).ok();
                                        (true, None, data)
                                    }
                                    aspen_jobs::JobResult::Failure(failure) => {
                                        (false, Some(failure.reason.clone()), None)
                                    }
                                    aspen_jobs::JobResult::Cancelled => {
                                        (false, Some("Job cancelled".to_string()), None)
                                    }
                                };

                                let complete_request = ClientRpcRequest::WorkerCompleteJob {
                                    worker_id: worker_id_clone.clone(),
                                    job_id: job_info.job_id.clone(),
                                    receipt_handle: job_info.receipt_handle,
                                    execution_token: job_info.execution_token,
                                    is_success,
                                    error_message,
                                    output_data,
                                    processing_time_ms,
                                };

                                match rpc_for_poll.send(complete_request).await {
                                    Ok(ClientRpcResponse::WorkerCompleteJobResult(result)) => {
                                        if result.is_success {
                                            info!(
                                                worker_id = %worker_id_clone,
                                                job_id = %job_info.job_id,
                                                processing_time_ms,
                                                "job completion reported"
                                            );
                                        } else {
                                            warn!(
                                                worker_id = %worker_id_clone,
                                                job_id = %job_info.job_id,
                                                error = ?result.error,
                                                "failed to report job completion"
                                            );
                                        }
                                    }
                                    Ok(resp) => {
                                        warn!(
                                            job_id = %job_info.job_id,
                                            response = ?resp,
                                            "unexpected response to job completion"
                                        );
                                    }
                                    Err(e) => {
                                        error!(
                                            job_id = %job_info.job_id,
                                            error = %e,
                                            "failed to send job completion"
                                        );
                                    }
                                }

                                active_jobs.retain(|id| id != &job_info.job_id);
                                if is_success {
                                    total_processed += 1;
                                } else {
                                    total_failed += 1;
                                }
                            }
                        }
                        Ok(resp) => {
                            warn!(
                                worker_id = %worker_id_clone,
                                response = ?resp,
                                "unexpected response to job polling"
                            );
                        }
                        Err(e) => {
                            debug!(
                                worker_id = %worker_id_clone,
                                error = %e,
                                "job polling request failed"
                            );
                        }
                    }
                }
                _ = heartbeat_interval.tick() => {
                    debug!(
                        worker_id = %worker_id_clone,
                        active_jobs = active_jobs.len(),
                        total_processed,
                        total_failed,
                        "sending heartbeat"
                    );

                    let heartbeat_request = ClientRpcRequest::WorkerHeartbeat {
                        worker_id: worker_id_clone.clone(),
                        active_jobs: active_jobs.clone(),
                    };

                    if let Err(e) = rpc_for_poll.send(heartbeat_request).await {
                        debug!(
                            worker_id = %worker_id_clone,
                            error = %e,
                            "heartbeat request failed"
                        );
                    }
                }
            }
        }
    });

    info!(
        cluster_id = %cluster_id,
        endpoint_id = %endpoint_id.fmt_short(),
        worker_id = %worker_id,
        "ephemeral CI worker ready - polling for jobs"
    );

    // Wait for shutdown signal
    tokio::select! {
        _ = shutdown_signal() => {
            info!(worker_id = %worker_id, "shutdown signal received");
        }
        _ = polling_task => {
            warn!(worker_id = %worker_id, "polling task ended unexpectedly");
        }
    }

    // Deregister worker on shutdown
    info!(worker_id = %worker_id, "deregistering worker from cluster");

    let deregister_request = ClientRpcRequest::WorkerDeregister {
        worker_id: worker_id.clone(),
    };

    if let Err(e) = rpc_client_clone.send(deregister_request).await {
        warn!(worker_id = %worker_id, error = %e, "failed to deregister worker");
    }

    info!("shutting down ephemeral CI worker");

    // Cleanup - drop the manager which closes the endpoint
    drop(iroh_manager);

    Ok(())
}
