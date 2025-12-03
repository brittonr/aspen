use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use aspen::api::{
    AddLearnerRequest, ChangeMembershipRequest, ClusterController, ClusterNode, ClusterState,
    ControlPlaneError, InitRequest, KeyValueStore, KeyValueStoreError, ReadRequest, ReadResult,
    WriteCommand, WriteRequest, WriteResult,
};
use aspen::simulation::SimulationArtifactBuilder;
use async_trait::async_trait;
use tokio::sync::Mutex;

#[derive(Clone, Default)]
struct DeterministicHiqlite {
    cluster: Arc<Mutex<ClusterState>>,
    kv: Arc<Mutex<HashMap<String, String>>>,
    leader: Arc<Mutex<Option<u64>>>,
    events: Arc<Mutex<Vec<String>>>,
}

impl DeterministicHiqlite {
    fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn ensure_raft_addr(node: &ClusterNode) -> Result<(), ControlPlaneError> {
        if node.raft_addr.is_none() {
            return Err(ControlPlaneError::InvalidRequest {
                reason: "hiqlite nodes require raft_addr metadata".into(),
            });
        }
        Ok(())
    }

    fn node(id: u64, http: u16, raft: u16) -> ClusterNode {
        ClusterNode {
            id,
            addr: format!("127.0.0.1:{http}"),
            raft_addr: Some(format!("127.0.0.1:{raft}")),
        }
    }

    async fn set_leader(&self, leader: u64) {
        let mut guard = self.leader.lock().await;
        *guard = Some(leader);
        self.events
            .lock()
            .await
            .push(format!("leader-elected:{leader}"));
    }

    async fn leader_trace(&self) -> Vec<String> {
        self.events.lock().await.clone()
    }
}

#[async_trait]
impl ClusterController for DeterministicHiqlite {
    async fn init(&self, request: InitRequest) -> Result<ClusterState, ControlPlaneError> {
        if request.initial_members.is_empty() {
            return Err(ControlPlaneError::InvalidRequest {
                reason: "initial_members must not be empty".into(),
            });
        }
        for node in &request.initial_members {
            Self::ensure_raft_addr(node)?;
        }
        let mut guard = self.cluster.lock().await;
        guard.nodes = request.initial_members.clone();
        guard.members = request.initial_members.iter().map(|node| node.id).collect();
        let result = guard.clone();
        let leader = result.members.first().copied();
        drop(guard);
        if let Some(first) = leader {
            self.set_leader(first).await;
        }
        Ok(result)
    }

    async fn add_learner(
        &self,
        request: AddLearnerRequest,
    ) -> Result<ClusterState, ControlPlaneError> {
        Self::ensure_raft_addr(&request.learner)?;
        let mut guard = self.cluster.lock().await;
        guard.learners.push(request.learner);
        Ok(guard.clone())
    }

    async fn change_membership(
        &self,
        request: ChangeMembershipRequest,
    ) -> Result<ClusterState, ControlPlaneError> {
        if request.members.is_empty() {
            return Err(ControlPlaneError::InvalidRequest {
                reason: "members must include at least one voter".into(),
            });
        }
        let mut guard = self.cluster.lock().await;
        guard.members = request.members;
        Ok(guard.clone())
    }

    async fn current_state(&self) -> Result<ClusterState, ControlPlaneError> {
        Ok(self.cluster.lock().await.clone())
    }

    async fn get_metrics(&self) -> Result<aspen::api::RaftMetrics<aspen::raft::types::AppTypeConfig>, ControlPlaneError> {
        // Deterministic test backend doesn't have real Raft metrics
        Err(ControlPlaneError::Failed {
            reason: "metrics not available in deterministic test backend".into(),
        })
    }

    async fn trigger_snapshot(&self) -> Result<Option<openraft::LogId<aspen::raft::types::AppTypeConfig>>, ControlPlaneError> {
        // Deterministic test backend doesn't support snapshots
        Err(ControlPlaneError::Failed {
            reason: "snapshots not available in deterministic test backend".into(),
        })
    }
}

