
use async_trait::async_trait;
use crate::types::{
    job::Job,
    work_result::WorkResult,
};
use uuid::Uuid;

#[async_trait]
pub trait WorkRepository: Send + Sync {
    async fn get_work(&self, worker_id: &Uuid) -> Result<Option<Job>, anyhow::Error>;
    async fn submit_work_result(&self, result: WorkResult) -> Result<(), anyhow::Error>;
}
