//! Raft consensus implementation with actor-based control plane.
//!
//! This module provides a production-ready Raft consensus engine built on openraft,
//! integrated with the ractor actor framework for distributed coordination. The Raft
//! actor serves as both a cluster controller and key-value store, proxying operations
//! through the underlying openraft instance while maintaining actor lifecycle semantics.
//!
//! # Architecture
//!
//! - **RaftActor**: Main actor implementing ClusterController and KeyValueStore traits
//! - **Storage**: Hybrid storage backend (redb for log, SQLite for state machine)
//! - **Network**: IRPC-based network transport over Iroh P2P connections
//! - **Supervision**: Automatic actor restart with exponential backoff and meltdown detection
//!
//! # Key Components
//!
//! - `RaftActor`: Core actor driving the Raft state machine via ractor messages
//! - `RaftControlClient`: Client-side proxy for cluster operations
//! - `SqliteStateMachine`: ACID state machine storage with snapshot support
//! - `IrpcRaftNetwork`: Network layer implementing RaftNetworkV2 over IRPC/Iroh
//! - `NodeFailureDetector`: Distinguishes actor crashes from node-level failures
//!
//! # Tiger Style Compliance
//!
//! - Bounded resources: MAX_BATCH_SIZE (1000), MAX_SNAPSHOT_SIZE (1GB)
//! - Explicit error handling: snafu-based error types with context
//! - Fixed limits: Connection pools, restart counters, unreachable node tracking
//!
//! # Usage
//!
//! ```rust,ignore
//! use aspen::raft::{RaftActor, RaftConfig};
//!
//! // Bootstrap Raft actor with storage and network
//! let (actor_ref, bootstrap_handle) = RaftActor::spawn(config).await?;
//!
//! // Initialize cluster
//! bootstrap_handle.cluster.init(node_id).await?;
//!
//! // Write to state machine
//! bootstrap_handle.kv.write("key", b"value").await?;
//! ```

pub mod bounded_proxy;
pub mod constants;
pub mod learner_promotion;
pub mod madsim_network;
pub mod network;
pub mod node_failure_detection;
pub mod rpc;
pub mod server;
pub mod server_actor;
pub mod storage;
pub mod storage_sqlite;
pub mod storage_validation;
pub mod supervision;
pub mod types;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use async_trait::async_trait;
use iroh::EndpointAddr;
use openraft::metrics::RaftMetrics;
use openraft::{LogId, Raft, ReadPolicy};
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort, call_t};
use tracing::{info, instrument, warn};

use crate::api::{
    AddLearnerRequest, ChangeMembershipRequest, ClusterController, ClusterNode, ClusterState,
    ControlPlaneError, DeleteRequest, DeleteResult, InitRequest, KeyValueStore, KeyValueStoreError,
    ReadRequest, ReadResult, WriteCommand, WriteRequest, WriteResult,
};
use crate::raft::constants::{MAX_KEY_SIZE, MAX_SETMULTI_KEYS, MAX_VALUE_SIZE};
use crate::raft::storage::StateMachineStore;
use crate::raft::storage_sqlite::SqliteStateMachine;
use crate::raft::types::{AppRequest, AppTypeConfig, AspenNode};

/// State machine variant that can hold either in-memory, redb-backed, or sqlite-backed storage.
///
/// This enum allows the RaftActor to read from the same state machine that
/// receives writes through the Raft core, fixing the NotFound bug where reads
/// queried a placeholder state machine.
#[derive(Clone, Debug)]
pub enum StateMachineVariant {
    InMemory(Arc<StateMachineStore>),
    Redb(Arc<SqliteStateMachine>),
    Sqlite(Arc<SqliteStateMachine>),
}

impl StateMachineVariant {
    /// Read a value from the state machine.
    pub async fn get(&self, key: &str) -> Option<String> {
        match self {
            Self::InMemory(sm) => sm.get(key).await,
            Self::Redb(sm) => sm.get(key).await.ok().flatten(),
            Self::Sqlite(sm) => sm.get(key).await.ok().flatten(),
        }
    }
}

