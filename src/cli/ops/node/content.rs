use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt};
use molten::content_store_adapter::*;
use molten::error::{MoltenError, Result};
use molten::node_content;
use molten::node_state::{NodeStatePath, NodeStateRoot};
use serde_json::json;
use std::{
    io::{Read, Write},
    net::SocketAddr,
    path::{Path, PathBuf},
};

#[derive(Debug, clap::Args)]
pub(crate) struct Command {
    #[command(subcommand)]
    action: Action,
}
#[derive(Debug, clap::Subcommand)]
enum Action {
    /// Read the node's existing public transport identity. Never generates a key.
    Identity(State),
    /// Admit an exact archive and retain its canonical manifest and chunks.
    Prepare(Prepare),
    /// Fetch through an independently constrained handoff, then publish verified bytes.
    Fetch(Box<Fetch>),
    /// Read bounded content-service observations, not package provenance.
    Status(State),
}
#[derive(Debug, clap::Args)]
struct State {
    #[arg(long)]
    state_root: PathBuf,
}
#[derive(Debug, clap::Args)]
struct Prepare {
    #[command(flatten)]
    state: State,
    #[arg(long)]
    archive: PathBuf,
    #[arg(long)]
    expected: String,
}
#[derive(Debug, clap::Args)]
struct Fetch {
    #[command(flatten)]
    state: State,
    #[arg(long)]
    handoff: PathBuf,
    #[arg(long)]
    manifest: String,
    #[arg(long)]
    provider: String,
    #[arg(long)]
    address: SocketAddr,
    #[arg(long)]
    bind: SocketAddr,
    #[arg(long)]
    expected: String,
    #[arg(long)]
    out: PathBuf,
}

pub(crate) fn run(command: Command) -> Result<()> {
    let event = match command.action {
        Action::Identity(state) => {
            let root = NodeStateRoot::open_existing(&state.state_root)?;
            let id = node_content::identity(&root)?;
            json!({"event":"identity", "public_key":id.public_key})
        }
        Action::Prepare(input) => {
            let bytes = read_input(&input.archive, NODE_CONTENT_MAX_BYTES)?;
            admit_node_archive(&bytes, &input.expected).map_err(MoltenError::invalid_harness)?;
            let root = NodeStateRoot::open_existing(&input.state.state_root)?;
            let manifest = node_content::prepare(&root, &bytes, &input.expected)?;
            json!({"event":"prepared", "manifest_ref":manifest, "archive_blake3":input.expected, "bytes":bytes.len(), "pinned":true})
        }
        Action::Fetch(input) => {
            let handoff = read_input(&input.handoff, MAX_LIVE_HANDOFF_BYTES as u64)?;
            let root = NodeStateRoot::open_existing(&input.state.state_root)?;
            let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build()?;
            let (bytes, receipt) = runtime.block_on(node_content::fetch(node_content::FetchInput {
                root: &root,
                handoff: &handoff,
                manifest: &input.manifest,
                provider: &input.provider,
                address: input.address,
                bind: input.bind,
                expected: &input.expected,
            }))?;
            publish_new(&input.out, &bytes)?;
            json!({"event":"fetched", "manifest_ref":input.manifest, "archive_blake3":input.expected,
                "bytes":bytes.len(), "receipt_ref":receipt, "package_provenance":false})
        }
        Action::Status(state) => {
            let root = NodeStateRoot::open_existing(&state.state_root)?;
            let bytes = root.read(&NodeStatePath::parse(node_content::STATUS_FILE)?, 65_536)?;
            let status: serde_json::Value = serde_json::from_slice(&bytes).map_err(json_error)?;
            json!({"event":"status", "content":status})
        }
    };
    println!("MOLTEN_NODE_CONTENT {}", serde_json::to_string(&event).map_err(json_error)?);
    std::io::stdout().flush()?;
    Ok(())
}

/// The path is an explicit CLI input grant, not a reopened node descendant.
pub(crate) fn read_input(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let (dir, leaf) = parent_grant(path)?;
    let mut options = cap_std::fs::OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let file = dir.open_with(&leaf, &options)?;
    if !file.metadata()?.is_file() {
        return Err(MoltenError::invalid_harness("node content input is not a regular file"));
    }
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(MoltenError::invalid_harness("node content input exceeds bound"));
    }
    Ok(bytes)
}

fn parent_grant(path: &Path) -> Result<(cap_std::fs::Dir, std::ffi::OsString)> {
    let leaf = path.file_name().ok_or_else(|| MoltenError::invalid_harness("node content file name required"))?;
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    Ok((cap_std::fs::Dir::open_ambient_dir(parent, cap_std::ambient_authority())?, leaf.to_os_string()))
}

/// Publish a complete file with an atomic no-replace hard link, on one filesystem.
fn publish_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let (dir, leaf) = parent_grant(path)?;
    let temp = cap_tempfile::TempDir::new_in(&dir)?;
    let mut options = cap_std::fs::OpenOptions::new();
    options.write(true).create_new(true).follow(FollowSymlinks::No);
    let mut file = temp.open_with("payload", &options)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    temp.hard_link("payload", &dir, &leaf)?;
    temp.close()?;
    Ok(())
}

fn json_error(error: serde_json::Error) -> MoltenError {
    MoltenError::invalid_harness(format!("node content JSON: {error}"))
}
