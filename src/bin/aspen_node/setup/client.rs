//! Client protocol handler setup.
//!
//! Configures the ClientProtocolContext with all subsystem integrations including
//! authentication, forge, CI/CD, federation, secrets, and worker coordination.

use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use aspen::ClientProtocolContext;
use aspen::api::ClusterController;
use aspen::api::KeyValueStore;
use aspen::auth::TokenVerifier;
use aspen::cluster::config::NodeConfig;
use aspen_core::context::WatchRegistry;
use aspen_jobs::JobManager;
use aspen_raft::node::RaftNode;
#[cfg(feature = "secrets")]
use aspen_rpc_handlers::handlers::SecretsService;
#[allow(unused_imports)]
use tracing::debug;
#[allow(unused_imports)]
use tracing::info;
#[allow(unused_imports)]
use tracing::warn;

use crate::args::Args;
use crate::node_mode::NodeMode;

/// Setup client protocol context and handler.
#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
pub async fn setup_client_protocol(
    args: &Args,
    config: &NodeConfig,
    node_mode: &NodeMode,
    controller: &Arc<dyn ClusterController>,
    kv_store: &Arc<dyn KeyValueStore>,
    primary_raft_node: &Arc<RaftNode>,
    network_factory: &Arc<aspen::cluster::IrpcRaftNetworkFactory>,
    watch_registry: Arc<dyn WatchRegistry>,
) -> Result<(
    Option<Arc<TokenVerifier>>,
    ClientProtocolContext,
    Option<Arc<aspen::cluster::worker_service::WorkerService>>,
    Option<Arc<aspen_coordination::DistributedWorkerCoordinator<dyn KeyValueStore>>>,
)> {
    // Load secrets if configured
    #[cfg(feature = "secrets")]
    let secrets_manager = load_secrets(config, args).await?;

    // Create token verifier if authentication is enabled
    #[cfg(feature = "secrets")]
    let token_verifier_arc = setup_token_authentication(args, node_mode, secrets_manager.as_ref()).await?.map(Arc::new);
    #[cfg(not(feature = "secrets"))]
    let token_verifier_arc = setup_token_authentication(args, node_mode).await?.map(Arc::new);

    // Load Nix cache signer if configured
    #[cfg(all(feature = "secrets", feature = "nix-cache-gateway"))]
    let nix_cache_signer = crate::config::load_nix_cache_signer(config, secrets_manager.as_ref(), kv_store).await?;

    // Create adapter for docs_sync if available
    let docs_sync_arc: Option<Arc<dyn aspen::api::DocsSyncProvider>> = node_mode.docs_sync().map(|ds| {
        Arc::new(aspen::protocol_adapters::DocsSyncProviderAdapter::new(ds.clone(), node_mode.blob_store().cloned()))
            as Arc<dyn aspen::api::DocsSyncProvider>
    });
    // Create adapter for peer_manager if available
    let peer_manager_arc: Option<Arc<dyn aspen::api::PeerManager>> = node_mode.peer_manager().map(|pm| {
        Arc::new(aspen::protocol_adapters::PeerManagerAdapter::new(pm.clone())) as Arc<dyn aspen::api::PeerManager>
    });

    // Create adapter for endpoint manager
    let endpoint_manager_adapter =
        Arc::new(aspen::protocol_adapters::EndpointProviderAdapter::new(node_mode.iroh_manager().clone()))
            as Arc<dyn aspen::api::EndpointProvider>;

    // Use the real IrpcRaftNetworkFactory which implements aspen_core::NetworkFactory
    let network_factory_arc: Arc<dyn aspen::api::NetworkFactory> = network_factory.clone();

    // Initialize ForgeNode if blob_store is available
    #[cfg(feature = "forge")]
    let mut forge_node = node_mode.blob_store().map(|blob_store| {
        let secret_key = node_mode.iroh_manager().secret_key().clone();
        Arc::new(aspen::forge::ForgeNode::new(blob_store.clone(), kv_store.clone(), secret_key))
    });

    // Initialize PijulStore if blob_store and data_dir are available
    #[cfg(feature = "pijul")]
    let pijul_store = node_mode.blob_store().and_then(|blob_store| {
        config.data_dir.as_ref().map(|data_dir| {
            let store = Arc::new(aspen::pijul::PijulStore::new(blob_store.clone(), kv_store.clone(), data_dir.clone()));
            info!("Pijul store initialized at {:?}", data_dir);
            store
        })
    });

    // Initialize JobManager and start worker service if enabled
    let (job_manager, worker_service_handle, _worker_service_cancel) =
        initialize_job_system(config, node_mode, kv_store.clone(), token_verifier_arc.clone()).await?;

    // Create distributed worker coordinator for external worker registration
    let worker_coordinator = Arc::new(aspen_coordination::DistributedWorkerCoordinator::new(kv_store.clone()));

    // Start coordinator background tasks if distributed mode is enabled
    let coordinator_for_shutdown = if config.worker.enable_distributed {
        Arc::clone(&worker_coordinator)
            .start()
            .await
            .context("failed to start distributed worker coordinator")?;
        info!("distributed worker coordinator started with background tasks");
        Some(Arc::clone(&worker_coordinator))
    } else {
        None
    };

    // Initialize secrets service if secrets feature is enabled
    #[cfg(feature = "secrets")]
    let secrets_service = {
        let mount_registry =
            Arc::new(aspen_secrets::MountRegistry::new(kv_store.clone() as Arc<dyn aspen_core::KeyValueStore>));

        info!("Secrets service initialized with multi-mount support");
        Some(Arc::new(SecretsService::new(mount_registry)) as Arc<dyn std::any::Any + Send + Sync>)
    };

    // Enable Forge gossip BEFORE creating CI trigger service to avoid Arc::get_mut failure
    #[cfg(feature = "forge")]
    if config.forge.enable_gossip
        && let Some(ref mut forge_arc) = forge_node
        && let Some(gossip) = node_mode.iroh_manager().gossip()
    {
        if let Some(forge) = Arc::get_mut(forge_arc) {
            forge.enable_gossip(gossip.clone(), None).await.context("failed to enable forge gossip")?;
            info!("Forge gossip enabled (handler will be set after CI trigger service init)");
        } else {
            warn!("forge.enable_gossip: Arc has multiple owners, cannot enable gossip");
        }
    } else if config.forge.enable_gossip && forge_node.is_some() {
        #[cfg(feature = "forge")]
        warn!("forge.enable_gossip is true but gossip service not available");
    }

    // Initialize CI orchestrator if CI is enabled
    #[cfg(feature = "ci")]
    let ci_orchestrator = if config.ci.is_enabled {
        use std::time::Duration;

        use aspen_ci::PipelineOrchestrator;
        use aspen_ci::orchestrator::PipelineOrchestratorConfig;
        use aspen_jobs::WorkflowManager;

        let workflow_manager = Arc::new(WorkflowManager::new(job_manager.clone(), kv_store.clone()));

        let orchestrator_config = PipelineOrchestratorConfig {
            max_runs_per_repo: config.ci.max_concurrent_runs.min(10),
            max_total_runs: 50,
            default_step_timeout: Duration::from_secs(config.ci.pipeline_timeout_secs.min(86400)),
            avoid_leader: config.ci.avoid_leader,
            resource_isolation: config.ci.resource_isolation,
            max_job_memory_bytes: config.ci.max_job_memory_bytes,
        };

        #[cfg(feature = "blob")]
        let blob_store_opt = node_mode.blob_store().map(|b| b.clone() as Arc<dyn aspen_blob::BlobStore>);
        #[cfg(not(feature = "blob"))]
        let blob_store_opt: Option<Arc<dyn aspen_blob::BlobStore>> = None;

        let orchestrator = Arc::new(PipelineOrchestrator::new(
            orchestrator_config,
            workflow_manager,
            job_manager.clone(),
            blob_store_opt,
            kv_store.clone(),
        ));

        orchestrator.init().await;

        info!(
            max_concurrent = config.ci.max_concurrent_runs,
            auto_trigger = config.ci.auto_trigger,
            "CI pipeline orchestrator initialized"
        );

        Some(orchestrator)
    } else {
        None
    };

    // Initialize CI trigger service when auto_trigger is enabled
    #[cfg(all(feature = "ci", feature = "forge"))]
    let ci_trigger_service: Option<Arc<aspen_ci::TriggerService>> = if config.ci.is_enabled && config.ci.auto_trigger {
        use aspen_ci::ForgeConfigFetcher;
        use aspen_ci::OrchestratorPipelineStarter;
        use aspen_ci::TriggerService;
        use aspen_ci::TriggerServiceConfig;

        match (&forge_node, &ci_orchestrator) {
            (Some(forge), Some(orchestrator)) => {
                let config_fetcher = Arc::new(ForgeConfigFetcher::new(forge.clone()));
                let pipeline_starter = Arc::new(OrchestratorPipelineStarter::new(orchestrator.clone(), forge.clone()));
                let trigger_config = TriggerServiceConfig::default();

                let service = TriggerService::new(trigger_config, config_fetcher, pipeline_starter);

                if !config.ci.watched_repos.is_empty() {
                    for repo_id_hex in &config.ci.watched_repos {
                        match aspen_forge::identity::RepoId::from_hex(repo_id_hex) {
                            Ok(repo_id) => {
                                if let Err(e) = service.watch_repo(repo_id).await {
                                    warn!(
                                        repo_id = %repo_id_hex,
                                        error = %e,
                                        "Failed to watch repository for CI triggers"
                                    );
                                } else {
                                    info!(
                                        repo_id = %repo_id_hex,
                                        "Watching repository for CI triggers"
                                    );
                                }
                            }
                            Err(e) => {
                                warn!(
                                    repo_id = %repo_id_hex,
                                    error = %e,
                                    "Invalid repo ID in watched_repos config - skipping"
                                );
                            }
                        }
                    }
                }

                info!(
                    auto_trigger = true,
                    watched_repos = config.ci.watched_repos.len(),
                    "CI trigger service initialized - will auto-trigger on ref updates"
                );

                Some(service)
            }
            _ => {
                warn!("CI auto_trigger enabled but forge or orchestrator not available - disabling auto-trigger");
                None
            }
        }
    } else {
        None
    };

    #[cfg(all(feature = "ci", not(feature = "forge")))]
    let ci_trigger_service: Option<Arc<aspen_ci::TriggerService>> = None;

    // Set the CI trigger handler on the gossip service
    #[cfg(all(feature = "forge", feature = "ci"))]
    if config.forge.enable_gossip
        && config.ci.auto_trigger
        && let Some(ref forge_arc) = forge_node
        && let Some(ref trigger_service) = ci_trigger_service
    {
        let ci_handler: Arc<dyn aspen::forge::gossip::AnnouncementCallback> =
            Arc::new(aspen_ci::CiTriggerHandler::new(trigger_service.clone()));

        if let Err(e) = forge_arc.set_gossip_handler(Some(ci_handler)).await {
            warn!("Failed to set CI trigger handler on gossip: {}", e);
        } else {
            info!("CI trigger handler registered with forge gossip");
        }
    }

    // Initialize federation identity and trust manager if federation is enabled
    #[cfg(feature = "forge")]
    let (federation_identity, federation_trust_manager) = if config.federation.is_enabled {
        let cluster_identity = crate::config::load_federation_identity(config)?;
        let signed_identity = Arc::new(cluster_identity.to_signed());
        let trusted_keys = crate::config::parse_trusted_cluster_keys(&config.federation.trusted_clusters)?;
        let trust_manager = Arc::new(aspen_cluster::federation::TrustManager::with_trusted(trusted_keys));

        info!(
            cluster_name = %signed_identity.name(),
            cluster_key = %signed_identity.public_key(),
            trusted_clusters = config.federation.trusted_clusters.len(),
            "federation identity initialized"
        );

        (Some(signed_identity), Some(trust_manager))
    } else {
        (None, None)
    };

    let client_context = ClientProtocolContext {
        node_id: config.node_id,
        controller: controller.clone(),
        kv_store: kv_store.clone(),
        #[cfg(feature = "sql")]
        sql_executor: primary_raft_node.clone(),
        state_machine: Some(primary_raft_node.state_machine().clone()),
        endpoint_manager: endpoint_manager_adapter,
        #[cfg(feature = "blob")]
        blob_store: node_mode.blob_store().cloned(),
        #[cfg(feature = "blob")]
        blob_replication_manager: node_mode.blob_replication_manager(),
        peer_manager: peer_manager_arc,
        docs_sync: docs_sync_arc,
        cluster_cookie: config.cookie.clone(),
        start_time: std::time::Instant::now(),
        network_factory: Some(network_factory_arc),
        token_verifier: token_verifier_arc.clone(),
        require_auth: args.require_token_auth,
        topology: node_mode.topology().clone(),
        #[cfg(feature = "global-discovery")]
        content_discovery: node_mode.content_discovery().map(|service| {
            Arc::new(aspen::protocol_adapters::ContentDiscoveryAdapter::new(Arc::new(service)))
                as Arc<dyn aspen_core::ContentDiscovery>
        }),
        #[cfg(feature = "forge")]
        forge_node,
        #[cfg(feature = "pijul")]
        pijul_store,
        job_manager: Some(job_manager),
        worker_service: worker_service_handle.clone(),
        worker_coordinator: Some(worker_coordinator),
        watch_registry: Some(watch_registry),
        #[cfg(feature = "hooks")]
        hook_service: node_mode.hook_service(),
        hooks_config: node_mode.hooks_config(),
        #[cfg(feature = "secrets")]
        secrets_service,
        #[cfg(not(feature = "secrets"))]
        secrets_service: None,
        #[cfg(feature = "forge")]
        federation_identity,
        #[cfg(feature = "forge")]
        federation_trust_manager,
        #[cfg(all(feature = "forge", feature = "global-discovery"))]
        federation_discovery: None,
        #[cfg(feature = "ci")]
        ci_orchestrator,
        #[cfg(feature = "ci")]
        ci_trigger_service,
        #[cfg(feature = "nix-cache-gateway")]
        nix_cache_signer,
    };

    Ok((token_verifier_arc, client_context, worker_service_handle, coordinator_for_shutdown))
}

