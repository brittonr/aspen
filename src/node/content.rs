//! Capability adapter for the normal node's optional protected content service.
use crate::content_store_adapter::*;
use crate::error::{MoltenError, Result};
use crate::node_state::{NodeStatePath, NodeStateRoot};
use crate::{chunk_store as chunks, preserves_rail as rail};
use serde_json::json;
use std::net::SocketAddr;
use std::time::Duration;

pub const HANDOFF_FILE: &str = "control/service/content-handoff.bin";
pub const STATUS_FILE: &str = "control/service/content-status.json";
pub const MANIFEST_FILE: &str = "content-manifest-ref";

pub fn profile(policy_ref: String) -> Result<ContentAdapterProfile> {
    content_adapter_profile(
        "bounded-node-archive-v1",
        policy_ref.clone(),
        ContentAdapterClass::IrohBlobs,
        node_content_bounds(),
        vec![policy_ref],
    )
}

pub fn identity(root: &NodeStateRoot) -> Result<LiveIrohIdentitySummary> {
    let bytes = root.read(&NodeStatePath::parse("identity.preserves")?, 65_536)?;
    let value = rail::parse_text(
        std::str::from_utf8(&bytes).map_err(|_| MoltenError::invalid_harness("node identity encoding"))?,
    )?;
    let identity = crate::node_identity::parse_identity(&value)?;
    let namespace = root.identity()?;
    let summary = inspect_live_iroh_identity(&namespace, &identity.backend_ref)?;
    if summary.endpoint_id != identity.endpoint_id || summary.handle_ref != identity.secret_ref {
        return Err(MoltenError::invalid_harness("node content identity binding denied"));
    }
    Ok(summary)
}

pub fn prepare(root: &NodeStateRoot, bytes: &[u8], expected: &str) -> Result<String> {
    admit_node_archive(bytes, expected).map_err(MoltenError::invalid_harness)?;
    identity(root)?;
    let store = root.chunk_store()?;
    let stored = chunks::put_bytes_with_root(&store, "artifact", bytes, NODE_CONTENT_CHUNK_BYTES)?;
    chunks::pin_manifest_with_root(&store, &stored.manifest_ref)?;
    root.write(&NodeStatePath::parse(MANIFEST_FILE)?, stored.manifest_ref.as_bytes())?;
    Ok(stored.manifest_ref)
}

/// Created only after the daemon admits the active startup and service lock.
/// This object lives inside the normal tick loop, not in a fixture server.
pub(crate) struct Session {
    runtime: tokio::runtime::Runtime,
    publication: LiveIrohPublication,
    tick_ms: u64,
    status: serde_json::Value,
}

impl Session {
    pub(crate) fn start(root: &NodeStateRoot, plan: &NodeContentPlan, startup: &str, lock: &str) -> Result<Self> {
        remove_handoff(root)?;
        write_status(root, &json!({"state":"starting"}))?;
        let store = root.chunk_store()?;
        if !chunks::manifest_is_pinned_with_root(&store, plan.grant().manifest_ref())? {
            return Err(MoltenError::invalid_harness("node content canonical pin missing"));
        }
        let id = identity(root)?;
        let namespace = root.identity()?;
        let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build()?;
        let publication = runtime.block_on(publish_protected_live_iroh_chunks(
            &profile(plan.grant().policy_ref().to_string())?,
            &store,
            plan.grant().manifest_ref(),
            id.bind(&namespace),
            LiveIrohServeOptions {
                bind_addr: plan.bind_addr(),
                read_grant: plan.grant().clone(),
            },
        ))?;
        let wire = publication.export_handoff()?;
        let status = json!({"schema":"molten.node-content-status.v1", "state":"ready",
            "manifest_ref":plan.grant().manifest_ref(), "provider":id.public_key,
            "handoff_blake3":blake3::hash(&wire).to_hex().to_string(),
            "startup_ref":startup, "service_lock_ref":lock, "denied_connections":0, "ticks":0});
        let written =
            root.write(&NodeStatePath::parse(HANDOFF_FILE)?, &wire).and_then(|()| write_status(root, &status));
        if let Err(error) = written {
            runtime.block_on(publication.shutdown())?;
            remove_handoff(root)?;
            return Err(error);
        }
        Ok(Self {
            runtime,
            publication,
            tick_ms: plan.tick_ms(),
            status,
        })
    }

