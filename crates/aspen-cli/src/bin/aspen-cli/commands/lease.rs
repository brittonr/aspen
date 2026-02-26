//! Lease commands.
//!
//! Commands for time-based resource management with leases.
//! Keys attached to leases are automatically deleted when the lease expires.

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;

/// Lease operations.
#[derive(Subcommand)]
pub enum LeaseCommand {
    /// Grant a new lease with specified TTL.
    Grant(GrantArgs),

    /// Revoke a lease and delete all attached keys.
    Revoke(RevokeArgs),

    /// Refresh a lease's TTL (keepalive).
    Keepalive(KeepaliveArgs),

    /// Get lease information including TTL and attached keys.
    Ttl(TtlArgs),

    /// List all active leases.
    List,
}

#[derive(Args)]
pub struct GrantArgs {
    /// Time-to-live in seconds.
    pub ttl_secs: u32,

    /// Optional client-provided lease ID (0 = auto-generate).
    #[arg(long)]
    pub id: Option<u64>,
}

#[derive(Args)]
pub struct RevokeArgs {
    /// Lease ID to revoke.
    pub lease_id: u64,
}

#[derive(Args)]
pub struct KeepaliveArgs {
    /// Lease ID to refresh.
    pub lease_id: u64,
}

#[derive(Args)]
pub struct TtlArgs {
    /// Lease ID to query.
    pub lease_id: u64,

    /// Include list of keys attached to the lease.
    #[arg(long)]
    pub keys: bool,
}

/// Lease grant output.
pub struct LeaseGrantOutput {
    pub is_success: bool,
    pub lease_id: Option<u64>,
    pub ttl_seconds: Option<u32>,
    pub error: Option<String>,
}

