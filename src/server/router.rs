//! Axum router configuration
//!
//! Modular router organization with focused sub-routers for different API surfaces.
//!
//! ## Router Structure
//!
//! ```text
//! /
//! ├── /dashboard/*      - HTMX monitoring UI (browser access)
//! ├── /api/queue/*      - Work queue REST API (worker access)
//! ├── /api/iroh/*       - P2P blob storage and gossip API
//! └── /health/*         - Health check endpoints
//! ```
//!
//! This modular approach provides:
//! - **Clear API boundaries** - Logical grouping of related routes
//! - **Route-specific middleware** - Apply middleware to route groups
//! - **Easier maintenance** - Add/remove routes within focused modules
//! - **Better documentation** - Self-describing route structure

use axum::{routing::{get, post}, Router};
use std::sync::Arc;

#[cfg(feature = "tofu-support")]
use crate::api::tofu_handlers::*;
use crate::handlers::dashboard::*;
use crate::handlers::queue::*;
use crate::handlers::worker::*;
use crate::iroh_api;
use crate::middleware;
use crate::state::{DomainState, InfraState, ConfigState, FeaturesState};
use crate::domain::{ClusterStatusService, JobCommandService, JobQueryService, HealthService, WorkerManagementService};

/// Build the complete Axum router with all routes
///
/// This creates a router serving both HTTP/1.1 (localhost) and HTTP/3 (iroh+h3 P2P).
/// All routes are registered here for consistency across transports.
///
/// Routes are organized into logical sub-routers with focused state containers:
/// - Dashboard UI - uses DomainState (ClusterStatusService, JobQueryService)
/// - Work Queue API - uses DomainState (JobCommandService, JobQueryService, HealthService)
/// - Worker API - uses DomainState (WorkerManagementService)
/// - Iroh P2P API - uses InfraState (IrohService)
/// - Tofu State API - uses FeaturesState (TofuService)
/// - Health checks - uses DomainState (HealthService)
pub fn build_router(
    domain: DomainState,
    infra: InfraState,
    config: ConfigState,
    features: FeaturesState,
) -> Router {
    // Extract specific services from DomainState
    let cluster_service = domain.cluster_status();
    let job_commands = domain.job_commands();
    let job_queries = domain.job_queries();
    let health_service = domain.health();
    let worker_service = domain.worker_management();
    let auth_config = config.auth();

    let router = Router::new()
        .nest("/dashboard", dashboard_router(cluster_service.clone(), job_commands.clone(), job_queries.clone()))
        .nest("/api/queue", queue_api_router(job_commands.clone(), job_queries.clone(), health_service.clone(), auth_config.clone()))
        .nest("/api/workers", worker_api_router(worker_service.clone(), auth_config.clone()))
        .nest("/api/iroh", iroh_api_router(auth_config.clone()).with_state(infra));

    #[cfg(feature = "tofu-support")]
    let router = router.nest("/api/tofu", tofu_api_router(auth_config.clone()).with_state(features));

    router
        .nest("/health", health_router(health_service.clone()))
}

/// Dashboard routes - HTMX monitoring UI
///
/// Primary user interface for cluster monitoring and job management.
/// Renders HTML fragments for real-time updates via HTMX.
///
/// Uses ClusterStatusService and JobLifecycleService dependencies.
///
/// Routes:
/// - `GET  /dashboard` - Main dashboard page
/// - `GET  /dashboard/cluster-health` - Cluster health HTMX fragment (uses ClusterStatusService)
/// - `GET  /dashboard/queue-stats` - Queue statistics HTMX fragment (uses JobLifecycleService)
/// - `GET  /dashboard/recent-jobs` - Recent jobs list HTMX fragment (uses JobLifecycleService)
/// - `GET  /dashboard/control-plane-nodes` - Control plane nodes HTMX fragment (uses ClusterStatusService)
/// - `GET  /dashboard/workers` - Worker nodes HTMX fragment (uses ClusterStatusService)
/// - `POST /dashboard/submit-job` - Submit new job via web UI (uses JobQueryService)
fn dashboard_router(
    cluster_service: Arc<ClusterStatusService>,
    job_commands: Arc<JobCommandService>,
    job_queries: Arc<JobQueryService>,
) -> Router {
    Router::new()
        .route("/", get(dashboard))
        .route("/cluster-health", get(dashboard_cluster_health).with_state(cluster_service.clone()))
        .route("/queue-stats", get(dashboard_queue_stats).with_state(job_queries.clone()))
        .route("/recent-jobs", get(dashboard_recent_jobs).with_state(job_queries.clone()))
        .route("/control-plane-nodes", get(dashboard_control_plane_nodes).with_state(cluster_service.clone()))
        .route("/workers", get(dashboard_workers).with_state(cluster_service))
        .route("/submit-job", post(dashboard_submit_job).with_state((job_commands, job_queries)))
    // Future: Add dashboard-specific middleware
    // .layer(/* HTMX headers, caching policy, etc. */)
}

