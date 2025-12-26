//! Output formatting for CLI responses.
//!
//! Supports both human-readable and JSON output formats for
//! integration with scripts and other tools.

/// Trait for types that can be output in multiple formats.
pub trait Outputable {
    /// Convert to JSON value for structured output.
    fn to_json(&self) -> serde_json::Value;

    /// Convert to human-readable string.
    fn to_human(&self) -> String;
}

/// Print a value in the appropriate format.
pub fn print_output<T: Outputable>(value: &T, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&value.to_json())
                .unwrap_or_else(|e| { format!("{{\"error\": \"failed to serialize: {}\"}}", e) })
        );
    } else {
        println!("{}", value.to_human());
    }
}

/// Print a success message.
pub fn print_success(message: &str, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::json!({
                "status": "success",
                "message": message
            })
        );
    } else {
        println!("{}", message);
    }
}

/// Health status output.
pub struct HealthOutput {
    pub status: String,
    pub node_id: u64,
    pub raft_node_id: Option<u64>,
    pub uptime_seconds: u64,
}

impl Outputable for HealthOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "status": self.status,
            "node_id": self.node_id,
            "raft_node_id": self.raft_node_id,
            "uptime_seconds": self.uptime_seconds
        })
    }

    fn to_human(&self) -> String {
        let raft_id = self.raft_node_id.map(|id| id.to_string()).unwrap_or_else(|| "N/A".to_string());

        format!(
            "Health Status\n\
             =============\n\
             Status:         {}\n\
             Node ID:        {}\n\
             Raft Node ID:   {}\n\
             Uptime:         {}s",
            self.status, self.node_id, raft_id, self.uptime_seconds
        )
    }
}

/// Raft metrics output.
pub struct RaftMetricsOutput {
    pub state: String,
    pub current_leader: Option<u64>,
    pub current_term: u64,
    pub last_log_index: u64,
    pub last_applied: u64,
    pub snapshot_index: u64,
}

impl Outputable for RaftMetricsOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "state": self.state,
            "current_leader": self.current_leader,
            "current_term": self.current_term,
            "last_log_index": self.last_log_index,
            "last_applied": self.last_applied,
            "snapshot_index": self.snapshot_index
        })
    }

    fn to_human(&self) -> String {
        let leader = self.current_leader.map(|id| id.to_string()).unwrap_or_else(|| "none".to_string());

        format!(
            "Raft Metrics\n\
             ============\n\
             State:          {}\n\
             Current Leader: {}\n\
             Current Term:   {}\n\
             Last Log Index: {}\n\
             Last Applied:   {}\n\
             Snapshot Index: {}",
            self.state, leader, self.current_term, self.last_log_index, self.last_applied, self.snapshot_index
        )
    }
}

/// Cluster state output.
pub struct ClusterStateOutput {
    pub nodes: Vec<NodeInfo>,
}

pub struct NodeInfo {
    pub node_id: u64,
    pub endpoint_id: String,
    pub is_leader: bool,
    pub is_voter: bool,
}

impl Outputable for ClusterStateOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "nodes": self.nodes.iter().map(|n| {
                serde_json::json!({
                    "node_id": n.node_id,
                    "endpoint_id": n.endpoint_id,
                    "is_leader": n.is_leader,
                    "is_voter": n.is_voter
                })
            }).collect::<Vec<_>>()
        })
    }

    fn to_human(&self) -> String {
        if self.nodes.is_empty() {
            return "No nodes in cluster".to_string();
        }

        let mut output = String::from("Cluster State\n=============\n\n");
        output.push_str("Node ID  | Leader | Voter | Endpoint ID\n");
        output.push_str("---------+--------+-------+------------------------------------------\n");

        for node in &self.nodes {
            let leader_marker = if node.is_leader { "*" } else { " " };
            let voter_marker = if node.is_voter { "Y" } else { "N" };
            output.push_str(&format!(
                "{:8} | {:6} | {:5} | {}\n",
                node.node_id,
                leader_marker,
                voter_marker,
                &node.endpoint_id[..std::cmp::min(40, node.endpoint_id.len())]
            ));
        }

        output
    }
}

/// Key-value read result output.
pub struct KvReadOutput {
    pub key: String,
    pub value: Option<Vec<u8>>,
    pub exists: bool,
}

