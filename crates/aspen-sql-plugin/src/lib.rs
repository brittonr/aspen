//! SQL query handler WASM plugin.
//!
//! Migrated from the native `aspen-query-handler` crate. Handles `ExecuteSql`
//! requests by delegating to the host's SQL query executor via the `sql_query`
//! host function.
//!
//! The host function handles all the heavy lifting (query parsing, execution,
//! result formatting). This plugin maps the client RPC types to/from the
//! host function's JSON protocol.

use aspen_wasm_guest_sdk::*;

struct SqlHandlerPlugin;

impl AspenPlugin for SqlHandlerPlugin {
    fn info() -> PluginInfo {
        PluginInfo {
            name: "aspen-sql-handler".to_string(),
            version: "0.1.0".to_string(),
            handles: vec!["ExecuteSql".to_string()],
            priority: 500,
            app_id: Some("sql".to_string()),
            kv_prefixes: vec![],
            permissions: PluginPermissions {
                sql_query: true,
                ..PluginPermissions::default()
            },
        }
    }

    fn init() -> Result<(), String> {
        host::log_info_msg("aspen-sql-handler: initialized");
        Ok(())
    }

    fn handle(request: ClientRpcRequest) -> ClientRpcResponse {
        match request {
            ClientRpcRequest::ExecuteSql {
                query,
                params,
                consistency,
                limit,
                timeout_ms,
            } => handle_execute_sql(query, params, consistency, limit, timeout_ms),

            _ => ClientRpcResponse::Error(response::error_response(
                "UNHANDLED",
                "aspen-sql-handler does not handle this request type",
            )),
        }
    }
}

/// Handle an `ExecuteSql` request.
///
/// Delegates to the host's `sql_query` function, then maps the host result
/// into the `SqlResultResponse` client response type.
///
/// The host function handles:
/// - SQL parsing and validation
/// - Consistency level selection (linearizable vs stale)
/// - Query execution against the SQLite-backed state machine
/// - Result pagination and truncation
/// - Blob values are returned as `"base64:..."` strings
fn handle_execute_sql(
    query: String,
    params: String,
    consistency: String,
    limit: Option<u32>,
    timeout_ms: Option<u32>,
) -> ClientRpcResponse {
    match host::execute_sql(&query, &params, &consistency, limit, timeout_ms) {
        Ok(result) => {
            // Map host JSON rows to typed SqlCellValue rows.
            let rows: Vec<Vec<SqlCellValue>> =
                result.rows.into_iter().map(|row| row.into_iter().map(json_to_sql_cell).collect()).collect();

            let columns = result.columns;

            ClientRpcResponse::SqlResult(SqlResultResponse {
                is_success: true,
                columns: Some(columns),
                rows: Some(rows),
                row_count: Some(result.row_count),
                is_truncated: Some(result.is_truncated),
                execution_time_ms: Some(result.execution_time_ms),
                error: None,
            })
        }
        Err(e) => ClientRpcResponse::SqlResult(SqlResultResponse {
            is_success: false,
            columns: None,
            rows: None,
            row_count: None,
            is_truncated: None,
            execution_time_ms: None,
            error: Some(e),
        }),
    }
}

/// Convert a serde_json::Value to SqlCellValue.
///
/// The host returns SQL results as JSON values:
/// - null → Null
/// - number (integer) → Integer
/// - number (float) → Real
/// - string → Text (or Blob if prefixed with "base64:")
/// - bool → Integer (1/0)
fn json_to_sql_cell(value: serde_json::Value) -> SqlCellValue {
    match value {
        serde_json::Value::Null => SqlCellValue::Null,
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                SqlCellValue::Integer(i)
            } else if let Some(f) = n.as_f64() {
                SqlCellValue::Real(f)
            } else {
                SqlCellValue::Text(n.to_string())
            }
        }
        serde_json::Value::String(s) => {
            // Host encodes blob values as "base64:..." strings
            if let Some(b64) = s.strip_prefix("base64:") {
                SqlCellValue::Blob(b64.to_string())
            } else {
                SqlCellValue::Text(s)
            }
        }
        serde_json::Value::Bool(b) => SqlCellValue::Integer(if b { 1 } else { 0 }),
        other => SqlCellValue::Text(other.to_string()),
    }
}

register_plugin!(SqlHandlerPlugin);
