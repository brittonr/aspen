
use async_trait::async_trait;
use crate::types::{
    vm_config::VmConfig,
    vm_instance::VmInstance,
};
use uuid::Uuid;

#[async_trait]
pub trait VmManagement: Send + Sync {
    async fn create_vm(&self, config: VmConfig) -> Result<VmInstance, anyhow::Error>;
    async fn get_vm(&self, id: &Uuid) -> Result<Option<VmInstance>, anyhow::Error>;
    async fn start_vm(&self, id: &Uuid) -> Result<(), anyhow::Error>;
    async fn stop_vm(&self, id: &Uuid) -> Result<(), anyhow::Error>;
}
