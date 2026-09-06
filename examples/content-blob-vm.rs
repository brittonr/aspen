//! VM-only fixture driver. Not a node service, package verifier, or authority mint.
//! All networking goes through the production Molten content adapter.
use clap::{Parser, Subcommand};
use molten::content_store_adapter::*;
use molten::node_state::{NodeStateNamespace, NodeStateNamespaceKind};
use molten::{chunk_store as chunks, fabric_crypto_identity as crypto, preserves_rail as rail};
use serde::Deserialize;
use serde_json::json;
use std::{
    fs,
    io::{Read, Write},
    net::SocketAddr,
    path::{Path, PathBuf},
    time::Duration,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const POLICY: &[u8] = include_bytes!("../verification/content-blob-vm/policy.json");
#[cfg(test)]
#[path = "content-blob-vm/tests.rs"]
mod tests;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    schema: String,
    max_archive_bytes: u64,
    chunk_bytes: u64,
    max_connections: usize,
    service_seconds: u64,
    read_seconds: u64,
}

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Action,
}

#[derive(Subcommand)]
enum Action {
    Identity {
        #[arg(long)]
        root: PathBuf,
    },
    Prepare {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        archive: PathBuf,
        #[arg(long)]
        expected: String,
    },
    /// Adversarial fixture only: corrupt a cloned VM test store, not a GC API.
    Fault {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        manifest: String,
        #[arg(long, value_parser = ["missing", "corrupt"])]
        kind: String,
    },
    Serve {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        manifest: String,
        #[arg(long, required = true)]
        reader: Vec<String>,
        #[arg(long)]
        bind: SocketAddr,
        #[arg(long)]
        handoff: PathBuf,
    },
    Fetch {
        #[arg(long)]
        root: PathBuf,
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
        #[arg(long)]
        anonymous: bool,
    },
}

