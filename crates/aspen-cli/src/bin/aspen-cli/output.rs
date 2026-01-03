//! Output formatting for CLI responses.
//!
//! Supports both human-readable and JSON output formats for
//! integration with scripts and other tools.

#[cfg(feature = "dns")]
use aspen_client_rpc::DnsRecordResponse;
#[cfg(feature = "dns")]
use aspen_client_rpc::DnsZoneResponse;

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
                    let value_str =
                        String::from_utf8(v.clone()).unwrap_or_else(|_| format!("<binary: {} bytes>", v.len()));
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
#[cfg(feature = "sql")]
pub struct SqlQueryOutput {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<aspen_client_rpc::SqlCellValue>>,
    pub row_count: u32,
    pub is_truncated: bool,
    pub execution_time_ms: u64,
    pub show_headers: bool,
    pub format: crate::commands::sql::OutputFormat,
}

#[cfg(feature = "sql")]
impl Outputable for SqlQueryOutput {
    fn to_json(&self) -> serde_json::Value {
        use aspen_client_rpc::SqlCellValue;

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
                            SqlCellValue::Integer(i) => serde_json::json!(i),
                            SqlCellValue::Real(f) => serde_json::json!(f),
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
            return format!("No rows returned ({} ms)", self.execution_time_ms);
        }

        match self.format {
            OutputFormat::Table => self.format_table(),
            OutputFormat::Tsv => self.format_delimited('\t'),
            OutputFormat::Csv => self.format_delimited(','),
            OutputFormat::Vertical => self.format_vertical(),
        }
    }
}

#[cfg(feature = "sql")]
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
                    let truncated = if s.len() > 50 { format!("{}...", &s[..47]) } else { s };
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

// =============================================================================
// DNS Output Types
// =============================================================================

#[cfg(feature = "dns")]
/// DNS record output.
pub struct DnsRecordOutput {
    pub domain: String,
    pub record_type: String,
    pub ttl_seconds: u32,
    pub data_json: String,
    pub updated_at_ms: u64,
}

#[cfg(feature = "dns")]
impl DnsRecordOutput {
    /// Create from a DnsRecordResponse.
    pub fn from_response(resp: DnsRecordResponse) -> Self {
        Self {
            domain: resp.domain,
            record_type: resp.record_type,
            ttl_seconds: resp.ttl_seconds,
            data_json: resp.data_json,
            updated_at_ms: resp.updated_at_ms,
        }
    }

    /// Format record data for human-readable output.
    fn format_data(&self) -> String {
        // Try to parse as JSON and format nicely
        match serde_json::from_str::<serde_json::Value>(&self.data_json) {
            Ok(data) => {
                // Extract the relevant data based on record type
                match self.record_type.as_str() {
                    "A" => data
                        .get("addresses")
                        .and_then(|a| a.as_array())
                        .map(|addrs| addrs.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", "))
                        .unwrap_or_else(|| self.data_json.clone()),
                    "AAAA" => data
                        .get("addresses")
                        .and_then(|a| a.as_array())
                        .map(|addrs| addrs.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", "))
                        .unwrap_or_else(|| self.data_json.clone()),
                    "CNAME" | "PTR" => data
                        .get("target")
                        .and_then(|t| t.as_str())
                        .map(String::from)
                        .unwrap_or_else(|| self.data_json.clone()),
                    "TXT" => data
                        .get("strings")
                        .and_then(|s| s.as_array())
                        .map(|strs| {
                            strs.iter()
                                .filter_map(|v| v.as_str())
                                .map(|s| format!("\"{}\"", s))
                                .collect::<Vec<_>>()
                                .join(" ")
                        })
                        .unwrap_or_else(|| self.data_json.clone()),
                    "MX" => data
                        .get("records")
                        .and_then(|r| r.as_array())
                        .map(|records| {
                            records
                                .iter()
                                .filter_map(|r| {
                                    let priority = r.get("priority").and_then(|p| p.as_u64())?;
                                    let exchange = r.get("exchange").and_then(|e| e.as_str())?;
                                    Some(format!("{} {}", priority, exchange))
                                })
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_else(|| self.data_json.clone()),
                    "NS" => data
                        .get("nameservers")
                        .and_then(|n| n.as_array())
                        .map(|ns| ns.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", "))
                        .unwrap_or_else(|| self.data_json.clone()),
                    _ => self.data_json.clone(),
                }
            }
            Err(_) => self.data_json.clone(),
        }
    }
}

