//! SNIX RPC handler for ephemeral worker operations.
//!
//! Handles SNIX-related RPC requests from ephemeral CI workers:
//! - DirectoryService get/put operations
//! - PathInfoService get/put operations
//!
//! Workers use these RPCs to upload build artifacts to the cluster's
//! SNIX binary cache without needing direct Raft access.

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::SnixDirectoryGetResultResponse;
use aspen_client_api::SnixDirectoryPutResultResponse;
use aspen_client_api::SnixPathInfoGetResultResponse;
use aspen_client_api::SnixPathInfoPutResultResponse;
use aspen_core::kv::ReadRequest;
use aspen_core::kv::WriteCommand;
use aspen_core::kv::WriteRequest;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;
use async_trait::async_trait;
use base64::Engine;
use prost::Message;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::instrument;

/// Key prefix for directory entries in the KV store.
const DIRECTORY_KEY_PREFIX: &str = "snix:dir:";

/// Key prefix for path info entries in the KV store.
const PATHINFO_KEY_PREFIX: &str = "snix:pathinfo:";

/// Handler for SNIX-related RPC requests.
///
/// Implements DirectoryService and PathInfoService operations using the
/// underlying Raft KV store. This allows ephemeral workers to upload
/// artifacts to the cluster's binary cache via RPC.
pub struct SnixHandler;

#[async_trait]
impl RequestHandler for SnixHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::SnixDirectoryGet { .. }
                | ClientRpcRequest::SnixDirectoryPut { .. }
                | ClientRpcRequest::SnixPathInfoGet { .. }
                | ClientRpcRequest::SnixPathInfoPut { .. }
        )
    }

    #[instrument(skip(self, ctx))]
    async fn handle(&self, request: ClientRpcRequest, ctx: &ClientProtocolContext) -> Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::SnixDirectoryGet { digest } => self.handle_directory_get(&digest, ctx).await,
            ClientRpcRequest::SnixDirectoryPut { directory_bytes } => {
                self.handle_directory_put(&directory_bytes, ctx).await
            }
            ClientRpcRequest::SnixPathInfoGet { digest } => self.handle_pathinfo_get(&digest, ctx).await,
            ClientRpcRequest::SnixPathInfoPut { pathinfo_bytes } => {
                self.handle_pathinfo_put(&pathinfo_bytes, ctx).await
            }
            _ => Err(anyhow::anyhow!("unexpected request type for SnixHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "snix"
    }
}

impl SnixHandler {
    /// Handle SnixDirectoryGet request.
    #[instrument(skip(self, ctx))]
    async fn handle_directory_get(&self, digest: &str, ctx: &ClientProtocolContext) -> Result<ClientRpcResponse> {
        let key = format!("{}{}", DIRECTORY_KEY_PREFIX, digest);
        debug!(key = %key, "SNIX directory get");

        let result = match ctx.kv_store.read(ReadRequest::new(&key)).await {
            Ok(result) => result,
            Err(aspen_core::error::KeyValueStoreError::NotFound { .. }) => {
                return Ok(ClientRpcResponse::SnixDirectoryGetResult(SnixDirectoryGetResultResponse {
                    was_found: false,
                    directory_bytes: None,
                    error: None,
                }));
            }
            Err(e) => {
                return Ok(ClientRpcResponse::SnixDirectoryGetResult(SnixDirectoryGetResultResponse {
                    was_found: false,
                    directory_bytes: None,
                    error: Some(format!("KV read error: {}", e)),
                }));
            }
        };

        match result.kv {
            Some(kv) => {
                debug!(key = %key, "directory found");
                Ok(ClientRpcResponse::SnixDirectoryGetResult(SnixDirectoryGetResultResponse {
                    was_found: true,
                    directory_bytes: Some(kv.value),
                    error: None,
                }))
            }
            None => Ok(ClientRpcResponse::SnixDirectoryGetResult(SnixDirectoryGetResultResponse {
                was_found: false,
                directory_bytes: None,
                error: None,
            })),
        }
    }

