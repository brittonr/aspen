//! Key-value store commands.
//!
//! Commands for reading, writing, deleting, and scanning keys
//! in the distributed key-value store.

use anyhow::Result;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::KvBatchReadOutput;
use crate::output::KvBatchWriteOutput;
use crate::output::KvReadOutput;
use crate::output::KvScanOutput;
use crate::output::print_output;
use aspen_client_rpc::BatchWriteOperation;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;

/// Key-value store operations.
#[derive(Subcommand)]
pub enum KvCommand {
    /// Read a key from the store.
    Get(GetArgs),

    /// Write a key-value pair to the store.
    Set(SetArgs),

    /// Delete a key from the store.
    Delete(DeleteArgs),

    /// Compare-and-swap: atomically update if current value matches.
    Cas(CasArgs),

    /// Compare-and-delete: atomically delete if current value matches.
    Cad(CadArgs),

    /// Scan keys by prefix.
    Scan(ScanArgs),

    /// Read multiple keys in a single request.
    ///
    /// More efficient than multiple get commands. All keys are read atomically
    /// from a consistent snapshot.
    BatchRead(BatchReadArgs),

    /// Write multiple key-value pairs atomically.
    ///
    /// All operations are applied in a single Raft log entry,
    /// ensuring atomic all-or-nothing execution.
    BatchWrite(BatchWriteArgs),
}

#[derive(Args)]
pub struct GetArgs {
    /// Key to read.
    pub key: String,
}

#[derive(Args)]
pub struct SetArgs {
    /// Key to write.
    pub key: String,

    /// Value to write (string, use --file for binary).
    pub value: String,

    /// Read value from file instead of argument.
    #[arg(long, short)]
    pub file: Option<std::path::PathBuf>,
}

#[derive(Args)]
pub struct DeleteArgs {
    /// Key to delete.
    pub key: String,
}

#[derive(Args)]
pub struct CasArgs {
    /// Key to update.
    pub key: String,

    /// Expected current value (omit for create-if-absent).
    #[arg(long)]
    pub expected: Option<String>,

    /// New value to set.
    #[arg(long)]
    pub new_value: String,
}

#[derive(Args)]
pub struct CadArgs {
    /// Key to delete.
    pub key: String,

    /// Expected current value.
    #[arg(long)]
    pub expected: String,
}

#[derive(Args)]
pub struct ScanArgs {
    /// Key prefix to match (empty matches all).
    #[arg(default_value = "")]
    pub prefix: String,

    /// Maximum number of results.
    #[arg(long, default_value = "100")]
    pub limit: u32,

    /// Continuation token from previous scan.
    #[arg(long)]
    pub token: Option<String>,
}

#[derive(Args)]
pub struct BatchReadArgs {
    /// Keys to read (space-separated, max 100).
    #[arg(required = true, num_args = 1..=100)]
    pub keys: Vec<String>,
}

#[derive(Args)]
pub struct BatchWriteArgs {
    /// Key-value pairs to write (format: key=value, space-separated, max 100).
    ///
    /// Example: aspen-cli kv batch-write foo=bar baz=qux
    #[arg(required = true, num_args = 1..=100, value_parser = parse_kv_pair)]
    pub pairs: Vec<(String, String)>,
}

/// Parse a key=value pair from command line argument.
fn parse_kv_pair(s: &str) -> Result<(String, String), String> {
    let pos = s.find('=').ok_or_else(|| format!("invalid KEY=VALUE pair: no '=' found in '{}'", s))?;
    let key = s[..pos].to_string();
    let value = s[pos + 1..].to_string();
    if key.is_empty() {
        return Err("key cannot be empty".to_string());
    }
    Ok((key, value))
}

impl KvCommand {
    /// Execute the key-value command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            KvCommand::Get(args) => kv_get(client, args, json).await,
            KvCommand::Set(args) => kv_set(client, args, json).await,
            KvCommand::Delete(args) => kv_delete(client, args, json).await,
            KvCommand::Cas(args) => kv_cas(client, args, json).await,
            KvCommand::Cad(args) => kv_cad(client, args, json).await,
            KvCommand::Scan(args) => kv_scan(client, args, json).await,
            KvCommand::BatchRead(args) => kv_batch_read(client, args, json).await,
            KvCommand::BatchWrite(args) => kv_batch_write(client, args, json).await,
        }
    }
}