    pub(crate) fn tick(&mut self, root: &NodeStateRoot, tick: u64) -> Result<()> {
        self.status["denied_connections"] = self.publication.denied_connections().into();
        self.status["ticks"] = tick.into();
        write_status(root, &self.status)?;
        std::thread::sleep(Duration::from_millis(self.tick_ms));
        Ok(())
    }

    pub(crate) fn close(mut self, root: &NodeStateRoot) -> Result<()> {
        self.status["denied_connections"] = self.publication.denied_connections().into();
        self.runtime.block_on(self.publication.shutdown())?;
        remove_handoff(root)?;
        self.status["state"] = "closed".into();
        write_status(root, &self.status)
    }
}

fn write_status(root: &NodeStateRoot, status: &serde_json::Value) -> Result<()> {
    root.control_service()?.write_atomic_leaf(
        &NodeStatePath::parse("content-status.json")?,
        &serde_json::to_vec(status).map_err(json_error)?,
    )
}

fn remove_handoff(root: &NodeStateRoot) -> Result<()> {
    let path = NodeStatePath::parse(HANDOFF_FILE)?;
    if root.try_exists(&path)? {
        root.remove_regular_file(&path)?;
    }
    Ok(())
}
fn json_error(error: serde_json::Error) -> MoltenError {
    MoltenError::invalid_harness(format!("node content JSON: {error}"))
}

pub struct FetchInput<'a> {
    pub root: &'a NodeStateRoot,
    pub handoff: &'a [u8],
    pub manifest: &'a str,
    pub provider: &'a str,
    pub address: SocketAddr,
    pub bind: SocketAddr,
    pub expected: &'a str,
}

/// Returns verified bytes; the CLI's explicit output grant owns publication.
pub async fn fetch(input: FetchInput<'_>) -> Result<(Vec<u8>, String)> {
    if !valid_reader_key(input.expected) {
        return Err(MoltenError::invalid_harness("node content expected digest denied"));
    }
    let policy_ref = rail::content_ref_from_bytes(NODE_CONTENT_SCHEMA.as_bytes());
    let profile = profile(policy_ref.clone())?;
    let remote = admit_live_handoff(
        &profile,
        input.handoff,
        LiveHandoffExpectation {
            manifest_ref: input.manifest,
            provider: input
                .provider
                .parse()
                .map_err(|_| MoltenError::invalid_harness("node content provider denied"))?,
            address: input.address,
        },
    )?;
    let command = content_command(
        &profile,
        ContentCommandInput {
            operation_ref: rail::content_ref_from_bytes(b"node-content-get-v1"),
            operation: ContentOperation::Get,
            manifest: remote.manifest(),
            range: None,
            submitted_tick: 0,
            deadline_tick: 30,
            retry_count: 0,
            cancelled: false,
            policy_refs: vec![policy_ref],
        },
    )?;
    let id = identity(input.root)?;
    let namespace = input.root.identity()?;
    let result = execute_live_iroh_remote_get(
        &profile,
        &remote,
        &command,
        1,
        None,
        LiveIrohReadOptions {
            identity: Some(id.bind(&namespace)),
            bind_addr: Some(input.bind),
            timeout: Duration::from_secs(NODE_CONTENT_READ_SECONDS),
        },
    )
    .await?;
    if result.state.artifact.terminal != ContentTerminal::Verified {
        return Err(MoltenError::invalid_harness("node content transfer not verified; no archive published"));
    }
    let bytes = assemble_verified_content(remote.manifest(), &result.state.artifact, &result.verified_chunks)?;
    admit_node_archive(&bytes, input.expected).map_err(MoltenError::invalid_harness)?;
    Ok((bytes, result.state.artifact_ref))
}
