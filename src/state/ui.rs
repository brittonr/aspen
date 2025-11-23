//! UI state container
//!
//! Holds UI-specific state (legacy job progress tracking for the old job list UI).

use std::sync::Arc;
use tokio::sync::Mutex;

/// Tuple of (color, status, url) for UI display
type ProgressEntry = (String, String, String);

/// Job progress tracking for UI display
#[derive(Debug, Clone)]
pub struct JobProgress {
    pub url: String,
    pub list: Vec<ProgressEntry>,
}

/// Job status for UI updates
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Status {
    /// Request in progress
    Request,
    /// Parsing in progress
    Parse,
    /// Finished
    Done,
    /// Error
    Error,
}

impl Status {
    pub fn to_string(&self) -> String {
        match self {
            Status::Request => "REQ".to_string(),
            Status::Parse => "PARS".to_string(),
            Status::Done => "DONE".to_string(),
            Status::Error => "ERR".to_string(),
        }
    }

    pub fn to_color(&self) -> String {
        match self {
            Status::Request => "orange".to_string(),
            Status::Parse => "yellow".to_string(),
            Status::Done => "green".to_string(),
            Status::Error => "red".to_string(),
        }
    }
}

/// UI update message from workflow
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct UpdateUI {
    pub id: usize,
    pub status: Status,
    pub url: String,
    pub urls_left: usize,
}

/// Container for UI-specific state
///
/// This holds the legacy job progress tracking used by the old job list UI.
/// The dashboard uses the distributed work queue instead.
#[derive(Clone)]
pub struct UiState {
    progress: Arc<Mutex<Vec<JobProgress>>>,
}

impl UiState {
    /// Create a new UI state container
    pub fn new() -> Self {
        Self {
            progress: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get all job progress entries
    pub async fn progress(&self) -> Vec<JobProgress> {
        self.progress.lock().await.clone()
    }

    /// Add a new job to track
    pub async fn add_job(&self, url: String) -> usize {
        let mut progress = self.progress.lock().await;
        progress.push(JobProgress {
            url,
            list: Vec::new(),
        });
        progress.len() - 1
    }

    /// Update a job's status
    pub async fn update_job(&self, id: usize, status: Status, url: String) {
        let mut progress = self.progress.lock().await;
        let job = progress.get_mut(id).unwrap();

        let list_element = if url == "last" {
            job.list.last_mut()
        } else {
            None
        };

        match list_element {
            Some(list_element) => {
                list_element.0 = status.to_color();
                list_element.1 = status.to_string();
            }
            None => job.list.push((status.to_color(), status.to_string(), url)),
        }
    }
}
