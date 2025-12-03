pub mod network;
pub mod rpc;
pub mod server;
pub mod storage;
pub mod types;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use async_trait::async_trait;
use openraft::{BasicNode, Raft, ReadPolicy};
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort, call_t};
use tracing::{info, warn};

use crate::api::{
    AddLearnerRequest, ChangeMembershipRequest, ClusterController, ClusterNode, ClusterState,
    ControlPlaneError, InitRequest, KeyValueStore, KeyValueStoreError, ReadRequest, ReadResult,
    WriteCommand, WriteRequest, WriteResult,
};
use crate::raft::storage::StateMachineStore;
use crate::raft::types::{AppRequest, AppTypeConfig};

/// Configuration used to initialize a Raft actor instance.
#[derive(Clone, Debug)]
pub struct RaftActorConfig {
    pub node_id: u64,
    pub raft: Raft<AppTypeConfig>,
    pub state_machine: Arc<StateMachineStore>,
}

/// Empty actor shell that will eventually drive the Raft state machine.
pub struct RaftActor;

#[derive(Debug)]
pub struct RaftActorState {
    node_id: u64,
    raft: Raft<AppTypeConfig>,
    state_machine: Arc<StateMachineStore>,
    cluster_state: ClusterState,
    initialized: bool,
}

#[derive(Debug)]
pub enum RaftActorMessage {
    /// Lightweight RPC used by tests/monitors to confirm the actor is alive.
    GetNodeId(RpcReplyPort<u64>),
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
            RaftActorMessage::CurrentState(reply) => {
                let result = ensure_initialized(state).map(|_| state.cluster_state.clone());
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

fn ensure_raft_addr(node: &ClusterNode) -> Result<(), ControlPlaneError> {
    if node.raft_addr.is_none() {
        Err(ControlPlaneError::InvalidRequest {
            reason: "raft_addr must be set for every node".into(),
        })
    } else {
        Ok(())
    }
}

async fn handle_init(
    state: &mut RaftActorState,
    request: InitRequest,
) -> Result<ClusterState, ControlPlaneError> {
    if request.initial_members.is_empty() {
        return Err(ControlPlaneError::InvalidRequest {
            reason: "initial_members must not be empty".into(),
        });
    }
    for member in &request.initial_members {
        ensure_raft_addr(member)?;
    }
    let mut nodes = BTreeMap::new();
    for node in &request.initial_members {
        nodes.insert(
            node.id,
            BasicNode {
                addr: node.addr.clone(),
            },
        );
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

async fn handle_add_learner(
    state: &mut RaftActorState,
    request: AddLearnerRequest,
) -> Result<ClusterState, ControlPlaneError> {
    let learner = request.learner;
    ensure_raft_addr(&learner)?;
    let node = BasicNode {
        addr: learner.addr.clone(),
    };
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

async fn handle_write(
    state: &mut RaftActorState,
    request: WriteRequest,
) -> Result<WriteResult, KeyValueStoreError> {
    let cmd = request.command.clone();
    let app_req = match cmd.clone() {
        WriteCommand::Set { ref key, ref value } => AppRequest::Set {
            key: key.clone(),
            value: value.clone(),
        },
        WriteCommand::SetMulti { ref pairs } => AppRequest::SetMulti {
            pairs: pairs.clone(),
        },
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
#[derive(Clone)]
pub struct RaftControlClient {
    actor: ActorRef<RaftActorMessage>,
}

impl RaftControlClient {
    pub fn new(actor: ActorRef<RaftActorMessage>) -> Self {
        Self { actor }
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
        call_t!(self.actor, RaftActorMessage::AddLearner, 500, request).map_err(|err| {
            ControlPlaneError::Failed {
                reason: err.to_string(),
            }
        })?
    }

    async fn change_membership(
        &self,
        request: ChangeMembershipRequest,
    ) -> Result<ClusterState, ControlPlaneError> {
        call_t!(self.actor, RaftActorMessage::ChangeMembership, 500, request).map_err(|err| {
            ControlPlaneError::Failed {
                reason: err.to_string(),
            }
        })?
    }

    async fn current_state(&self) -> Result<ClusterState, ControlPlaneError> {
        call_t!(self.actor, RaftActorMessage::CurrentState, 500).map_err(|err| {
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
}
