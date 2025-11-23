//! Queue statistics view model

use askama::Template;
use crate::domain::QueueStats;

/// View model for queue statistics display
#[derive(Template)]
#[template(path = "partials/queue_stats.html")]
pub struct QueueStatsView {
    pub pending: usize,
    pub in_progress: usize,
    pub completed: usize,
    pub failed: usize,
}

impl From<QueueStats> for QueueStatsView {
    fn from(stats: QueueStats) -> Self {
        Self {
            pending: stats.pending,
            in_progress: stats.in_progress,
            completed: stats.completed,
            failed: stats.failed,
        }
    }
}
