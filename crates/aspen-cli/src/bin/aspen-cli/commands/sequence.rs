//! Sequence generator commands.
//!
//! Commands for distributed sequence generators that produce
//! unique, monotonically increasing IDs.

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;

/// Sequence generator operations.
#[derive(Subcommand)]
pub enum SequenceCommand {
    /// Get the next unique ID from a sequence.
    Next(NextArgs),

    /// Reserve a range of IDs from a sequence.
    ///
    /// Returns the start of the reserved range [start, start+count).
    Reserve(ReserveArgs),

    /// Get the current (next available) value without consuming it.
    Current(CurrentArgs),
}

#[derive(Args)]
pub struct NextArgs {
    /// Sequence key.
    pub key: String,
}

#[derive(Args)]
pub struct ReserveArgs {
    /// Sequence key.
    pub key: String,

    /// Number of IDs to reserve.
    pub count: u64,
}

#[derive(Args)]
pub struct CurrentArgs {
    /// Sequence key.
    pub key: String,
}

/// Sequence operation output.
pub struct SequenceOutput {
    pub key: String,
    pub operation: String,
    pub is_success: bool,
    pub value: Option<u64>,
    pub error: Option<String>,
}

impl Outputable for SequenceOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "key": self.key,
            "operation": self.operation,
            "is_success": self.is_success,
            "value": self.value,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.is_success {
            match self.value {
                Some(v) => v.to_string(),
                None => "OK".to_string(),
            }
        } else {
            match &self.error {
                Some(e) => format!("{} failed: {}", self.operation, e),
                None => format!("{} failed", self.operation),
            }
        }
    }
}

impl SequenceCommand {
    /// Execute the sequence command.
    pub async fn run(self, client: &AspenClient, is_json_output: bool) -> Result<()> {
        match self {
            SequenceCommand::Next(args) => sequence_next(client, args, is_json_output).await,
            SequenceCommand::Reserve(args) => sequence_reserve(client, args, is_json_output).await,
            SequenceCommand::Current(args) => sequence_current(client, args, is_json_output).await,
        }
    }
}

fn print_checked_sequence_output(output: &SequenceOutput, is_json_output: bool) {
    print_output(output, is_json_output);
    if !output.is_success {
        std::process::exit(1);
    }
}

async fn sequence_next(client: &AspenClient, args: NextArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.key.is_empty(), "sequence key must not be empty");
    let response = client.send(ClientRpcRequest::SequenceNext { key: args.key.clone() }).await?;

    match response {
        ClientRpcResponse::SequenceResult(result) => {
            let output = SequenceOutput {
                key: args.key,
                operation: "next".to_string(),
                is_success: result.is_success,
                value: result.value,
                error: result.error,
            };
            print_checked_sequence_output(&output, is_json_output);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn sequence_reserve(client: &AspenClient, args: ReserveArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.key.is_empty(), "sequence key must not be empty");
    debug_assert!(args.count >= 1, "reserve count must be positive");
    let response = client
        .send(ClientRpcRequest::SequenceReserve {
            key: args.key.clone(),
            count: args.count,
        })
        .await?;

    match response {
        ClientRpcResponse::SequenceResult(result) => {
            let output = SequenceOutput {
                key: args.key,
                operation: "reserve".to_string(),
                is_success: result.is_success,
                value: result.value,
                error: result.error,
            };
            print_checked_sequence_output(&output, is_json_output);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn sequence_current(client: &AspenClient, args: CurrentArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.key.is_empty(), "sequence key must not be empty");
    let response = client.send(ClientRpcRequest::SequenceCurrent { key: args.key.clone() }).await?;

    match response {
        ClientRpcResponse::SequenceResult(result) => {
            let output = SequenceOutput {
                key: args.key,
                operation: "current".to_string(),
                is_success: result.is_success,
                value: result.value,
                error: result.error,
            };
            print_checked_sequence_output(&output, is_json_output);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}