/// Load secrets from SOPS-encrypted file if configured.
#[cfg(feature = "secrets")]
async fn load_secrets(config: &NodeConfig, args: &Args) -> Result<Option<Arc<aspen_secrets::SecretsManager>>> {
    use aspen_secrets::SecretsManager;
    use aspen_secrets::decrypt_secrets_file;
    use aspen_secrets::load_age_identity;

    if !config.secrets.is_enabled {
        return Ok(None);
    }

    let secrets_file = args
        .secrets_file
        .as_ref()
        .or(config.secrets.secrets_file.as_ref())
        .ok_or_else(|| anyhow::anyhow!("secrets enabled but no secrets_file specified"))?;

    let identity_file = args.age_identity_file.as_ref().or(config.secrets.age_identity_file.as_ref());

    info!(
        secrets_file = %secrets_file.display(),
        identity_file = ?identity_file.map(|p| p.display().to_string()),
        "loading SOPS-encrypted secrets"
    );

    let identity = load_age_identity(identity_file.map(|p| p.as_path()), &config.secrets.age_identity_env)
        .await
        .context("failed to load age identity for secrets decryption")?;

    let secrets_file_data =
        decrypt_secrets_file(secrets_file, &identity).await.context("failed to decrypt secrets file")?;

    let manager = SecretsManager::new(config.secrets.clone(), secrets_file_data)
        .context("failed to initialize secrets manager")?;

    info!(trusted_roots = manager.trusted_root_count(), "secrets loaded successfully");

    Ok(Some(Arc::new(manager)))
}

