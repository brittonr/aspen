use std::collections::{BTreeMap, HashMap, HashSet};
use std::error::Error as StdError;
use std::future::Future;
use std::io;
use std::sync::{Arc, Once};
use std::time::{Duration, Instant};

use aspen::kv::store::{KvLogStore, KvStateMachine, new_storage};
use aspen::kv::types::{self, NodeId, TypeConfig};
use mad_turmoil::rand as mad_rand;
use openraft::error::{InstallSnapshotError, NetworkError, RPCError, RaftError, RemoteError};
use openraft::metrics::WaitError;
use openraft::network::{RaftNetwork, RaftNetworkFactory};
use openraft::raft::{
    AppendEntriesRequest, AppendEntriesResponse, ClientWriteResponse, InstallSnapshotRequest,
    InstallSnapshotResponse, VoteRequest, VoteResponse,
};
use openraft::{BasicNode, Config, Raft};
use rand09::SeedableRng;
use rand09::rngs::StdRng;
use tempfile::TempDir;
use tokio::sync::RwLock;

const RPC_LATENCY: Duration = Duration::from_millis(5);

#[cfg_attr(madsim, madsim::test)]
#[cfg_attr(not(madsim), tokio::test(flavor = "multi_thread", worker_threads = 4))]
async fn kv_cluster_replication() {
    init_determinism();
    let cluster = TestCluster::bootstrap(&[1, 2, 3]).await;
    let leader = cluster
        .wait_for_leader_in(cluster.members(), "initial leader")
        .await;

    cluster
        .write_and_replicate(
            leader,
            types::Request::Set {
                key: "alpha".into(),
                value: "one".into(),
            },
        )
        .await;
    cluster.assert_value("alpha", Some("one")).await;

    cluster
        .write_and_replicate(
            leader,
            types::Request::Delete {
                key: "alpha".into(),
            },
        )
        .await;
    cluster.assert_value("alpha", None).await;
}

#[cfg_attr(madsim, madsim::test)]
#[cfg_attr(not(madsim), tokio::test(flavor = "multi_thread", worker_threads = 4))]
async fn kv_cluster_partition_failover() {
    init_determinism();
    let cluster = TestCluster::bootstrap(&[10, 20, 30]).await;
    let initial_leader = cluster
        .wait_for_leader_in(cluster.members(), "initial leader")
        .await;

    cluster
        .write_and_replicate(
            initial_leader,
            types::Request::Set {
                key: "stable".into(),
                value: "v1".into(),
            },
        )
        .await;
    cluster.assert_value("stable", Some("v1")).await;

    cluster.isolate_node(initial_leader).await;
    let survivors: Vec<_> = cluster
        .members()
        .iter()
        .copied()
        .filter(|id| *id != initial_leader)
        .collect();
    let failover_leader = cluster
        .wait_for_leader_in(&survivors, "failover leader")
        .await;
    assert_ne!(failover_leader, initial_leader);

    let replicated = cluster
        .write_and_replicate_scoped(
            failover_leader,
            types::Request::Set {
                key: "after".into(),
                value: "v2".into(),
            },
            &survivors,
        )
        .await;

    cluster.heal_all().await;
    cluster
        .wait_for_applied_on(replicated, None, "heal replication")
        .await;

    cluster.assert_value("stable", Some("v1")).await;
    cluster.assert_value("after", Some("v2")).await;
}

fn init_determinism() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let rng = StdRng::seed_from_u64(42);
        mad_rand::set_rng(rng);
    });
}

struct TestNode {
    #[allow(dead_code)]
    dir: TempDir,
    raft: Raft<TypeConfig>,
    state_machine: KvStateMachine,
    _log_store: KvLogStore,
}

struct TestCluster {
    nodes: BTreeMap<NodeId, TestNode>,
    router: TestRouter,
    members: Vec<NodeId>,
}

