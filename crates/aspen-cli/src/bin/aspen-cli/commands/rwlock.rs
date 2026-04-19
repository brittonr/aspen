//! Read-write lock commands.
//!
//! Commands for distributed reader-writer locks that allow multiple
//! concurrent readers or a single exclusive writer.

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;

/// Read-write lock operations.
#[derive(Subcommand)]
pub enum RwLockCommand {
    /// Acquire read lock (blocking with timeout).
    Read(ReadArgs),

    /// Try to acquire read lock (non-blocking).
    TryRead(TryReadArgs),

    /// Acquire write lock (blocking with timeout).
    Write(WriteArgs),

    /// Try to acquire write lock (non-blocking).
    TryWrite(TryWriteArgs),

    /// Release read lock.
    ReleaseRead(ReleaseReadArgs),

    /// Release write lock.
    ReleaseWrite(ReleaseWriteArgs),

    /// Downgrade write lock to read lock.
    Downgrade(DowngradeArgs),

    /// Query lock status.
    Status(StatusArgs),
}

#[derive(Args)]
pub struct ReadArgs {
    /// Lock name.
    pub name: String,

    /// Holder identifier.
    #[arg(long)]
    pub holder: String,

    /// TTL in milliseconds.
    #[arg(long = "ttl", default_value = "30000")]
    pub ttl_ms: u64,

    /// Timeout in milliseconds (0 = no timeout).
    #[arg(long = "timeout", default_value = "5000")]
    pub timeout_ms: u64,
}

#[derive(Args)]
pub struct TryReadArgs {
    /// Lock name.
    pub name: String,

    /// Holder identifier.
    #[arg(long)]
    pub holder: String,

    /// TTL in milliseconds.
    #[arg(long = "ttl", default_value = "30000")]
    pub ttl_ms: u64,
}

#[derive(Args)]
pub struct WriteArgs {
    /// Lock name.
    pub name: String,

    /// Holder identifier.
    #[arg(long)]
    pub holder: String,

    /// TTL in milliseconds.
    #[arg(long = "ttl", default_value = "30000")]
    pub ttl_ms: u64,

    /// Timeout in milliseconds (0 = no timeout).
    #[arg(long = "timeout", default_value = "5000")]
    pub timeout_ms: u64,
}

#[derive(Args)]
pub struct TryWriteArgs {
    /// Lock name.
    pub name: String,

    /// Holder identifier.
    #[arg(long)]
    pub holder: String,

    /// TTL in milliseconds.
    #[arg(long = "ttl", default_value = "30000")]
    pub ttl_ms: u64,
}

#[derive(Args)]
pub struct ReleaseReadArgs {
    /// Lock name.
    pub name: String,

    /// Holder identifier.
    #[arg(long)]
    pub holder: String,
}

#[derive(Args)]
pub struct ReleaseWriteArgs {
    /// Lock name.
    pub name: String,

    /// Holder identifier.
    #[arg(long)]
    pub holder: String,

    /// Fencing token for verification.
    #[arg(long = "fencing-token")]
    pub fencing_token: u64,
}

#[derive(Args)]
pub struct DowngradeArgs {
    /// Lock name.
    pub name: String,

    /// Holder identifier.
    #[arg(long)]
    pub holder: String,

    /// Fencing token for verification.
    #[arg(long = "fencing-token")]
    pub fencing_token: u64,

    /// New TTL in milliseconds.
    #[arg(long = "ttl", default_value = "30000")]
    pub ttl_ms: u64,
}

#[derive(Args)]
pub struct StatusArgs {
    /// Lock name.
    pub name: String,
}

/// RWLock operation output.
pub struct RwLockOutput {
    pub operation: String,
    pub name: String,
    pub is_success: bool,
    pub mode: Option<String>,
    pub fencing_token: Option<u64>,
    pub deadline_ms: Option<u64>,
    pub reader_count: Option<u32>,
    pub writer_holder: Option<String>,
    pub error: Option<String>,
}

