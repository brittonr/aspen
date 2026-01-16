//! Tag management commands.
//!
//! Commands for listing, creating, and deleting tags.

use std::path::PathBuf;

use anyhow::Result;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use aspen_forge::refs::SignedRefUpdate;
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

    /// Path to secret key file for signing (required for canonical refs).
    ///
    /// The key file should contain a 32-byte Ed25519 secret key in hex format.
    /// Signing is required for tags in repositories with delegate enforcement.
    #[arg(short, long)]
    pub key: Option<PathBuf>,
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

    // Parse repo_id for signing
    let repo_id_bytes = hex::decode(&args.repo).map_err(|e| anyhow::anyhow!("invalid repo ID hex: {}", e))?;
    if repo_id_bytes.len() != 32 {
        anyhow::bail!("repo ID must be 32 bytes (64 hex characters)");
    }
    let mut repo_id_arr = [0u8; 32];
    repo_id_arr.copy_from_slice(&repo_id_bytes);
    let repo_id = aspen_forge::identity::RepoId::from(repo_id_arr);

    // Parse new_hash for signing
    let new_hash_bytes = hex::decode(&args.commit).map_err(|e| anyhow::anyhow!("invalid commit hash hex: {}", e))?;
    if new_hash_bytes.len() != 32 {
        anyhow::bail!("commit hash must be 32 bytes (64 hex characters)");
    }
    let mut new_hash_arr = [0u8; 32];
    new_hash_arr.copy_from_slice(&new_hash_bytes);
    let new_hash = blake3::Hash::from_bytes(new_hash_arr);

    // Load and sign if key provided
    let (signer, signature, timestamp_ms) = if let Some(key_path) = &args.key {
        let key_data =
            std::fs::read_to_string(key_path).map_err(|e| anyhow::anyhow!("failed to read key file: {}", e))?;
        let key_bytes =
            hex::decode(key_data.trim()).map_err(|e| anyhow::anyhow!("failed to decode key (expected hex): {}", e))?;
        if key_bytes.len() != 32 {
            anyhow::bail!("secret key must be 32 bytes");
        }
        let mut key_arr = [0u8; 32];
        key_arr.copy_from_slice(&key_bytes);
        let secret_key = iroh::SecretKey::from_bytes(&key_arr);

        // Create signed update (old_hash is None for tag creation)
        let update = SignedRefUpdate::sign(repo_id, &ref_name, new_hash, None, &secret_key);

        (
            Some(update.signer.to_string()),
            Some(hex::encode(update.signature.to_bytes())),
            Some(update.timestamp_ms),
        )
    } else {
        (None, None, None)
    };

    let response = client
        .send(ClientRpcRequest::ForgeCasRef {
            repo_id: args.repo,
            ref_name: ref_name.clone(),
            expected: None, // Must not exist
            new_hash: args.commit,
            signer,
            signature,
            timestamp_ms,
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
