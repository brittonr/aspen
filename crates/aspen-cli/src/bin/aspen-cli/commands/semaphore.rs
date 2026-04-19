//! Distributed semaphore commands.
//!
//! Commands for coordinating access to limited resources with permit-based
//! semaphores that support TTL and automatic release.

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;

/// Distributed semaphore operations.
#[derive(Subcommand)]
pub enum SemaphoreCommand {
    /// Acquire permits (blocking with timeout).
    Acquire(AcquireArgs),

    /// Try to acquire permits (non-blocking).
    TryAcquire(TryAcquireArgs),

    /// Release permits back to the semaphore.
    Release(ReleaseArgs),

    /// Query semaphore status.
    Status(StatusArgs),
}

#[derive(Args)]
pub struct AcquireArgs {
    /// Semaphore name.
    pub name: String,

    /// Holder ID for tracking ownership.
    #[arg(long)]
    pub holder: String,

    /// Number of permits to acquire.
    #[arg(long, default_value = "1")]
    pub permits: u32,

    /// Maximum permits (semaphore capacity).
    #[arg(long = "capacity")]
    pub max_permits: u32,

    /// TTL in milliseconds for automatic release.
    #[arg(long = "ttl", default_value = "30000")]
    pub ttl_ms: u64,

    /// Timeout in milliseconds (0 = no timeout).
    #[arg(long = "timeout", default_value = "5000")]
    pub timeout_ms: u64,
}

#[derive(Args)]
pub struct TryAcquireArgs {
    /// Semaphore name.
    pub name: String,

    /// Holder ID for tracking ownership.
    #[arg(long)]
    pub holder: String,

    /// Number of permits to acquire.
    #[arg(long, default_value = "1")]
    pub permits: u32,

    /// Maximum permits (semaphore capacity).
    #[arg(long = "capacity")]
    pub max_permits: u32,

    /// TTL in milliseconds for automatic release.
    #[arg(long = "ttl", default_value = "30000")]
    pub ttl_ms: u64,
}

#[derive(Args)]
pub struct ReleaseArgs {
    /// Semaphore name.
    pub name: String,

    /// Holder ID that acquired the permits.
    #[arg(long)]
    pub holder: String,

    /// Number of permits to release (0 = all).
    #[arg(long, default_value = "0")]
    pub permits: u32,
}

#[derive(Args)]
pub struct StatusArgs {
    /// Semaphore name.
    pub name: String,
}

/// Semaphore operation output.
pub struct SemaphoreOutput {
    pub operation: String,
    pub name: String,
    pub is_success: bool,
    pub permits_acquired: Option<u32>,
    pub available: Option<u32>,
    pub max_permits: Option<u32>,
    pub retry_after_ms: Option<u64>,
    pub error: Option<String>,
}

impl Outputable for SemaphoreOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "operation": self.operation,
            "name": self.name,
            "is_success": self.is_success,
            "permits_acquired": self.permits_acquired,
            "available": self.available,
            "max_permits": self.max_permits,
            "retry_after_ms": self.retry_after_ms,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.is_success {
            match self.operation.as_str() {
                "acquire" | "try_acquire" => {
                    let acquired = self.permits_acquired.unwrap_or(0);
                    let available = self.available.unwrap_or(0);
                    let max_permits = self.max_permits.unwrap_or(0);
                    format!("Acquired {} permits ({}/{} available)", acquired, available, max_permits)
                }
                "release" => {
                    let available = self.available.unwrap_or(0);
                    let max_permits = self.max_permits.unwrap_or(0);
                    format!("Released ({}/{} available)", available, max_permits)
                }
                "status" => {
                    let available = self.available.unwrap_or(0);
                    let max_permits = self.max_permits.unwrap_or(0);
                    format!("Available: {}/{}", available, max_permits)
                }
                _ => "OK".to_string(),
            }
        } else {
            match self.retry_after_ms {
                Some(ms) => format!("No permits available. Retry after {} ms", ms),
                None => match &self.error {
                    Some(e) => format!("{} failed: {}", self.operation, e),
                    None => format!("{} failed", self.operation),
                },
            }
        }
    }
}