impl Outputable for RwLockOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "operation": self.operation,
            "name": self.name,
            "is_success": self.is_success,
            "mode": self.mode,
            "fencing_token": self.fencing_token,
            "deadline_ms": self.deadline_ms,
            "reader_count": self.reader_count,
            "writer_holder": self.writer_holder,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.is_success {
            match self.operation.as_str() {
                "read" | "try_read" => {
                    format!("Read lock acquired (readers: {})", self.reader_count.unwrap_or(1))
                }
                "write" | "try_write" => {
                    format!("Write lock acquired. Fencing token: {}", self.fencing_token.unwrap_or(0))
                }
                "release_read" => "Read lock released".to_string(),
                "release_write" => "Write lock released".to_string(),
                "downgrade" => "Write lock downgraded to read lock".to_string(),
                "status" => {
                    let mode = self.mode.as_deref().unwrap_or("unknown");
                    let readers = self.reader_count.unwrap_or(0);
                    match mode {
                        "free" => "Lock is free".to_string(),
                        "read" => format!("Read mode ({} readers)", readers),
                        "write" => {
                            format!("Write mode (holder: {})", self.writer_holder.as_deref().unwrap_or("unknown"))
                        }
                        _ => format!("Mode: {}", mode),
                    }
                }
                _ => "OK".to_string(),
            }
        } else {
            match &self.error {
                Some(e) => format!("{} failed: {}", self.operation, e),
                None => format!("{} failed", self.operation),
            }
        }
    }
}

impl RwLockCommand {
    /// Execute the rwlock command.
    pub async fn run(self, client: &AspenClient, is_json: bool) -> Result<()> {
        match self {
            RwLockCommand::Read(args) => rwlock_read(client, args, is_json).await,
            RwLockCommand::TryRead(args) => rwlock_try_read(client, args, is_json).await,
            RwLockCommand::Write(args) => rwlock_write(client, args, is_json).await,
            RwLockCommand::TryWrite(args) => rwlock_try_write(client, args, is_json).await,
            RwLockCommand::ReleaseRead(args) => rwlock_release_read(client, args, is_json).await,
            RwLockCommand::ReleaseWrite(args) => rwlock_release_write(client, args, is_json).await,
            RwLockCommand::Downgrade(args) => rwlock_downgrade(client, args, is_json).await,
            RwLockCommand::Status(args) => rwlock_status(client, args, is_json).await,
        }
    }
}

