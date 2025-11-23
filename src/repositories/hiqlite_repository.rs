//! Hiqlite-based implementation of StateRepository

use anyhow::Result;
use async_trait::async_trait;

use crate::hiqlite_service::{HiqliteService, ClusterHealth};
use crate::repositories::StateRepository;

/// StateRepository implementation backed by HiqliteService
#[derive(Clone)]
pub struct HiqliteStateRepository {
    hiqlite: HiqliteService,
}

impl HiqliteStateRepository {
    /// Create a new Hiqlite-backed state repository
    pub fn new(hiqlite: HiqliteService) -> Self {
        Self { hiqlite }
    }
}

#[async_trait]
impl StateRepository for HiqliteStateRepository {
    async fn health_check(&self) -> Result<ClusterHealth> {
        self.hiqlite.health_check().await
    }
}
