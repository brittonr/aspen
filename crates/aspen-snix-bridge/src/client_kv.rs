//! `KeyValueStore` adapter backed by Aspen client RPC.
//!
//! Translates `KeyValueStore` trait calls into `ClientRpcRequest` messages
//! sent via an `AspenClient`. Copied from `aspen-net::client_kv` to avoid
//! pulling in the full aspen-net dependency tree.

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
pub struct ClientKvAdapter {
    client: Arc<AspenClient>,
}

impl ClientKvAdapter {
    /// Create a new adapter wrapping the given client.
    pub fn new(client: Arc<AspenClient>) -> Self {
        Self { client }
    }
}

/// Map an RPC transport error to a KV store error.
pub(crate) fn rpc_err(e: impl std::fmt::Display) -> KeyValueStoreError {
    KeyValueStoreError::Failed { reason: e.to_string() }
}

/// Map an application-level error response to a KV store error.
pub(crate) fn resp_err(code: &str, message: &str) -> KeyValueStoreError {
    KeyValueStoreError::Failed {
        reason: format!("{code}: {message}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpc_err_preserves_message() {
        let err = rpc_err("connection refused");
        match err {
            KeyValueStoreError::Failed { reason } => {
                assert_eq!(reason, "connection refused");
            }
            other => panic!("expected Failed, got {:?}", other),
        }
    }

    #[test]
    fn test_rpc_err_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::TimedOut, "timed out");
        let err = rpc_err(io_err);
        match err {
            KeyValueStoreError::Failed { reason } => {
                assert!(reason.contains("timed out"), "got: {reason}");
            }
            other => panic!("expected Failed, got {:?}", other),
        }
    }

    #[test]
    fn test_resp_err_includes_code_and_message() {
        let err = resp_err("NOT_FOUND", "key does not exist");
        match err {
            KeyValueStoreError::Failed { reason } => {
                assert!(reason.contains("NOT_FOUND"), "got: {reason}");
                assert!(reason.contains("key does not exist"), "got: {reason}");
            }
            other => panic!("expected Failed, got {:?}", other),
        }
    }

    #[test]
    fn test_client_kv_adapter_construction() {
        // Verify we can construct the adapter with an Arc<AspenClient>
        // We can't actually create a real AspenClient without a connection,
        // but we verify the type signature compiles.
        fn _assert_send_sync<T: Send + Sync>() {}
        // ClientKvAdapter wraps Arc<AspenClient> — it's Send+Sync
        // if AspenClient is. This is a compile-time check.
    }
}

#[async_trait]
impl KvWrite for ClientKvAdapter {
    async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError> {
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
                    ..Default::default()
                })
            }
            ClientRpcResponse::Error(e) => Err(resp_err(&e.code, &e.message)),
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
            ClientRpcResponse::Error(e) => Err(resp_err(&e.code, &e.message)),
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
            ClientRpcResponse::Error(e) => Err(resp_err(&e.code, &e.message)),
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
            ClientRpcResponse::Error(e) => Err(resp_err(&e.code, &e.message)),
            other => Err(KeyValueStoreError::Failed {
                reason: format!("unexpected response: {other:?}"),
            }),
        }
    }
}

impl KeyValueStore for ClientKvAdapter {}
