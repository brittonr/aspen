//! `KeyValueStore` adapter backed by Aspen client RPC.
//!
//! Translates `KeyValueStore` trait calls into `ClientRpcRequest` messages
//! sent via an `AspenClient`. This lets daemon-side components (`ServiceRegistry`,
//! `NameResolver`) work against a remote Raft cluster without direct KV access.

use std::sync::Arc;

use aspen_client::AspenClient;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_kv_types::DeleteRequest;
use aspen_kv_types::DeleteResult;
use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::KeyValueWithRevision;
use aspen_kv_types::ReadRequest;
use aspen_kv_types::ReadResult;
use aspen_kv_types::ScanRequest;
use aspen_kv_types::ScanResult;
use aspen_kv_types::WriteRequest;
use aspen_kv_types::WriteResult;
use aspen_traits::KeyValueStore;
use aspen_traits::KvDelete;
use aspen_traits::KvRead;
use aspen_traits::KvScan;
use aspen_traits::KvWrite;
use async_trait::async_trait;

/// A `KeyValueStore` that delegates to an `AspenClient` via RPC.
///
/// Each KV operation is translated into the corresponding `ClientRpcRequest`
/// variant, sent to the cluster, and the response is translated back into
/// the `KeyValueStore` result types.
pub struct ClientKvAdapter {
    client: Arc<AspenClient>,
}

impl ClientKvAdapter {
    /// Create a new adapter wrapping the given client.
    pub fn new(client: Arc<AspenClient>) -> Self {
        Self { client }
    }
}

#[derive(Clone, Copy)]
struct RpcErrorMessage<'a> {
    code: &'a str,
    message: &'a str,
}

/// Map an RPC transport error to a KV store error.
fn rpc_err(e: impl std::fmt::Display) -> KeyValueStoreError {
    KeyValueStoreError::Failed { reason: e.to_string() }
}

/// Map an application-level error response to a KV store error.
fn resp_err(error: RpcErrorMessage<'_>) -> KeyValueStoreError {
    KeyValueStoreError::Failed {
        reason: format!("{}: {}", error.code, error.message),
    }
}

fn empty_write_result() -> WriteResult {
    WriteResult {
        command: None,
        batch_applied: None,
        conditions_met: None,
        failed_condition_index: None,
        lease_id: None,
        ttl_seconds: None,
        keys_deleted: None,
        succeeded: None,
        txn_results: None,
        header_revision: None,
        occ_conflict: None,
        conflict_key: None,
        conflict_expected_version: None,
        conflict_actual_version: None,
    }
}

#[async_trait]
impl KvWrite for ClientKvAdapter {
    async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError> {
        // Extract key and value from the WriteCommand
        let (key, value) = match &request.command {
            aspen_kv_types::WriteCommand::Set { key, value } => (key.clone(), value.as_bytes().to_vec()),
            other => {
                return Err(KeyValueStoreError::Failed {
                    reason: format!("unsupported write command for RPC adapter: {other:?}"),
                });
            }
        };

        let rpc = ClientRpcRequest::WriteKey { key, value };
        let resp = self.client.send(rpc).await.map_err(rpc_err)?;
        match resp {
            ClientRpcResponse::WriteResult(r) => {
                if let Some(err) = r.error {
                    return Err(KeyValueStoreError::Failed { reason: err });
                }
                Ok(WriteResult {
                    command: Some(request.command),
                    succeeded: Some(r.is_success),
                    ..empty_write_result()
                })
            }
            ClientRpcResponse::Error(e) => Err(resp_err(RpcErrorMessage {
                code: &e.code,
                message: &e.message,
            })),
            other => Err(KeyValueStoreError::Failed {
                reason: format!("unexpected response: {other:?}"),
            }),
        }
    }

}

#[async_trait]
impl KvRead for ClientKvAdapter {
    async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError> {
        let rpc = ClientRpcRequest::ReadKey {
            key: request.key.clone(),
        };
        let resp = self.client.send(rpc).await.map_err(rpc_err)?;
        match resp {
            ClientRpcResponse::ReadResult(r) => {
                if let Some(err) = r.error {
                    return Err(KeyValueStoreError::Failed { reason: err });
                }
                let kv = if r.was_found {
                    r.value.map(|v| KeyValueWithRevision {
                        key: request.key.clone(),
                        value: String::from_utf8_lossy(&v).into_owned(),
                        version: 0,
                        create_revision: 0,
                        mod_revision: 0,
                    })
                } else {
                    None
                };
                Ok(ReadResult { kv })
            }
            ClientRpcResponse::Error(e) => Err(resp_err(RpcErrorMessage {
                code: &e.code,
                message: &e.message,
            })),
            other => Err(KeyValueStoreError::Failed {
                reason: format!("unexpected response: {other:?}"),
            }),
        }
    }
}

