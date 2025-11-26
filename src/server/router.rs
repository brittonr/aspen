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

use axum::{routing::{get, post, delete}, Router};

#[cfg(feature = "tofu-support")]
use crate::api::tofu_handlers::*;
use crate::handlers::dashboard::*;
use crate::handlers::queue::*;
use crate::handlers::worker::*;
use crate::iroh_api;
use crate::middleware;
use crate::state::AppState;

/// Build the complete Axum router with all routes
///
/// This creates a router serving both HTTP/1.1 (localhost) and HTTP/3 (iroh+h3 P2P).
/// All routes are registered here for consistency across transports.
///
/// Routes are organized into logical sub-routers:
/// - Dashboard UI (monitoring, job submission)
/// - Work Queue API (worker operations)
/// - Iroh P2P API (blob storage, gossip)
/// - Health checks
pub fn build_router(state: &AppState) -> Router {
    let router = Router::new()
        .nest("/dashboard", dashboard_router())
        .nest("/api/queue", queue_api_router())
        .nest("/api/workers", worker_api_router())
        .nest("/api/iroh", iroh_api_router());

    #[cfg(feature = "tofu-support")]
    let router = router.nest("/api/tofu", tofu_api_router());

    router
        .nest("/health", health_router())
        .with_state(state.clone())
}

/// Dashboard routes - HTMX monitoring UI
///
/// Primary user interface for cluster monitoring and job management.
/// Renders HTML fragments for real-time updates via HTMX.
///
/// Routes:
/// - `GET  /dashboard` - Main dashboard page
/// - `GET  /dashboard/cluster-health` - Cluster health HTMX fragment
/// - `GET  /dashboard/queue-stats` - Queue statistics HTMX fragment
/// - `GET  /dashboard/recent-jobs` - Recent jobs list HTMX fragment
/// - `GET  /dashboard/control-plane-nodes` - Control plane nodes HTMX fragment
/// - `GET  /dashboard/workers` - Worker nodes HTMX fragment
/// - `POST /dashboard/submit-job` - Submit new job via web UI
fn dashboard_router() -> Router<AppState> {
    Router::new()
        .route("/", get(dashboard))
        .route("/cluster-health", get(dashboard_cluster_health))
        .route("/queue-stats", get(dashboard_queue_stats))
        .route("/recent-jobs", get(dashboard_recent_jobs))
        .route("/control-plane-nodes", get(dashboard_control_plane_nodes))
        .route("/workers", get(dashboard_workers))
        .route("/submit-job", post(dashboard_submit_job))
    // Future: Add dashboard-specific middleware
    // .layer(/* HTMX headers, caching policy, etc. */)
}

/// Work Queue API routes - Worker operations
///
/// REST API for distributed workers to claim and process jobs.
/// Used by worker binaries and CLI tools.
///
/// Routes:
/// - `POST /api/queue/publish` - Submit new job to queue
/// - `POST /api/queue/claim` - Claim next available job
/// - `GET  /api/queue/list` - List all jobs
/// - `GET  /api/queue/stats` - Get queue statistics
/// - `POST /api/queue/status/{job_id}` - Update job status
fn queue_api_router() -> Router<AppState> {
    Router::new()
        .route("/publish", post(queue_publish))
        .route("/claim", post(queue_claim))
        .route("/list", get(queue_list))
        .route("/stats", get(queue_stats))
        .route("/status/{job_id}", post(queue_update_status))
        // Apply API key authentication to all queue routes
        .layer(axum::middleware::from_fn(middleware::api_key_auth))
}

/// Worker Management API routes - Worker lifecycle
///
/// REST API for worker registration, heartbeats, and management.
/// Used by worker binaries to register with the control plane.
///
/// Routes:
/// - `POST /api/workers/register` - Register a new worker
/// - `POST /api/workers/{worker_id}/heartbeat` - Send worker heartbeat
/// - `GET  /api/workers` - List all workers
/// - `GET  /api/workers/{worker_id}` - Get worker details
/// - `POST /api/workers/{worker_id}/drain` - Mark worker as draining
/// - `GET  /api/workers/stats` - Get worker pool statistics
fn worker_api_router() -> Router<AppState> {
    Router::new()
        .route("/register", post(worker_register))
        .route("/{worker_id}/heartbeat", post(worker_heartbeat))
        .route("/", get(worker_list))
        .route("/{worker_id}", get(worker_get))
        .route("/{worker_id}/drain", post(worker_drain))
        .route("/stats", get(worker_stats))
        // Apply API key authentication to all worker routes
        .layer(axum::middleware::from_fn(middleware::api_key_auth))
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
fn iroh_api_router() -> Router<AppState> {
    Router::new()
        .route("/blob/store", post(iroh_api::store_blob))
        .route("/blob/{hash}", get(iroh_api::retrieve_blob))
        .route("/gossip/join", post(iroh_api::join_gossip_topic))
        .route("/gossip/broadcast", post(iroh_api::broadcast_gossip))
        .route("/gossip/subscribe/{topic_id}", get(iroh_api::subscribe_gossip))
        .route("/connect", post(iroh_api::connect_peer))
        .route("/info", get(iroh_api::endpoint_info))
        // Apply API key authentication to all iroh routes
        .layer(axum::middleware::from_fn(middleware::api_key_auth))
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
fn tofu_api_router() -> Router<AppState> {
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
        .layer(axum::middleware::from_fn(middleware::api_key_auth))
}

/// Health check routes
///
/// Endpoints for monitoring service health and readiness.
/// Used by load balancers, monitoring systems, and orchestrators.
///
/// Routes:
/// - `GET /health/hiqlite` - Hiqlite database health check
///
/// Future health checks:
/// - `/health/ready` - Readiness probe (can serve traffic)
/// - `/health/live` - Liveness probe (process is alive)
/// - `/health/startup` - Startup probe (initialization complete)
fn health_router() -> Router<AppState> {
    Router::new()
        .route("/hiqlite", get(hiqlite_health))
    // Future: Add comprehensive health checks
    // .route("/ready", get(readiness_check))
    // .route("/live", get(liveness_check))
    // .route("/startup", get(startup_check))
}
