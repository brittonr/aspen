//! Axum router configuration
//!
//! Centralizes all route definitions for the application.

use axum::{routing::{get, post}, Router};

use crate::handlers::*;
use crate::iroh_api;
use crate::state::AppState;

/// Build the complete Axum router with all routes
///
/// This creates a router serving both HTTP/1.1 (localhost) and HTTP/3 (iroh+h3 P2P).
/// All routes are registered here for consistency across transports.
pub fn build_router(state: &AppState) -> Router {
    Router::new()
        // Dashboard routes (HTMX monitoring UI - primary UI)
        .route("/dashboard", get(dashboard))
        .route("/dashboard/cluster-health", get(dashboard_cluster_health))
        .route("/dashboard/queue-stats", get(dashboard_queue_stats))
        .route("/dashboard/recent-jobs", get(dashboard_recent_jobs))
        .route(
            "/dashboard/control-plane-nodes",
            get(dashboard_control_plane_nodes),
        )
        .route("/dashboard/workers", get(dashboard_workers))
        .route("/dashboard/submit-job", post(dashboard_submit_job))
        // Iroh API routes (P2P blob storage and gossip)
        .route("/iroh/blob/store", post(iroh_api::store_blob))
        .route("/iroh/blob/{hash}", get(iroh_api::retrieve_blob))
        .route("/iroh/gossip/join", post(iroh_api::join_gossip_topic))
        .route("/iroh/gossip/broadcast", post(iroh_api::broadcast_gossip))
        .route(
            "/iroh/gossip/subscribe/{topic_id}",
            get(iroh_api::subscribe_gossip),
        )
        .route("/iroh/connect", post(iroh_api::connect_peer))
        .route("/iroh/info", get(iroh_api::endpoint_info))
        // Work Queue API routes (for workers)
        .route("/queue/publish", post(queue_publish))
        .route("/queue/claim", post(queue_claim))
        .route("/queue/list", get(queue_list))
        .route("/queue/stats", get(queue_stats))
        .route("/queue/status/{job_id}", post(queue_update_status))
        // Hiqlite API routes (health check)
        .route("/hiqlite/health", get(hiqlite_health))
        .with_state(state.clone())
}
