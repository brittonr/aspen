//! Sequence generator commands.
//!
//! Commands for distributed sequence generators that produce
//! unique, monotonically increasing IDs.

use anyhow::Result;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
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
    pub success: bool,
    pub value: Option<u64>,
    pub error: Option<String>,
}

impl Outputable for SequenceOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "key": self.key,
            "operation": self.operation,
            "success": self.success,
            "value": self.value,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.success {
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
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            SequenceCommand::Next(args) => sequence_next(client, args, json).await,
            SequenceCommand::Reserve(args) => sequence_reserve(client, args, json).await,
            SequenceCommand::Current(args) => sequence_current(client, args, json).await,
        }
    }
}

async fn sequence_next(client: &AspenClient, args: NextArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::SequenceNext { key: args.key.clone() }).await?;

    match response {
        ClientRpcResponse::SequenceResult(result) => {
            let output = SequenceOutput {
                key: args.key,
                operation: "next".to_string(),
                success: result.success,
                value: result.value,
                error: result.error,
            };
            print_output(&output, json);
            if !result.success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn sequence_reserve(client: &AspenClient, args: ReserveArgs, json: bool) -> Result<()> {
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
                success: result.success,
                value: result.value,
                error: result.error,
            };
            print_output(&output, json);
            if !result.success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn sequence_current(client: &AspenClient, args: CurrentArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::SequenceCurrent { key: args.key.clone() }).await?;

    match response {
        ClientRpcResponse::SequenceResult(result) => {
            let output = SequenceOutput {
                key: args.key,
                operation: "current".to_string(),
                success: result.success,
                value: result.value,
                error: result.error,
            };
            print_output(&output, json);
            if !result.success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}