#[cfg(feature = "dns")]
impl Outputable for DnsRecordOutput {
    fn to_json(&self) -> serde_json::Value {
        // Try to parse data_json as JSON, otherwise use as string
        let data = serde_json::from_str::<serde_json::Value>(&self.data_json)
            .unwrap_or_else(|_| serde_json::Value::String(self.data_json.clone()));

        serde_json::json!({
            "domain": self.domain,
            "record_type": self.record_type,
            "ttl_seconds": self.ttl_seconds,
            "data": data,
            "updated_at_ms": self.updated_at_ms
        })
    }

    fn to_human(&self) -> String {
        format!("{:<30} {:6} IN {:5} {}", self.domain, self.ttl_seconds, self.record_type, self.format_data())
    }
}

#[cfg(feature = "dns")]
/// DNS records list output.
pub struct DnsRecordsOutput {
    pub records: Vec<DnsRecordOutput>,
}

#[cfg(feature = "dns")]
impl DnsRecordsOutput {
    /// Create from a list of DnsRecordResponse.
    pub fn from_responses(responses: Vec<DnsRecordResponse>) -> Self {
        Self {
            records: responses.into_iter().map(DnsRecordOutput::from_response).collect(),
        }
    }
}

#[cfg(feature = "dns")]
impl Outputable for DnsRecordsOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "count": self.records.len(),
            "records": self.records.iter().map(|r| r.to_json()).collect::<Vec<_>>()
        })
    }

    fn to_human(&self) -> String {
        if self.records.is_empty() {
            return "No records found".to_string();
        }

        let mut output = format!("Found {} record(s)\n\n", self.records.len());

        // Format as DNS zone file style
        for record in &self.records {
            output.push_str(&record.to_human());
            output.push('\n');
        }

        output
    }
}

#[cfg(feature = "dns")]
/// DNS zone output.
pub struct DnsZoneOutput {
    pub name: String,
    pub enabled: bool,
    pub default_ttl: u32,
    pub serial: u32,
    pub last_modified_ms: u64,
    pub description: Option<String>,
}

#[cfg(feature = "dns")]
impl DnsZoneOutput {
    /// Create from a DnsZoneResponse.
    pub fn from_response(resp: DnsZoneResponse) -> Self {
        Self {
            name: resp.name,
            enabled: resp.enabled,
            default_ttl: resp.default_ttl,
            serial: resp.serial,
            last_modified_ms: resp.last_modified_ms,
            description: resp.description,
        }
    }
}

#[cfg(feature = "dns")]
impl Outputable for DnsZoneOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "enabled": self.enabled,
            "default_ttl": self.default_ttl,
            "serial": self.serial,
            "last_modified_ms": self.last_modified_ms,
            "description": self.description
        })
    }

    fn to_human(&self) -> String {
        let status = if self.enabled { "enabled" } else { "disabled" };
        let desc = self.description.as_deref().unwrap_or("-");

        format!(
            "Zone: {}\n\
             ======{}\n\
             Status:      {}\n\
             Default TTL: {} seconds\n\
             Serial:      {}\n\
             Description: {}",
            self.name,
            "=".repeat(self.name.len()),
            status,
            self.default_ttl,
            self.serial,
            desc
        )
    }
}

#[cfg(feature = "dns")]
/// DNS zones list output.
pub struct DnsZonesOutput {
    pub zones: Vec<DnsZoneOutput>,
}

#[cfg(feature = "dns")]
impl DnsZonesOutput {
    /// Create from a list of DnsZoneResponse.
    pub fn from_responses(responses: Vec<DnsZoneResponse>) -> Self {
        Self {
            zones: responses.into_iter().map(DnsZoneOutput::from_response).collect(),
        }
    }
}

#[cfg(feature = "dns")]
impl Outputable for DnsZonesOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "count": self.zones.len(),
            "zones": self.zones.iter().map(|z| z.to_json()).collect::<Vec<_>>()
        })
    }

    fn to_human(&self) -> String {
        if self.zones.is_empty() {
            return "No zones configured".to_string();
        }

        let mut output = format!("Found {} zone(s)\n\n", self.zones.len());
        output.push_str("Name                           | Status   | TTL     | Serial\n");
        output.push_str("-------------------------------|----------|---------|----------\n");

        for zone in &self.zones {
            let status = if zone.enabled { "enabled" } else { "disabled" };
            output.push_str(&format!(
                "{:<30} | {:<8} | {:>7} | {}\n",
                truncate(&zone.name, 30),
                status,
                zone.default_ttl,
                zone.serial
            ));
        }

        output
    }
}

