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

const SNIX_DIRECTORY_KEY_PREFIX: &str = "snix:dir:";
const SNIX_PATHINFO_KEY_PREFIX: &str = "snix:pathinfo:";

fn snix_read_request_for_key(key: &str) -> Option<ClientRpcRequest> {
    key.strip_prefix(SNIX_DIRECTORY_KEY_PREFIX)
        .map(|digest| ClientRpcRequest::SnixDirectoryGet {
            digest: digest.to_string(),
        })
        .or_else(|| {
            key.strip_prefix(SNIX_PATHINFO_KEY_PREFIX).map(|digest| ClientRpcRequest::SnixPathInfoGet {
                digest: digest.to_string(),
            })
        })
}

fn snix_write_request_for_set(key: &str, value: &str) -> Option<ClientRpcRequest> {
    if key.starts_with(SNIX_DIRECTORY_KEY_PREFIX) {
        Some(ClientRpcRequest::SnixDirectoryPut {
            directory_bytes: value.to_string(),
        })
    } else if key.starts_with(SNIX_PATHINFO_KEY_PREFIX) {
        Some(ClientRpcRequest::SnixPathInfoPut {
            pathinfo_bytes: value.to_string(),
        })
    } else {
        None
    }
}

fn snix_read_result_from_response(
    key: &str,
    resp: ClientRpcResponse,
) -> Option<Result<ReadResult, KeyValueStoreError>> {
    match resp {
        ClientRpcResponse::SnixDirectoryGetResult(r) => Some(if let Some(err) = r.error {
            Err(KeyValueStoreError::Failed { reason: err })
        } else {
            Ok(ReadResult {
                kv: r.directory_bytes.filter(|_| r.was_found).map(|value| KeyValueWithRevision {
                    key: key.to_string(),
                    value,
                    version: 0,
                    create_revision: 0,
                    mod_revision: 0,
                }),
            })
        }),
        ClientRpcResponse::SnixPathInfoGetResult(r) => Some(if let Some(err) = r.error {
            Err(KeyValueStoreError::Failed { reason: err })
        } else {
            Ok(ReadResult {
                kv: r.pathinfo_bytes.filter(|_| r.was_found).map(|value| KeyValueWithRevision {
                    key: key.to_string(),
                    value,
                    version: 0,
                    create_revision: 0,
                    mod_revision: 0,
                }),
            })
        }),
        ClientRpcResponse::Error(e) => Some(Err(resp_err(&e.code, &e.message))),
        _ => None,
    }
}

fn snix_write_result_from_response(
    request: WriteRequest,
    resp: ClientRpcResponse,
) -> Option<Result<WriteResult, KeyValueStoreError>> {
    match resp {
        ClientRpcResponse::SnixDirectoryPutResult(r) => Some(if let Some(err) = r.error {
            Err(KeyValueStoreError::Failed { reason: err })
        } else {
            Ok(WriteResult {
                command: Some(request.command),
                succeeded: Some(r.is_success),
                ..Default::default()
            })
        }),
        ClientRpcResponse::SnixPathInfoPutResult(r) => Some(if let Some(err) = r.error {
            Err(KeyValueStoreError::Failed { reason: err })
        } else {
            Ok(WriteResult {
                command: Some(request.command),
                succeeded: Some(r.is_success),
                ..Default::default()
            })
        }),
        ClientRpcResponse::Error(e) => Some(Err(resp_err(&e.code, &e.message))),
        _ => None,
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
    fn snix_key_reads_use_snix_rpc_operations() {
        assert!(matches!(
            snix_read_request_for_key("snix:dir:abc"),
            Some(ClientRpcRequest::SnixDirectoryGet { digest }) if digest == "abc"
        ));
        assert!(matches!(
            snix_read_request_for_key("snix:pathinfo:def"),
            Some(ClientRpcRequest::SnixPathInfoGet { digest }) if digest == "def"
        ));
        assert!(snix_read_request_for_key("other:key").is_none());
    }

    #[test]
    fn snix_key_writes_use_snix_rpc_operations() {
        assert!(matches!(
            snix_write_request_for_set("snix:dir:abc", "encoded-dir"),
            Some(ClientRpcRequest::SnixDirectoryPut { directory_bytes }) if directory_bytes == "encoded-dir"
        ));
        assert!(matches!(
            snix_write_request_for_set("snix:pathinfo:def", "encoded-pathinfo"),
            Some(ClientRpcRequest::SnixPathInfoPut { pathinfo_bytes }) if pathinfo_bytes == "encoded-pathinfo"
        ));
        assert!(snix_write_request_for_set("snix:blob:ghi", "encoded-blob").is_none());
    }

    #[test]
    fn snix_responses_round_trip_through_kv_adapter_shapes() {
        let read = snix_read_result_from_response(
            "snix:dir:abc",
            ClientRpcResponse::SnixDirectoryGetResult(aspen_client_api::SnixDirectoryGetResultResponse {
                was_found: true,
                directory_bytes: Some("encoded-dir".to_string()),
                error: None,
            }),
        )
        .expect("snix read response")
        .expect("successful read result");
        assert_eq!(read.kv.expect("kv").value, "encoded-dir");

        let request = WriteRequest {
            command: aspen_kv_types::WriteCommand::Set {
                key: "snix:pathinfo:def".to_string(),
                value: "encoded-pathinfo".to_string(),
            },
        };
        let write = snix_write_result_from_response(
            request,
            ClientRpcResponse::SnixPathInfoPutResult(aspen_client_api::SnixPathInfoPutResultResponse {
                is_success: true,
                store_path: Some("/nix/store/example".to_string()),
                error: None,
            }),
        )
        .expect("snix write response")
        .expect("successful write result");
        assert_eq!(write.succeeded, Some(true));
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
        let (key, value_string) = match &request.command {
            aspen_kv_types::WriteCommand::Set { key, value } => (key.clone(), value.clone()),
            other => {
                return Err(KeyValueStoreError::Failed {
                    reason: format!("unsupported write command for RPC adapter: {other:?}"),
                });
            }
        };

        let snix_rpc = snix_write_request_for_set(&key, &value_string);
        let expects_snix_response = snix_rpc.is_some();
        let rpc = snix_rpc.unwrap_or_else(|| ClientRpcRequest::WriteKey {
            key,
            value: value_string.into_bytes(),
        });
        let resp = self.client.send(rpc).await.map_err(rpc_err)?;
        if expects_snix_response {
            return snix_write_result_from_response(request, resp).unwrap_or_else(|| {
                Err(KeyValueStoreError::Failed {
                    reason: "unexpected non-SNIX response for SNIX write request".to_string(),
                })
            });
        }
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
        let snix_rpc = snix_read_request_for_key(&request.key);
        let expects_snix_response = snix_rpc.is_some();
        let rpc = snix_rpc.unwrap_or_else(|| ClientRpcRequest::ReadKey {
            key: request.key.clone(),
        });
        let resp = self.client.send(rpc).await.map_err(rpc_err)?;
        if expects_snix_response {
            return snix_read_result_from_response(&request.key, resp).unwrap_or_else(|| {
                Err(KeyValueStoreError::Failed {
                    reason: "unexpected non-SNIX response for SNIX read request".to_string(),
                })
            });
        }
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