#[async_trait]
impl KvDelete for ClientKvAdapter {
    async fn delete(&self, request: DeleteRequest) -> Result<DeleteResult, KeyValueStoreError> {
        let rpc = ClientRpcRequest::DeleteKey {
            key: request.key.clone(),
        };
        let resp = self.client.send(rpc).await.map_err(rpc_err)?;
        match resp {
            ClientRpcResponse::DeleteResult(r) => {
                if let Some(err) = r.error {
                    return Err(KeyValueStoreError::Failed { reason: err });
                }
                Ok(DeleteResult {
                    key: request.key,
                    is_deleted: r.was_deleted,
                })
            }
            ClientRpcResponse::Error(e) => Err(resp_err(RpcErrorMessage {
                code: &e.code,
                message: &e.message,
            })),
            other => Err(KeyValueStoreError::Failed {
                reason: format!("unexpected response: {other:?}"),
            }),
        }
    }
}

#[async_trait]
impl KvScan for ClientKvAdapter {
    async fn scan(&self, request: ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
        let rpc = ClientRpcRequest::ScanKeys {
            prefix: request.prefix.clone(),
            limit: request.limit_results,
            continuation_token: request.continuation_token.clone(),
        };
        let resp = self.client.send(rpc).await.map_err(rpc_err)?;
        match resp {
            ClientRpcResponse::ScanResult(r) => {
                if let Some(err) = r.error {
                    return Err(KeyValueStoreError::Failed { reason: err });
                }
                let result_count = r.count;
                let entries = r
                    .entries
                    .into_iter()
                    .map(|e| KeyValueWithRevision {
                        key: e.key,
                        value: e.value,
                        version: e.version,
                        create_revision: e.create_revision,
                        mod_revision: e.mod_revision,
                    })
                    .collect();
                Ok(ScanResult {
                    entries,
                    result_count,
                    is_truncated: r.is_truncated,
                    continuation_token: r.continuation_token,
                })
            }
            ClientRpcResponse::Error(e) => Err(resp_err(RpcErrorMessage {
                code: &e.code,
                message: &e.message,
            })),
            other => Err(KeyValueStoreError::Failed {
                reason: format!("unexpected response: {other:?}"),
            }),
        }
    }
}

impl KeyValueStore for ClientKvAdapter {}

#[cfg(test)]
mod tests {
    use aspen_kv_types::WriteRequest;

    use super::*;

    #[test]
    fn write_request_to_rpc() {
        let req = WriteRequest::set("test/key", "hello");
        let (key, value) = match &req.command {
            aspen_kv_types::WriteCommand::Set { key, value } => (key.clone(), value.as_bytes().to_vec()),
            _ => panic!("expected Set"),
        };
        let rpc = ClientRpcRequest::WriteKey { key, value };
        match rpc {
            ClientRpcRequest::WriteKey { key, value } => {
                assert_eq!(key, "test/key");
                assert_eq!(value, b"hello");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn read_request_to_rpc() {
        let req = ReadRequest::new("test/key");
        let rpc = ClientRpcRequest::ReadKey { key: req.key.clone() };
        match rpc {
            ClientRpcRequest::ReadKey { key } => {
                assert_eq!(key, "test/key");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn scan_request_to_rpc() {
        let req = ScanRequest {
            prefix: "/_sys/net/svc/".to_string(),
            limit_results: Some(100),
            continuation_token: None,
        };
        let rpc = ClientRpcRequest::ScanKeys {
            prefix: req.prefix.clone(),
            limit: req.limit_results,
            continuation_token: req.continuation_token.clone(),
        };
        match rpc {
            ClientRpcRequest::ScanKeys {
                prefix,
                limit,
                continuation_token,
            } => {
                assert_eq!(prefix, "/_sys/net/svc/");
                assert_eq!(limit, Some(100));
                assert!(continuation_token.is_none());
            }
            _ => panic!("wrong variant"),
        }
    }
}