#[cfg(feature = "dns")]
/// Truncate a string to max length, adding "..." if truncated.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

// =============================================================================
// Forge output types (decentralized git)
// =============================================================================

/// Repository list output.
pub struct RepoListOutput {
    pub repos: Vec<RepoListItem>,
    pub count: u32,
}

/// A single repository in a list.
pub struct RepoListItem {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub default_branch: String,
}

impl Outputable for RepoListOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "repos": self.repos.iter().map(|r| serde_json::json!({
                "id": r.id,
                "name": r.name,
                "description": r.description,
                "default_branch": r.default_branch
            })).collect::<Vec<_>>(),
            "count": self.count
        })
    }

    fn to_human(&self) -> String {
        if self.repos.is_empty() {
            return "No repositories found.".to_string();
        }

        let mut output = format!("Repositories ({}):\n", self.count);
        for repo in &self.repos {
            let desc = repo.description.as_deref().unwrap_or("");
            let desc_preview = if desc.len() > 40 {
                format!("{}...", &desc[..37])
            } else {
                desc.to_string()
            };
            output.push_str(&format!(
                "  {} ({}) - {}\n",
                repo.name,
                &repo.id[..16],
                if desc_preview.is_empty() { "-" } else { &desc_preview }
            ));
        }
        output
    }
}

/// Repository output.
pub struct RepoOutput {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub default_branch: String,
    pub delegates: Vec<String>,
    pub threshold: u32,
    pub created_at_ms: u64,
}

impl Outputable for RepoOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "name": self.name,
            "description": self.description,
            "default_branch": self.default_branch,
            "delegates": self.delegates,
            "threshold": self.threshold,
            "created_at_ms": self.created_at_ms
        })
    }

    fn to_human(&self) -> String {
        let desc = self.description.as_deref().unwrap_or("-");
        format!(
            "Repository: {}\n\
             ID:             {}\n\
             Default Branch: {}\n\
             Description:    {}\n\
             Delegates:      {}\n\
             Threshold:      {}",
            self.name,
            self.id,
            self.default_branch,
            desc,
            self.delegates.len(),
            self.threshold
        )
    }
}

/// Commit output.
pub struct CommitOutput {
    pub hash: String,
    pub tree: String,
    pub parents: Vec<String>,
    pub author_name: String,
    pub author_email: Option<String>,
    pub message: String,
    pub timestamp_ms: u64,
}

impl Outputable for CommitOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "hash": self.hash,
            "tree": self.tree,
            "parents": self.parents,
            "author_name": self.author_name,
            "author_email": self.author_email,
            "message": self.message,
            "timestamp_ms": self.timestamp_ms
        })
    }

    fn to_human(&self) -> String {
        let email = self.author_email.as_deref().unwrap_or("");
        let author = if email.is_empty() {
            self.author_name.clone()
        } else {
            format!("{} <{}>", self.author_name, email)
        };
        let parents = if self.parents.is_empty() {
            String::new()
        } else {
            format!("\nParent: {}", self.parents.join(", "))
        };
        format!(
            "commit {}\n\
             Author: {}\n\
             Date:   {}{}\n\n\
             {}",
            self.hash,
            author,
            self.timestamp_ms,
            parents,
            self.message.lines().map(|l| format!("    {}", l)).collect::<Vec<_>>().join("\n")
        )
    }
}

/// Commit log output.
pub struct LogOutput {
    pub commits: Vec<CommitOutput>,
    pub count: u32,
}

impl Outputable for LogOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "count": self.count,
            "commits": self.commits.iter().map(|c| c.to_json()).collect::<Vec<_>>()
        })
    }

    fn to_human(&self) -> String {
        if self.commits.is_empty() {
            return "No commits found".to_string();
        }
        self.commits.iter().map(|c| c.to_human()).collect::<Vec<_>>().join("\n\n")
    }
}

/// Ref output (branch or tag).
pub struct RefOutput {
    pub name: String,
    pub hash: String,
}

impl Outputable for RefOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "hash": self.hash
        })
    }

    fn to_human(&self) -> String {
        format!("{} -> {}", self.name, self.hash)
    }
}

