//! Distributed lock commands.
//!
//! Commands for acquiring, releasing, and managing distributed locks
//! with fencing tokens for safe external operations.

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;

/// Distributed lock operations.
#[derive(Subcommand)]
pub enum LockCommand {
    /// Acquire a distributed lock (blocking).
    ///
    /// Blocks until the lock is acquired or timeout is reached.
    /// Returns a fencing token on success for safe external operations.
    Acquire(AcquireArgs),

    /// Try to acquire a lock (non-blocking).
    ///
    /// Returns immediately with success or failure.
    TryAcquire(TryAcquireArgs),

    /// Release a distributed lock.
    ///
    /// The fencing token must match the current lock holder.
    Release(ReleaseArgs),

    /// Renew a lock's TTL.
    ///
    /// Extends the lock deadline without releasing it.
    Renew(RenewArgs),
}

#[derive(Args)]
pub struct AcquireArgs {
    /// Lock key (unique identifier for this lock).
    pub key: String,

    /// Holder ID (unique identifier for this lock holder).
    #[arg(long)]
    pub holder: String,

    /// Lock TTL in milliseconds (how long before auto-expire).
    #[arg(long)]
    pub ttl: u64,

    /// Acquire timeout in milliseconds (how long to wait).
    #[arg(long, default_value = "5000")]
    pub timeout: u64,
}

#[derive(Args)]
pub struct TryAcquireArgs {
    /// Lock key.
    pub key: String,

    /// Holder ID.
    #[arg(long)]
    pub holder: String,

    /// Lock TTL in milliseconds.
    #[arg(long)]
    pub ttl: u64,
}

#[derive(Args)]
pub struct ReleaseArgs {
    /// Lock key.
    pub key: String,

    /// Holder ID that acquired the lock.
    #[arg(long)]
    pub holder: String,

    /// Fencing token from acquire operation.
    #[arg(long = "fencing-token")]
    pub fencing_token: u64,
}

#[derive(Args)]
pub struct RenewArgs {
    /// Lock key.
    pub key: String,

    /// Holder ID.
    #[arg(long)]
    pub holder: String,

    /// Fencing token from acquire operation.
    #[arg(long = "fencing-token")]
    pub fencing_token: u64,

    /// New TTL in milliseconds.
    #[arg(long)]
    pub ttl: u64,
}

/// Lock operation output.
pub struct LockOutput {
    pub operation: String,
    pub key: String,
    pub success: bool,
    pub fencing_token: Option<u64>,
    pub holder_id: Option<String>,
    pub deadline_ms: Option<u64>,
    pub error: Option<String>,
}

impl Outputable for LockOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "operation": self.operation,
            "key": self.key,
            "success": self.success,
            "fencing_token": self.fencing_token,
            "holder_id": self.holder_id,
            "deadline_ms": self.deadline_ms,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.success {
            match self.operation.as_str() {
                "acquire" | "try_acquire" => {
                    let token = self.fencing_token.map(|t| t.to_string()).unwrap_or_else(|| "N/A".to_string());
                    format!("Lock acquired. Fencing token: {}", token)
                }
                "release" => "Lock released.".to_string(),
                "renew" => {
                    let deadline = self.deadline_ms.map(|d| d.to_string()).unwrap_or_else(|| "N/A".to_string());
                    format!("Lock renewed. New deadline: {} ms", deadline)
                }
                _ => format!("{} succeeded", self.operation),
            }
        } else {
            match &self.error {
                Some(e) => format!("{} failed: {}", self.operation, e),
                None => {
                    if let Some(ref holder) = self.holder_id {
                        format!("Lock held by: {}", holder)
                    } else {
                        format!("{} failed", self.operation)
                    }
                }
            }
        }
    }
}

impl LockCommand {
    /// Execute the lock command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            LockCommand::Acquire(args) => lock_acquire(client, args, json).await,
            LockCommand::TryAcquire(args) => lock_try_acquire(client, args, json).await,
            LockCommand::Release(args) => lock_release(client, args, json).await,
            LockCommand::Renew(args) => lock_renew(client, args, json).await,
        }
    }
}

async fn lock_acquire(client: &AspenClient, args: AcquireArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::LockAcquire {
            key: args.key.clone(),
            holder_id: args.holder,
            ttl_ms: args.ttl,
            timeout_ms: args.timeout,
        })
        .await?;

    match response {
        ClientRpcResponse::LockResult(result) => {
            let output = LockOutput {
                operation: "acquire".to_string(),
                key: args.key,
                success: result.success,
                fencing_token: result.fencing_token,
                holder_id: result.holder_id,
                deadline_ms: result.deadline_ms,
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

async fn lock_try_acquire(client: &AspenClient, args: TryAcquireArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::LockTryAcquire {
            key: args.key.clone(),
            holder_id: args.holder,
            ttl_ms: args.ttl,
        })
        .await?;

    match response {
        ClientRpcResponse::LockResult(result) => {
            let output = LockOutput {
                operation: "try_acquire".to_string(),
                key: args.key,
                success: result.success,
                fencing_token: result.fencing_token,
                holder_id: result.holder_id,
                deadline_ms: result.deadline_ms,
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

async fn lock_release(client: &AspenClient, args: ReleaseArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::LockRelease {
            key: args.key.clone(),
            holder_id: args.holder,
            fencing_token: args.fencing_token,
        })
        .await?;

    match response {
        ClientRpcResponse::LockResult(result) => {
            let output = LockOutput {
                operation: "release".to_string(),
                key: args.key,
                success: result.success,
                fencing_token: None,
                holder_id: result.holder_id,
                deadline_ms: None,
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

async fn lock_renew(client: &AspenClient, args: RenewArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::LockRenew {
            key: args.key.clone(),
            holder_id: args.holder,
            fencing_token: args.fencing_token,
            ttl_ms: args.ttl,
        })
        .await?;

    match response {
        ClientRpcResponse::LockResult(result) => {
            let output = LockOutput {
                operation: "renew".to_string(),
                key: args.key,
                success: result.success,
                fencing_token: result.fencing_token,
                holder_id: result.holder_id,
                deadline_ms: result.deadline_ms,
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