impl Outputable for KvReadOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "key": self.key,
            "exists": self.exists,
            "value": self.value.as_ref().map(|v| {
                // Try to decode as UTF-8, fall back to base64
                String::from_utf8(v.clone())
                    .map(serde_json::Value::String)
                    .unwrap_or_else(|_| serde_json::json!({"base64": base64_encode(v)}))
            })
        })
    }

    fn to_human(&self) -> String {
        if !self.exists {
            return format!("Key '{}' not found", self.key);
        }

        match &self.value {
            Some(v) => {
                // Try to display as UTF-8 string
                match String::from_utf8(v.clone()) {
                    Ok(s) => s,
                    Err(_) => format!("<binary: {} bytes>", v.len()),
                }
            }
            None => format!("Key '{}' exists but has no value", self.key),
        }
    }
}

/// Key-value scan result output.
pub struct KvScanOutput {
    pub entries: Vec<(String, Vec<u8>)>,
    pub continuation_token: Option<String>,
}

impl Outputable for KvScanOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "entries": self.entries.iter().map(|(k, v)| {
                serde_json::json!({
                    "key": k,
                    "value": String::from_utf8(v.clone())
                        .map(serde_json::Value::String)
                        .unwrap_or_else(|_| serde_json::json!({"base64": base64_encode(v)}))
                })
            }).collect::<Vec<_>>(),
            "count": self.entries.len(),
            "continuation_token": self.continuation_token
        })
    }

    fn to_human(&self) -> String {
        if self.entries.is_empty() {
            return "No keys found".to_string();
        }

        let mut output = format!("Found {} key(s)\n\n", self.entries.len());

        for (key, value) in &self.entries {
            let value_str =
                String::from_utf8(value.clone()).unwrap_or_else(|_| format!("<binary: {} bytes>", value.len()));

            // Truncate long values for display
            let display_value = if value_str.len() > 60 {
                format!("{}...", &value_str[..57])
            } else {
                value_str
            };

            output.push_str(&format!("{}: {}\n", key, display_value));
        }

        if let Some(ref token) = self.continuation_token {
            output.push_str(&format!("\nMore results available. Use --token {}", token));
        }

        output
    }
}

/// Simple base64 encoding helper.
fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

/// Batch read result output.
pub struct KvBatchReadOutput {
    pub keys: Vec<String>,
    pub values: Vec<Option<Vec<u8>>>,
}

impl Outputable for KvBatchReadOutput {
    fn to_json(&self) -> serde_json::Value {
        let results: Vec<_> = self
            .keys
            .iter()
            .zip(self.values.iter())
            .map(|(key, value)| {
                serde_json::json!({
                    "key": key,
                    "exists": value.is_some(),
                    "value": value.as_ref().map(|v| {
                        String::from_utf8(v.clone())
                            .map(serde_json::Value::String)
                            .unwrap_or_else(|_| serde_json::json!({"base64": base64_encode(v)}))
                    })
                })
            })
            .collect();

        serde_json::json!({
            "count": self.keys.len(),
            "results": results
        })
    }

    fn to_human(&self) -> String {
        if self.keys.is_empty() {
            return "No keys requested".to_string();
        }

        let mut output = format!("Batch read {} key(s)\n\n", self.keys.len());

        for (key, value) in self.keys.iter().zip(self.values.iter()) {
            match value {
                Some(v) => {
                    let value_str = String::from_utf8(v.clone())
                        .unwrap_or_else(|_| format!("<binary: {} bytes>", v.len()));
                    let display_value = if value_str.len() > 50 {
                        format!("{}...", &value_str[..47])
                    } else {
                        value_str
                    };
                    output.push_str(&format!("{}: {}\n", key, display_value));
                }
                None => {
                    output.push_str(&format!("{}: <not found>\n", key));
                }
            }
        }

        output
    }
}

/// Batch write result output.
pub struct KvBatchWriteOutput {
    pub success: bool,
    pub operations_applied: u32,
}

impl Outputable for KvBatchWriteOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "success": self.success,
            "operations_applied": self.operations_applied
        })
    }

    fn to_human(&self) -> String {
        if self.success {
            format!("OK: {} operation(s) applied", self.operations_applied)
        } else {
            "Batch write failed".to_string()
        }
    }
}

/// SQL query result output.
pub struct SqlQueryOutput {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<aspen::client_rpc::SqlCellValue>>,
    pub row_count: u32,
    pub is_truncated: bool,
    pub execution_time_ms: u64,
    pub show_headers: bool,
    pub format: crate::commands::sql::OutputFormat,
}