/// Configuration used to initialize a Raft actor instance.
#[derive(Clone, Debug)]
pub struct RaftActorConfig {
    pub node_id: u64,
    pub raft: Raft<AppTypeConfig>,
    pub state_machine: StateMachineVariant,
    /// Log store reference for cross-storage validation (only set for SQLite backend)
    pub log_store: Option<crate::raft::storage::RedbLogStore>,
}

/// Empty actor shell that will eventually drive the Raft state machine.
pub struct RaftActor;

#[derive(Debug)]
pub struct RaftActorState {
    node_id: u64,
    raft: Raft<AppTypeConfig>,
    state_machine: StateMachineVariant,
    cluster_state: ClusterState,
    initialized: bool,
}

#[derive(Debug)]
pub enum RaftActorMessage {
    /// Lightweight RPC used by tests/monitors to confirm the actor is alive.
    GetNodeId(RpcReplyPort<u64>),
    /// Health check ping - respond immediately to confirm actor is responsive.
    Ping(RpcReplyPort<()>),
    /// Return the current cluster snapshot.
    CurrentState(RpcReplyPort<Result<ClusterState, ControlPlaneError>>),
    /// Initialize the cluster membership set.
    InitCluster(
        InitRequest,
        RpcReplyPort<Result<ClusterState, ControlPlaneError>>,
    ),
    /// Add a learner node.
    AddLearner(
        AddLearnerRequest,
        RpcReplyPort<Result<ClusterState, ControlPlaneError>>,
    ),
    /// Change the active membership set.
    ChangeMembership(
        ChangeMembershipRequest,
        RpcReplyPort<Result<ClusterState, ControlPlaneError>>,
    ),
    /// Apply a write command.
    Write(
        WriteRequest,
        RpcReplyPort<Result<WriteResult, KeyValueStoreError>>,
    ),
    /// Read the latest value for a key.
    Read(
        ReadRequest,
        RpcReplyPort<Result<ReadResult, KeyValueStoreError>>,
    ),
    /// Delete a key from the store.
    Delete(
        DeleteRequest,
        RpcReplyPort<Result<DeleteResult, KeyValueStoreError>>,
    ),
    /// Get current Raft metrics for observability.
    GetMetrics(RpcReplyPort<Result<RaftMetrics<AppTypeConfig>, ControlPlaneError>>),
    /// Trigger a snapshot immediately.
    TriggerSnapshot(RpcReplyPort<Result<Option<LogId<AppTypeConfig>>, ControlPlaneError>>),
    /// Graceful shutdown signal.
    Shutdown,
}

impl ractor::Message for RaftActorMessage {}

