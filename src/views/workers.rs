//! Workers view model

use askama::Template;
use crate::domain::cluster_status::WorkerStats;
use crate::domain::format_time_ago;

/// Individual worker for display
#[derive(Debug, Clone)]
pub struct WorkerRow {
    pub node_id_short: String,
    pub is_active: bool,
    pub active_css_class: String,
    pub active_jobs: usize,
    pub completed_jobs: usize,
    pub last_seen: String,
}

impl From<WorkerStats> for WorkerRow {
    fn from(worker: WorkerStats) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let time_ago = format_time_ago(now - worker.last_seen_timestamp);

        Self {
            node_id_short: worker.node_id[..std::cmp::min(32, worker.node_id.len())].to_string(),
            is_active: worker.is_active,
            active_css_class: if worker.is_active {
                " active".to_string()
            } else {
                String::new()
            },
            active_jobs: worker.active_jobs,
            completed_jobs: worker.completed_jobs,
            last_seen: time_ago,
        }
    }
}

/// View model for workers list
#[derive(Template)]
#[template(path = "partials/workers.html")]
pub struct WorkersView {
    pub workers: Vec<WorkerRow>,
}

impl WorkersView {
    pub fn new(workers: Vec<WorkerStats>) -> Self {
        Self {
            workers: workers.into_iter().map(WorkerRow::from).collect(),
        }
    }

    pub fn empty() -> Self {
        Self {
            workers: Vec::new(),
        }
    }
}
