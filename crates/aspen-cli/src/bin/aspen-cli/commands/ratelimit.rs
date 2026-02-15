//! Rate limiter commands.
//!
//! Commands for distributed rate limiting using the token bucket algorithm.
//! Supports configurable capacity and refill rates.

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;

/// Rate limiter operations.
#[derive(Subcommand)]
pub enum RateLimitCommand {
    /// Try to acquire tokens (non-blocking).
    ///
    /// Returns immediately with success or failure.
    TryAcquire(TryAcquireArgs),

    /// Acquire tokens (blocking with timeout).
    ///
    /// Waits until tokens are available or timeout is reached.
    Acquire(AcquireArgs),

    /// Check available tokens without consuming.
    Available(AvailableArgs),

    /// Reset a rate limiter to full capacity.
    Reset(ResetArgs),
}

#[derive(Args)]
pub struct TryAcquireArgs {
    /// Rate limiter key.
    pub key: String,

    /// Number of tokens to acquire.
    #[arg(long, default_value = "1")]
    pub tokens: u64,

    /// Maximum bucket capacity.
    #[arg(long)]
    pub capacity: u64,

    /// Token refill rate per second.
    #[arg(long)]
    pub rate: f64,
}

#[derive(Args)]
pub struct AcquireArgs {
    /// Rate limiter key.
    pub key: String,

    /// Number of tokens to acquire.
    #[arg(long, default_value = "1")]
    pub tokens: u64,

    /// Maximum bucket capacity.
    #[arg(long)]
    pub capacity: u64,

    /// Token refill rate per second.
    #[arg(long)]
    pub rate: f64,

    /// Timeout in milliseconds.
    #[arg(long = "timeout", default_value = "5000")]
    pub timeout_ms: u64,
}

#[derive(Args)]
pub struct AvailableArgs {
    /// Rate limiter key.
    pub key: String,

    /// Maximum bucket capacity.
    #[arg(long)]
    pub capacity: u64,

    /// Token refill rate per second.
    #[arg(long)]
    pub rate: f64,
}

#[derive(Args)]
pub struct ResetArgs {
    /// Rate limiter key.
    pub key: String,

    /// Maximum bucket capacity.
    #[arg(long)]
    pub capacity: u64,

    /// Token refill rate per second.
    #[arg(long)]
    pub rate: f64,
}

/// Rate limiter output.
pub struct RateLimitOutput {
    pub operation: String,
    pub key: String,
    pub success: bool,
    pub tokens_remaining: Option<u64>,
    pub retry_after_ms: Option<u64>,
    pub error: Option<String>,
}

impl Outputable for RateLimitOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "operation": self.operation,
            "key": self.key,
            "success": self.success,
            "tokens_remaining": self.tokens_remaining,
            "retry_after_ms": self.retry_after_ms,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.success {
            match self.tokens_remaining {
                Some(remaining) => format!("OK ({} tokens remaining)", remaining),
                None => "OK".to_string(),
            }
        } else {
            match self.retry_after_ms {
                Some(ms) => format!("Rate limited. Retry after {} ms", ms),
                None => match &self.error {
                    Some(e) => format!("{} failed: {}", self.operation, e),
                    None => format!("{} failed", self.operation),
                },
            }
        }
    }
}

impl RateLimitCommand {
    /// Execute the rate limit command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            RateLimitCommand::TryAcquire(args) => ratelimit_try_acquire(client, args, json).await,
            RateLimitCommand::Acquire(args) => ratelimit_acquire(client, args, json).await,
            RateLimitCommand::Available(args) => ratelimit_available(client, args, json).await,
            RateLimitCommand::Reset(args) => ratelimit_reset(client, args, json).await,
        }
    }
}

async fn ratelimit_try_acquire(client: &AspenClient, args: TryAcquireArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::RateLimiterTryAcquire {
            key: args.key.clone(),
            tokens: args.tokens,
            capacity: args.capacity,
            refill_rate: args.rate,
        })
        .await?;

    match response {
        ClientRpcResponse::RateLimiterResult(result) => {
            let output = RateLimitOutput {
                operation: "try_acquire".to_string(),
                key: args.key,
                success: result.success,
                tokens_remaining: result.tokens_remaining,
                retry_after_ms: result.retry_after_ms,
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

async fn ratelimit_acquire(client: &AspenClient, args: AcquireArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::RateLimiterAcquire {
            key: args.key.clone(),
            tokens: args.tokens,
            capacity: args.capacity,
            refill_rate: args.rate,
            timeout_ms: args.timeout_ms,
        })
        .await?;

    match response {
        ClientRpcResponse::RateLimiterResult(result) => {
            let output = RateLimitOutput {
                operation: "acquire".to_string(),
                key: args.key,
                success: result.success,
                tokens_remaining: result.tokens_remaining,
                retry_after_ms: result.retry_after_ms,
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

async fn ratelimit_available(client: &AspenClient, args: AvailableArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::RateLimiterAvailable {
            key: args.key.clone(),
            capacity: args.capacity,
            refill_rate: args.rate,
        })
        .await?;

    match response {
        ClientRpcResponse::RateLimiterResult(result) => {
            let output = RateLimitOutput {
                operation: "available".to_string(),
                key: args.key,
                success: result.success,
                tokens_remaining: result.tokens_remaining,
                retry_after_ms: None,
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

async fn ratelimit_reset(client: &AspenClient, args: ResetArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::RateLimiterReset {
            key: args.key.clone(),
            capacity: args.capacity,
            refill_rate: args.rate,
        })
        .await?;

    match response {
        ClientRpcResponse::RateLimiterResult(result) => {
            let output = RateLimitOutput {
                operation: "reset".to_string(),
                key: args.key,
                success: result.success,
                tokens_remaining: result.tokens_remaining,
                retry_after_ms: None,
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
