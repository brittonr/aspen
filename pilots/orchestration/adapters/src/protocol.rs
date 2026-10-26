// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::BTreeMap;
use std::net::SocketAddr;

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use molten_batch_application::*;
use molten_batch_core::*;
use serde::Deserialize;
use serde::Serialize;

pub const ALPN: &[u8] = b"molten-orchestrator/batch-rpc/v1";
pub const REQUEST_SCHEMA: &str = "batch-rpc.request.v1";
pub const REPLY_SCHEMA: &str = "batch-rpc.reply.v1";
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Controller,
    Worker,
    Client,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PeerAddress {
    pub id: Id,
    pub address: SocketAddr,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Enrollment {
    pub peer: PeerAddress,
    pub capacity: Resources,
    pub labels: BTreeMap<String, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeConfig {
    pub role: Role,
    pub bind: SocketAddr,
    pub cluster: Id,
    pub generation: u64,
    pub controller: Option<Id>,
    pub operators: Vec<Id>,
    pub workers: Vec<Enrollment>,
    pub capacity: Resources,
    pub rpc_timeout_ms: u64,
    pub tick_ms: u64,
    pub freshness_ticks: u64,
}
impl NodeConfig {
    pub fn validate(&self) -> Result<(), Error> {
        if self.generation == 0
            || self.operators.len() > 32
            || self.workers.len() > MAX_NODES
            || self.rpc_timeout_ms < 100
            || self.rpc_timeout_ms > 60_000
            || self.tick_ms < 10
            || self.tick_ms > 60_000
            || self.freshness_ticks == 0
        {
            return Err(Error::Bound);
        }
        if self.role == Role::Worker && self.controller.is_none() {
            return Err(Error::Invalid("worker controller key missing".into()));
        }
        let mut unique = std::collections::BTreeSet::new();
        for e in &self.workers {
            if !unique.insert(e.peer.id) || e.peer.address.port() == 0 || e.peer.address.ip().is_unspecified() {
                return Err(Error::Invalid("worker enrollment".into()));
            }
            validate_node(&Node {
                id: e.peer.id,
                capacity: e.capacity,
                labels: e.labels.clone(),
                draining: false,
                enabled: true,
                last_seen: None,
                boot: 0,
            })?;
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireContent {
    pub manifest: ContentManifest,
    pub base64: String,
}
impl WireContent {
    pub fn from_content(c: Content) -> Result<Self, Error> {
        crate::content_id(&c)?;
        Ok(Self {
            manifest: c.manifest,
            base64: STANDARD.encode(c.bytes),
        })
    }
    pub fn content(self) -> Result<Content, Error> {
        if self.base64.len() > MAX_CONTENT_BYTES.div_ceil(3) * 4 {
            return Err(Error::Bound);
        }
        let bytes = STANDARD.decode(&self.base64).map_err(crate::invalid)?;
        if STANDARD.encode(&bytes) != self.base64 {
            return Err(Error::Invalid("noncanonical base64".into()));
        }
        let c = Content {
            manifest: self.manifest,
            bytes,
        };
        crate::content_id(&c)?;
        Ok(c)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    Info,
    Put { id: Id, content: WireContent },
    Get { id: Id },
    Start { request: ExecutionRequest },
    Observe { request: ExecutionRequest },
    CancelAttempt { request: ExecutionRequest },
    Submit { key: String, spec: JobSpec },
    Status,
    CancelRun { run: Id },
    DrainNode { node: Id, draining: bool },
    WorkerDrain,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireInfo {
    pub node: Id,
    pub cluster: Id,
    pub generation: u64,
    pub policy_ref: Id,
    pub capacity: Resources,
    pub boot: u64,
    pub draining: bool,
}
impl From<WorkerInfo> for WireInfo {
    fn from(i: WorkerInfo) -> Self {
        Self {
            node: i.node,
            cluster: i.cluster,
            generation: i.generation,
            policy_ref: i.policy_ref,
            capacity: i.capacity,
            boot: i.boot,
            draining: i.draining,
        }
    }
}
impl From<WireInfo> for WorkerInfo {
    fn from(i: WireInfo) -> Self {
        Self {
            node: i.node,
            cluster: i.cluster,
            generation: i.generation,
            policy_ref: i.policy_ref,
            capacity: i.capacity,
            boot: i.boot,
            draining: i.draining,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WireObservation {
    Missing,
    Accepted { request_ref: Id },
    Running { request_ref: Id },
    Terminal { request_ref: Id, result: ExecutionResult },
}
impl From<WorkerObservation> for WireObservation {
    fn from(o: WorkerObservation) -> Self {
        match o {
            WorkerObservation::Missing => Self::Missing,
            WorkerObservation::Accepted { request_ref } => Self::Accepted { request_ref },
            WorkerObservation::Running { request_ref } => Self::Running { request_ref },
            WorkerObservation::Terminal { request_ref, result } => Self::Terminal { request_ref, result },
        }
    }
}
impl From<WireObservation> for WorkerObservation {
    fn from(o: WireObservation) -> Self {
        match o {
            WireObservation::Missing => Self::Missing,
            WireObservation::Accepted { request_ref } => Self::Accepted { request_ref },
            WireObservation::Running { request_ref } => Self::Running { request_ref },
            WireObservation::Terminal { request_ref, result } => Self::Terminal { request_ref, result },
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Reply {
    Ok,
    Id(Id),
    Info(WireInfo),
    Content(WireContent),
    Observation(WireObservation),
    State(State),
    Error(String),
}
pub fn error_reply(e: Error) -> Reply {
    // Raw paths, key material, policy data and backend exceptions never cross RPC.
    let code = match e {
        Error::Unavailable => "unavailable",
        Error::Denied => "denied",
        Error::Conflict => "conflict",
        Error::Unknown => "unknown-reconcile",
        Error::Invalid(_) => "invalid",
        Error::Bound => "bound-exceeded",
        Error::Unsupported => "unsupported",
    };
    Reply::Error(code.to_owned())
}