/// Setup token authentication if enabled.
#[cfg(feature = "secrets")]
async fn setup_token_authentication(
    args: &Args,
    node_mode: &NodeMode,
    secrets_manager: Option<&Arc<aspen_secrets::SecretsManager>>,
) -> Result<Option<TokenVerifier>> {
    if !args.enable_token_auth {
        return Ok(None);
    }

    if let Some(manager) = secrets_manager {
        let verifier = manager.build_token_verifier();
        info!(
            trusted_roots = manager.trusted_root_count(),
            "Token auth enabled with trusted roots from SOPS secrets"
        );
        return Ok(Some(verifier));
    }

    build_token_verifier_from_args(args, node_mode)
}

/// Setup token authentication if enabled (non-secrets version).
#[cfg(not(feature = "secrets"))]
async fn setup_token_authentication(args: &Args, node_mode: &NodeMode) -> Result<Option<TokenVerifier>> {
    if !args.enable_token_auth {
        return Ok(None);
    }

    build_token_verifier_from_args(args, node_mode)
}

/// Build token verifier from CLI args or node's own key.
fn build_token_verifier_from_args(args: &Args, node_mode: &NodeMode) -> Result<Option<TokenVerifier>> {
    let mut verifier = TokenVerifier::new();

    if args.trusted_root_key.is_empty() {
        let node_public_key = node_mode.iroh_manager().endpoint().id();
        verifier = verifier.with_trusted_root(node_public_key);
        info!(
            trusted_root = %node_public_key,
            "Token auth enabled with node's own key as trusted root"
        );
    } else {
        for key_hex in &args.trusted_root_key {
            let key_bytes = hex::decode(key_hex).context("Invalid hex in --trusted-root-key")?;
            if key_bytes.len() != 32 {
                anyhow::bail!("Invalid key length: expected 32 bytes (64 hex chars), got {}", key_bytes.len());
            }
            let mut key_array = [0u8; 32];
            key_array.copy_from_slice(&key_bytes);
            let public_key = iroh::SecretKey::from_bytes(&key_array).public();
            verifier = verifier.with_trusted_root(public_key);
            info!(
                trusted_root = %public_key,
                "Token auth enabled with explicit trusted root"
            );
        }
    }

    Ok(Some(verifier))
}

