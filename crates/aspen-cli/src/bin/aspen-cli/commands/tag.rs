//! Tag management commands.
//!
//! Commands for listing, creating, and deleting tags.

use anyhow::Result;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::RefListOutput;
use crate::output::RefOutput;
use crate::output::print_output;

/// Tag management operations.
#[derive(Subcommand)]
pub enum TagCommand {
    /// List tags in a repository.
    List(TagListArgs),

    /// Create a new tag.
    Create(TagCreateArgs),

    /// Delete a tag.
    Delete(TagDeleteArgs),
}

#[derive(Args)]
pub struct TagListArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,
}

#[derive(Args)]
pub struct TagCreateArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Tag name (without tags/ prefix).
    pub name: String,

    /// Commit to tag.
    #[arg(long)]
    pub commit: String,
}

#[derive(Args)]
pub struct TagDeleteArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Tag name (without tags/ prefix).
    pub name: String,
}

impl TagCommand {
    /// Execute the tag command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            TagCommand::List(args) => tag_list(client, args, json).await,
            TagCommand::Create(args) => tag_create(client, args, json).await,
            TagCommand::Delete(args) => tag_delete(client, args, json).await,
        }
    }
}

async fn tag_list(client: &AspenClient, args: TagListArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::ForgeListTags { repo_id: args.repo }).await?;

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

async fn tag_create(client: &AspenClient, args: TagCreateArgs, json: bool) -> Result<()> {
    let ref_name = format!("tags/{}", args.name);

    let response = client
        .send(ClientRpcRequest::ForgeCasRef {
            repo_id: args.repo,
            ref_name: ref_name.clone(),
            expected: None, // Must not exist
            new_hash: args.commit,
            signer: None, // TODO: Add --key flag for signing
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
                    println!("Tag {} created", args.name);
                }
                Ok(())
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            if result.success {
                if !json {
                    println!("Tag {} created", args.name);
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

async fn tag_delete(client: &AspenClient, args: TagDeleteArgs, json: bool) -> Result<()> {
    let ref_name = format!("tags/{}", args.name);

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
                    println!("Tag {} deleted", args.name);
                } else {
                    println!(r#"{{"deleted": true, "tag": "{}"}}"#, args.name);
                }
                Ok(())
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            if result.success {
                if !json {
                    println!("Tag {} deleted", args.name);
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
