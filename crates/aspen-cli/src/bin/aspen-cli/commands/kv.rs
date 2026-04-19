//! Key-value store commands.
//!
//! Commands for reading, writing, deleting, and scanning keys
//! in the distributed key-value store.

use anyhow::Result;
use aspen_client_api::BatchWriteOperation;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::KvBatchReadOutput;
use crate::output::KvBatchWriteOutput;
use crate::output::KvReadOutput;
use crate::output::KvScanOutput;
use crate::output::print_output;

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

    /// Value to write (string). Omit when using --file.
    #[arg(required_unless_present = "file")]
    pub value: Option<String>,

    /// Read value from file instead of argument.
    #[arg(long, short, conflicts_with = "value")]
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
    #[arg(long = "limit", default_value = "100")]
    pub max_results: u32,

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
fn parse_kv_pair(input: &str) -> Result<(String, String), String> {
    let separator_index =
        input.find('=').ok_or_else(|| format!("invalid KEY=VALUE pair: no '=' found in '{}'", input))?;
    let value_start = separator_index.checked_add(1).ok_or_else(|| "pair separator offset overflowed".to_string())?;
    let key = input[..separator_index].to_string();
    let value = input
        .get(value_start..)
        .ok_or_else(|| "pair value slice fell outside the argument bounds".to_string())?
        .to_string();
    if key.is_empty() {
        return Err("key cannot be empty".to_string());
    }
    Ok((key, value))
}

impl KvCommand {
    /// Execute the key-value command.
    pub async fn run(self, client: &AspenClient, is_json: bool) -> Result<()> {
        match self {
            KvCommand::Get(args) => kv_get(client, args, is_json).await,
            KvCommand::Set(args) => kv_set(client, args, is_json).await,
            KvCommand::Delete(args) => kv_delete(client, args, is_json).await,
            KvCommand::Cas(args) => kv_cas(client, args, is_json).await,
            KvCommand::Cad(args) => kv_cad(client, args, is_json).await,
            KvCommand::Scan(args) => kv_scan(client, args, is_json).await,
            KvCommand::BatchRead(args) => kv_batch_read(client, args, is_json).await,
            KvCommand::BatchWrite(args) => kv_batch_write(client, args, is_json).await,
        }
    }
}