async fn kv_get(client: &AspenClient, args: GetArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::ReadKey { key: args.key.clone() }).await?;

    match response {
        ClientRpcResponse::ReadResult(result) => {
            let output = KvReadOutput {
                key: args.key,
                value: result.value,
                exists: result.found,
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

async fn kv_set(client: &AspenClient, args: SetArgs, json: bool) -> Result<()> {
    // Get value from file or argument
    let value = if let Some(ref path) = args.file {
        std::fs::read(path).map_err(|e| anyhow::anyhow!("failed to read file: {}", e))?
    } else {
        args.value.as_bytes().to_vec()
    };

    let response = client
        .send(ClientRpcRequest::WriteKey {
            key: args.key.clone(),
            value,
        })
        .await?;

    match response {
        ClientRpcResponse::WriteResult(result) => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": if result.success { "success" } else { "failed" },
                        "key": args.key
                    })
                );
            } else if result.success {
                println!("OK");
            } else {
                println!("Write failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()));
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn kv_delete(client: &AspenClient, args: DeleteArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::DeleteKey { key: args.key.clone() }).await?;

    match response {
        ClientRpcResponse::DeleteResult(result) => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "success",
                        "key": args.key,
                        "deleted": result.deleted
                    })
                );
            } else if result.deleted {
                println!("Deleted");
            } else {
                println!("Key not found");
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn kv_cas(client: &AspenClient, args: CasArgs, json: bool) -> Result<()> {
    let expected = args.expected.map(|s| s.into_bytes());
    let new_value = args.new_value.into_bytes();

    let response = client
        .send(ClientRpcRequest::CompareAndSwapKey {
            key: args.key.clone(),
            expected,
            new_value,
        })
        .await?;

    match response {
        ClientRpcResponse::CompareAndSwapResult(result) => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": if result.success { "success" } else { "conflict" },
                        "key": args.key,
                        "success": result.success
                    })
                );
            } else if result.success {
                println!("OK");
            } else {
                println!("CONFLICT: value has changed");
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn kv_cad(client: &AspenClient, args: CadArgs, json: bool) -> Result<()> {
    let expected = args.expected.into_bytes();

    let response = client
        .send(ClientRpcRequest::CompareAndDeleteKey {
            key: args.key.clone(),
            expected,
        })
        .await?;

    // CompareAndDeleteKey returns CompareAndSwapResult (same response type)
    match response {
        ClientRpcResponse::CompareAndSwapResult(result) => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": if result.success { "success" } else { "conflict" },
                        "key": args.key,
                        "success": result.success
                    })
                );
            } else if result.success {
                println!("Deleted");
            } else {
                println!("CONFLICT: value has changed");
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn kv_scan(client: &AspenClient, args: ScanArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ScanKeys {
            prefix: args.prefix,
            limit: Some(args.limit),
            continuation_token: args.token,
        })
        .await?;

    match response {
        ClientRpcResponse::ScanResult(result) => {
            // ScanEntry has value as String, convert to Vec<u8> for output
            let output = KvScanOutput {
                entries: result.entries.into_iter().map(|e| (e.key, e.value.into_bytes())).collect(),
                continuation_token: result.continuation_token,
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

async fn kv_batch_read(client: &AspenClient, args: BatchReadArgs, json: bool) -> Result<()> {
    let keys = args.keys.clone();
    let response = client.send(ClientRpcRequest::BatchRead { keys: args.keys }).await?;

    match response {
        ClientRpcResponse::BatchReadResult(result) => {
            if !result.success {
                anyhow::bail!("batch read failed");
            }
            let values = result.values.unwrap_or_default();
            let output = KvBatchReadOutput { keys, values };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn kv_batch_write(client: &AspenClient, args: BatchWriteArgs, json: bool) -> Result<()> {
    let operations: Vec<BatchWriteOperation> = args
        .pairs
        .into_iter()
        .map(|(key, value)| BatchWriteOperation::Set {
            key,
            value: value.into_bytes(),
        })
        .collect();

    let op_count = operations.len() as u32;
    let response = client.send(ClientRpcRequest::BatchWrite { operations }).await?;

    match response {
        ClientRpcResponse::BatchWriteResult(result) => {
            let output = KvBatchWriteOutput {
                success: result.success,
                operations_applied: result.operations_applied.unwrap_or(op_count),
            };
            print_output(&output, json);
            if !result.success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}