#[async_trait]
impl Actor for RaftActor {
    type Msg = RaftActorMessage;
    type State = RaftActorState;
    type Arguments = RaftActorConfig;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        config: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        info!(node_id = config.node_id, "raft actor starting");
        Ok(RaftActorState {
            node_id: config.node_id,
            raft: config.raft,
            state_machine: config.state_machine,
            cluster_state: ClusterState::default(),
            initialized: false,
        })
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            RaftActorMessage::GetNodeId(reply) => {
                let _ = reply.send(state.node_id);
            }
            RaftActorMessage::Ping(reply) => {
                // Immediately respond to health check ping (actor is alive)
                let _ = reply.send(());
            }
            RaftActorMessage::CurrentState(reply) => {
                let result = ensure_initialized(state).map(|_| {
                    // Get the actual membership from Raft metrics instead of using cached state
                    let metrics = state.raft.metrics().borrow().clone();
                    let membership = &metrics.membership_config;

                    // Build ClusterState from actual membership
                    let mut nodes = Vec::new();
                    let mut learners = Vec::new();
                    let mut members = Vec::new();

                    // Get voter IDs first
                    let voter_ids: std::collections::HashSet<u64> =
                        membership.membership().voter_ids().collect();

                    // Get all nodes from membership (includes AspenNode with Iroh addresses)
                    for (node_id, aspen_node) in membership.membership().nodes() {
                        let cluster_node = ClusterNode {
                            id: *node_id,
                            addr: aspen_node.iroh_addr.id.to_string(),
                            raft_addr: None, // Legacy field, not needed with Iroh
                            iroh_addr: Some(aspen_node.iroh_addr.clone()),
                        };

                        if voter_ids.contains(node_id) {
                            members.push(*node_id);
                            nodes.push(cluster_node);
                        } else {
                            learners.push(cluster_node);
                        }
                    }

                    ClusterState {
                        nodes,
                        members,
                        learners,
                    }
                });
                let _ = reply.send(result);
            }
            RaftActorMessage::InitCluster(request, reply) => {
                let result = handle_init(state, request).await;
                let _ = reply.send(result);
            }
            RaftActorMessage::AddLearner(request, reply) => {
                let result = async {
                    ensure_initialized(state)?;
                    handle_add_learner(state, request).await
                }
                .await;
                let _ = reply.send(result);
            }
            RaftActorMessage::ChangeMembership(request, reply) => {
                let result = async {
                    ensure_initialized(state)?;
                    handle_change_membership(state, request).await
                }
                .await;
                let _ = reply.send(result);
            }
            RaftActorMessage::Write(request, reply) => {
                let result = async {
                    ensure_initialized_kv(state)?;
                    handle_write(state, request).await
                }
                .await;
                let _ = reply.send(result);
            }
            RaftActorMessage::Read(request, reply) => {
                let result = async {
                    ensure_initialized_kv(state)?;
                    handle_read(state, request).await
                }
                .await;
                let _ = reply.send(result);
            }
            RaftActorMessage::Delete(request, reply) => {
                let result = async {
                    ensure_initialized_kv(state)?;
                    handle_delete(state, request).await
                }
                .await;
                let _ = reply.send(result);
            }
            RaftActorMessage::GetMetrics(reply) => {
                // Get metrics from the Raft instance's watch channel
                let metrics = state.raft.metrics().borrow().clone();
                let _ = reply.send(Ok(metrics));
            }
            RaftActorMessage::TriggerSnapshot(reply) => {
                // Trigger snapshot creation
                let result = match state.raft.trigger().snapshot().await {
                    Ok(()) => {
                        // Snapshot triggered successfully
                        // Get the current snapshot ID from metrics
                        let metrics = state.raft.metrics().borrow().clone();
                        Ok(metrics.snapshot)
                    }
                    Err(err) => Err(ControlPlaneError::Failed {
                        reason: err.to_string(),
                    }),
                };
                let _ = reply.send(result);
            }
            RaftActorMessage::Shutdown => {
                warn!(node_id = state.node_id, "raft actor shutting down");
                myself.stop(Some("raft-actor-shutdown".into()));
            }
        }
        Ok(())
    }
}

fn ensure_initialized(state: &RaftActorState) -> Result<(), ControlPlaneError> {
    if state.initialized {
        Ok(())
    } else {
        Err(ControlPlaneError::NotInitialized)
    }
}

fn ensure_initialized_kv(state: &RaftActorState) -> Result<(), KeyValueStoreError> {
    // Check if node is part of a cluster (either as voter or learner)
    // This allows reads from learners and promoted voters even if init() wasn't called
    let node_id = state.node_id;
    let is_voter = state.raft.voter_ids().any(|id| id == node_id);
    let is_learner = state.raft.learner_ids().any(|id| id == node_id);

    if state.initialized || is_voter || is_learner {
        Ok(())
    } else {
        Err(KeyValueStoreError::Failed {
            reason: "cluster not initialized".into(),
        })
    }
}

/// Ensure the node has an Iroh address for Raft communication.
///
/// With the introduction of `AspenNode`, the primary address source is `iroh_addr`.
/// The legacy `raft_addr` is no longer required.
fn ensure_iroh_addr(node: &ClusterNode) -> Result<&EndpointAddr, ControlPlaneError> {
    node.iroh_addr
        .as_ref()
        .ok_or_else(|| ControlPlaneError::InvalidRequest {
            reason: format!(
                "iroh_addr must be set for node {} (use ClusterNode::with_iroh_addr)",
                node.id
            ),
        })
}