impl Outputable for SqlQueryOutput {
    fn to_json(&self) -> serde_json::Value {
        use aspen::client_rpc::SqlCellValue;

        let rows: Vec<serde_json::Value> = self
            .rows
            .iter()
            .map(|row| {
                let obj: serde_json::Map<String, serde_json::Value> = self
                    .columns
                    .iter()
                    .zip(row.iter())
                    .map(|(col, val)| {
                        let json_val = match val {
                            SqlCellValue::Null => serde_json::Value::Null,
                            SqlCellValue::Integer(i) => serde_json::json!(*i),
                            SqlCellValue::Real(f) => serde_json::json!(*f),
                            SqlCellValue::Text(s) => serde_json::Value::String(s.clone()),
                            SqlCellValue::Blob(b64) => serde_json::json!({"base64": b64}),
                        };
                        (col.clone(), json_val)
                    })
                    .collect();
                serde_json::Value::Object(obj)
            })
            .collect();

        serde_json::json!({
            "columns": self.columns,
            "rows": rows,
            "row_count": self.row_count,
            "is_truncated": self.is_truncated,
            "execution_time_ms": self.execution_time_ms
        })
    }

    fn to_human(&self) -> String {
        use crate::commands::sql::OutputFormat;

        if self.rows.is_empty() {
            return format!(
                "No rows returned ({} ms)",
                self.execution_time_ms
            );
        }

        match self.format {
            OutputFormat::Table => self.format_table(),
            OutputFormat::Tsv => self.format_delimited('\t'),
            OutputFormat::Csv => self.format_delimited(','),
            OutputFormat::Vertical => self.format_vertical(),
        }
    }
}

impl SqlQueryOutput {
    /// Format as ASCII table with borders.
    fn format_table(&self) -> String {

        // Calculate column widths
        let mut widths: Vec<usize> = self.columns.iter().map(|c| c.len()).collect();

        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    let cell_str = cell.to_display_string();
                    widths[i] = widths[i].max(cell_str.len().min(50)); // Cap at 50 chars
                }
            }
        }

        let mut output = String::new();

        // Header separator
        let separator: String = widths.iter().map(|w| "-".repeat(*w + 2)).collect::<Vec<_>>().join("+");
        let separator = format!("+{}+", separator);

        // Header row
        if self.show_headers {
            output.push_str(&separator);
            output.push('\n');

            let header: String = self
                .columns
                .iter()
                .zip(widths.iter())
                .map(|(c, w)| format!(" {:width$} ", c, width = w))
                .collect::<Vec<_>>()
                .join("|");
            output.push_str(&format!("|{}|", header));
            output.push('\n');
            output.push_str(&separator);
            output.push('\n');
        }

        // Data rows
        for row in &self.rows {
            let row_str: String = row
                .iter()
                .zip(widths.iter())
                .map(|(cell, w)| {
                    let s = cell.to_display_string();
                    let truncated = if s.len() > 50 {
                        format!("{}...", &s[..47])
                    } else {
                        s
                    };
                    format!(" {:width$} ", truncated, width = w)
                })
                .collect::<Vec<_>>()
                .join("|");
            output.push_str(&format!("|{}|", row_str));
            output.push('\n');
        }

        output.push_str(&separator);
        output.push('\n');

        // Footer
        output.push_str(&format!(
            "{} row(s) returned{} ({} ms)",
            self.row_count,
            if self.is_truncated { " (truncated)" } else { "" },
            self.execution_time_ms
        ));

        output
    }

    /// Format as delimiter-separated values.
    fn format_delimited(&self, delimiter: char) -> String {

        let mut output = String::new();

        // Header
        if self.show_headers {
            output.push_str(&self.columns.join(&delimiter.to_string()));
            output.push('\n');
        }

        // Data rows
        for row in &self.rows {
            let row_str: String = row
                .iter()
                .map(|cell| {
                    let s = cell.to_display_string();
                    // Escape delimiter and quotes in CSV mode
                    if delimiter == ',' && (s.contains(',') || s.contains('"') || s.contains('\n')) {
                        format!("\"{}\"", s.replace('"', "\"\""))
                    } else {
                        s
                    }
                })
                .collect::<Vec<_>>()
                .join(&delimiter.to_string());
            output.push_str(&row_str);
            output.push('\n');
        }

        output
    }

    /// Format in vertical mode (one column per line).
    fn format_vertical(&self) -> String {

        let mut output = String::new();
        let max_col_len = self.columns.iter().map(|c| c.len()).max().unwrap_or(0);

        for (i, row) in self.rows.iter().enumerate() {
            output.push_str(&format!("*************************** {}. row ***************************\n", i + 1));

            for (col, cell) in self.columns.iter().zip(row.iter()) {
                let value = cell.to_display_string();
                output.push_str(&format!("{:>width$}: {}\n", col, value, width = max_col_len));
            }
        }

        output.push_str(&format!(
            "{} row(s) in set{} ({} ms)",
            self.row_count,
            if self.is_truncated { " (truncated)" } else { "" },
            self.execution_time_ms
        ));

        output
    }
}
