// Repository Builder - Creates data access layer
//
// Responsibilities:
// - Create StateRepository
// - Create WorkRepository with PersistentStore
// - Create WorkerRepository
//
// All repositories are independent and can be built in parallel.

use anyhow::Result;
use std::sync::Arc;

use crate::hiqlite::HiqliteService;
use crate::hiqlite_persistent_store::HiqlitePersistentStore;
use crate::repositories::{
    HiqliteStateRepository, HiqliteWorkerRepository, StateRepository, WorkRepository,
    WorkerRepository,
};
use crate::work::WorkRepositoryImpl;

use super::components::RepositoryComponents;

pub struct RepositoryBuilder {
    hiqlite: Arc<HiqliteService>,
}

impl RepositoryBuilder {
    pub fn new(hiqlite: Arc<HiqliteService>) -> Self {
        Self { hiqlite }
    }

    /// Build all repository components
    pub async fn build(self) -> Result<RepositoryComponents> {
        let state = self.build_state_repo();
        let work = self.build_work_repo().await?;
        let worker = self.build_worker_repo();

        Ok(RepositoryComponents {
            state,
            work,
            worker,
        })
    }

    fn build_state_repo(&self) -> Arc<dyn StateRepository> {
        Arc::new(HiqliteStateRepository::new(self.hiqlite.clone()))
    }

    async fn build_work_repo(&self) -> Result<Arc<dyn WorkRepository>> {
        let persistent_store =
            Arc::new(HiqlitePersistentStore::new(self.hiqlite.as_ref().clone()));
        Ok(Arc::new(WorkRepositoryImpl::new(persistent_store)))
    }

    fn build_worker_repo(&self) -> Arc<dyn WorkerRepository> {
        Arc::new(HiqliteWorkerRepository::new(self.hiqlite.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::mocks::{
        MockStateRepository, MockWorkRepository, MockWorkerRepository,
    };

    pub struct TestRepositoryBuilder;

    impl TestRepositoryBuilder {
        pub fn new() -> Self {
            Self
        }

        pub async fn build(self) -> Result<RepositoryComponents> {
            Ok(RepositoryComponents {
                state: Arc::new(MockStateRepository::new()),
                work: Arc::new(MockWorkRepository::new()),
                worker: Arc::new(MockWorkerRepository::new()),
            })
        }
    }
}
