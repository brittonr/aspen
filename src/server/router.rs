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

use crate::handlers::*;
use crate::iroh_api;
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
    Router::new()
        .nest("/dashboard", dashboard_router())
        .nest("/api/queue", queue_api_router())
        .nest("/api/iroh", iroh_api_router())
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
    // Future: Add API-specific middleware
    // .layer(/* rate limiting, API versioning, auth, etc. */)
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
    // Future: Add P2P-specific middleware
    // .layer(/* peer authentication, encryption verification, etc. */)
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
