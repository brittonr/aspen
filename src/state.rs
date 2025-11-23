//! Application state shared across all HTTP handlers

use std::sync::Arc;
use tokio::sync::Mutex;
use flawless_utils::DeployedModule;
use serde::{Deserialize, Serialize};

use crate::iroh_service::IrohService;
use crate::hiqlite_service::HiqliteService;
use crate::work_queue::WorkQueue;

/// Shared application state passed to all Axum handlers
#[derive(Debug, Clone)]
pub struct AppState {
    inner: Arc<StateInner>,
}

#[derive(Debug)]
struct StateInner {
    module: DeployedModule,
    progress: Mutex<Vec<JobProgress>>,
    iroh: IrohService,
    hiqlite: HiqliteService,
    work_queue: WorkQueue,
}

impl AppState {
    pub fn new(module: DeployedModule, iroh: IrohService, hiqlite: HiqliteService, work_queue: WorkQueue) -> Self {
        AppState {
            inner: Arc::new(StateInner {
                module,
                progress: Mutex::new(Vec::new()),
                iroh,
                hiqlite,
                work_queue,
            }),
        }
    }

    pub fn module(&self) -> &DeployedModule {
        &self.inner.module
    }

    pub fn iroh(&self) -> &IrohService {
        &self.inner.iroh
    }

    pub fn hiqlite(&self) -> &HiqliteService {
        &self.inner.hiqlite
    }

    pub fn work_queue(&self) -> &WorkQueue {
        &self.inner.work_queue
    }

    pub async fn progress(&self) -> Vec<JobProgress> {
        self.inner.progress.lock().await.clone()
    }

    pub async fn add_job(&self, url: String) -> usize {
        let mut progress = self.inner.progress.lock().await;
        progress.push(JobProgress {
            url,
            list: Vec::new(),
        });
        progress.len() - 1
    }

    pub async fn update_job(&self, id: usize, status: Status, url: String) {
        let mut progress = self.inner.progress.lock().await;
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

/// Job progress tracking for UI display
#[derive(Debug, Clone)]
pub struct JobProgress {
    pub url: String,
    /// Tuple of (color, status, url)
    pub list: Vec<(String, String, String)>,
}

/// Job status for UI updates
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateUI {
    pub id: usize,
    pub status: Status,
    pub url: String,
    pub urls_left: usize,
}
