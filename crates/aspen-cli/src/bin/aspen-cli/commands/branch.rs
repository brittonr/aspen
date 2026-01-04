//! Branch management commands.
//!
//! Commands for listing, creating, and deleting branches.

use anyhow::Result;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::RefListOutput;
use crate::output::RefOutput;
use crate::output::print_output;

/// Branch management operations.
#[derive(Subcommand)]
pub enum BranchCommand {
    /// List branches in a repository.
    List(BranchListArgs),

    /// Create a new branch.
    Create(BranchCreateArgs),

    /// Delete a branch.
    Delete(BranchDeleteArgs),
}

#[derive(Args)]
pub struct BranchListArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,
}

#[derive(Args)]
pub struct BranchCreateArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Branch name (without heads/ prefix).
    pub name: String,

    /// Starting point (commit hash).
    #[arg(long)]
    pub from: String,
}

#[derive(Args)]
pub struct BranchDeleteArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Branch name (without heads/ prefix).
    pub name: String,

    /// Force delete even if not fully merged.
    #[arg(short, long)]
    pub force: bool,
}

impl BranchCommand {
    /// Execute the branch command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            BranchCommand::List(args) => branch_list(client, args, json).await,
            BranchCommand::Create(args) => branch_create(client, args, json).await,
            BranchCommand::Delete(args) => branch_delete(client, args, json).await,
        }
    }
}

async fn branch_list(client: &AspenClient, args: BranchListArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::ForgeListBranches { repo_id: args.repo }).await?;

    match response {
        ClientRpcResponse::ForgeRefListResult(result) => {
            if result.success {
                let refs: Vec<RefOutput> = result
                    .refs
                    .into_iter()
                    .map(|r| RefOutput {
                        name: r.name,
                        hash: r.hash,
                    })
                    .collect();
                let output = RefListOutput {
                    refs,
                    count: result.count,
                };
                print_output(&output, json);
                Ok(())
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            anyhow::bail!("{}", result.error.unwrap_or_else(|| "operation failed".to_string()))
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn branch_create(client: &AspenClient, args: BranchCreateArgs, json: bool) -> Result<()> {
    let ref_name = format!("heads/{}", args.name);

    let response = client
        .send(ClientRpcRequest::ForgeCasRef {
            repo_id: args.repo,
            ref_name: ref_name.clone(),
            expected: None, // Must not exist
            new_hash: args.from,
            signer: None, // Non-canonical refs don't require signing
            signature: None,
            timestamp_ms: None,
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeRefResult(result) => {
            if result.success {
                if let Some(ref_info) = result.ref_info {
                    let output = RefOutput {
                        name: ref_info.name,
                        hash: ref_info.hash,
                    };
                    print_output(&output, json);
                } else if !json {
                    println!("Branch {} created", args.name);
                }
                Ok(())
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            if result.success {
                if !json {
                    println!("Branch {} created", args.name);
                }
                Ok(())
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "operation failed".to_string()))
            }
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn branch_delete(client: &AspenClient, args: BranchDeleteArgs, json: bool) -> Result<()> {
    let ref_name = format!("heads/{}", args.name);

    let response = client
        .send(ClientRpcRequest::ForgeDeleteRef {
            repo_id: args.repo,
            ref_name: ref_name.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeRefResult(result) => {
            if result.success {
                if !json {
                    println!("Branch {} deleted", args.name);
                } else {
                    println!(r#"{{"deleted": true, "branch": "{}"}}"#, args.name);
                }
                Ok(())
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            if result.success {
                if !json {
                    println!("Branch {} deleted", args.name);
                }
                Ok(())
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "operation failed".to_string()))
            }
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}
