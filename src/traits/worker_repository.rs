
use async_trait::async_trait;
use crate::types::worker::Worker;
use uuid::Uuid;

#[async_trait]
pub trait WorkerRepository: Send + Sync {
    async fn get_workers(&self) -> Result<Vec<Worker>, anyhow::Error>;
    async fn get_worker(&self, id: &Uuid) -> Result<Option<Worker>, anyhow::Error>;
}
