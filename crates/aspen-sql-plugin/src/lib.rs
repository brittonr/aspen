//! WASM guest plugin for SQL query execution.
//!
//! Delegates `ExecuteSql` requests to the host's `sql_query` host function,
//! which executes read-only SQL queries against the node's state machine.
//!
//! This plugin is a thin wrapper — all query validation, execution, and
//! result formatting happens on the host side. The plugin exists to enable
//! SQL capabilities via WASM plugin deployment without compiling SQL support
//! into the node's native handler set.

use aspen_client_api::SqlCellValue;
use aspen_client_api::SqlResultResponse;
use aspen_wasm_guest_sdk::AspenPlugin;
use aspen_wasm_guest_sdk::ClientRpcRequest;
use aspen_wasm_guest_sdk::ClientRpcResponse;
use aspen_wasm_guest_sdk::PluginInfo;
use aspen_wasm_guest_sdk::PluginPermissions;
use aspen_wasm_guest_sdk::host;
use aspen_wasm_guest_sdk::register_plugin;

struct SqlPlugin;

impl AspenPlugin for SqlPlugin {
    fn info() -> PluginInfo {
        PluginInfo {
            name: "sql".to_string(),
            version: "0.1.0".to_string(),
            handles: vec!["ExecuteSql".to_string()],
            priority: 940,
            app_id: Some("sql".to_string()),
            kv_prefixes: vec!["__plugin:sql:".to_string()],
            permissions: PluginPermissions {
                sql_query: true,
                ..PluginPermissions::default()
            },
        }
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

            _ => ClientRpcResponse::Error(aspen_client_api::ErrorResponse {
                code: "UNHANDLED_REQUEST".to_string(),
                message: "Request not handled by SQL plugin".to_string(),
            }),
        }
    }
}

register_plugin!(SqlPlugin);

fn handle_execute_sql(
    query: String,
    params: String,
    consistency: String,
    limit: Option<u32>,
    timeout_ms: Option<u32>,
) -> ClientRpcResponse {
    match host::execute_sql(&query, &params, &consistency, limit, timeout_ms) {
        Ok(result) => {
            // Convert the host result into the client API response format
            let columns: Vec<String> = result.columns;
            let rows: Vec<Vec<SqlCellValue>> = result
                .rows
                .into_iter()
                .map(|row| {
                    row.into_iter()
                        .map(|v| match v {
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
                                if let Some(b64) = s.strip_prefix("base64:") {
                                    SqlCellValue::Blob(b64.to_string())
                                } else {
                                    SqlCellValue::Text(s)
                                }
                            }
                            serde_json::Value::Bool(b) => SqlCellValue::Integer(if b { 1 } else { 0 }),
                            _ => SqlCellValue::Text(v.to_string()),
                        })
                        .collect()
                })
                .collect();

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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_info_matches_manifest() {
        let info = SqlPlugin::info();
        let manifest: serde_json::Value =
            serde_json::from_str(include_str!("../plugin.json")).expect("valid plugin.json");

        assert_eq!(info.name, manifest["name"].as_str().unwrap());
        assert_eq!(info.version, manifest["version"].as_str().unwrap());
        assert_eq!(info.priority, manifest["priority"].as_u64().unwrap() as u32);
        assert_eq!(info.app_id.as_deref(), manifest["app_id"].as_str());
        assert_eq!(info.handles.len(), manifest["handles"].as_array().unwrap().len(), "handle count mismatch");
    }
}
