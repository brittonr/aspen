use std::collections::BTreeMap;
use std::time::Duration;

use aspen::kv::types::{self, NodeId};
use aspen::{KvService, KvServiceBuilder};
use openraft::BasicNode;
use openraft::metrics::WaitError;
use tempfile::TempDir;

const LEADER_WAIT_MS: u64 = 2_000;

#[cfg_attr(not(madsim), tokio::test(flavor = "multi_thread", worker_threads = 2))]
async fn kv_service_single_node_smoke() {
    let dir = TempDir::new().expect("tempdir");
    let service = KvServiceBuilder::new(NodeId::from(1u64), dir.path())
        .start()
        .await
        .expect("service startup");

    initialize_single_node(&service).await;

    let client = service.client();
    client
        .set("smoke".into(), "ok".into())
        .await
        .expect("set succeeds");
    client
        .delete("missing".to_string())
        .await
        .expect("deleting absent keys is safe");

    let value = client.get("smoke").await.expect("get succeeds");
    assert_eq!(value, Some("ok".into()));

    let txn = client
        .txn(vec![
            types::TxnOp::Assert {
                key: "smoke".into(),
                equals: Some("ok".into()),
            },
            types::TxnOp::Set {
                key: "smoke".into(),
                value: "updated".into(),
            },
        ])
        .await
        .expect("txn succeeds");
    let committed = match txn.data {
        types::Response::Txn { committed, .. } => committed,
        other => panic!("unexpected txn response {other:?}"),
    };
    assert!(committed, "txn with satisfied assertion should commit");

    let value = client.get("smoke").await.expect("get succeeds");
    assert_eq!(value, Some("updated".into()));
}

async fn initialize_single_node(service: &KvService) {
    let mut membership = BTreeMap::new();
    membership.insert(1, BasicNode::new("single-node"));
    service
        .node()
        .raft
        .initialize(membership)
        .await
        .expect("cluster init");

    service
        .node()
        .raft
        .wait(Some(Duration::from_millis(LEADER_WAIT_MS)))
        .metrics(
            |m| m.current_leader == Some(1),
            "single-node leader election",
        )
        .await
        .unwrap_or_else(|err| match err {
            WaitError::Timeout(_, _) => panic!("timed out waiting for single-node leader"),
            other => panic!("wait error: {other:?}"),
        });
}