/// Initialize job manager and optionally start worker service.
#[allow(unused_variables)]
async fn initialize_job_system(
    config: &NodeConfig,
    node_mode: &NodeMode,
    kv_store: Arc<dyn KeyValueStore>,
    token_verifier: Option<Arc<TokenVerifier>>,
) -> Result<(
    Arc<JobManager<dyn KeyValueStore>>,
    Option<Arc<aspen::cluster::worker_service::WorkerService>>,
    Option<tokio_util::sync::CancellationToken>,
)> {
    use aspen::cluster::worker_service::WorkerService;
    use aspen_jobs_worker_maintenance::MaintenanceWorker;
    use tokio_util::sync::CancellationToken;
    #[allow(unused_imports)]
    use tracing::error;

    let job_manager = Arc::new(JobManager::new(kv_store.clone()));

    match job_manager.initialize().await {
        Ok(()) => info!("job manager initialized with priority queues"),
        Err(e) => {
            debug!("job manager initialization deferred: {}", e);
        }
    }

    if config.worker.is_enabled {
        info!(
            worker_count = config.worker.worker_count,
            job_types = ?config.worker.job_types,
            tags = ?config.worker.tags,
            "initializing worker service"
        );

        let endpoint_provider =
            Arc::new(aspen::protocol_adapters::EndpointProviderAdapter::new(node_mode.iroh_manager().clone()))
                as Arc<dyn aspen_core::EndpointProvider>;

        let mut worker_service = WorkerService::new(
            config.node_id,
            config.worker.clone(),
            job_manager.clone(),
            kv_store.clone(),
            endpoint_provider,
        )
        .context("failed to create worker service")?;

        // Register maintenance worker for system tasks (only when using Redb storage)
        if let Some(db) = node_mode.db() {
            let maintenance_worker = MaintenanceWorker::new(config.node_id, db.clone());
            worker_service
                .register_handler("compact_storage", maintenance_worker.clone())
                .await
                .context("failed to register maintenance worker")?;
            worker_service
                .register_handler("cleanup_blobs", maintenance_worker.clone())
                .await
                .context("failed to register maintenance worker")?;
            worker_service
                .register_handler("health_check", maintenance_worker)
                .await
                .context("failed to register maintenance worker")?;
            info!("maintenance workers registered (Redb storage backend)");
        } else {
            info!("maintenance workers skipped (in-memory storage backend)");
        }

        // Register VM executor worker for sandboxed job execution
        #[cfg(all(feature = "vm-executor", target_os = "linux"))]
        {
            use aspen_jobs::HyperlightWorker;
            if let Some(blob_store) = node_mode.blob_store() {
                let blob_store_dyn: Arc<dyn aspen_blob::BlobStore> = blob_store.clone();
                match HyperlightWorker::new(blob_store_dyn) {
                    Ok(vm_worker) => {
                        worker_service
                            .register_handler("vm_execute", vm_worker)
                            .await
                            .context("failed to register VM executor worker")?;
                        info!("VM executor worker registered (Hyperlight with blob store)");
                    }
                    Err(e) => {
                        warn!("Failed to create HyperlightWorker: {}. VM execution disabled.", e);
                        warn!("VM execution requires KVM support on Linux");
                    }
                }
            } else {
                warn!("No blob store available for VM executor. VM execution disabled.");
                warn!("Enable blob storage in node configuration to use VM executor.");
            }
        }

        // CI/CD infrastructure setup
        #[cfg(feature = "ci")]
        {
            use aspen_cache::CacheIndex;
            use aspen_cache::KvCacheIndex;

            let cache_index: Option<Arc<dyn CacheIndex>> =
                Some(Arc::new(KvCacheIndex::new(kv_store.clone())) as Arc<dyn CacheIndex>);

            #[cfg(feature = "snix")]
            #[allow(clippy::type_complexity)]
            let (snix_blob_service, snix_directory_service, snix_pathinfo_service): (
                Option<Arc<dyn snix_castore::blobservice::BlobService>>,
                Option<Arc<dyn snix_castore::directoryservice::DirectoryService>>,
                Option<Arc<dyn snix_store::pathinfoservice::PathInfoService>>,
            ) = if config.snix.is_enabled {
                warn!(
                    "SNIX services requested but Raft-based services need fixes. \
                     Use worker-only mode with RPC services for SNIX uploads."
                );
                (None, None, None)
            } else {
                (None, None, None)
            };

            #[cfg(not(feature = "snix"))]
            let (snix_blob_service, snix_directory_service, snix_pathinfo_service) = (None, None, None);

            let use_cluster_cache = config.nix_cache.is_enabled && config.nix_cache.enable_ci_substituter;

            let iroh_endpoint = if use_cluster_cache {
                Some(Arc::new(node_mode.iroh_manager().endpoint().clone()))
            } else {
                None
            };

            let gateway_node = if use_cluster_cache {
                Some(node_mode.iroh_manager().endpoint().id())
            } else {
                None
            };

            let cache_public_key = if use_cluster_cache {
                crate::config::read_nix_cache_public_key(&kv_store).await
            } else {
                None
            };

            if use_cluster_cache {
                if cache_public_key.is_some() {
                    let gateway_short = gateway_node
                        .as_ref()
                        .map(|g| g.fmt_short().to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    info!(
                        gateway_node = %gateway_short,
                        "Nix cache substituter enabled for CI workers"
                    );
                } else {
                    warn!(
                        "Nix cache substituter requested but public key not available in KV store. \
                         Cache substitution will be disabled until key is stored at _system:nix-cache:public-key"
                    );
                }
            }

            #[cfg(feature = "nix-executor")]
            {
                use aspen_ci::NixBuildWorker;
                use aspen_ci::NixBuildWorkerConfig;

                let nix_config = NixBuildWorkerConfig {
                    node_id: config.node_id,
                    cluster_id: config.cookie.clone(),
                    blob_store: node_mode.blob_store().map(|b| b.clone() as Arc<dyn aspen_blob::BlobStore>),
                    cache_index: cache_index.clone(),
                    snix_blob_service: snix_blob_service.clone(),
                    snix_directory_service: snix_directory_service.clone(),
                    snix_pathinfo_service: snix_pathinfo_service.clone(),
                    output_dir: std::path::PathBuf::from("/tmp/aspen-ci/builds"),
                    nix_binary: "nix".to_string(),
                    verbose: false,
                    use_cluster_cache,
                    iroh_endpoint: iroh_endpoint.clone(),
                    gateway_node,
                    cache_public_key: cache_public_key.clone(),
                };

                let all_services_available = nix_config.validate();
                if !all_services_available {
                    warn!(
                        node_id = config.node_id,
                        "NixBuildWorker will operate with reduced functionality due to missing services"
                    );
                }

                let nix_worker = NixBuildWorker::new(nix_config);
                worker_service
                    .register_handler("ci_nix_build", nix_worker)
                    .await
                    .context("failed to register Nix build worker")?;
                info!(
                    cluster_id = %config.cookie,
                    node_id = config.node_id,
                    snix_enabled = config.snix.is_enabled,
                    "Nix build worker registered for CI/CD flake builds"
                );
            }

            #[cfg(target_os = "linux")]
            {
                let use_local_executor = std::env::var("ASPEN_CI_LOCAL_EXECUTOR")
                    .map(|v| v == "1" || v.to_lowercase() == "true")
                    .unwrap_or(false);

                if use_local_executor {
                    use aspen_ci::LocalExecutorWorker;
                    use aspen_ci::LocalExecutorWorkerConfig;

                    let workspace_dir = std::env::var("ASPEN_CI_WORKSPACE_DIR")
                        .map(std::path::PathBuf::from)
                        .unwrap_or_else(|_| std::path::PathBuf::from("/workspace"));

                    let local_iroh_endpoint = if use_cluster_cache {
                        Some(Arc::new(node_mode.iroh_manager().endpoint().clone()))
                    } else {
                        None
                    };

                    let local_config = LocalExecutorWorkerConfig {
                        workspace_dir: workspace_dir.clone(),
                        should_cleanup_workspaces: true,
                        snix_blob_service: snix_blob_service.clone(),
                        snix_directory_service: snix_directory_service.clone(),
                        snix_pathinfo_service: snix_pathinfo_service.clone(),
                        cache_index: cache_index.clone(),
                        kv_store: Some(kv_store.clone()),
                        use_cluster_cache,
                        iroh_endpoint: local_iroh_endpoint,
                        gateway_node,
                        cache_public_key: cache_public_key.clone(),
                    };

                    let blob_store_opt = node_mode.blob_store().map(|b| b.clone() as Arc<dyn aspen_blob::BlobStore>);

                    let local_worker = if let Some(blob_store) = blob_store_opt {
                        LocalExecutorWorker::with_blob_store(local_config, blob_store)
                    } else {
                        LocalExecutorWorker::new(local_config)
                    };

                    worker_service
                        .register_handler("ci_vm", local_worker)
                        .await
                        .context("failed to register Local Executor worker")?;
                    info!(
                        workspace_dir = %workspace_dir.display(),
                        snix_enabled = snix_blob_service.is_some(),
                        cache_substituter = use_cluster_cache,
                        "Local Executor worker registered for CI jobs (no VM isolation)"
                    );
                }
            }
        }

        // CloudHypervisorWorker for VM-isolated CI jobs
        #[cfg(all(feature = "ci", feature = "ci-vm-executor", target_os = "linux"))]
        {
            use aspen_ci::CloudHypervisorWorker;
            use aspen_ci::CloudHypervisorWorkerConfig;
            use aspen_ci::NetworkMode;
            use aspen_jobs::Worker;

            let use_local_executor = std::env::var("ASPEN_CI_LOCAL_EXECUTOR")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(false);

            if !use_local_executor {
                let ch_state_dir = config
                    .data_dir
                    .as_ref()
                    .map(|d| d.join("ci").join("vms"))
                    .unwrap_or_else(|| std::path::PathBuf::from("/var/lib/aspen/ci/vms"));

                let default_config = CloudHypervisorWorkerConfig::default();

                let host_iroh_port: Option<u16> = {
                    let endpoint_addr = node_mode.iroh_manager().endpoint().addr();
                    endpoint_addr.addrs.iter().find_map(|transport_addr| {
                        if let iroh::TransportAddr::Ip(socket_addr) = transport_addr {
                            Some(socket_addr.port())
                        } else {
                            None
                        }
                    })
                };

                let cluster_ticket_file = if let Some(ref data_dir) = config.data_dir {
                    data_dir.join("cluster-ticket.txt")
                } else {
                    std::path::PathBuf::from(format!("/tmp/aspen-node-{}-ticket.txt", config.node_id))
                };

                let ch_config = CloudHypervisorWorkerConfig {
                    node_id: config.node_id,
                    state_dir: ch_state_dir.clone(),
                    cluster_ticket_file: Some(cluster_ticket_file.clone()),
                    kernel_path: std::env::var("ASPEN_CI_KERNEL_PATH")
                        .map(std::path::PathBuf::from)
                        .unwrap_or_default(),
                    initrd_path: std::env::var("ASPEN_CI_INITRD_PATH")
                        .map(std::path::PathBuf::from)
                        .unwrap_or_default(),
                    toplevel_path: std::env::var("ASPEN_CI_TOPLEVEL_PATH")
                        .map(std::path::PathBuf::from)
                        .unwrap_or_default(),
                    cloud_hypervisor_path: std::env::var("CLOUD_HYPERVISOR_PATH").map(std::path::PathBuf::from).ok(),
                    virtiofsd_path: std::env::var("VIRTIOFSD_PATH").map(std::path::PathBuf::from).ok(),
                    vm_memory_mib: std::env::var("ASPEN_CI_VM_MEMORY_MIB")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(default_config.vm_memory_mib),
                    vm_vcpus: std::env::var("ASPEN_CI_VM_VCPUS")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(default_config.vm_vcpus),
                    host_iroh_port,
                    network_mode: match std::env::var("ASPEN_CI_NETWORK_MODE").as_deref().unwrap_or("tap") {
                        "none" | "isolated" => NetworkMode::None,
                        "helper" | "tap-helper" => NetworkMode::TapWithHelper,
                        _ => NetworkMode::Tap,
                    },
                    tap_helper_path: std::env::var("ASPEN_CI_TAP_HELPER_PATH").map(std::path::PathBuf::from).ok(),
                    ..default_config
                };

                if !ch_config.kernel_path.as_os_str().is_empty() {
                    match CloudHypervisorWorker::new(ch_config.clone()) {
                        Ok(ch_worker) => {
                            let ch_worker = Arc::new(ch_worker);
                            tokio::spawn({
                                let worker = ch_worker.clone();
                                async move {
                                    if let Err(e) = worker.on_start().await {
                                        error!(
                                            error = %e,
                                            "Failed to initialize CloudHypervisorWorker VM pool"
                                        );
                                    }
                                }
                            });
                            info!(
                                state_dir = ?ch_state_dir,
                                ticket_file = %cluster_ticket_file.display(),
                                pool_size = ch_config.pool_size,
                                max_vms = ch_config.max_vms,
                                vm_memory_mib = ch_config.vm_memory_mib,
                                vm_vcpus = ch_config.vm_vcpus,
                                host_iroh_port = ?ch_config.host_iroh_port,
                                bridge_addr = ?ch_config.bridge_socket_addr(),
                                network_mode = ?ch_config.network_mode,
                                "Cloud Hypervisor VM pool manager initialized (VMs will poll for ci_vm jobs)"
                            );
                        }
                        Err(e) => {
                            warn!(
                                error = %e,
                                "Failed to create CloudHypervisorWorker. VM-isolated CI jobs disabled."
                            );
                        }
                    }
                } else {
                    debug!(
                        "Cloud Hypervisor worker not registered: ASPEN_CI_KERNEL_PATH not set. \
                         Set this environment variable to enable VM-isolated CI jobs, \
                         or set ASPEN_CI_LOCAL_EXECUTOR=1 for direct process execution."
                    );
                }
            }
        }

        // Register shell command worker
        #[cfg(feature = "shell-worker")]
        {
            use aspen_jobs_worker_shell::ShellCommandWorker;
            use aspen_jobs_worker_shell::ShellCommandWorkerConfig;

            let has_auth = token_verifier.is_some();

            let mut default_env = std::collections::HashMap::new();
            for var in ["PATH", "HOME", "USER", "SHELL", "TERM", "LANG", "LC_ALL"] {
                if let Ok(value) = std::env::var(var) {
                    default_env.insert(var.to_string(), value);
                }
            }
            for var in [
                "NIX_PATH",
                "NIX_SSL_CERT_FILE",
                "SSL_CERT_FILE",
                "CARGO_HOME",
                "RUSTUP_HOME",
            ] {
                if let Ok(value) = std::env::var(var) {
                    default_env.insert(var.to_string(), value);
                }
            }

            let shell_config = ShellCommandWorkerConfig {
                node_id: config.node_id,
                token_verifier: token_verifier.clone(),
                blob_store: node_mode.blob_store().map(|b| b.clone() as Arc<dyn aspen_blob::BlobStore>),
                default_working_dir: std::env::temp_dir(),
                default_env,
            };
            info!(
                path_length = shell_config.default_env.get("PATH").map(|p| p.len()).unwrap_or(0),
                env_vars = shell_config.default_env.len(),
                "shell command worker configured with inherited environment"
            );
            let shell_worker = ShellCommandWorker::new(shell_config);
            worker_service
                .register_handler("shell_command", shell_worker)
                .await
                .context("failed to register shell command worker")?;
            if has_auth {
                info!("shell command worker registered (auth enabled)");
            } else {
                warn!("shell command worker registered WITHOUT auth (dev mode only)");
            }
        }

        // Register hook job worker if hooks are enabled
        #[cfg(feature = "hooks")]
        if config.hooks.is_enabled {
            if let Some(hook_service) = node_mode.hook_service() {
                use aspen_hooks::HOOK_JOB_TYPE;
                use aspen_hooks::worker::job_worker::HookWorkerImpl;

                let hook_worker = HookWorkerImpl::new(hook_service.registry());
                worker_service
                    .register_handler(HOOK_JOB_TYPE, hook_worker)
                    .await
                    .context("failed to register hook job worker")?;
                info!("hook job worker registered for job-mode handler execution");
            } else {
                debug!("hook job worker not registered: hook service not available");
            }
        }

        // Register echo worker as fallback for unregistered job types
        use aspen_jobs::workers::EchoWorker;
        worker_service.register_handler("*", EchoWorker).await.context("failed to register echo worker")?;
        info!("echo worker registered as fallback handler");

        worker_service.start().await.context("failed to start worker service")?;

        let worker_service = Arc::new(worker_service);
        let cancel_token = CancellationToken::new();

        info!(worker_count = config.worker.worker_count, "worker service started");

        Ok((job_manager, Some(worker_service), Some(cancel_token)))
    } else {
        info!("worker service disabled in configuration");
        Ok((job_manager, None, None))
    }
}
