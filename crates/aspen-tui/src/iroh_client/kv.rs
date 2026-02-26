//! Key-value and SQL RPC methods for IrohClient.

use anyhow::Result;
use aspen_client::ClientRpcRequest;
use aspen_client::ClientRpcResponse;
use aspen_client::DeleteResultResponse;
use aspen_client::ReadResultResponse;
use aspen_client::ScanResultResponse;
use aspen_client::SqlResultResponse;
use aspen_client::WriteResultResponse;

use super::IrohClient;

impl IrohClient {
    /// Read a key from the key-value store.
    pub async fn read_key(&self, key: String) -> Result<ReadResultResponse> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::ReadKey { key }).await?;

        match response {
            ClientRpcResponse::ReadResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for ReadKey"),
        }
    }

    /// Write a key-value pair to the store.
    pub async fn write_key(&self, key: String, value: Vec<u8>) -> Result<WriteResultResponse> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::WriteKey { key, value }).await?;

        match response {
            ClientRpcResponse::WriteResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for WriteKey"),
        }
    }

    /// Delete a key from the key-value store.
    pub async fn delete_key(&self, key: String) -> Result<DeleteResultResponse> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::DeleteKey { key }).await?;

        match response {
            ClientRpcResponse::DeleteResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for DeleteKey"),
        }
    }

    /// Scan keys with a prefix from the key-value store.
    pub async fn scan_keys(
        &self,
        prefix: String,
        limit: Option<u32>,
        continuation_token: Option<String>,
    ) -> Result<ScanResultResponse> {
        let response = self
            .send_rpc_with_retry(ClientRpcRequest::ScanKeys {
                prefix,
                limit,
                continuation_token,
            })
            .await?;

        match response {
            ClientRpcResponse::ScanResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for ScanKeys"),
        }
    }

    /// Execute a SQL query against the cluster.
    pub async fn execute_sql(
        &self,
        query: String,
        consistency: String,
        limit: Option<u32>,
        timeout_ms: Option<u32>,
    ) -> Result<SqlResultResponse> {
        let response = self
            .send_rpc_with_retry(ClientRpcRequest::ExecuteSql {
                query,
                params: "[]".to_string(), // Empty params array
                consistency,
                limit,
                timeout_ms,
            })
            .await?;

        match response {
            ClientRpcResponse::SqlResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for ExecuteSql"),
        }
    }
}
