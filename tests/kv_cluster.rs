use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
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
use tracing_subscriber::EnvFilter;

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

#[cfg_attr(madsim, madsim::test)]
#[cfg_attr(not(madsim), tokio::test(flavor = "multi_thread", worker_threads = 4))]
async fn kv_cluster_txn_semantics() {
    init_determinism();
    let cluster = TestCluster::bootstrap(&[5, 6, 7]).await;
    let leader = cluster
        .wait_for_leader_in(cluster.members(), "txn leader")
        .await;
    let primary = "txn/main";
    let secondary = "txn/secondary";

    let success = cluster
        .write_and_replicate_with_response(
            leader,
            types::Request::Txn {
                operations: vec![
                    types::TxnOp::Assert {
                        key: primary.into(),
                        equals: None,
                    },
                    types::TxnOp::Set {
                        key: primary.into(),
                        value: "one".into(),
                    },
                    types::TxnOp::Set {
                        key: secondary.into(),
                        value: "two".into(),
                    },
                ],
            },
            "txn success",
            None,
        )
        .await;
    let (committed, results) = match success.data {
        types::Response::Txn { committed, results } => (committed, results),
        other => panic!("expected txn response, got {other:?}"),
    };
    assert!(committed, "txn with satisfied assertions should commit");
    assert_eq!(results.len(), 3, "all operations should report results");
    match &results[0] {
        types::TxnOpResult::Assert {
            key,
            current,
            success,
        } => {
            assert_eq!(key, primary);
            assert!(success);
            assert!(current.is_none());
        }
        other => panic!("unexpected first result {other:?}"),
    }
    match &results[1] {
        types::TxnOpResult::Set { key, previous } => {
            assert_eq!(key, primary);
            assert!(previous.is_none());
        }
        other => panic!("unexpected second result {other:?}"),
    }
    match &results[2] {
        types::TxnOpResult::Set { key, previous } => {
            assert_eq!(key, secondary);
            assert!(previous.is_none());
        }
        other => panic!("unexpected third result {other:?}"),
    }
    cluster.assert_value(primary, Some("one")).await;
    cluster.assert_value(secondary, Some("two")).await;

    let failure = cluster
        .write_and_replicate_with_response(
            leader,
            types::Request::Txn {
                operations: vec![
                    types::TxnOp::Assert {
                        key: primary.into(),
                        equals: Some("mismatch".into()),
                    },
                    types::TxnOp::Delete {
                        key: secondary.into(),
                    },
                ],
            },
            "txn failure",
            None,
        )
        .await;
    let (committed, failed_results) = match failure.data {
        types::Response::Txn { committed, results } => (committed, results),
        other => panic!("expected txn response, got {other:?}"),
    };
    assert!(!committed, "failed assertions must abort the txn");
    assert_eq!(
        failed_results.len(),
        1,
        "abort should stop evaluation early"
    );
    match &failed_results[0] {
        types::TxnOpResult::Assert {
            key,
            current,
            success,
        } => {
            assert_eq!(key, primary);
            assert!(!success);
            assert_eq!(current.as_deref(), Some("one"));
        }
        other => panic!("unexpected abort result {other:?}"),
    }
    cluster.assert_value(primary, Some("one")).await;
    cluster.assert_value(secondary, Some("two")).await;
}

