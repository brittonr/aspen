//! Distributed barrier commands.
//!
//! Commands for coordinating multiple participants at synchronization points.
//! Participants wait until all have arrived before proceeding.

use anyhow::Result;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;

/// Distributed barrier operations.
#[derive(Subcommand)]
pub enum BarrierCommand {
    /// Enter a barrier, waiting until all participants arrive.
    Enter(EnterArgs),

    /// Leave a barrier after work is complete.
    Leave(LeaveArgs),

    /// Query barrier status without blocking.
    Status(StatusArgs),
}

#[derive(Args)]
pub struct EnterArgs {
    /// Barrier name (unique identifier).
    pub name: String,

    /// Unique identifier for this participant.
    #[arg(long)]
    pub participant: String,

    /// Number of participants required to release the barrier.
    #[arg(long)]
    pub count: u32,

    /// Timeout in milliseconds (0 = no timeout).
    #[arg(long, default_value = "30000")]
    pub timeout: u64,
}

#[derive(Args)]
pub struct LeaveArgs {
    /// Barrier name.
    pub name: String,

    /// Participant ID that is leaving.
    #[arg(long)]
    pub participant: String,

    /// Timeout in milliseconds (0 = no timeout).
    #[arg(long, default_value = "30000")]
    pub timeout: u64,
}

#[derive(Args)]
pub struct StatusArgs {
    /// Barrier name.
    pub name: String,
}

/// Barrier operation output.
pub struct BarrierOutput {
    pub operation: String,
    pub name: String,
    pub success: bool,
    pub current_count: Option<u32>,
    pub required_count: Option<u32>,
    pub phase: Option<String>,
    pub error: Option<String>,
}

impl Outputable for BarrierOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "operation": self.operation,
            "name": self.name,
            "success": self.success,
            "current_count": self.current_count,
            "required_count": self.required_count,
            "phase": self.phase,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.success {
            let current = self.current_count.unwrap_or(0);
            let required = self.required_count.unwrap_or(0);
            let phase = self.phase.as_deref().unwrap_or("unknown");
            format!("OK ({}/{} participants, phase: {})", current, required, phase)
        } else {
            match &self.error {
                Some(e) => format!("{} failed: {}", self.operation, e),
                None => format!("{} failed", self.operation),
            }
        }
    }
}

impl BarrierCommand {
    /// Execute the barrier command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            BarrierCommand::Enter(args) => barrier_enter(client, args, json).await,
            BarrierCommand::Leave(args) => barrier_leave(client, args, json).await,
            BarrierCommand::Status(args) => barrier_status(client, args, json).await,
        }
    }
}

async fn barrier_enter(client: &AspenClient, args: EnterArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::BarrierEnter {
            name: args.name.clone(),
            participant_id: args.participant,
            required_count: args.count,
            timeout_ms: args.timeout,
        })
        .await?;

    match response {
        ClientRpcResponse::BarrierEnterResult(result) => {
            let output = BarrierOutput {
                operation: "enter".to_string(),
                name: args.name,
                success: result.success,
                current_count: result.current_count,
                required_count: result.required_count,
                phase: result.phase,
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

async fn barrier_leave(client: &AspenClient, args: LeaveArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::BarrierLeave {
            name: args.name.clone(),
            participant_id: args.participant,
            timeout_ms: args.timeout,
        })
        .await?;

    match response {
        ClientRpcResponse::BarrierLeaveResult(result) => {
            let output = BarrierOutput {
                operation: "leave".to_string(),
                name: args.name,
                success: result.success,
                current_count: result.current_count,
                required_count: result.required_count,
                phase: result.phase,
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

async fn barrier_status(client: &AspenClient, args: StatusArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::BarrierStatus {
            name: args.name.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::BarrierStatusResult(result) => {
            let output = BarrierOutput {
                operation: "status".to_string(),
                name: args.name,
                success: result.success,
                current_count: result.current_count,
                required_count: result.required_count,
                phase: result.phase,
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
