//! SQL query commands.
//!
//! Commands for executing read-only SQL queries against the distributed
//! state machine. Supports parameterized queries, consistency levels,
//! and various output formats.

use anyhow::Result;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use clap::Args;
use clap::Subcommand;
use clap::ValueEnum;

use crate::client::AspenClient;
use crate::output::SqlQueryOutput;
use crate::output::print_output;

/// SQL query operations.
///
/// Execute read-only SQL queries against the distributed key-value store.
/// Only SELECT and WITH...SELECT statements are allowed.
#[derive(Subcommand)]
pub enum SqlCommand {
    /// Execute a SQL query.
    ///
    /// Run read-only SQL queries against the state machine. The query must
    /// be a SELECT or WITH...SELECT statement. All user values should be
    /// passed as parameters to prevent SQL injection.
    Query(QueryArgs),

    /// Execute a SQL query from a file.
    ///
    /// Read the SQL query from a file and execute it. Useful for complex
    /// queries or when using shell scripts.
    File(FileArgs),
}

/// Consistency level for SQL queries.
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum Consistency {
    /// Linearizable reads (default).
    ///
    /// Uses Raft ReadIndex protocol to ensure reads reflect the most
    /// recent committed state. Slightly higher latency but guaranteed
    /// consistency.
    #[default]
    Linearizable,

    /// Stale reads from local state machine.
    ///
    /// Reads directly from the node's state machine without Raft
    /// coordination. Faster but may return stale data.
    Stale,
}

impl Consistency {
    /// Convert to string for RPC protocol.
    fn as_str(&self) -> &'static str {
        match self {
            Consistency::Linearizable => "linearizable",
            Consistency::Stale => "stale",
        }
    }
}

#[derive(Args)]
pub struct QueryArgs {
    /// SQL query to execute.
    ///
    /// Must be a SELECT or WITH...SELECT statement. Use ?1, ?2, etc.
    /// for parameters.
    ///
    /// Examples:
    ///   SELECT * FROM state_machine_kv
    ///   SELECT key, value FROM state_machine_kv WHERE key LIKE ?1
    ///   SELECT COUNT(*) FROM state_machine_kv
    pub query: String,

    /// Query parameters (JSON array).
    ///
    /// Parameters are bound to ?1, ?2, etc. in the query.
    /// Format: '["value1", 123, null]'
    ///
    /// Supported types: string, integer, float, null
    #[arg(long, short, default_value = "[]")]
    pub params: String,

    /// Consistency level for the query.
    #[arg(long, short, value_enum, default_value = "linearizable")]
    pub consistency: Consistency,

    /// Maximum rows to return.
    ///
    /// Default: 1000, Maximum: 10000
    #[arg(long, short, default_value = "1000")]
    pub limit: u32,

    /// Query timeout in milliseconds.
    ///
    /// Default: 5000ms (5 seconds), Maximum: 30000ms (30 seconds)
    #[arg(long = "query-timeout", default_value = "5000")]
    pub query_timeout: u32,

    /// Show column headers in output.
    #[arg(long, default_value = "true")]
    pub headers: bool,

    /// Output format for human-readable mode.
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,
}

#[derive(Args)]
pub struct FileArgs {
    /// Path to SQL file.
    pub file: std::path::PathBuf,

    /// Query parameters (JSON array).
    #[arg(long, short, default_value = "[]")]
    pub params: String,

    /// Consistency level for the query.
    #[arg(long, short, value_enum, default_value = "linearizable")]
    pub consistency: Consistency,

    /// Maximum rows to return.
    #[arg(long, short, default_value = "1000")]
    pub limit: u32,

    /// Query timeout in milliseconds.
    #[arg(long = "query-timeout", default_value = "5000")]
    pub query_timeout: u32,

    /// Show column headers in output.
    #[arg(long, default_value = "true")]
    pub headers: bool,

    /// Output format for human-readable mode.
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,
}

/// Output format for SQL results.
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum OutputFormat {
    /// Formatted table with borders.
    #[default]
    Table,

    /// Tab-separated values.
    Tsv,

    /// Comma-separated values.
    Csv,

    /// Vertical format (one column per line).
    Vertical,
}

impl SqlCommand {
    /// Execute the SQL command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            SqlCommand::Query(args) => sql_query(client, args, json).await,
            SqlCommand::File(args) => sql_file(client, args, json).await,
        }
    }
}

/// Configuration for SQL query execution.
struct SqlExecConfig {
    query: String,
    params: String,
    consistency: Consistency,
    limit: u32,
    timeout: u32,
    show_headers: bool,
    format: OutputFormat,
}

async fn sql_query(client: &AspenClient, args: QueryArgs, json: bool) -> Result<()> {
    let config = SqlExecConfig {
        query: args.query,
        params: args.params,
        consistency: args.consistency,
        limit: args.limit,
        timeout: args.query_timeout,
        show_headers: args.headers,
        format: args.format,
    };
    execute_sql(client, config, json).await
}

async fn sql_file(client: &AspenClient, args: FileArgs, json: bool) -> Result<()> {
    let query = std::fs::read_to_string(&args.file).map_err(|e| anyhow::anyhow!("failed to read file: {}", e))?;

    let config = SqlExecConfig {
        query,
        params: args.params,
        consistency: args.consistency,
        limit: args.limit,
        timeout: args.query_timeout,
        show_headers: args.headers,
        format: args.format,
    };
    execute_sql(client, config, json).await
}

async fn execute_sql(client: &AspenClient, config: SqlExecConfig, json: bool) -> Result<()> {
    // Validate limit bounds
    if config.limit > 10_000 {
        anyhow::bail!("limit cannot exceed 10000 rows");
    }

    // Validate timeout bounds
    if config.timeout > 30_000 {
        anyhow::bail!("timeout cannot exceed 30000ms");
    }

    // Validate params is valid JSON array
    let _: serde_json::Value =
        serde_json::from_str(&config.params).map_err(|e| anyhow::anyhow!("invalid params JSON: {}", e))?;

    let response = client
        .send(ClientRpcRequest::ExecuteSql {
            query: config.query,
            params: config.params,
            consistency: config.consistency.as_str().to_string(),
            limit: Some(config.limit),
            timeout_ms: Some(config.timeout),
        })
        .await?;

    match response {
        ClientRpcResponse::SqlResult(result) => {
            if !result.success {
                let error = result.error.unwrap_or_else(|| "unknown error".to_string());
                anyhow::bail!("query failed: {}", error);
            }

            let output = SqlQueryOutput {
                columns: result.columns.unwrap_or_default(),
                rows: result.rows.unwrap_or_default(),
                row_count: result.row_count.unwrap_or(0),
                is_truncated: result.is_truncated.unwrap_or(false),
                execution_time_ms: result.execution_time_ms.unwrap_or(0),
                show_headers: config.show_headers,
                format: config.format,
            };

            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}