fn read_bounded(path: &Path, maximum: u64) -> Result<Vec<u8>> {
    if !fs::symlink_metadata(path)?.file_type().is_file() {
        return Err("fixture input must be a regular file".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(path)?.take(maximum + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > maximum {
        return Err("fixture input exceeds bound".into());
    }
    Ok(bytes)
}

fn create_output(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = fs::OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn policy_ref() -> String {
    rail::content_ref_from_bytes(POLICY)
}
fn reference(label: &str) -> String {
    rail::content_ref_from_bytes(label.as_bytes())
}

fn identity(root: &Path, generate: bool) -> Result<(NodeStateNamespace, LiveIrohIdentitySummary)> {
    let namespace = NodeStateNamespace::open(NodeStateNamespaceKind::Identity, &root.join("identity"))?;
    let profile = crypto::canonical_crypto_profile(&crypto::production_ed25519_profile(
        policy_ref(),
        reference("vm-fixture-os-entropy"),
    ))?;
    let backend = reference("vm-fixture-capability-identity");
    crypto::IrohEd25519FileAdapter::new(&namespace, profile, backend.clone())?.resolve_or_generate(
        crypto::KeyPurpose::TransportEndpoint,
        &policy_ref(),
        generate,
    )?;
    let summary = inspect_live_iroh_identity(&namespace, &backend)?;
    Ok((namespace, summary))
}

fn profile(policy: &Policy) -> Result<ContentAdapterProfile> {
    Ok(content_adapter_profile(
        "vm-readonly-v1",
        policy_ref(),
        ContentAdapterClass::IrohBlobs,
        ContentResourceBounds {
            max_total_bytes: policy.max_archive_bytes,
            max_chunk_count: 16,
            max_chunk_bytes: policy.chunk_bytes,
            max_range_bytes: policy.max_archive_bytes,
            max_concurrent_operations: policy.max_connections,
            max_queued_bytes: policy.max_archive_bytes,
            max_memory_bytes: policy.max_archive_bytes * 2,
            max_deadline_ticks: 60,
            max_retries: 1,
            max_events: 64,
            max_status_entries: 16,
        },
        vec![policy_ref()],
    )?)
}

fn report(value: serde_json::Value) -> Result<()> {
    println!("MOLTEN_BLOB_VM {}", serde_json::to_string(&value)?);
    std::io::stdout().flush()?;
    Ok(())
}

fn check_archive(bytes: &[u8], expected: &str) -> Result<()> {
    if expected.len() != 64 || blake3::hash(bytes).to_hex().as_str() != expected {
        return Err("fixture archive BLAKE3 mismatch".into());
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // An operator rail, not a hostile-host detection claim. Never launch this
    // fixture's server on the host, even for a smoke test.
    if !fs::read_to_string("/proc/cmdline")?.split_whitespace().any(|part| part == "molten_blob_vm=1") {
        return Err("vm-only fixture: required guest kernel marker absent".into());
    }
    let policy: Policy = serde_json::from_slice(POLICY)?;
    if policy.schema != "molten.content-blob-vm-policy.v1"
        || policy.max_archive_bytes > 1_048_576
        || policy.max_archive_bytes == 0
        || policy.service_seconds == 0
        || policy.read_seconds == 0
        || policy.service_seconds > 300
        || policy.read_seconds > 10
        || policy.chunk_bytes != 65_536
        || policy.max_connections == 0
        || policy.max_connections > 2
    {
        return Err("VM fixture policy denied".into());
    }
    match Cli::parse().command {
        Action::Identity { root } => {
            let (_, id) = identity(&root, true)?;
            report(json!({"event":"identity", "public_key":id.public_key}))?;
        }
        Action::Prepare {
            root,
            archive,
            expected,
        } => {
            let bytes = read_bounded(&archive, policy.max_archive_bytes)?;
            check_archive(&bytes, &expected)?;
            let stored = chunks::put_bytes(&root.join("chunks"), "artifact", &bytes, policy.chunk_bytes)?;
            chunks::pin_manifest(&root.join("chunks"), &stored.manifest_ref)?;
            let (_, id) = identity(&root, true)?;
            create_output(&root.join("manifest-ref"), stored.manifest_ref.as_bytes())?;
            report(json!({"event":"prepared", "manifest_ref":stored.manifest_ref,
                "archive_blake3":expected, "public_key":id.public_key, "pinned":true}))?;
        }
        Action::Fault { root, manifest, kind } => {
            let chunk_root = chunks::open_capability_chunk_root(&root.join("chunks"))?;
            let stored = chunks::read_manifest_with_root(&chunk_root, &manifest)?;
            let chunk = stored.chunks.first().ok_or("fault fixture has no chunk")?;
            let hex = rail::content_ref_hex(&chunk.chunk_ref)?;
            let path = molten::local_store::LocalStorePath::parse(&format!("chunks/blake3_{hex}.bin"))?;
            // Deliberate storage damage in a disposable cloned guest disk.
            // This is not canonical deletion, unpinning, or retention authority.
            if kind == "missing" {
                chunk_root.root().remove_file(&path)?;
            } else {
                let mut bytes = chunk_root.root().read(&path)?;
                let first = bytes.first_mut().ok_or("fault fixture chunk is empty")?;
                *first ^= 1;
                chunk_root.root().write(&path, &bytes)?;
            }
            report(json!({"event":"fault-injected", "kind":kind, "manifest_ref":manifest,
                "vm_only":true, "deletion_authority":false}))?;
        }
        Action::Serve {
            root,
            manifest,
            reader,
            bind,
            handoff,
        } => {
            let grant_value = rail::record(
                "vm-configured-read-policy-v1",
                vec![
                    rail::string(&manifest),
                    rail::sequence(reader.iter().map(rail::string).collect()),
                    rail::string(policy_ref()),
                ],
            );
            let grant =
                ContentReadGrant::from_operator_policy(rail::canonical_hash(&grant_value)?, manifest.clone(), reader)
                    .map_err(|error| format!("read grant denied: {error:?}"))?;
            if !chunks::manifest_is_pinned(&root.join("chunks"), &manifest)? {
                return Err("canonical pin missing".into());
            }
            let chunk_root = chunks::open_capability_chunk_root(&root.join("chunks"))?;
            let (namespace, id) = identity(&root, false)?;
            let publication = publish_protected_live_iroh_chunks(
                &profile(&policy)?,
                &chunk_root,
                &manifest,
                id.bind(&namespace),
                LiveIrohServeOptions {
                    bind_addr: bind,
                    read_grant: grant,
                },
            )
            .await?;
            let wire = publication.export_handoff()?;
            create_output(&handoff, &wire)?;
            report(json!({"event":"serving", "manifest_ref":manifest, "provider":id.public_key,
                "handoff_blake3":blake3::hash(&wire).to_hex().to_string(), "pinned":true}))?;
            for _ in 0..policy.service_seconds {
                tokio::time::sleep(Duration::from_secs(1)).await;
                let denied = publication.denied_connections();
                if denied != 0 {
                    report(json!({"event":"reader-denial", "count":denied}))?;
                }
            }
            publication.shutdown().await?;
        }
        Action::Fetch {
            root,
            handoff,
            manifest,
            provider,
            address,
            bind,
            expected,
            out,
            anonymous,
        } => {
            let profile = profile(&policy)?;
            let wire = read_bounded(&handoff, MAX_LIVE_HANDOFF_BYTES as u64)?;
            let remote = admit_live_handoff(
                &profile,
                &wire,
                LiveHandoffExpectation {
                    manifest_ref: &manifest,
                    provider: provider.parse()?,
                    address,
                },
            )?;
            let command = content_command(
                &profile,
                ContentCommandInput {
                    operation_ref: reference("vm-fixture-stream-get"),
                    operation: ContentOperation::Get,
                    manifest: remote.manifest(),
                    range: None,
                    submitted_tick: 0,
                    deadline_tick: 30,
                    retry_count: 0,
                    cancelled: false,
                    policy_refs: vec![policy_ref()],
                },
            )?;
            let (namespace, id) = identity(&root, false)?;
            let result = execute_live_iroh_remote_get(
                &profile,
                &remote,
                &command,
                1,
                None,
                LiveIrohReadOptions {
                    identity: if anonymous { None } else { Some(id.bind(&namespace)) },
                    bind_addr: Some(bind),
                    timeout: Duration::from_secs(policy.read_seconds),
                },
            )
            .await?;
            if result.state.artifact.terminal != ContentTerminal::Verified {
                report(json!({"event":"fetch-failed", "terminal":result.state.artifact.terminal.as_str(),
                    "failure":result.state.artifact.failure.map(|failure| failure.as_str())}))?;
                return Err("live transfer did not verify; no archive published".into());
            }
            let bytes = assemble_verified_content(remote.manifest(), &result.state.artifact, &result.verified_chunks)?;
            check_archive(&bytes, &expected)?;
            create_output(&out, &bytes)?;
            report(json!({"event":"fetched", "manifest_ref":manifest, "archive_blake3":expected,
                "bytes":bytes.len(), "receipt_ref":result.state.artifact_ref,
                "chunks":result.verified_chunks.len(), "vm_only":true, "deployable":false}))?;
        }
    }
    Ok(())
}
