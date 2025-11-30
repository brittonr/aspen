
use async_trait::async_trait;
use crate::types::{
    execution_handle::ExecutionHandle,
    job::Job,
    work_result::WorkResult,
};

#[async_trait]
pub trait ExecutionBackend: Send + Sync {
    async fn execute(&self, job: Job) -> Result<ExecutionHandle, anyhow::Error>;
    async fn get_status(&self, handle: &ExecutionHandle) -> Result<WorkResult, anyhow::Error>;
}