#[cfg_attr(madsim, madsim::test)]
#[cfg_attr(not(madsim), tokio::test(flavor = "multi_thread", worker_threads = 4))]
async fn kv_cluster_leases_and_watches() {
    init_determinism();
    let cluster = TestCluster::bootstrap(&[11, 22, 33]).await;
    let leader = cluster
        .wait_for_leader_in(cluster.members(), "lease leader")
        .await;
    let watch_id = 77;
    let lease_id = 909;
    let ttl_ms = 2_000;
    let watch_prefix = "watched/".to_string();
    let watch_key = "watched/alpha".to_string();

    let registered = cluster
        .write_and_replicate_with_response(
            leader,
            types::Request::WatchRegister {
                watch_id,
                key: watch_prefix.clone(),
                prefix: true,
            },
            "watch register",
            None,
        )
        .await;
    match registered.data {
        types::Response::WatchRegistered { watch_id: id } => assert_eq!(id, watch_id),
        other => panic!("unexpected watch register response {other:?}"),
    }

    let granted = cluster
        .write_and_replicate_with_response(
            leader,
            types::Request::LeaseGrant {
                lease_id,
                ttl_ms,
                timestamp_ms: 500,
            },
            "lease grant",
            None,
        )
        .await;
    let lease = match granted.data {
        types::Response::LeaseGranted { lease } => lease,
        other => panic!("unexpected lease grant response {other:?}"),
    };
    assert_eq!(lease.lease_id, lease_id);
    assert_eq!(lease.ttl_ms, ttl_ms);

    let attached = cluster
        .write_and_replicate_with_response(
            leader,
            types::Request::LeaseAttach {
                lease_id,
                key: watch_key.clone(),
            },
            "lease attach",
            None,
        )
        .await;
    let lease = match attached.data {
        types::Response::LeaseAttached { lease } => lease,
        other => panic!("unexpected lease attach response {other:?}"),
    };
    assert!(
        lease.keys.contains(&watch_key),
        "lease should track attached key"
    );

    let set_resp = cluster
        .write_and_replicate_with_response(
            leader,
            types::Request::Set {
                key: watch_key.clone(),
                value: "value-one".into(),
            },
            "watched set",
            None,
        )
        .await;
    assert!(
        matches!(set_resp.data, types::Response::Set { .. }),
        "set response must be returned"
    );
    cluster.assert_value(&watch_key, Some("value-one")).await;

    let keepalive = cluster
        .write_and_replicate_with_response(
            leader,
            types::Request::LeaseKeepAlive {
                lease_id,
                timestamp_ms: 5_000,
            },
            "lease keepalive",
            None,
        )
        .await;
    let lease = match keepalive.data {
        types::Response::LeaseKeepAlive { lease } => lease.expect("lease must exist"),
        other => panic!("unexpected keepalive response {other:?}"),
    };
    assert_eq!(lease.ttl_ms, ttl_ms);
    assert_eq!(lease.expires_at_ms, 5_000 + ttl_ms);

    let revoked = cluster
        .write_and_replicate_with_response(
            leader,
            types::Request::LeaseRevoke { lease_id },
            "lease revoke",
            None,
        )
        .await;
    match revoked.data {
        types::Response::LeaseRevoked {
            lease_id: id,
            released_keys,
        } => {
            assert_eq!(id, lease_id);
            assert_eq!(released_keys, vec![watch_key.clone()]);
        }
        other => panic!("unexpected revoke response {other:?}"),
    }

    let delete_resp = cluster
        .write_and_replicate_with_response(
            leader,
            types::Request::Delete {
                key: watch_key.clone(),
            },
            "watched delete",
            None,
        )
        .await;
    assert!(
        matches!(delete_resp.data, types::Response::Delete { .. }),
        "delete response must be returned"
    );
    cluster.assert_value(&watch_key, None).await;

    let fetched = cluster
        .write_and_replicate_with_response(
            leader,
            types::Request::WatchFetch { watch_id },
            "watch fetch",
            None,
        )
        .await;
    let events = match fetched.data {
        types::Response::WatchEvents {
            watch_id: id,
            events,
        } => {
            assert_eq!(id, watch_id);
            events
        }
        other => panic!("unexpected watch fetch response {other:?}"),
    };
    assert_eq!(events.len(), 2, "watch should capture put and delete");
    assert_eq!(events[0].key, watch_key);
    assert_eq!(events[1].key, watch_key);
    assert!(
        matches!(events[0].kind, types::WatchEventKind::Put),
        "first event must be put"
    );
    assert!(
        matches!(events[1].kind, types::WatchEventKind::Delete),
        "second event must be delete"
    );
    assert!(events[0].revision < events[1].revision);
    assert_eq!(events[0].value.as_deref(), Some("value-one"));
    assert!(events[1].value.is_none());

    let canceled = cluster
        .write_and_replicate_with_response(
            leader,
            types::Request::WatchCancel { watch_id },
            "watch cancel",
            None,
        )
        .await;
    match canceled.data {
        types::Response::WatchCanceled {
            watch_id: id,
            existed,
        } => {
            assert_eq!(id, watch_id);
            assert!(existed, "watch should exist for cancel");
        }
        other => panic!("unexpected watch cancel response {other:?}"),
    }

    let fetch_after_cancel = cluster
        .write_and_replicate_with_response(
            leader,
            types::Request::WatchFetch { watch_id },
            "watch fetch after cancel",
            None,
        )
        .await;
    let post_events = match fetch_after_cancel.data {
        types::Response::WatchEvents { events, .. } => events,
        other => panic!("unexpected fetch after cancel response {other:?}"),
    };
    assert!(
        post_events.is_empty(),
        "canceled watch should not retain events"
    );
}

