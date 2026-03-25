//! CLI commands for commit DAG operations.
//!
//! Uses existing KV read/scan RPCs to access commit metadata stored
//! at `_sys:commit:` and `_sys:commit-tip:` prefixes.

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_commit_dag::COMMIT_KV_PREFIX;
use aspen_commit_dag::COMMIT_TIP_PREFIX;
use aspen_commit_dag::types::Commit;
use aspen_commit_dag::types::MutationType;
use aspen_commit_dag::verified::commit_hash::verify_commit_integrity;
use clap::Subcommand;

use crate::client::AspenClient;

/// Commit DAG operations.
#[derive(Subcommand)]
pub enum CommitCommand {
    /// Show details of a commit by its hex ID.
    Show {
        /// Commit ID (64 hex characters).
        commit_id: String,
    },
    /// Show the commit log for a branch.
    Log {
        /// Branch identifier.
        branch_id: String,
        /// Maximum number of commits to show.
        #[arg(short = 'n', long, default_value = "50")]
        max_depth: u32,
    },
    /// Diff two commits.
    Diff {
        /// First commit ID (hex).
        commit_a: String,
        /// Second commit ID (hex).
        commit_b: String,
    },
}

impl CommitCommand {
    pub async fn run(&self, client: &AspenClient) -> Result<()> {
        match self {
            CommitCommand::Show { commit_id } => show(client, commit_id).await,
            CommitCommand::Log { branch_id, max_depth } => log(client, branch_id, *max_depth).await,
            CommitCommand::Diff { commit_a, commit_b } => diff_cmd(client, commit_a, commit_b).await,
        }
    }
}

async fn read_kv(client: &AspenClient, key: &str) -> Result<Option<String>> {
    let resp = client.send(ClientRpcRequest::ReadKey { key: key.to_string() }).await?;

    match resp {
        ClientRpcResponse::ReadResult(result) => {
            if result.was_found {
                match result.value {
                    Some(bytes) => {
                        let s = String::from_utf8(bytes).context("value is not valid UTF-8")?;
                        Ok(Some(s))
                    }
                    None => Ok(None),
                }
            } else {
                Ok(None)
            }
        }
        ClientRpcResponse::Error(e) => bail!("{}: {}", e.code, e.message),
        _ => bail!("unexpected response"),
    }
}

fn deserialize_commit(hex_value: &str) -> Result<Commit> {
    let bytes = hex::decode(hex_value).context("hex decode")?;
    let commit: Commit = postcard::from_bytes(&bytes).context("postcard deserialize")?;
    Ok(commit)
}

async fn show(client: &AspenClient, commit_id_hex: &str) -> Result<()> {
    let key = format!("{COMMIT_KV_PREFIX}{commit_id_hex}");
    let value = read_kv(client, &key).await?.ok_or_else(|| anyhow::anyhow!("commit not found: {commit_id_hex}"))?;

    let commit = deserialize_commit(&value)?;
    let verified = verify_commit_integrity(&commit);

    println!("Commit:    {}", hex::encode(commit.id));
    match &commit.parent {
        Some(p) => println!("Parent:    {}", hex::encode(p)),
        None => println!("Parent:    (none - genesis)"),
    }
    println!("Branch:    {}", commit.branch_id);
    println!("Revision:  {}", commit.raft_revision);
    println!("Timestamp: {} ms", commit.timestamp_ms);
    println!("Verified:  {}", if verified { "✓" } else { "✗ INTEGRITY FAILURE" });
    println!("Mutations: {} entries", commit.mutations.len());

    for (key, mutation) in &commit.mutations {
        match mutation {
            MutationType::Set(v) => {
                let disp = if v.len() > 80 {
                    format!("{}...", &v[..80])
                } else {
                    v.clone()
                };
                println!("  SET {key} = {disp}");
            }
            MutationType::Delete => println!("  DEL {key}"),
        }
    }

    Ok(())
}

async fn log(client: &AspenClient, branch_id: &str, max_depth: u32) -> Result<()> {
    let tip_key = format!("{COMMIT_TIP_PREFIX}{branch_id}");
    let tip_hex = match read_kv(client, &tip_key).await? {
        Some(h) => h,
        None => {
            println!("No commits on branch '{branch_id}'");
            return Ok(());
        }
    };

    println!("Commit log for branch '{branch_id}':\n");

    let mut current_hex = tip_hex;
    let mut count = 0u32;

    while count < max_depth {
        let key = format!("{COMMIT_KV_PREFIX}{current_hex}");
        let value = match read_kv(client, &key).await? {
            Some(v) => v,
            None => break,
        };

        let commit = deserialize_commit(&value)?;
        let verified = verify_commit_integrity(&commit);
        let status = if verified { "✓" } else { "✗" };

        println!(
            "{status} {} | rev={} | {} mutations | {}ms",
            &hex::encode(commit.id)[..16],
            commit.raft_revision,
            commit.mutations.len(),
            commit.timestamp_ms,
        );

        match commit.parent {
            Some(p) => current_hex = hex::encode(p),
            None => break,
        }
        count += 1;
    }

    Ok(())
}

async fn diff_cmd(client: &AspenClient, commit_a_hex: &str, commit_b_hex: &str) -> Result<()> {
    let key_a = format!("{COMMIT_KV_PREFIX}{commit_a_hex}");
    let key_b = format!("{COMMIT_KV_PREFIX}{commit_b_hex}");

    let val_a = read_kv(client, &key_a).await?.ok_or_else(|| anyhow::anyhow!("commit A not found"))?;
    let val_b = read_kv(client, &key_b).await?.ok_or_else(|| anyhow::anyhow!("commit B not found"))?;

    let commit_a = deserialize_commit(&val_a)?;
    let commit_b = deserialize_commit(&val_b)?;

    let entries = aspen_commit_dag::diff(&commit_a, &commit_b);

    if entries.is_empty() {
        println!("No differences between commits.");
        return Ok(());
    }

    println!(
        "Diff: {} → {}",
        &commit_a_hex[..16.min(commit_a_hex.len())],
        &commit_b_hex[..16.min(commit_b_hex.len())]
    );
    println!("{} changes:", entries.len());

    for entry in &entries {
        match entry {
            aspen_commit_dag::DiffEntry::Added { key, value } => {
                let v = match value {
                    MutationType::Set(s) => format!("= {s}"),
                    MutationType::Delete => "(delete)".into(),
                };
                println!("  + {key} {v}");
            }
            aspen_commit_dag::DiffEntry::Removed { key } => {
                println!("  - {key}");
            }
            aspen_commit_dag::DiffEntry::Changed { key, old, new } => {
                let o = match old {
                    MutationType::Set(s) => s.clone(),
                    MutationType::Delete => "(del)".into(),
                };
                let n = match new {
                    MutationType::Set(s) => s.clone(),
                    MutationType::Delete => "(del)".into(),
                };
                println!("  ~ {key}: {o} → {n}");
            }
        }
    }

    Ok(())
}