impl SemaphoreCommand {
    /// Execute the semaphore command.
    pub async fn run(self, client: &AspenClient, is_json_output: bool) -> Result<()> {
        match self {
            SemaphoreCommand::Acquire(args) => semaphore_acquire(client, args, is_json_output).await,
            SemaphoreCommand::TryAcquire(args) => semaphore_try_acquire(client, args, is_json_output).await,
            SemaphoreCommand::Release(args) => semaphore_release(client, args, is_json_output).await,
            SemaphoreCommand::Status(args) => semaphore_status(client, args, is_json_output).await,
        }
    }
}

fn print_semaphore_output(output: SemaphoreOutput, is_json_output: bool) {
    print_output(&output, is_json_output);
    if !output.is_success {
        std::process::exit(1);
    }
}

async fn semaphore_acquire(client: &AspenClient, args: AcquireArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.name.is_empty(), "semaphore name must not be empty");
    debug_assert!(args.max_permits >= args.permits, "capacity must cover requested permits");
    let response = client
        .send(ClientRpcRequest::SemaphoreAcquire {
            name: args.name.clone(),
            holder_id: args.holder,
            permits: args.permits,
            capacity_permits: args.max_permits,
            ttl_ms: args.ttl_ms,
            timeout_ms: args.timeout_ms,
        })
        .await?;

    match response {
        ClientRpcResponse::SemaphoreAcquireResult(result) => {
            print_semaphore_output(
                SemaphoreOutput {
                    operation: "acquire".to_string(),
                    name: args.name,
                    is_success: result.is_success,
                    permits_acquired: result.permits_acquired,
                    available: result.available,
                    max_permits: result.capacity_permits,
                    retry_after_ms: result.retry_after_ms,
                    error: result.error,
                },
                is_json_output,
            );
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn semaphore_try_acquire(client: &AspenClient, args: TryAcquireArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.name.is_empty(), "semaphore name must not be empty");
    debug_assert!(args.max_permits >= args.permits, "capacity must cover requested permits");
    let response = client
        .send(ClientRpcRequest::SemaphoreTryAcquire {
            name: args.name.clone(),
            holder_id: args.holder,
            permits: args.permits,
            capacity_permits: args.max_permits,
            ttl_ms: args.ttl_ms,
        })
        .await?;

    match response {
        ClientRpcResponse::SemaphoreTryAcquireResult(result) => {
            print_semaphore_output(
                SemaphoreOutput {
                    operation: "try_acquire".to_string(),
                    name: args.name,
                    is_success: result.is_success,
                    permits_acquired: result.permits_acquired,
                    available: result.available,
                    max_permits: result.capacity_permits,
                    retry_after_ms: result.retry_after_ms,
                    error: result.error,
                },
                is_json_output,
            );
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn semaphore_release(client: &AspenClient, args: ReleaseArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.name.is_empty(), "semaphore name must not be empty");
    debug_assert!(!args.holder.is_empty(), "holder id must not be empty");
    let response = client
        .send(ClientRpcRequest::SemaphoreRelease {
            name: args.name.clone(),
            holder_id: args.holder,
            permits: args.permits,
        })
        .await?;

    match response {
        ClientRpcResponse::SemaphoreReleaseResult(result) => {
            print_semaphore_output(
                SemaphoreOutput {
                    operation: "release".to_string(),
                    name: args.name,
                    is_success: result.is_success,
                    permits_acquired: None,
                    available: result.available,
                    max_permits: result.capacity_permits,
                    retry_after_ms: None,
                    error: result.error,
                },
                is_json_output,
            );
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn semaphore_status(client: &AspenClient, args: StatusArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.name.is_empty(), "semaphore name must not be empty");
    debug_assert!(is_json_output || !is_json_output, "output mode is boolean by construction");
    let response = client
        .send(ClientRpcRequest::SemaphoreStatus {
            name: args.name.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::SemaphoreStatusResult(result) => {
            print_semaphore_output(
                SemaphoreOutput {
                    operation: "status".to_string(),
                    name: args.name,
                    is_success: result.is_success,
                    permits_acquired: None,
                    available: result.available,
                    max_permits: result.capacity_permits,
                    retry_after_ms: None,
                    error: result.error,
                },
                is_json_output,
            );
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}
