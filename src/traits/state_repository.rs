
use async_trait::async_trait;
use crate::types::{
    job::Job,
    worker::Worker,
};
use uuid::Uuid;

#[async_trait]
pub trait StateRepository: Send + Sync {
    async fn get_job(&self, id: &Uuid) -> Result<Option<Job>, anyhow::Error>;
    async fn put_job(&self, job: Job) -> Result<(), anyhow::Error>;
    async fn get_worker(&self, id: &Uuid) -> Result<Option<Worker>, anyhow::Error>;
    async fn put_worker(&self, worker: Worker) -> Result<(), anyhow::Error>;
}
