use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use super::{
    AddLearnerRequest, ChangeMembershipRequest, ClusterController, ClusterNode, ClusterState,
    ControlPlaneError, InitRequest, KeyValueStore, KeyValueStoreError, ReadRequest, ReadResult,
    WriteCommand, WriteRequest, WriteResult,
};

#[derive(Clone, Default)]
pub struct DeterministicClusterController {
    state: Arc<Mutex<ClusterState>>,
}

impl DeterministicClusterController {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
}

#[async_trait]
impl ClusterController for DeterministicClusterController {
    async fn init(&self, request: InitRequest) -> Result<ClusterState, ControlPlaneError> {
        if request.initial_members.is_empty() {
            return Err(ControlPlaneError::InvalidRequest {
                reason: "initial_members must not be empty".into(),
            });
        }
        let mut guard = self.state.lock().await;
        guard.nodes = request.initial_members.clone();
        guard.members = request.initial_members.iter().map(|node| node.id).collect();
        Ok(guard.clone())
    }

    async fn add_learner(
        &self,
        request: AddLearnerRequest,
    ) -> Result<ClusterState, ControlPlaneError> {
        let mut guard = self.state.lock().await;
        guard.learners.push(request.learner);
        Ok(guard.clone())
    }

    async fn change_membership(
        &self,
        request: ChangeMembershipRequest,
    ) -> Result<ClusterState, ControlPlaneError> {
        if request.members.is_empty() {
            return Err(ControlPlaneError::InvalidRequest {
                reason: "members must include at least one voter".into(),
            });
        }
        let mut guard = self.state.lock().await;
        guard.members = request.members;
        Ok(guard.clone())
    }

    async fn current_state(&self) -> Result<ClusterState, ControlPlaneError> {
        Ok(self.state.lock().await.clone())
    }
}

#[derive(Clone, Default)]
pub struct DeterministicKeyValueStore {
    inner: Arc<Mutex<HashMap<String, String>>>,
}

impl DeterministicKeyValueStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
}

#[async_trait]
impl KeyValueStore for DeterministicKeyValueStore {
    async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError> {
        let mut inner = self.inner.lock().await;
        match request.command.clone() {
            WriteCommand::Set { key, value } => {
                inner.insert(key.clone(), value.clone());
                Ok(WriteResult {
                    command: WriteCommand::Set { key, value },
                })
            }
            WriteCommand::SetMulti { ref pairs } => {
                for (key, value) in pairs {
                    inner.insert(key.clone(), value.clone());
                }
                Ok(WriteResult {
                    command: request.command.clone(),
                })
            }
        }
    }

    async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError> {
        let guard = self.inner.lock().await;
        match guard.get(&request.key) {
            Some(value) => Ok(ReadResult {
                key: request.key,
                value: value.clone(),
            }),
            None => Err(KeyValueStoreError::NotFound { key: request.key }),
        }
    }
}