/// Work Queue API routes - Worker operations
///
/// REST API for distributed workers to claim and process jobs.
/// Used by worker binaries and CLI tools.
///
/// Uses JobCommandService and JobQueryService for queue operations.
///
/// Routes:
/// - `POST /api/queue/publish` - Submit new job to queue
/// - `POST /api/queue/claim` - Claim next available job
/// - `GET  /api/queue/list` - List all jobs
/// - `GET  /api/queue/stats` - Get queue statistics
/// - `POST /api/queue/status/{job_id}` - Update job status
fn queue_api_router(
    job_commands: Arc<JobCommandService>,
    job_queries: Arc<JobQueryService>,
    _health_service: Arc<HealthService>,
    auth_config: Arc<crate::config::AuthConfig>,
) -> Router {
    Router::new()
        .route("/publish", post(queue_publish).with_state(job_commands.clone()))
        .route("/claim", post(queue_claim).with_state((job_commands.clone(), job_queries.clone())))
        .route("/list", get(queue_list).with_state(job_queries.clone()))
        .route("/stats", get(queue_stats).with_state(job_queries.clone()))
        .route("/status/{job_id}", post(queue_update_status).with_state(job_commands))
        // Apply API key authentication to all queue routes
        .layer(axum::middleware::from_fn_with_state(auth_config, middleware::api_key_auth))
}

/// Worker Management API routes - Worker lifecycle
///
/// REST API for worker registration, heartbeats, and management.
/// Used by worker binaries to register with the control plane.
///
/// Uses WorkerManagementService dependency.
///
/// Routes:
/// - `POST /api/workers/register` - Register a new worker
/// - `POST /api/workers/{worker_id}/heartbeat` - Send worker heartbeat
/// - `GET  /api/workers` - List all workers
/// - `GET  /api/workers/{worker_id}` - Get worker details
/// - `POST /api/workers/{worker_id}/drain` - Mark worker as draining
/// - `GET  /api/workers/stats` - Get worker pool statistics
fn worker_api_router(
    worker_service: Arc<WorkerManagementService>,
    auth_config: Arc<crate::config::AuthConfig>,
) -> Router {
    Router::new()
        .route("/register", post(worker_register).with_state(worker_service.clone()))
        .route("/{worker_id}/heartbeat", post(worker_heartbeat).with_state(worker_service.clone()))
        .route("/", get(worker_list).with_state(worker_service.clone()))
        .route("/{worker_id}", get(worker_get).with_state(worker_service.clone()))
        .route("/{worker_id}/drain", post(worker_drain).with_state(worker_service.clone()))
        .route("/stats", get(worker_stats).with_state(worker_service))
        // Apply API key authentication to all worker routes
        .layer(axum::middleware::from_fn_with_state(auth_config, middleware::api_key_auth))
}

