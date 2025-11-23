// Flawless WASM Worker Backend
//
// Executes jobs using the Flawless framework (WASM workflows)

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use flawless_utils::{DeployedModule, Server};

use crate::worker_trait::{WorkerBackend, WorkResult};
use crate::WorkItem;

/// Worker backend that executes jobs using Flawless WASM workflows
pub struct FlawlessWorker {
    module: DeployedModule,
}

impl FlawlessWorker {
    /// Create a new Flawless worker
    ///
    /// # Arguments
    /// * `flawless_url` - URL of the Flawless server (e.g., "http://localhost:27288")
    pub async fn new(flawless_url: &str) -> Result<Self> {
        tracing::info!(flawless_url = %flawless_url, "Connecting to Flawless server");

        let flawless = Server::new(flawless_url, None);
        let flawless_module = flawless_utils::load_module_from_build!("module1");
        let module = flawless
            .deploy(flawless_module)
            .await
            .map_err(|e| anyhow!("Failed to deploy Flawless module: {}", e))?;

        tracing::info!("Flawless module deployed successfully");

        Ok(Self { module })
    }
}

#[async_trait]
impl WorkerBackend for FlawlessWorker {
    async fn execute(&self, job: WorkItem) -> Result<WorkResult> {
        // Parse the job payload to get id and url
        let payload = &job.payload;

        let id = payload
            .get("id")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("Missing 'id' in job payload"))?
            as usize;

        let url = payload
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'url' in job payload"))?
            .to_string();

        tracing::info!(job_id = %job.job_id, url = %url, "Executing Flawless workflow");

        // Execute the workflow via Flawless
        match self
            .module
            .start::<module1::start_crawler>(module1::Job { id, url: url.clone() })
            .await
        {
            Ok(_) => {
                tracing::info!(job_id = %job.job_id, "Workflow completed successfully");
                Ok(WorkResult::success())
            }
            Err(e) => {
                tracing::error!(job_id = %job.job_id, error = %e, "Workflow execution failed");
                Ok(WorkResult::failure(format!("Workflow failed: {}", e)))
            }
        }
    }
}
