use openraft::BasicNode;
use openraft::Raft;
use openraft::error::{ClientWriteError, RaftError};
use openraft::raft::ClientWriteResponse;

use crate::kv::api;
use crate::kv::error::Error;
use crate::kv::store::KvStateMachine;
use crate::kv::types::{self, TypeConfig};

type ClientWriteResult = Result<
    ClientWriteResponse<TypeConfig>,
    RaftError<types::NodeId, ClientWriteError<types::NodeId, BasicNode>>,
>;

/// High-level client wrapper that hides the raw Raft handles.
#[derive(Clone)]
pub struct KvClient {
    raft: Raft<TypeConfig>,
    state_machine: KvStateMachine,
}

impl KvClient {
    pub fn new(raft: Raft<TypeConfig>, state_machine: KvStateMachine) -> Self {
        Self {
            raft,
            state_machine,
        }
    }

    pub fn raft(&self) -> Raft<TypeConfig> {
        self.raft.clone()
    }

    pub fn state_machine(&self) -> KvStateMachine {
        self.state_machine.clone()
    }

    pub async fn set(&self, key: String, value: String) -> ClientWriteResult {
        api::set(&self.raft, key, value).await
    }

    pub async fn delete(&self, key: String) -> ClientWriteResult {
        api::delete(&self.raft, key).await
    }

    pub async fn txn(&self, operations: Vec<types::TxnOp>) -> ClientWriteResult {
        api::txn(&self.raft, operations).await
    }

    pub async fn get(&self, key: &str) -> Result<Option<String>, Error> {
        match api::get(&self.raft, &self.state_machine, key).await? {
            types::Response::Get { value } => Ok(value),
            other => Err(Error::Raft {
                message: format!("unexpected response {other:?} for get"),
            }),
        }
    }

    pub async fn lease_grant(
        &self,
        lease_id: u64,
        ttl_ms: u64,
        timestamp_ms: u64,
    ) -> ClientWriteResult {
        api::lease_grant(&self.raft, lease_id, ttl_ms, timestamp_ms).await
    }

    pub async fn lease_attach(&self, lease_id: u64, key: String) -> ClientWriteResult {
        api::lease_attach(&self.raft, lease_id, key).await
    }

    pub async fn lease_keep_alive(&self, lease_id: u64, timestamp_ms: u64) -> ClientWriteResult {
        api::lease_keep_alive(&self.raft, lease_id, timestamp_ms).await
    }

    pub async fn lease_revoke(&self, lease_id: u64) -> ClientWriteResult {
        api::lease_revoke(&self.raft, lease_id).await
    }

    pub async fn watch_register(
        &self,
        watch_id: u64,
        key: String,
        prefix: bool,
    ) -> ClientWriteResult {
        api::watch_register(&self.raft, watch_id, key, prefix).await
    }

    pub async fn watch_cancel(&self, watch_id: u64) -> ClientWriteResult {
        api::watch_cancel(&self.raft, watch_id).await
    }

    pub async fn watch_fetch(&self, watch_id: u64) -> ClientWriteResult {
        api::watch_fetch(&self.raft, watch_id).await
    }
}