#[instrument(skip(state), fields(node_id = state.node_id, members = request.initial_members.len()))]
async fn handle_init(
    state: &mut RaftActorState,
    request: InitRequest,
) -> Result<ClusterState, ControlPlaneError> {
    if request.initial_members.is_empty() {
        return Err(ControlPlaneError::InvalidRequest {
            reason: "initial_members must not be empty".into(),
        });
    }

    // Build AspenNode map from ClusterNodes with Iroh addresses
    let mut nodes = BTreeMap::new();
    for cluster_node in &request.initial_members {
        let iroh_addr = ensure_iroh_addr(cluster_node)?;
        nodes.insert(cluster_node.id, AspenNode::new(iroh_addr.clone()));
    }

    state
        .raft
        .initialize(nodes)
        .await
        .map_err(|err| ControlPlaneError::Failed {
            reason: err.to_string(),
        })?;
    state.cluster_state.nodes = request.initial_members.clone();
    state.cluster_state.learners.clear();
    state.cluster_state.members = request.initial_members.iter().map(|node| node.id).collect();
    state.initialized = true;
    Ok(state.cluster_state.clone())
}

#[instrument(skip(state), fields(node_id = state.node_id, learner_id = request.learner.id))]
async fn handle_add_learner(
    state: &mut RaftActorState,
    request: AddLearnerRequest,
) -> Result<ClusterState, ControlPlaneError> {
    let learner = request.learner;
    let iroh_addr = ensure_iroh_addr(&learner)?;
    let node = AspenNode::new(iroh_addr.clone());

    info!(
        learner_id = learner.id,
        endpoint_id = %iroh_addr.id,
        "adding learner with Iroh address stored in Raft membership"
    );

    state
        .raft
        .add_learner(learner.id, node, true)
        .await
        .map_err(|err| ControlPlaneError::Failed {
            reason: err.to_string(),
        })?;
    state.cluster_state.learners.push(learner);
    Ok(state.cluster_state.clone())
}

#[instrument(skip(state), fields(node_id = state.node_id, new_members = ?request.members))]
async fn handle_change_membership(
    state: &mut RaftActorState,
    request: ChangeMembershipRequest,
) -> Result<ClusterState, ControlPlaneError> {
    if request.members.is_empty() {
        return Err(ControlPlaneError::InvalidRequest {
            reason: "members must include at least one voter".into(),
        });
    }
    let new_members = request.members;
    let members: BTreeSet<u64> = new_members.iter().copied().collect();
    state
        .raft
        .change_membership(members, false)
        .await
        .map_err(|err| ControlPlaneError::Failed {
            reason: err.to_string(),
        })?;
    state.cluster_state.members = new_members;
    Ok(state.cluster_state.clone())
}

/// Validate key size against MAX_KEY_SIZE.
fn validate_key_size(key: &str) -> Result<(), KeyValueStoreError> {
    if key.len() > MAX_KEY_SIZE as usize {
        return Err(KeyValueStoreError::KeyTooLarge {
            size: key.len(),
            max: MAX_KEY_SIZE,
        });
    }
    Ok(())
}

/// Validate value size against MAX_VALUE_SIZE.
fn validate_value_size(value: &str) -> Result<(), KeyValueStoreError> {
    if value.len() > MAX_VALUE_SIZE as usize {
        return Err(KeyValueStoreError::ValueTooLarge {
            size: value.len(),
            max: MAX_VALUE_SIZE,
        });
    }
    Ok(())
}

/// Validate batch size against MAX_SETMULTI_KEYS.
fn validate_batch_size(size: usize) -> Result<(), KeyValueStoreError> {
    if size > MAX_SETMULTI_KEYS as usize {
        return Err(KeyValueStoreError::BatchTooLarge {
            size,
            max: MAX_SETMULTI_KEYS,
        });
    }
    Ok(())
}