impl TestCluster {
    async fn bootstrap(ids: &[NodeId]) -> Self {
        let router = TestRouter::new();
        let config = Arc::new(build_config());
        let mut nodes = BTreeMap::new();

        for &id in ids {
            let dir = TempDir::new().expect("tempdir");
            let db_path = dir.path().join("kv.redb");
            let (log_store, state_machine) = new_storage(db_path.to_str().expect("path"))
                .await
                .expect("storage");
            let network = TestNetworkFactory::new(id, router.clone());
            let raft = Raft::new(
                id,
                config.clone(),
                network.clone(),
                log_store.clone(),
                state_machine.clone(),
            )
            .await
            .expect("raft node");
            router.register(id, raft.clone()).await;

            nodes.insert(
                id,
                TestNode {
                    dir,
                    raft,
                    state_machine,
                    _log_store: log_store,
                },
            );
        }

        let cluster = Self {
            nodes,
            router,
            members: ids.to_vec(),
        };
        cluster.initialize().await;
        cluster
    }

    async fn initialize(&self) {
        let first = self.members[0];
        let membership = self.members.iter().fold(BTreeMap::new(), |mut acc, id| {
            acc.insert(*id, BasicNode::new(format!("node-{id}")));
            acc
        });
        self.nodes
            .get(&first)
            .expect("initializer")
            .raft
            .initialize(membership)
            .await
            .expect("cluster init");
    }

    fn members(&self) -> &[NodeId] {
        &self.members
    }

