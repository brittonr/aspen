//! Atomic counter commands.
//!
//! Commands for distributed atomic counters with increment, decrement,
//! add, subtract, set, and compare-and-set operations.

use anyhow::Result;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;

/// Atomic counter operations.
#[derive(Subcommand)]
pub enum CounterCommand {
    /// Get the current value of a counter.
    Get(GetArgs),

    /// Increment a counter by 1.
    Incr(IncrArgs),

    /// Decrement a counter by 1 (saturates at 0).
    Decr(DecrArgs),

    /// Add an amount to a counter.
    Add(AddArgs),

    /// Subtract an amount from a counter (saturates at 0).
    Sub(SubArgs),

    /// Set a counter to a specific value.
    Set(SetArgs),

    /// Compare-and-set: update only if current value matches expected.
    Cas(CasArgs),
}

#[derive(Args)]
pub struct GetArgs {
    /// Counter key.
    pub key: String,
}

#[derive(Args)]
pub struct IncrArgs {
    /// Counter key.
    pub key: String,
}

#[derive(Args)]
pub struct DecrArgs {
    /// Counter key.
    pub key: String,
}

#[derive(Args)]
pub struct AddArgs {
    /// Counter key.
    pub key: String,

    /// Amount to add.
    pub amount: u64,
}

#[derive(Args)]
pub struct SubArgs {
    /// Counter key.
    pub key: String,

    /// Amount to subtract.
    pub amount: u64,
}

#[derive(Args)]
pub struct SetArgs {
    /// Counter key.
    pub key: String,

    /// New value.
    pub value: u64,
}

#[derive(Args)]
pub struct CasArgs {
    /// Counter key.
    pub key: String,

    /// Expected current value.
    #[arg(long)]
    pub expected: u64,

    /// New value to set.
    #[arg(long)]
    pub new_value: u64,
}

/// Counter operation output.
pub struct CounterOutput {
    pub key: String,
    pub operation: String,
    pub success: bool,
    pub value: Option<u64>,
    pub error: Option<String>,
}

impl Outputable for CounterOutput {
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

impl CounterCommand {
    /// Execute the counter command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            CounterCommand::Get(args) => counter_get(client, args, json).await,
            CounterCommand::Incr(args) => counter_incr(client, args, json).await,
            CounterCommand::Decr(args) => counter_decr(client, args, json).await,
            CounterCommand::Add(args) => counter_add(client, args, json).await,
            CounterCommand::Sub(args) => counter_sub(client, args, json).await,
            CounterCommand::Set(args) => counter_set(client, args, json).await,
            CounterCommand::Cas(args) => counter_cas(client, args, json).await,
        }
    }
}

async fn counter_get(client: &AspenClient, args: GetArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::CounterGet { key: args.key.clone() }).await?;

    match response {
        ClientRpcResponse::CounterResult(result) => {
            let output = CounterOutput {
                key: args.key,
                operation: "get".to_string(),
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

async fn counter_incr(client: &AspenClient, args: IncrArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::CounterIncrement { key: args.key.clone() }).await?;

    match response {
        ClientRpcResponse::CounterResult(result) => {
            let output = CounterOutput {
                key: args.key,
                operation: "incr".to_string(),
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

async fn counter_decr(client: &AspenClient, args: DecrArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::CounterDecrement { key: args.key.clone() }).await?;

    match response {
        ClientRpcResponse::CounterResult(result) => {
            let output = CounterOutput {
                key: args.key,
                operation: "decr".to_string(),
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

async fn counter_add(client: &AspenClient, args: AddArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::CounterAdd {
            key: args.key.clone(),
            amount: args.amount,
        })
        .await?;

    match response {
        ClientRpcResponse::CounterResult(result) => {
            let output = CounterOutput {
                key: args.key,
                operation: "add".to_string(),
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

async fn counter_sub(client: &AspenClient, args: SubArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::CounterSubtract {
            key: args.key.clone(),
            amount: args.amount,
        })
        .await?;

    match response {
        ClientRpcResponse::CounterResult(result) => {
            let output = CounterOutput {
                key: args.key,
                operation: "sub".to_string(),
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

async fn counter_set(client: &AspenClient, args: SetArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::CounterSet {
            key: args.key.clone(),
            value: args.value,
        })
        .await?;

    match response {
        ClientRpcResponse::CounterResult(result) => {
            let output = CounterOutput {
                key: args.key,
                operation: "set".to_string(),
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

async fn counter_cas(client: &AspenClient, args: CasArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::CounterCompareAndSet {
            key: args.key.clone(),
            expected: args.expected,
            new_value: args.new_value,
        })
        .await?;

    match response {
        ClientRpcResponse::CounterResult(result) => {
            let output = CounterOutput {
                key: args.key,
                operation: "cas".to_string(),
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
