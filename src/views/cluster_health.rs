//! Cluster health view model

use askama::Template;
use crate::domain::cluster_status::ClusterHealth;

/// View model for cluster health display
#[derive(Template)]
#[template(path = "partials/cluster_health.html")]
pub struct ClusterHealthView {
    pub is_healthy: bool,
    pub health_status_text: String,
    pub health_css_class: String,
    pub node_count: usize,
    pub leader_status: String,
    pub active_worker_count: usize,
}

impl From<ClusterHealth> for ClusterHealthView {
    fn from(health: ClusterHealth) -> Self {
        Self {
            is_healthy: health.is_healthy,
            health_status_text: if health.is_healthy {
                "Healthy".to_string()
            } else {
                "Unhealthy".to_string()
            },
            health_css_class: if health.is_healthy {
                "healthy".to_string()
            } else {
                "unhealthy".to_string()
            },
            node_count: health.node_count,
            leader_status: if health.has_leader {
                ", leader elected".to_string()
            } else {
                ", no leader".to_string()
            },
            active_worker_count: health.active_worker_count,
        }
    }
}