fn init_determinism() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let rng = StdRng::seed_from_u64(42);
        mad_rand::set_rng(rng);
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_test_writer()
            .try_init();
    });
}

struct TestNode {
    #[allow(dead_code)]
    dir: TempDir,
    raft: Raft<TypeConfig>,
    state_machine: KvStateMachine,
    _log_store: KvLogStore,
}

impl TestNode {
    fn raft(&self) -> Raft<TypeConfig> {
        self.raft.clone()
    }

    fn state_machine(&self) -> KvStateMachine {
        self.state_machine.clone()
    }
}

struct TestCluster {
    nodes: BTreeMap<NodeId, TestNode>,
    router: TestRouter,
    members: Vec<NodeId>,
    config: Arc<Config>,
}

impl TestCluster {
    async fn bootstrap(ids: &[NodeId]) -> Self {
        assert!(
            !ids.is_empty(),
            "cluster bootstrap requires at least one node"
        );
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
                network,
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
            config,
        };
        cluster.initialize_all().await;
        cluster
    }

    async fn initialize_all(&self) {
        let leader_id = self.members[0];
        let single = BTreeSet::from([leader_id]);
        self.nodes
            .get(&leader_id)
            .expect("initializer")
            .raft()
            .initialize(single)
            .await
            .expect("single-node init");
        tracing::info!(node = leader_id, "single-node cluster initialized");
        let _ = self
            .wait_for_leader_in(&[leader_id], "single-node leader")
            .await;

        for &id in self.members.iter().skip(1) {
            tracing::info!(leader = leader_id, learner = id, "calling add_learner");
            let response = self
                .nodes
                .get(&leader_id)
                .expect("leader")
                .raft()
                .add_learner(id, BasicNode::new(format!("node-{id}")), false)
                .await
                .expect("add learner");
            self.wait_for_applied_on(response.log_id.index, Some(&[id]), "learner sync")
                .await;
            tracing::info!(leader = leader_id, learner = id, "learner ready");
        }

        if self.members.len() > 1 {
            self.nodes
                .get(&leader_id)
                .expect("leader")
                .raft()
                .change_membership(self.members.clone(), true)
                .await
                .expect("membership change");
            tracing::info!(node = leader_id, "cluster membership updated");
        }

        let _ = self
            .wait_for_leader_in(self.members(), "bootstrap leader")
            .await;
    }

    fn members(&self) -> &[NodeId] {
        &self.members
    }

    async fn wait_for_leader_in(&self, scope: &[NodeId], label: &str) -> NodeId {
        let metrics_wait = Duration::from_millis(500);
        let retry = Duration::from_millis(25);
        let deadline = Instant::now() + Duration::from_millis(5_000);
        tracing::info!(
            scope = ?scope,
            label,
            wait_ms = metrics_wait.as_millis() as u64,
            retry_ms = retry.as_millis() as u64,
            timeout_ms = (deadline.saturating_duration_since(Instant::now())).as_millis() as u64,
            "waiting for leader"
        );
        loop {
            for &id in scope {
                let node = self.nodes.get(&id).expect("node");
                tracing::debug!(node = id, label, "polling for leader metrics");
                match node
                    .raft()
                    .wait(Some(metrics_wait))
                    .metrics(
                        |m| m.current_leader.is_some(),
                        format!("{label} via node {id}"),
                    )
                    .await
                {
                    Ok(metrics) => {
                        let leader = metrics.current_leader.expect("leader id");
                        tracing::info!(node = id, leader, label, "leader discovered");
                        return leader;
                    }
                    Err(WaitError::Timeout(_, _)) => {
                        if Instant::now() >= deadline {
                            panic!("timed out waiting for leader via node {}", id);
                        }
                        tracing::warn!(node = id, label, "leader wait timed out, retrying");
                    }
                    Err(err) => panic!("wait error for node {}: {:?}", id, err),
                }
            }
            tokio::time::sleep(retry).await;
        }
    }

    async fn write_and_replicate(&self, leader: NodeId, request: types::Request) -> u64 {
        let label = format!("replicate:{}", describe_request(&request));
        let response = self
            .write_and_replicate_with_response(leader, request, &label, None)
            .await;
        response.log_id.index
    }

    async fn write_and_replicate_scoped(
        &self,
        leader: NodeId,
        request: types::Request,
        scope: &[NodeId],
    ) -> u64 {
        let label = format!("scoped:{}", describe_request(&request));
        let response = self
            .write_and_replicate_with_response(leader, request, &label, Some(scope))
            .await;
        response.log_id.index
    }

    async fn write_and_replicate_with_response(
        &self,
        leader: NodeId,
        request: types::Request,
        label: &str,
        scope: Option<&[NodeId]>,
    ) -> ClientWriteResponse<TypeConfig> {
        let request_label = describe_request(&request);
        tracing::info!(leader, label, request = request_label, "client write");
        let response = self.write_only(leader, request).await;
        let index = response.log_id.index;
        self.wait_for_applied_on(index, scope, label).await;
        tracing::debug!(
            leader,
            index,
            request = request_label,
            "replication complete"
        );
        response
    }

    async fn write_only(
        &self,
        leader: NodeId,
        request: types::Request,
    ) -> ClientWriteResponse<TypeConfig> {
        let node = self.nodes.get(&leader).expect("leader");
        tracing::debug!(
            leader,
            request = describe_request(&request),
            "sending client write"
        );
        node.raft()
            .client_write(request)
            .await
            .expect("client write succeeds")
    }

    async fn wait_for_applied_on(&self, index: u64, scope: Option<&[NodeId]>, label: &str) {
        let targets: Vec<_> = scope
            .map(|ids| ids.to_vec())
            .unwrap_or_else(|| self.members.clone());
        for id in targets {
            let node = self.nodes.get(&id).expect("node");
            tracing::debug!(node = id, index, label, "waiting for application");
            node.raft()
                .wait(Some(Duration::from_millis(3_000)))
                .applied_index(Some(index), format!("{label} on node {id}"))
                .await
                .unwrap_or_else(|e| panic!("node {} failed to apply {}: {:?}", id, index, e));
            tracing::debug!(node = id, index, label, "application confirmed");
        }
    }

    async fn assert_value(&self, key: &str, expected: Option<&str>) {
        for (&id, node) in &self.nodes {
            let value = node
                .state_machine()
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
                tracing::warn!(a = node, b = other, "isolating link");
                self.router.isolate(node, other).await;
            }
        }
    }

    async fn heal_all(&self) {
        tracing::info!("healing all links");
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
        tracing::info!(node, "registering node in router");
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

fn describe_request(request: &types::Request) -> &'static str {
    match request {
        types::Request::Set { .. } => "set",
        types::Request::Delete { .. } => "delete",
        types::Request::Txn { .. } => "txn",
        types::Request::LeaseGrant { .. } => "lease_grant",
        types::Request::LeaseAttach { .. } => "lease_attach",
        types::Request::LeaseKeepAlive { .. } => "lease_keepalive",
        types::Request::LeaseRevoke { .. } => "lease_revoke",
        types::Request::WatchRegister { .. } => "watch_register",
        types::Request::WatchCancel { .. } => "watch_cancel",
        types::Request::WatchFetch { .. } => "watch_fetch",
    }
}