/// Ref list output (branches or tags).
pub struct RefListOutput {
    pub refs: Vec<RefOutput>,
    pub count: u32,
}

impl Outputable for RefListOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "count": self.count,
            "refs": self.refs.iter().map(|r| r.to_json()).collect::<Vec<_>>()
        })
    }

    fn to_human(&self) -> String {
        if self.refs.is_empty() {
            return "No refs found".to_string();
        }
        self.refs.iter().map(|r| r.to_human()).collect::<Vec<_>>().join("\n")
    }
}

/// Comment output.
pub struct CommentOutput {
    pub hash: String,
    pub author: String,
    pub body: String,
    pub timestamp_ms: u64,
}

impl Outputable for CommentOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "hash": self.hash,
            "author": self.author,
            "body": self.body,
            "timestamp_ms": self.timestamp_ms
        })
    }

    fn to_human(&self) -> String {
        format!("[{}] {}: {}", &self.hash[..8], &self.author[..8], self.body)
    }
}

/// Issue summary output (for lists).
pub struct IssueOutput {
    pub id: String,
    pub title: String,
    pub state: String,
    pub labels: Vec<String>,
    pub comment_count: u32,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

impl Outputable for IssueOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "title": self.title,
            "state": self.state,
            "labels": self.labels,
            "comment_count": self.comment_count,
            "created_at_ms": self.created_at_ms,
            "updated_at_ms": self.updated_at_ms
        })
    }

    fn to_human(&self) -> String {
        let labels = if self.labels.is_empty() {
            String::new()
        } else {
            format!(" [{}]", self.labels.join(", "))
        };
        format!(
            "#{} {} {}{} ({} comments)",
            &self.id[..8],
            self.state.to_uppercase(),
            self.title,
            labels,
            self.comment_count
        )
    }
}

/// Issue list output.
pub struct IssueListOutput {
    pub issues: Vec<IssueOutput>,
    pub count: u32,
}

impl Outputable for IssueListOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "count": self.count,
            "issues": self.issues.iter().map(|i| i.to_json()).collect::<Vec<_>>()
        })
    }

    fn to_human(&self) -> String {
        if self.issues.is_empty() {
            return "No issues found".to_string();
        }
        self.issues.iter().map(|i| i.to_human()).collect::<Vec<_>>().join("\n")
    }
}

/// Issue detail output.
pub struct IssueDetailOutput {
    pub id: String,
    pub title: String,
    pub body: String,
    pub state: String,
    pub labels: Vec<String>,
    pub assignees: Vec<String>,
    pub comment_count: u32,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub comments: Option<Vec<CommentOutput>>,
}

impl Outputable for IssueDetailOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "title": self.title,
            "body": self.body,
            "state": self.state,
            "labels": self.labels,
            "assignees": self.assignees,
            "comment_count": self.comment_count,
            "created_at_ms": self.created_at_ms,
            "updated_at_ms": self.updated_at_ms,
            "comments": self.comments.as_ref().map(|cs| cs.iter().map(|c| c.to_json()).collect::<Vec<_>>())
        })
    }

    fn to_human(&self) -> String {
        let labels = if self.labels.is_empty() {
            String::new()
        } else {
            format!("\nLabels: {}", self.labels.join(", "))
        };
        let assignees = if self.assignees.is_empty() {
            String::new()
        } else {
            format!("\nAssignees: {}", self.assignees.iter().map(|a| &a[..8]).collect::<Vec<_>>().join(", "))
        };
        let comments_str = self
            .comments
            .as_ref()
            .map(|cs| {
                if cs.is_empty() {
                    String::new()
                } else {
                    format!("\n\nComments:\n{}", cs.iter().map(|c| c.to_human()).collect::<Vec<_>>().join("\n"))
                }
            })
            .unwrap_or_default();

        format!(
            "Issue #{} [{}]\n\
             =================\n\
             Title: {}{}{}\n\n\
             {}{}",
            &self.id[..8],
            self.state.to_uppercase(),
            self.title,
            labels,
            assignees,
            self.body,
            comments_str
        )
    }
}

/// Patch revision output.
pub struct RevisionOutput {
    pub hash: String,
    pub head: String,
    pub message: Option<String>,
    pub author: String,
    pub timestamp_ms: u64,
}

/// Patch approval output.
pub struct ApprovalOutput {
    pub author: String,
    pub commit: String,
    pub message: Option<String>,
    pub timestamp_ms: u64,
}