async fn kv_get(client: &AspenClient, args: GetArgs, is_json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::ReadKey { key: args.key.clone() }).await?;

    match response {
        ClientRpcResponse::ReadResult(result) => {
            let output = KvReadOutput {
                key: args.key,
                value: result.value,
                does_exist: result.was_found,
            };
            print_output(&output, is_json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn kv_set(client: &AspenClient, args: SetArgs, is_json: bool) -> Result<()> {
    debug_assert!(!args.key.is_empty());
    debug_assert!(!args.key.contains('\0'));
    debug_assert!(args.file.is_some() || args.value.is_some());
    let key = args.key;

    let value = if let Some(ref path) = args.file {
        std::fs::read(path).map_err(|e| anyhow::anyhow!("failed to read file '{}': {}", path.display(), e))?
    } else {
        match args.value {
            Some(value_text) => value_text.into_bytes(),
            None => return Err(anyhow::anyhow!("value required when --file not provided")),
        }
    };

    let response = client
        .send(ClientRpcRequest::WriteKey {
            key: key.clone(),
            value,
        })
        .await?;

    match response {
        ClientRpcResponse::WriteResult(result) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": if result.is_success { "success" } else { "failed" },
                        "key": key,
                    })
                );
            } else if result.is_success {
                println!("OK");
            } else {
                println!("Write failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()));
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn kv_delete(client: &AspenClient, args: DeleteArgs, is_json: bool) -> Result<()> {
    debug_assert!(!args.key.is_empty());
    debug_assert!(!args.key.contains('\0'));
    let key = args.key;
    let response = client.send(ClientRpcRequest::DeleteKey { key: key.clone() }).await?;

    match response {
        ClientRpcResponse::DeleteResult(result) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "success",
                        "key": key,
                        "was_deleted": result.was_deleted,
                    })
                );
            } else if result.was_deleted {
                println!("Deleted");
            } else {
                println!("Key not found");
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn kv_cas(client: &AspenClient, args: CasArgs, is_json: bool) -> Result<()> {
    debug_assert!(!args.key.is_empty());
    debug_assert!(!args.key.contains('\0'));
    let key = args.key;
    let expected = args.expected.map(|text| text.into_bytes());
    let new_value = args.new_value.into_bytes();

    let response = client
        .send(ClientRpcRequest::CompareAndSwapKey {
            key: key.clone(),
            expected,
            new_value,
        })
        .await?;

    match response {
        ClientRpcResponse::CompareAndSwapResult(result) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": if result.is_success { "success" } else { "conflict" },
                        "key": key,
                        "is_success": result.is_success,
                    })
                );
            } else if result.is_success {
                println!("OK");
            } else {
                println!("CONFLICT: value has changed");
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn kv_cad(client: &AspenClient, args: CadArgs, is_json: bool) -> Result<()> {
    debug_assert!(!args.key.is_empty());
    debug_assert!(!args.key.contains('\0'));
    let key = args.key;
    let expected = args.expected.into_bytes();

    let response = client
        .send(ClientRpcRequest::CompareAndDeleteKey {
            key: key.clone(),
            expected,
        })
        .await?;

    match response {
        ClientRpcResponse::CompareAndSwapResult(result) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": if result.is_success { "success" } else { "conflict" },
                        "key": key,
                        "is_success": result.is_success,
                    })
                );
            } else if result.is_success {
                println!("Deleted");
            } else {
                println!("CONFLICT: value has changed");
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn kv_scan(client: &AspenClient, args: ScanArgs, is_json: bool) -> Result<()> {
    debug_assert!(args.max_results > 0);
    debug_assert!(!args.prefix.contains('\0'));
    let response = client
        .send(ClientRpcRequest::ScanKeys {
            prefix: args.prefix,
            limit: Some(args.max_results),
            continuation_token: args.token,
        })
        .await?;

    match response {
        ClientRpcResponse::ScanResult(result) => {
            let output = KvScanOutput {
                entries: result.entries.into_iter().map(|entry| (entry.key, entry.value.into_bytes())).collect(),
                continuation_token: result.continuation_token,
            };
            debug_assert!(output.continuation_token.as_deref().is_none_or(|token| !token.is_empty()));
            debug_assert!(output.entries.iter().all(|(key, _)| !key.is_empty()));
            print_output(&output, is_json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn kv_batch_read(client: &AspenClient, args: BatchReadArgs, is_json: bool) -> Result<()> {
    let keys = args.keys.clone();
    let response = client.send(ClientRpcRequest::BatchRead { keys: args.keys }).await?;

    match response {
        ClientRpcResponse::BatchReadResult(result) => {
            if !result.is_success {
                anyhow::bail!("batch read failed");
            }
            let values = result.values.unwrap_or_default();
            let output = KvBatchReadOutput { keys, values };
            print_output(&output, is_json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn kv_batch_write(client: &AspenClient, args: BatchWriteArgs, is_json: bool) -> Result<()> {
    debug_assert!(!args.pairs.is_empty());
    debug_assert!(args.pairs.len() <= 100);
    let operations: Vec<BatchWriteOperation> = args
        .pairs
        .into_iter()
        .map(|(key, value)| BatchWriteOperation::Set {
            key,
            value: value.into_bytes(),
        })
        .collect();

    let op_count = u32::try_from(operations.len()).unwrap_or(u32::MAX);
    let response = client.send(ClientRpcRequest::BatchWrite { operations }).await?;

    match response {
        ClientRpcResponse::BatchWriteResult(result) => {
            let output = KvBatchWriteOutput {
                is_success: result.is_success,
                operations_applied: result.operations_applied.unwrap_or(op_count),
            };
            debug_assert!(output.operations_applied <= op_count || !output.is_success);
            debug_assert!(output.is_success || output.operations_applied <= op_count);
            print_output(&output, is_json);
            if !result.is_success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}
