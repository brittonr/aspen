//! SQL query request handler.
//!
//! Handles: ExecuteSql.

use crate::context::ClientProtocolContext;
use crate::registry::RequestHandler;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use aspen_client_rpc::SqlCellValue;
use aspen_client_rpc::SqlResultResponse;

/// Handler for SQL query operations.
pub struct SqlHandler;

#[async_trait::async_trait]
impl RequestHandler for SqlHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(request, ClientRpcRequest::ExecuteSql { .. })
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::ExecuteSql {
                query,
                params,
                consistency,
                limit,
                timeout_ms,
            } => handle_execute_sql(ctx, query, params, consistency, limit, timeout_ms).await,
            _ => Err(anyhow::anyhow!("request not handled by SqlHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "SqlHandler"
    }
}

// ============================================================================
// SQL Operation Handlers
// ============================================================================

async fn handle_execute_sql(
    ctx: &ClientProtocolContext,
    query: String,
    params: String,
    consistency: String,
    limit: Option<u32>,
    timeout_ms: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    #[cfg(feature = "sql")]
    {
        use aspen_core::SqlConsistency;
        use aspen_core::SqlQueryExecutor;
        use aspen_core::SqlQueryRequest;
        use aspen_core::SqlValue;

        // Parse consistency level
        let consistency = match consistency.to_lowercase().as_str() {
            "stale" => SqlConsistency::Stale,
            _ => SqlConsistency::Linearizable, // Default to linearizable
        };

        // Parse parameters from JSON
        let params: Vec<SqlValue> = if params.is_empty() {
            Vec::new()
        } else {
            match serde_json::from_str::<Vec<serde_json::Value>>(&params) {
                Ok(values) => values
                    .into_iter()
                    .map(|v| match v {
                        serde_json::Value::Null => SqlValue::Null,
                        serde_json::Value::Bool(b) => SqlValue::Integer(if b { 1 } else { 0 }),
                        serde_json::Value::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                SqlValue::Integer(i)
                            } else if let Some(f) = n.as_f64() {
                                SqlValue::Real(f)
                            } else {
                                SqlValue::Text(n.to_string())
                            }
                        }
                        serde_json::Value::String(s) => SqlValue::Text(s),
                        serde_json::Value::Array(_) | serde_json::Value::Object(_) => SqlValue::Text(v.to_string()),
                    })
                    .collect(),
                Err(e) => {
                    return Ok(ClientRpcResponse::SqlResult(SqlResultResponse {
                        success: false,
                        columns: None,
                        rows: None,
                        row_count: None,
                        is_truncated: None,
                        execution_time_ms: None,
                        error: Some(format!("invalid params JSON: {}", e)),
                    }));
                }
            }
        };

        // Build request
        let request = SqlQueryRequest {
            query,
            params,
            consistency,
            limit,
            timeout_ms,
        };

        // Execute SQL query via SqlQueryExecutor trait
        match ctx.sql_executor.execute_sql(request).await {
            Ok(result) => {
                // Convert SqlValue to SqlCellValue for PostCard-compatible RPC transport
                use base64::Engine;
                let rows: Vec<Vec<SqlCellValue>> = result
                    .rows
                    .into_iter()
                    .map(|row| {
                        row.into_iter()
                            .map(|v| match v {
                                SqlValue::Null => SqlCellValue::Null,
                                SqlValue::Integer(i) => SqlCellValue::Integer(i),
                                SqlValue::Real(f) => SqlCellValue::Real(f),
                                SqlValue::Text(s) => SqlCellValue::Text(s),
                                SqlValue::Blob(b) => {
                                    // Encode blob as base64 for safe text transport
                                    SqlCellValue::Blob(base64::engine::general_purpose::STANDARD.encode(&b))
                                }
                            })
                            .collect()
                    })
                    .collect();

                let columns: Vec<String> = result.columns.into_iter().map(|c| c.name).collect();

                Ok(ClientRpcResponse::SqlResult(SqlResultResponse {
                    success: true,
                    columns: Some(columns),
                    rows: Some(rows),
                    row_count: Some(result.row_count),
                    is_truncated: Some(result.is_truncated),
                    execution_time_ms: Some(result.execution_time_ms),
                    error: None,
                }))
            }
            Err(e) => {
                // Return the error message (SQL errors are safe to show)
                Ok(ClientRpcResponse::SqlResult(SqlResultResponse {
                    success: false,
                    columns: None,
                    rows: None,
                    row_count: None,
                    is_truncated: None,
                    execution_time_ms: None,
                    error: Some(e.to_string()),
                }))
            }
        }
    }

    #[cfg(not(feature = "sql"))]
    {
        // Silence unused variable warnings
        let _ = (query, params, consistency, limit, timeout_ms);
        Ok(ClientRpcResponse::SqlResult(SqlResultResponse {
            success: false,
            columns: None,
            rows: None,
            row_count: None,
            is_truncated: None,
            execution_time_ms: None,
            error: Some("SQL queries not supported: compiled without 'sql' feature".into()),
        }))
    }
}