    /// Handle SnixDirectoryPut request.
    #[instrument(skip(self, ctx, directory_bytes))]
    async fn handle_directory_put(
        &self,
        directory_bytes: &str,
        ctx: &ClientProtocolContext,
    ) -> Result<ClientRpcResponse> {
        info!(base64_size = directory_bytes.len(), "SNIX directory put request received");

        // Decode base64
        let bytes = match base64::engine::general_purpose::STANDARD.decode(directory_bytes) {
            Ok(b) => {
                debug!(decoded_size = b.len(), "base64 decoded directory bytes");
                b
            }
            Err(e) => {
                error!(error = %e, "failed to decode base64 directory bytes");
                return Ok(ClientRpcResponse::SnixDirectoryPutResult(SnixDirectoryPutResultResponse {
                    is_success: false,
                    digest: None,
                    error: Some(format!("base64 decode error: {}", e)),
                }));
            }
        };

        // Decode protobuf to compute digest
        let proto_dir = match snix_castore::proto::Directory::decode(bytes.as_slice()) {
            Ok(d) => d,
            Err(e) => {
                error!(error = %e, size_bytes = bytes.len(), "failed to decode protobuf directory");
                return Ok(ClientRpcResponse::SnixDirectoryPutResult(SnixDirectoryPutResultResponse {
                    is_success: false,
                    digest: None,
                    error: Some(format!("protobuf decode error: {}", e)),
                }));
            }
        };

        let digest = proto_dir.digest();
        let digest_hex = hex::encode(digest.as_ref());
        let key = format!("{}{}", DIRECTORY_KEY_PREFIX, digest_hex);

        debug!(key = %key, size_bytes = bytes.len(), digest = %digest_hex, "storing SNIX directory in KV");

        // Write to KV store
        if let Err(e) = ctx
            .kv_store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: key.clone(),
                    value: directory_bytes.to_string(),
                },
            })
            .await
        {
            error!(error = %e, key = %key, digest = %digest_hex, "failed to write SNIX directory to KV store");
            return Ok(ClientRpcResponse::SnixDirectoryPutResult(SnixDirectoryPutResultResponse {
                is_success: false,
                digest: None,
                error: Some(format!("KV write error: {}", e)),
            }));
        }

        info!(key = %key, digest = %digest_hex, size_bytes = bytes.len(), "SNIX directory stored successfully");
        Ok(ClientRpcResponse::SnixDirectoryPutResult(SnixDirectoryPutResultResponse {
            is_success: true,
            digest: Some(digest_hex),
            error: None,
        }))
    }

    /// Handle SnixPathInfoGet request.
    #[instrument(skip(self, ctx))]
    async fn handle_pathinfo_get(&self, digest: &str, ctx: &ClientProtocolContext) -> Result<ClientRpcResponse> {
        let key = format!("{}{}", PATHINFO_KEY_PREFIX, digest);
        debug!(key = %key, "SNIX path info get");

        let result = match ctx.kv_store.read(ReadRequest::new(&key)).await {
            Ok(result) => result,
            Err(aspen_core::error::KeyValueStoreError::NotFound { .. }) => {
                return Ok(ClientRpcResponse::SnixPathInfoGetResult(SnixPathInfoGetResultResponse {
                    was_found: false,
                    pathinfo_bytes: None,
                    error: None,
                }));
            }
            Err(e) => {
                return Ok(ClientRpcResponse::SnixPathInfoGetResult(SnixPathInfoGetResultResponse {
                    was_found: false,
                    pathinfo_bytes: None,
                    error: Some(format!("KV read error: {}", e)),
                }));
            }
        };

        match result.kv {
            Some(kv) => {
                debug!(key = %key, "path info found");
                Ok(ClientRpcResponse::SnixPathInfoGetResult(SnixPathInfoGetResultResponse {
                    was_found: true,
                    pathinfo_bytes: Some(kv.value),
                    error: None,
                }))
            }
            None => Ok(ClientRpcResponse::SnixPathInfoGetResult(SnixPathInfoGetResultResponse {
                was_found: false,
                pathinfo_bytes: None,
                error: None,
            })),
        }
    }

    /// Handle SnixPathInfoPut request.
    #[instrument(skip(self, ctx, pathinfo_bytes))]
    async fn handle_pathinfo_put(
        &self,
        pathinfo_bytes: &str,
        ctx: &ClientProtocolContext,
    ) -> Result<ClientRpcResponse> {
        info!(base64_size = pathinfo_bytes.len(), "SNIX path info put request received");

        // Decode base64
        let bytes = match base64::engine::general_purpose::STANDARD.decode(pathinfo_bytes) {
            Ok(b) => {
                debug!(decoded_size = b.len(), "base64 decoded pathinfo bytes");
                b
            }
            Err(e) => {
                error!(error = %e, "failed to decode base64 pathinfo bytes");
                return Ok(ClientRpcResponse::SnixPathInfoPutResult(SnixPathInfoPutResultResponse {
                    is_success: false,
                    store_path: None,
                    error: Some(format!("base64 decode error: {}", e)),
                }));
            }
        };

        // Decode protobuf to extract store path
        let proto_pathinfo = match snix_store::proto::PathInfo::decode(bytes.as_slice()) {
            Ok(p) => p,
            Err(e) => {
                error!(error = %e, size_bytes = bytes.len(), "failed to decode protobuf pathinfo");
                return Ok(ClientRpcResponse::SnixPathInfoPutResult(SnixPathInfoPutResultResponse {
                    is_success: false,
                    store_path: None,
                    error: Some(format!("protobuf decode error: {}", e)),
                }));
            }
        };

        // Convert to PathInfo to get the store path
        let path_info = match snix_store::pathinfoservice::PathInfo::try_from(proto_pathinfo) {
            Ok(p) => p,
            Err(e) => {
                error!(error = %e, "failed to convert protobuf to PathInfo");
                return Ok(ClientRpcResponse::SnixPathInfoPutResult(SnixPathInfoPutResultResponse {
                    is_success: false,
                    store_path: None,
                    error: Some(format!("pathinfo conversion error: {}", e)),
                }));
            }
        };

        let store_path_str = path_info.store_path.to_string();
        let digest_hex = hex::encode(path_info.store_path.digest());
        let key = format!("{}{}", PATHINFO_KEY_PREFIX, digest_hex);
        let nar_size = path_info.nar_size;

        debug!(key = %key, store_path = %store_path_str, nar_size, size_bytes = bytes.len(), "storing SNIX path info in KV");

        // Write to KV store
        if let Err(e) = ctx
            .kv_store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: key.clone(),
                    value: pathinfo_bytes.to_string(),
                },
            })
            .await
        {
            error!(error = %e, key = %key, store_path = %store_path_str, "failed to write SNIX path info to KV store");
            return Ok(ClientRpcResponse::SnixPathInfoPutResult(SnixPathInfoPutResultResponse {
                is_success: false,
                store_path: None,
                error: Some(format!("KV write error: {}", e)),
            }));
        }

        info!(key = %key, store_path = %store_path_str, nar_size, "SNIX path info stored successfully");
        Ok(ClientRpcResponse::SnixPathInfoPutResult(SnixPathInfoPutResultResponse {
            is_success: true,
            store_path: Some(store_path_str),
            error: None,
        }))
    }
}