/// Iroh P2P API routes - Blob storage and gossip
///
/// Peer-to-peer networking API for distributed content storage and messaging.
/// Enables direct P2P communication between mvm-ci instances.
///
/// Routes:
/// - `POST /api/iroh/blob/store` - Store content-addressed blob
/// - `GET  /api/iroh/blob/{hash}` - Retrieve blob by hash
/// - `POST /api/iroh/gossip/join` - Join gossip topic
/// - `POST /api/iroh/gossip/broadcast` - Broadcast message to topic
/// - `GET  /api/iroh/gossip/subscribe/{topic_id}` - Subscribe to topic (SSE)
/// - `POST /api/iroh/connect` - Connect to peer
/// - `GET  /api/iroh/info` - Get endpoint information
fn iroh_api_router(auth_config: Arc<crate::config::AuthConfig>) -> Router<InfraState> {
    Router::new()
        .route("/blob/store", post(iroh_api::store_blob))
        .route("/blob/{hash}", get(iroh_api::retrieve_blob))
        .route("/gossip/join", post(iroh_api::join_gossip_topic))
        .route("/gossip/broadcast", post(iroh_api::broadcast_gossip))
        .route("/gossip/subscribe/{topic_id}", get(iroh_api::subscribe_gossip))
        .route("/connect", post(iroh_api::connect_peer))
        .route("/info", get(iroh_api::endpoint_info))
        // Apply API key authentication to all iroh routes
        .layer(axum::middleware::from_fn_with_state(auth_config, middleware::api_key_auth))
}

/// OpenTofu/Terraform State Backend API routes
///
/// HTTP backend protocol implementation for OpenTofu/Terraform remote state storage.
/// Provides distributed state management using Hiqlite for consistency.
///
/// State Backend Routes:
/// - `GET  /api/tofu/state/{workspace}` - Get current state
/// - `POST /api/tofu/state/{workspace}` - Update state
/// - `POST /api/tofu/lock/{workspace}` - Lock workspace
/// - `POST /api/tofu/unlock/{workspace}` - Unlock workspace
/// - `DELETE /api/tofu/lock/{workspace}` - Force unlock (admin)
/// - `GET  /api/tofu/workspaces` - List workspaces
/// - `DELETE /api/tofu/workspaces/{workspace}` - Delete workspace
/// - `GET  /api/tofu/history/{workspace}` - Get state history
/// - `POST /api/tofu/rollback/{workspace}/{version}` - Rollback state
///
/// Plan Execution Routes:
/// - `POST /api/tofu/plan` - Create and optionally execute plan
/// - `POST /api/tofu/apply/{plan_id}` - Apply stored plan
/// - `GET  /api/tofu/plans/{workspace}` - List plans for workspace
/// - `POST /api/tofu/destroy` - Destroy infrastructure
#[cfg(feature = "tofu-support")]
fn tofu_api_router(auth_config: Arc<crate::config::AuthConfig>) -> Router<FeaturesState> {
    use axum::routing::delete;

    Router::new()
        // State backend protocol endpoints
        .route("/state/{workspace}", get(get_state).post(update_state))
        .route("/lock/{workspace}", post(lock_workspace).delete(force_unlock_workspace))
        .route("/unlock/{workspace}", post(unlock_workspace))
        .route("/workspaces", get(list_workspaces))
        .route("/workspaces/{workspace}", delete(delete_workspace))
        .route("/history/{workspace}", get(get_state_history))
        .route("/rollback/{workspace}/{version}", post(rollback_state))
        // Plan execution endpoints
        .route("/plan", post(create_plan))
        .route("/apply/{plan_id}", post(apply_plan))
        .route("/plans/{workspace}", get(list_plans))
        .route("/destroy", post(destroy_infrastructure))
        // Apply API key authentication to all tofu routes
        .layer(axum::middleware::from_fn_with_state(auth_config, middleware::api_key_auth))
}

/// Health check routes
///
/// Endpoints for monitoring service health and readiness.
/// Used by load balancers, monitoring systems, and orchestrators.
///
/// Uses HealthService dependency.
///
/// Routes:
/// - `GET /health/hiqlite` - Hiqlite database health check
///
/// Future health checks:
/// - `/health/ready` - Readiness probe (can serve traffic)
/// - `/health/live` - Liveness probe (process is alive)
/// - `/health/startup` - Startup probe (initialization complete)
fn health_router(health_service: Arc<HealthService>) -> Router {
    Router::new()
        .route("/hiqlite", get(hiqlite_health).with_state(health_service))
    // Future: Add comprehensive health checks
    // .route("/ready", get(readiness_check))
    // .route("/live", get(liveness_check))
    // .route("/startup", get(startup_check))
}