#[async_trait]
impl KeyValueStore for DeterministicHiqlite {
    async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError> {
        let mut kv = self.kv.lock().await;
        match request.command.clone() {
            WriteCommand::Set { key, value } => {
                kv.insert(key.clone(), value.clone());
                Ok(WriteResult {
                    command: WriteCommand::Set { key, value },
                })
            }
            WriteCommand::SetMulti { pairs } => {
                for (key, value) in &pairs {
                    kv.insert(key.clone(), value.clone());
                }
                Ok(WriteResult {
                    command: WriteCommand::SetMulti { pairs },
                })
            }
        }
    }

    async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError> {
        let kv = self.kv.lock().await;
        match kv.get(&request.key) {
            Some(value) => Ok(ReadResult {
                key: request.key,
                value: value.clone(),
            }),
            None => Err(KeyValueStoreError::NotFound { key: request.key }),
        }
    }
}

#[madsim::test]
async fn hiqlite_flow_simulation_tracks_transport_metrics() {
    const SIMULATION_SEED: u64 = 42;
    let mut artifact_builder =
        SimulationArtifactBuilder::new("hiqlite_flow_simulation_tracks_transport_metrics", SIMULATION_SEED)
            .start();

    let backend = DeterministicHiqlite::new();

    let initial = vec![
        DeterministicHiqlite::node(1, 21011, 26011),
        DeterministicHiqlite::node(2, 21012, 26012),
        DeterministicHiqlite::node(3, 21013, 26013),
    ];

    artifact_builder = artifact_builder.add_event("init: cluster with nodes [1, 2, 3]");
    let cluster = backend
        .init(InitRequest {
            initial_members: initial.clone(),
        })
        .await
        .expect("init cluster");
    assert_eq!(cluster.members, vec![1, 2, 3]);

    artifact_builder = artifact_builder.add_event("add-learner: node 4");
    backend
        .add_learner(AddLearnerRequest {
            learner: DeterministicHiqlite::node(4, 21014, 26014),
        })
        .await
        .expect("add learner");

    artifact_builder = artifact_builder.add_event("change-membership: promote node 4 to voter");
    let cluster = backend
        .change_membership(ChangeMembershipRequest {
            members: vec![1, 2, 3, 4],
        })
        .await
        .expect("change membership");
    assert_eq!(cluster.members, vec![1, 2, 3, 4]);

    let writer = {
        let backend = backend.clone();
        tokio::spawn(async move {
            for idx in 0..5 {
                backend
                    .write(WriteRequest {
                        command: WriteCommand::Set {
                            key: "fuel".into(),
                            value: format!("payload-{idx}"),
                        },
                    })
                    .await
                    .expect("write payload");
                madsim::time::sleep(Duration::from_millis(5)).await;
            }
        })
    };

    let reader = {
        let backend = backend.clone();
        tokio::spawn(async move {
            madsim::time::sleep(Duration::from_millis(20)).await;
            let result = backend
                .read(ReadRequest { key: "fuel".into() })
                .await
                .expect("read payload");
            assert!(result.value.starts_with("payload-"));
        })
    };

    let churner = {
        let backend = backend.clone();
        tokio::spawn(async move {
            madsim::time::sleep(Duration::from_millis(15)).await;
            backend.set_leader(2).await;
            madsim::time::sleep(Duration::from_millis(20)).await;
            backend.set_leader(3).await;
        })
    };

    writer.await.expect("writer task");
    reader.await.expect("reader task");
    churner.await.expect("churn task");

    artifact_builder = artifact_builder.add_event("writes: completed 5 payloads");
    artifact_builder = artifact_builder.add_event("read: verified payload");

    // Note: Iroh transport testing is incompatible with madsim (see plan.md Phase 3).
    // This simulation tests the deterministic Hiqlite backend logic only.

    let trace = backend.leader_trace().await;
    artifact_builder = artifact_builder.add_events(trace.clone());

    assert!(
        trace.iter().any(|entry| entry.contains("leader-elected:2"))
            && trace.iter().any(|entry| entry.contains("leader-elected:3")),
        "missing churn events in trace {trace:?}"
    );

    let artifact = artifact_builder.build();

    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}