/// Patch summary output (for lists).
pub struct PatchOutput {
    pub id: String,
    pub title: String,
    pub state: String,
    pub base: String,
    pub head: String,
    pub labels: Vec<String>,
    pub revision_count: u32,
    pub approval_count: u32,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

impl Outputable for PatchOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "title": self.title,
            "state": self.state,
            "base": self.base,
            "head": self.head,
            "labels": self.labels,
            "revision_count": self.revision_count,
            "approval_count": self.approval_count,
            "created_at_ms": self.created_at_ms,
            "updated_at_ms": self.updated_at_ms
        })
    }

    fn to_human(&self) -> String {
        let labels = if self.labels.is_empty() {
            String::new()
        } else {
            format!(" [{}]", self.labels.join(", "))
        };
        format!(
            "!{} {} {}{} ({} revisions, {} approvals)",
            &self.id[..8],
            self.state.to_uppercase(),
            self.title,
            labels,
            self.revision_count,
            self.approval_count
        )
    }
}

/// Patch list output.
pub struct PatchListOutput {
    pub patches: Vec<PatchOutput>,
    pub count: u32,
}

impl Outputable for PatchListOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "count": self.count,
            "patches": self.patches.iter().map(|p| p.to_json()).collect::<Vec<_>>()
        })
    }

    fn to_human(&self) -> String {
        if self.patches.is_empty() {
            return "No patches found".to_string();
        }
        self.patches.iter().map(|p| p.to_human()).collect::<Vec<_>>().join("\n")
    }
}

/// Patch detail output.
pub struct PatchDetailOutput {
    pub id: String,
    pub title: String,
    pub description: String,
    pub state: String,
    pub base: String,
    pub head: String,
    pub labels: Vec<String>,
    pub assignees: Vec<String>,
    pub revision_count: u32,
    pub approval_count: u32,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub comments: Option<Vec<CommentOutput>>,
    pub revisions: Option<Vec<RevisionOutput>>,
    pub approvals: Option<Vec<ApprovalOutput>>,
}

impl Outputable for PatchDetailOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "title": self.title,
            "description": self.description,
            "state": self.state,
            "base": self.base,
            "head": self.head,
            "labels": self.labels,
            "assignees": self.assignees,
            "revision_count": self.revision_count,
            "approval_count": self.approval_count,
            "created_at_ms": self.created_at_ms,
            "updated_at_ms": self.updated_at_ms,
            "comments": self.comments.as_ref().map(|cs| cs.iter().map(|c| c.to_json()).collect::<Vec<_>>()),
            "revisions": self.revisions.as_ref().map(|rs| rs.iter().map(|r| {
                serde_json::json!({
                    "hash": r.hash,
                    "head": r.head,
                    "message": r.message,
                    "author": r.author,
                    "timestamp_ms": r.timestamp_ms
                })
            }).collect::<Vec<_>>()),
            "approvals": self.approvals.as_ref().map(|as_| as_.iter().map(|a| {
                serde_json::json!({
                    "author": a.author,
                    "commit": a.commit,
                    "message": a.message,
                    "timestamp_ms": a.timestamp_ms
                })
            }).collect::<Vec<_>>())
        })
    }

    fn to_human(&self) -> String {
        let labels = if self.labels.is_empty() {
            String::new()
        } else {
            format!("\nLabels: {}", self.labels.join(", "))
        };
        let assignees = if self.assignees.is_empty() {
            String::new()
        } else {
            format!("\nReviewers: {}", self.assignees.iter().map(|a| &a[..8]).collect::<Vec<_>>().join(", "))
        };
        let approvals_str = self
            .approvals
            .as_ref()
            .map(|as_| {
                if as_.is_empty() {
                    String::new()
                } else {
                    format!(
                        "\n\nApprovals ({}):\n{}",
                        as_.len(),
                        as_.iter()
                            .map(|a| {
                                let msg = a.message.as_deref().unwrap_or("");
                                format!("  {} approved {}... {}", &a.author[..8], &a.commit[..8], msg)
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                }
            })
            .unwrap_or_default();

        format!(
            "Patch !{} [{}]\n\
             =================\n\
             Title: {}\n\
             Base:  {}...\n\
             Head:  {}...{}{}\n\n\
             {}{}",
            &self.id[..8],
            self.state.to_uppercase(),
            self.title,
            &self.base[..12],
            &self.head[..12],
            labels,
            assignees,
            self.description,
            approvals_str
        )
    }
}