#[instrument(skip(state, request), fields(node_id = state.node_id, command = ?request.command))]
async fn handle_write(
    state: &mut RaftActorState,
    request: WriteRequest,
) -> Result<WriteResult, KeyValueStoreError> {
    let cmd = request.command.clone();

    // Tiger Style: Validate input sizes before processing
    let app_req = match cmd.clone() {
        WriteCommand::Set { ref key, ref value } => {
            validate_key_size(key)?;
            validate_value_size(value)?;
            AppRequest::Set {
                key: key.clone(),
                value: value.clone(),
            }
        }
        WriteCommand::SetMulti { ref pairs } => {
            validate_batch_size(pairs.len())?;
            for (key, value) in pairs {
                validate_key_size(key)?;
                validate_value_size(value)?;
            }
            AppRequest::SetMulti {
                pairs: pairs.clone(),
            }
        }
        WriteCommand::Delete { ref key } => {
            validate_key_size(key)?;
            AppRequest::Delete { key: key.clone() }
        }
        WriteCommand::DeleteMulti { ref keys } => {
            validate_batch_size(keys.len())?;
            for key in keys {
                validate_key_size(key)?;
            }
            AppRequest::DeleteMulti { keys: keys.clone() }
        }
    };
    state
        .raft
        .client_write(app_req)
        .await
        .map_err(|err| KeyValueStoreError::Failed {
            reason: err.to_string(),
        })?;
    Ok(WriteResult { command: cmd })
}

#[instrument(skip(state), fields(node_id = state.node_id, key = %request.key))]
async fn handle_delete(
    state: &mut RaftActorState,
    request: DeleteRequest,
) -> Result<DeleteResult, KeyValueStoreError> {
    // Validate key size
    validate_key_size(&request.key)?;

    // Check if key exists before deletion (for deleted flag)
    let existed = state.state_machine.get(&request.key).await.is_some();

    // Submit delete through Raft
    let app_req = AppRequest::Delete {
        key: request.key.clone(),
    };
    state
        .raft
        .client_write(app_req)
        .await
        .map_err(|err| KeyValueStoreError::Failed {
            reason: err.to_string(),
        })?;

    Ok(DeleteResult {
        key: request.key,
        deleted: existed,
    })
}

#[instrument(skip(state), fields(node_id = state.node_id, key = %request.key))]
async fn handle_read(
    state: &RaftActorState,
    request: ReadRequest,
) -> Result<ReadResult, KeyValueStoreError> {
    let linearizer = state
        .raft
        .get_read_linearizer(ReadPolicy::ReadIndex)
        .await
        .map_err(|err| KeyValueStoreError::Failed {
            reason: err.to_string(),
        })?;
    linearizer
        .await_ready(&state.raft)
        .await
        .map_err(|err| KeyValueStoreError::Failed {
            reason: err.to_string(),
        })?;
    if let Some(value) = state.state_machine.get(&request.key).await {
        Ok(ReadResult {
            key: request.key,
            value,
        })
    } else {
        Err(KeyValueStoreError::NotFound { key: request.key })
    }
}

/// Controller that proxies all operations through the Raft actor.
///
/// RaftControlClient now uses a bounded mailbox proxy by default to prevent
/// memory exhaustion under high load. The bounded mailbox enforces a capacity
/// limit and provides backpressure when the mailbox is full.
#[derive(Clone)]
pub struct RaftControlClient {
    actor: ActorRef<RaftActorMessage>,
    proxy: Option<bounded_proxy::BoundedRaftActorProxy>,
}

impl RaftControlClient {
    /// Create a new RaftControlClient without bounded mailbox (legacy).
    ///
    /// # Warning
    ///
    /// This creates an unbounded mailbox which can lead to memory exhaustion
    /// under high load. Consider using `new_bounded` instead.
    pub fn new(actor: ActorRef<RaftActorMessage>) -> Self {
        Self { actor, proxy: None }
    }

    /// Create a new RaftControlClient with bounded mailbox (recommended).
    ///
    /// Uses default capacity of 1000 messages.
    ///
    /// # Arguments
    ///
    /// * `actor` - The RaftActor reference to wrap
    /// * `node_id` - Node ID for logging and debugging
    pub fn new_bounded(actor: ActorRef<RaftActorMessage>, node_id: u64) -> Self {
        let proxy = bounded_proxy::BoundedRaftActorProxy::new(actor.clone(), node_id);
        Self {
            actor,
            proxy: Some(proxy),
        }
    }