async fn rwlock_read(client: &AspenClient, args: ReadArgs, is_json: bool) -> Result<()> {
    debug_assert!(!args.name.is_empty());
    debug_assert!(!args.holder.is_empty());
    let response = client
        .send(ClientRpcRequest::RWLockAcquireRead {
            name: args.name.clone(),
            holder_id: args.holder,
            ttl_ms: args.ttl_ms,
            timeout_ms: args.timeout_ms,
        })
        .await?;

    match response {
        ClientRpcResponse::RWLockAcquireReadResult(result) => {
            let output = RwLockOutput {
                operation: "read".to_string(),
                name: args.name,
                is_success: result.is_success,
                mode: result.mode,
                fencing_token: result.fencing_token,
                deadline_ms: result.deadline_ms,
                reader_count: result.reader_count,
                writer_holder: result.writer_holder,
                error: result.error,
            };
            debug_assert!(!output.operation.is_empty());
            debug_assert!(!output.name.is_empty());
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

async fn rwlock_try_read(client: &AspenClient, args: TryReadArgs, is_json: bool) -> Result<()> {
    debug_assert!(!args.name.is_empty());
    debug_assert!(!args.holder.is_empty());
    let response = client
        .send(ClientRpcRequest::RWLockTryAcquireRead {
            name: args.name.clone(),
            holder_id: args.holder,
            ttl_ms: args.ttl_ms,
        })
        .await?;

    match response {
        ClientRpcResponse::RWLockTryAcquireReadResult(result) => {
            let output = RwLockOutput {
                operation: "try_read".to_string(),
                name: args.name,
                is_success: result.is_success,
                mode: result.mode,
                fencing_token: result.fencing_token,
                deadline_ms: result.deadline_ms,
                reader_count: result.reader_count,
                writer_holder: result.writer_holder,
                error: result.error,
            };
            debug_assert!(!output.operation.is_empty());
            debug_assert!(!output.name.is_empty());
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

async fn rwlock_write(client: &AspenClient, args: WriteArgs, is_json: bool) -> Result<()> {
    debug_assert!(!args.name.is_empty());
    debug_assert!(!args.holder.is_empty());
    let response = client
        .send(ClientRpcRequest::RWLockAcquireWrite {
            name: args.name.clone(),
            holder_id: args.holder,
            ttl_ms: args.ttl_ms,
            timeout_ms: args.timeout_ms,
        })
        .await?;

    match response {
        ClientRpcResponse::RWLockAcquireWriteResult(result) => {
            let output = RwLockOutput {
                operation: "write".to_string(),
                name: args.name,
                is_success: result.is_success,
                mode: result.mode,
                fencing_token: result.fencing_token,
                deadline_ms: result.deadline_ms,
                reader_count: result.reader_count,
                writer_holder: result.writer_holder,
                error: result.error,
            };
            debug_assert!(!output.operation.is_empty());
            debug_assert!(!output.name.is_empty());
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

async fn rwlock_try_write(client: &AspenClient, args: TryWriteArgs, is_json: bool) -> Result<()> {
    debug_assert!(!args.name.is_empty());
    debug_assert!(!args.holder.is_empty());
    let response = client
        .send(ClientRpcRequest::RWLockTryAcquireWrite {
            name: args.name.clone(),
            holder_id: args.holder,
            ttl_ms: args.ttl_ms,
        })
        .await?;

    match response {
        ClientRpcResponse::RWLockTryAcquireWriteResult(result) => {
            let output = RwLockOutput {
                operation: "try_write".to_string(),
                name: args.name,
                is_success: result.is_success,
                mode: result.mode,
                fencing_token: result.fencing_token,
                deadline_ms: result.deadline_ms,
                reader_count: result.reader_count,
                writer_holder: result.writer_holder,
                error: result.error,
            };
            debug_assert!(!output.operation.is_empty());
            debug_assert!(!output.name.is_empty());
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

async fn rwlock_release_read(client: &AspenClient, args: ReleaseReadArgs, is_json: bool) -> Result<()> {
    debug_assert!(!args.name.is_empty());
    debug_assert!(!args.holder.is_empty());
    let response = client
        .send(ClientRpcRequest::RWLockReleaseRead {
            name: args.name.clone(),
            holder_id: args.holder,
        })
        .await?;

    match response {
        ClientRpcResponse::RWLockReleaseReadResult(result) => {
            let output = RwLockOutput {
                operation: "release_read".to_string(),
                name: args.name,
                is_success: result.is_success,
                mode: result.mode,
                fencing_token: None,
                deadline_ms: None,
                reader_count: result.reader_count,
                writer_holder: None,
                error: result.error,
            };
            debug_assert!(!output.operation.is_empty());
            debug_assert!(!output.name.is_empty());
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

async fn rwlock_release_write(client: &AspenClient, args: ReleaseWriteArgs, is_json: bool) -> Result<()> {
    debug_assert!(!args.name.is_empty());
    debug_assert!(!args.holder.is_empty());
    let response = client
        .send(ClientRpcRequest::RWLockReleaseWrite {
            name: args.name.clone(),
            holder_id: args.holder,
            fencing_token: args.fencing_token,
        })
        .await?;

    match response {
        ClientRpcResponse::RWLockReleaseWriteResult(result) => {
            let output = RwLockOutput {
                operation: "release_write".to_string(),
                name: args.name,
                is_success: result.is_success,
                mode: result.mode,
                fencing_token: None,
                deadline_ms: None,
                reader_count: result.reader_count,
                writer_holder: None,
                error: result.error,
            };
            debug_assert!(!output.operation.is_empty());
            debug_assert!(!output.name.is_empty());
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

async fn rwlock_downgrade(client: &AspenClient, args: DowngradeArgs, is_json: bool) -> Result<()> {
    debug_assert!(!args.name.is_empty());
    debug_assert!(!args.holder.is_empty());
    let response = client
        .send(ClientRpcRequest::RWLockDowngrade {
            name: args.name.clone(),
            holder_id: args.holder,
            fencing_token: args.fencing_token,
            ttl_ms: args.ttl_ms,
        })
        .await?;

    match response {
        ClientRpcResponse::RWLockDowngradeResult(result) => {
            let output = RwLockOutput {
                operation: "downgrade".to_string(),
                name: args.name,
                is_success: result.is_success,
                mode: result.mode,
                fencing_token: result.fencing_token,
                deadline_ms: result.deadline_ms,
                reader_count: result.reader_count,
                writer_holder: None,
                error: result.error,
            };
            debug_assert!(!output.operation.is_empty());
            debug_assert!(!output.name.is_empty());
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

async fn rwlock_status(client: &AspenClient, args: StatusArgs, is_json: bool) -> Result<()> {
    debug_assert!(!args.name.is_empty());
    debug_assert!(args.name.len() <= 1024);
    let response = client
        .send(ClientRpcRequest::RWLockStatus {
            name: args.name.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::RWLockStatusResult(result) => {
            let output = RwLockOutput {
                operation: "status".to_string(),
                name: args.name,
                is_success: result.is_success,
                mode: result.mode,
                fencing_token: result.fencing_token,
                deadline_ms: result.deadline_ms,
                reader_count: result.reader_count,
                writer_holder: result.writer_holder,
                error: result.error,
            };
            debug_assert!(!output.operation.is_empty());
            debug_assert!(!output.name.is_empty());
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
