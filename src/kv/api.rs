use openraft::BasicNode;
use openraft::Raft;
use openraft::error::{ClientWriteError, RaftError};
use openraft::raft::ClientWriteResponse;

use crate::kv::error::Error;
use crate::kv::store::KvStateMachine;
use crate::kv::types::{self, TypeConfig};

type ClientWriteResult = Result<
    ClientWriteResponse<TypeConfig>,
    RaftError<types::NodeId, ClientWriteError<types::NodeId, BasicNode>>,
>;

pub async fn set(raft: &Raft<TypeConfig>, key: String, value: String) -> ClientWriteResult {
    let request = types::Request::Set { key, value };
    raft.client_write(request).await
}

pub async fn delete(raft: &Raft<TypeConfig>, key: String) -> ClientWriteResult {
    let request = types::Request::Delete { key };
    raft.client_write(request).await
}

pub async fn txn(raft: &Raft<TypeConfig>, operations: Vec<types::TxnOp>) -> ClientWriteResult {
    let request = types::Request::Txn { operations };
    raft.client_write(request).await
}

pub async fn lease_grant(
    raft: &Raft<TypeConfig>,
    lease_id: u64,
    ttl_ms: u64,
    timestamp_ms: u64,
) -> ClientWriteResult {
    let request = types::Request::LeaseGrant {
        lease_id,
        ttl_ms,
        timestamp_ms,
    };
    raft.client_write(request).await
}

pub async fn lease_attach(
    raft: &Raft<TypeConfig>,
    lease_id: u64,
    key: String,
) -> ClientWriteResult {
    let request = types::Request::LeaseAttach { lease_id, key };
    raft.client_write(request).await
}

pub async fn lease_keep_alive(
    raft: &Raft<TypeConfig>,
    lease_id: u64,
    timestamp_ms: u64,
) -> ClientWriteResult {
    let request = types::Request::LeaseKeepAlive {
        lease_id,
        timestamp_ms,
    };
    raft.client_write(request).await
}

pub async fn lease_revoke(raft: &Raft<TypeConfig>, lease_id: u64) -> ClientWriteResult {
    let request = types::Request::LeaseRevoke { lease_id };
    raft.client_write(request).await
}

pub async fn watch_register(
    raft: &Raft<TypeConfig>,
    watch_id: u64,
    key: String,
    prefix: bool,
) -> ClientWriteResult {
    let request = types::Request::WatchRegister {
        watch_id,
        key,
        prefix,
    };
    raft.client_write(request).await
}

pub async fn watch_cancel(raft: &Raft<TypeConfig>, watch_id: u64) -> ClientWriteResult {
    let request = types::Request::WatchCancel { watch_id };
    raft.client_write(request).await
}

pub async fn watch_fetch(raft: &Raft<TypeConfig>, watch_id: u64) -> ClientWriteResult {
    let request = types::Request::WatchFetch { watch_id };
    raft.client_write(request).await
}

pub async fn get(
    raft: &Raft<TypeConfig>,
    sm: &KvStateMachine,
    key: &str,
) -> Result<types::Response, Error> {
    raft.ensure_linearizable().await.map_err(|e| Error::Raft {
        message: e.to_string(),
    })?;
    let value = sm.get(key).await?;
    Ok(types::Response::Get { value })
}