    /// Create a new RaftControlClient with custom mailbox capacity.
    ///
    /// # Arguments
    ///
    /// * `actor` - The RaftActor reference to wrap
    /// * `capacity` - Maximum number of messages in mailbox
    /// * `node_id` - Node ID for logging and debugging
    ///
    /// # Errors
    ///
    /// Returns `BoundedMailboxError::InvalidCapacity` if capacity is 0 or exceeds MAX_CAPACITY.
    pub fn new_with_capacity(
        actor: ActorRef<RaftActorMessage>,
        capacity: u32,
        node_id: u64,
    ) -> Result<Self, bounded_proxy::BoundedMailboxError> {
        let proxy =
            bounded_proxy::BoundedRaftActorProxy::with_capacity(actor.clone(), capacity, node_id)?;
        Ok(Self {
            actor,
            proxy: Some(proxy),
        })
    }

    /// Get the bounded mailbox proxy if available.
    ///
    /// Returns None if the client was created without bounded mailbox.
    pub fn proxy(&self) -> Option<&bounded_proxy::BoundedRaftActorProxy> {
        self.proxy.as_ref()
    }
}

#[async_trait]
impl ClusterController for RaftControlClient {
    async fn init(&self, request: InitRequest) -> Result<ClusterState, ControlPlaneError> {
        call_t!(self.actor, RaftActorMessage::InitCluster, 500, request).map_err(|err| {
            ControlPlaneError::Failed {
                reason: err.to_string(),
            }
        })?
    }

    async fn add_learner(
        &self,
        request: AddLearnerRequest,
    ) -> Result<ClusterState, ControlPlaneError> {
        // Tiger Style: Adding a learner may require initial connection setup and log sync.
        // Use a longer timeout (5s) to account for network establishment.
        call_t!(self.actor, RaftActorMessage::AddLearner, 5000, request).map_err(|err| {
            ControlPlaneError::Failed {
                reason: err.to_string(),
            }
        })?
    }

    async fn change_membership(
        &self,
        request: ChangeMembershipRequest,
    ) -> Result<ClusterState, ControlPlaneError> {
        // Tiger Style: Membership changes require waiting for replication to a quorum,
        // which can take multiple heartbeat cycles. Use a longer timeout (10s) to account for:
        // - Network latency establishing connections
        // - Log replication to new members
        // - Joint consensus commit (C-old,new -> C-new)
        call_t!(
            self.actor,
            RaftActorMessage::ChangeMembership,
            10000,
            request
        )
        .map_err(|err| ControlPlaneError::Failed {
            reason: err.to_string(),
        })?
    }

    async fn current_state(&self) -> Result<ClusterState, ControlPlaneError> {
        call_t!(self.actor, RaftActorMessage::CurrentState, 500).map_err(|err| {
            ControlPlaneError::Failed {
                reason: err.to_string(),
            }
        })?
    }

    async fn get_metrics(&self) -> Result<RaftMetrics<AppTypeConfig>, ControlPlaneError> {
        call_t!(self.actor, RaftActorMessage::GetMetrics, 100).map_err(|err| {
            ControlPlaneError::Failed {
                reason: err.to_string(),
            }
        })?
    }

    async fn trigger_snapshot(&self) -> Result<Option<LogId<AppTypeConfig>>, ControlPlaneError> {
        call_t!(self.actor, RaftActorMessage::TriggerSnapshot, 5000).map_err(|err| {
            ControlPlaneError::Failed {
                reason: err.to_string(),
            }
        })?
    }
}

#[async_trait]
impl KeyValueStore for RaftControlClient {
    async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError> {
        call_t!(self.actor, RaftActorMessage::Write, 500, request).map_err(|err| {
            KeyValueStoreError::Failed {
                reason: err.to_string(),
            }
        })?
    }

    async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError> {
        call_t!(self.actor, RaftActorMessage::Read, 500, request).map_err(|err| {
            KeyValueStoreError::Failed {
                reason: err.to_string(),
            }
        })?
    }

    async fn delete(&self, request: DeleteRequest) -> Result<DeleteResult, KeyValueStoreError> {
        call_t!(self.actor, RaftActorMessage::Delete, 500, request).map_err(|err| {
            KeyValueStoreError::Failed {
                reason: err.to_string(),
            }
        })?
    }
}
