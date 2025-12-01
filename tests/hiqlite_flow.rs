use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use aspen::api::{
    AddLearnerRequest, ChangeMembershipRequest, ClusterController, ClusterNode, ClusterState,
    ControlPlaneError, InitRequest, KeyValueStore, KeyValueStoreError, ReadRequest, ReadResult,
    WriteCommand, WriteRequest, WriteResult,
};
use aspen::cluster::{
    DeterministicClusterConfig, IrohClusterConfig, IrohClusterTransport, NodeServerConfig,
};
use async_trait::async_trait;
use iroh_metrics::{MetricsSource, Registry};
use tokio::sync::Mutex;

#[derive(Clone, Default)]
struct DeterministicHiqlite {
    cluster: Arc<Mutex<ClusterState>>,
    kv: Arc<Mutex<HashMap<String, String>>>,
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
        Ok(guard.clone())
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
    let backend = DeterministicHiqlite::new();

    let initial = vec![
        DeterministicHiqlite::node(1, 21011, 26011),
        DeterministicHiqlite::node(2, 21012, 26012),
        DeterministicHiqlite::node(3, 21013, 26013),
    ];
    let cluster = backend
        .init(InitRequest {
            initial_members: initial.clone(),
        })
        .await
        .expect("init cluster");
    assert_eq!(cluster.members, vec![1, 2, 3]);

    backend
        .add_learner(AddLearnerRequest {
            learner: DeterministicHiqlite::node(4, 21014, 26014),
        })
        .await
        .expect("add learner");
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

    writer.await.expect("writer task");
    reader.await.expect("reader task");

    let node_server = NodeServerConfig::new("hiqlite-sim", "127.0.0.1", 0, "sim-cookie")
        .with_determinism(DeterministicClusterConfig {
            simulation_seed: Some(42),
        })
        .launch()
        .await
        .expect("node server");
    let transport = IrohClusterTransport::spawn(&node_server, IrohClusterConfig::default())
        .await
        .expect("iroh transport");
    transport.wait_until_online().await;
    let mut registry = Registry::default();
    registry.register_all(transport.endpoint().metrics());
    let snapshot = registry
        .encode_openmetrics_to_string()
        .expect("encode metrics");
    assert!(
        snapshot.contains("magicsock_recv_datagrams"),
        "expected magicsock counters in {snapshot}"
    );
    transport.shutdown().await.expect("shutdown transport");
    node_server.shutdown().await.expect("shutdown node");
}