    async fn wait_for_leader_in(&self, scope: &[NodeId], label: &str) -> NodeId {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            for &id in scope {
                let node = self.nodes.get(&id).expect("node");
                match node
                    .raft
                    .wait(Some(Duration::from_millis(500)))
                    .metrics(
                        |m| m.current_leader.is_some(),
                        format!("{label} via node {id}"),
                    )
                    .await
                {
                    Ok(metrics) => return metrics.current_leader.expect("leader id"),
                    Err(WaitError::Timeout(_, _)) => {
                        if Instant::now() >= deadline {
                            panic!("timed out waiting for leader via node {}", id);
                        }
                    }
                    Err(err) => panic!("wait error for node {}: {:?}", id, err),
                }
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    async fn write_and_replicate(&self, leader: NodeId, request: types::Request) -> u64 {
        let index = self.write_only(leader, request).await;
        self.wait_for_applied_on(index, None, "full replication")
            .await;
        index
    }

    async fn write_and_replicate_scoped(
        &self,
        leader: NodeId,
        request: types::Request,
        scope: &[NodeId],
    ) -> u64 {
        let index = self.write_only(leader, request).await;
        self.wait_for_applied_on(index, Some(scope), "scoped replication")
            .await;
        index
    }

    async fn write_only(&self, leader: NodeId, request: types::Request) -> u64 {
        let node = self.nodes.get(&leader).expect("leader");
        let ClientWriteResponse { log_id, .. } = node
            .raft
            .client_write(request)
            .await
            .expect("client write succeeds");
        log_id.index
    }

    async fn wait_for_applied_on(&self, index: u64, scope: Option<&[NodeId]>, label: &str) {
        let targets: Vec<_> = scope
            .map(|ids| ids.to_vec())
            .unwrap_or_else(|| self.members.clone());
        for id in targets {
            let node = self.nodes.get(&id).expect("node");
            node.raft
                .wait(Some(Duration::from_secs(3)))
                .applied_index(Some(index), format!("{label} on node {id}"))
                .await
                .unwrap_or_else(|e| panic!("node {} failed to apply {}: {:?}", id, index, e));
        }
    }

    async fn assert_value(&self, key: &str, expected: Option<&str>) {
        for (&id, node) in &self.nodes {
            let value = node
                .state_machine
                .get(key)
                .await
                .unwrap_or_else(|e| panic!("state machine error on node {id}: {e:?}"));
            assert_eq!(
                value.as_deref(),
                expected,
                "value mismatch on node {id} for key {key}"
            );
        }
    }

    async fn isolate_node(&self, node: NodeId) {
        for &other in &self.members {
            if other != node {
                self.router.isolate(node, other).await;
            }
        }
    }

    async fn heal_all(&self) {
        self.router.heal_all().await;
    }
}

fn build_config() -> Config {
    let mut config = Config::build(&["aspen-kv-tests"]).expect("config build");
    config.election_timeout_min = 40;
    config.election_timeout_max = 80;
    config.heartbeat_interval = 20;
    config.validate().expect("config validate")
}

#[derive(Clone, Default)]
struct TestRouter {
    inner: Arc<TestRouterInner>,
}

#[derive(Default)]
struct TestRouterInner {
    peers: RwLock<HashMap<NodeId, Raft<TypeConfig>>>,
    partitions: RwLock<HashSet<(NodeId, NodeId)>>,
}

impl TestRouter {
    fn new() -> Self {
        Self {
            inner: Arc::new(TestRouterInner::default()),
        }
    }

    async fn register(&self, node: NodeId, raft: Raft<TypeConfig>) {
        self.inner.peers.write().await.insert(node, raft);
    }

    async fn isolate(&self, a: NodeId, b: NodeId) {
        let mut links = self.inner.partitions.write().await;
        links.insert((a, b));
        links.insert((b, a));
    }

    async fn heal_all(&self) {
        self.inner.partitions.write().await.clear();
    }

    async fn connected(&self, a: NodeId, b: NodeId) -> bool {
        !self.inner.partitions.read().await.contains(&(a, b))
    }

    async fn with_target<F, Fut, T, E>(
        &self,
        from: NodeId,
        to: NodeId,
        f: F,
    ) -> Result<T, RPCError<NodeId, BasicNode, RaftError<NodeId, E>>>
    where
        F: FnOnce(Raft<TypeConfig>) -> Fut,
        Fut: Future<Output = Result<T, RaftError<NodeId, E>>>,
        E: Send + Sync + std::fmt::Debug + StdError + 'static,
    {
        if !self.connected(from, to).await {
            return Err(RPCError::Network(partition_error()));
        }
        let target = self
            .inner
            .peers
            .read()
            .await
            .get(&to)
            .cloned()
            .ok_or_else(|| RPCError::Network(partition_error()))?;
        tokio::time::sleep(RPC_LATENCY).await;
        match f(target).await {
            Ok(resp) => Ok(resp),
            Err(err) => Err(RPCError::RemoteError(RemoteError::new(to, err))),
        }
    }
}

#[derive(Clone)]
struct TestNetworkFactory {
    node: NodeId,
    router: TestRouter,
}

impl TestNetworkFactory {
    fn new(node: NodeId, router: TestRouter) -> Self {
        Self { node, router }
    }
}

impl RaftNetworkFactory<TypeConfig> for TestNetworkFactory {
    type Network = TestNetwork;

    async fn new_client(&mut self, target: NodeId, _node: &BasicNode) -> Self::Network {
        TestNetwork {
            from: self.node,
            target,
            router: self.router.clone(),
        }
    }
}

struct TestNetwork {
    from: NodeId,
    target: NodeId,
    router: TestRouter,
}

impl RaftNetwork<TypeConfig> for TestNetwork {
    async fn append_entries(
        &mut self,
        req: AppendEntriesRequest<TypeConfig>,
        _option: openraft::network::RPCOption,
    ) -> Result<AppendEntriesResponse<NodeId>, RPCError<NodeId, BasicNode, RaftError<NodeId>>> {
        self.router
            .with_target(self.from, self.target, move |raft| async move {
                raft.append_entries(req).await
            })
            .await
    }

    async fn install_snapshot(
        &mut self,
        req: InstallSnapshotRequest<TypeConfig>,
        _option: openraft::network::RPCOption,
    ) -> Result<
        InstallSnapshotResponse<NodeId>,
        RPCError<NodeId, BasicNode, RaftError<NodeId, InstallSnapshotError>>,
    > {
        self.router
            .with_target(self.from, self.target, move |raft| async move {
                raft.install_snapshot(req).await
            })
            .await
    }

    async fn vote(
        &mut self,
        req: VoteRequest<NodeId>,
        _option: openraft::network::RPCOption,
    ) -> Result<VoteResponse<NodeId>, RPCError<NodeId, BasicNode, RaftError<NodeId>>> {
        self.router
            .with_target(self.from, self.target, move |raft| async move {
                raft.vote(req).await
            })
            .await
    }
}

fn partition_error() -> NetworkError {
    let err = io::Error::new(io::ErrorKind::Other, "link down");
    NetworkError::new(&err)
}