impl Outputable for LeaseGrantOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "is_success": self.is_success,
            "lease_id": self.lease_id,
            "ttl_seconds": self.ttl_seconds,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.is_success {
            format!(
                "Lease {} granted with TTL {}s",
                self.lease_id.map(|id| id.to_string()).unwrap_or_else(|| "N/A".to_string()),
                self.ttl_seconds.map(|t| t.to_string()).unwrap_or_else(|| "N/A".to_string())
            )
        } else {
            format!("Grant failed: {}", self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

/// Lease revoke output.
pub struct LeaseRevokeOutput {
    pub is_success: bool,
    pub keys_deleted: Option<u32>,
    pub error: Option<String>,
}

impl Outputable for LeaseRevokeOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "is_success": self.is_success,
            "keys_deleted": self.keys_deleted,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.is_success {
            format!("Lease revoked. {} key(s) deleted.", self.keys_deleted.unwrap_or(0))
        } else {
            format!("Revoke failed: {}", self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

/// Lease keepalive output.
pub struct LeaseKeepaliveOutput {
    pub is_success: bool,
    pub lease_id: Option<u64>,
    pub ttl_seconds: Option<u32>,
    pub error: Option<String>,
}

impl Outputable for LeaseKeepaliveOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "is_success": self.is_success,
            "lease_id": self.lease_id,
            "ttl_seconds": self.ttl_seconds,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.is_success {
            format!("Lease {} renewed. TTL: {}s", self.lease_id.unwrap_or(0), self.ttl_seconds.unwrap_or(0))
        } else {
            format!("Keepalive failed: {}", self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

/// Lease TTL output.
pub struct LeaseTtlOutput {
    pub is_success: bool,
    pub lease_id: Option<u64>,
    pub granted_ttl_seconds: Option<u32>,
    pub remaining_ttl_seconds: Option<u32>,
    pub keys: Option<Vec<String>>,
    pub error: Option<String>,
}

impl Outputable for LeaseTtlOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "is_success": self.is_success,
            "lease_id": self.lease_id,
            "granted_ttl_seconds": self.granted_ttl_seconds,
            "remaining_ttl_seconds": self.remaining_ttl_seconds,
            "keys": self.keys,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.is_success {
            let mut output = format!(
                "Lease {}\n  Granted TTL: {}s\n  Remaining: {}s",
                self.lease_id.unwrap_or(0),
                self.granted_ttl_seconds.unwrap_or(0),
                self.remaining_ttl_seconds.unwrap_or(0)
            );
            if let Some(ref keys) = self.keys {
                output.push_str(&format!("\n  Keys ({}):", keys.len()));
                for key in keys {
                    output.push_str(&format!("\n    - {}", key));
                }
            }
            output
        } else {
            format!("TTL query failed: {}", self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

/// Lease list output.
pub struct LeaseListOutput {
    pub is_success: bool,
    pub leases: Vec<LeaseInfoDisplay>,
    pub error: Option<String>,
}

pub struct LeaseInfoDisplay {
    pub lease_id: u64,
    pub granted_ttl_seconds: u32,
    pub remaining_ttl_seconds: u32,
    pub attached_keys: u32,
}

impl Outputable for LeaseListOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "is_success": self.is_success,
            "leases": self.leases.iter().map(|l| {
                serde_json::json!({
                    "lease_id": l.lease_id,
                    "granted_ttl_seconds": l.granted_ttl_seconds,
                    "remaining_ttl_seconds": l.remaining_ttl_seconds,
                    "attached_keys": l.attached_keys
                })
            }).collect::<Vec<_>>(),
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if !self.is_success {
            return format!("List failed: {}", self.error.as_deref().unwrap_or("unknown error"));
        }

        if self.leases.is_empty() {
            return "No active leases".to_string();
        }

        let mut output = format!("Active Leases ({})\n", self.leases.len());
        output.push_str("Lease ID         | Granted TTL | Remaining | Keys\n");
        output.push_str("-----------------+-------------+-----------+------\n");

        for lease in &self.leases {
            output.push_str(&format!(
                "{:16} | {:>10}s | {:>8}s | {:>4}\n",
                lease.lease_id, lease.granted_ttl_seconds, lease.remaining_ttl_seconds, lease.attached_keys
            ));
        }

        output
    }
}

impl LeaseCommand {
    /// Execute the lease command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            LeaseCommand::Grant(args) => lease_grant(client, args, json).await,
            LeaseCommand::Revoke(args) => lease_revoke(client, args, json).await,
            LeaseCommand::Keepalive(args) => lease_keepalive(client, args, json).await,
            LeaseCommand::Ttl(args) => lease_ttl(client, args, json).await,
            LeaseCommand::List => lease_list(client, json).await,
        }
    }
}

async fn lease_grant(client: &AspenClient, args: GrantArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::LeaseGrant {
            ttl_seconds: args.ttl_secs,
            lease_id: args.id,
        })
        .await?;

    match response {
        ClientRpcResponse::LeaseGrantResult(result) => {
            let output = LeaseGrantOutput {
                is_success: result.is_success,
                lease_id: result.lease_id,
                ttl_seconds: result.ttl_seconds,
                error: result.error,
            };
            print_output(&output, json);
            if !result.is_success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn lease_revoke(client: &AspenClient, args: RevokeArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::LeaseRevoke {
            lease_id: args.lease_id,
        })
        .await?;

    match response {
        ClientRpcResponse::LeaseRevokeResult(result) => {
            let output = LeaseRevokeOutput {
                is_success: result.is_success,
                keys_deleted: result.keys_deleted,
                error: result.error,
            };
            print_output(&output, json);
            if !result.is_success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn lease_keepalive(client: &AspenClient, args: KeepaliveArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::LeaseKeepalive {
            lease_id: args.lease_id,
        })
        .await?;

    match response {
        ClientRpcResponse::LeaseKeepaliveResult(result) => {
            let output = LeaseKeepaliveOutput {
                is_success: result.is_success,
                lease_id: result.lease_id,
                ttl_seconds: result.ttl_seconds,
                error: result.error,
            };
            print_output(&output, json);
            if !result.is_success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn lease_ttl(client: &AspenClient, args: TtlArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::LeaseTimeToLive {
            lease_id: args.lease_id,
            should_include_keys: args.keys,
        })
        .await?;

    match response {
        ClientRpcResponse::LeaseTimeToLiveResult(result) => {
            let output = LeaseTtlOutput {
                is_success: result.is_success,
                lease_id: result.lease_id,
                granted_ttl_seconds: result.granted_ttl_seconds,
                remaining_ttl_seconds: result.remaining_ttl_seconds,
                keys: result.keys,
                error: result.error,
            };
            print_output(&output, json);
            if !result.is_success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn lease_list(client: &AspenClient, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::LeaseList).await?;

    match response {
        ClientRpcResponse::LeaseListResult(result) => {
            let leases = result
                .leases
                .unwrap_or_default()
                .into_iter()
                .map(|l| LeaseInfoDisplay {
                    lease_id: l.lease_id,
                    granted_ttl_seconds: l.granted_ttl_seconds,
                    remaining_ttl_seconds: l.remaining_ttl_seconds,
                    attached_keys: l.attached_keys,
                })
                .collect();

            let output = LeaseListOutput {
                is_success: result.is_success,
                leases,
                error: result.error,
            };
            print_output(&output, json);
            if !result.is_success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}
