use super::*;

const VOTERS: [&str; 3] = [
    super::super::tests::NODE_A,
    super::super::tests::NODE_B,
    super::super::tests::NODE_C,
];

/// Three voters whose listeners each require a distinct peer context, each admitting the other
/// voters' descriptor identities under its own context, as reliability `Transport::open` does.
async fn distinct_context_listeners() -> (
    std::collections::BTreeMap<String, crate::fabric_transport::Listener>,
    std::collections::BTreeMap<String, crate::fabric_transport::Endpoint>,
) {
    let mut listeners = std::collections::BTreeMap::new();
    for node in VOTERS {
        let context = super::super::tests::test_ref(&format!("distinct-peer-context-{node}"));
        let listener = crate::fabric_transport::cross_process::tests::listener_for_peer(node, context).await;
        listeners.insert(node.to_string(), listener);
    }
    let endpoints = listeners
        .iter()
        .map(|(node, listener)| (node.clone(), listener.handoff().clone()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let contexts = endpoints
        .values()
        .map(|endpoint| endpoint.descriptor.expected_peer_context_ref.clone())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(contexts.len(), VOTERS.len(), "every voter requires its own peer context");
    for (node, listener) in &mut listeners {
        let local_context = listener.handoff().descriptor.expected_peer_context_ref.clone();
        for (peer, endpoint) in endpoints.iter().filter(|(peer, _endpoint)| *peer != node) {
            listener
                .admit_peer(peer.clone(), crate::fabric_transport::AdmittedPeer::for_endpoint(endpoint, &local_context))
                .expect("admitted voter");
        }
    }
    (listeners, endpoints)
}

// r[verify molten.fabric_transport.raft_peer_auth_and_isolation]
#[tokio::test]
async fn voters_with_distinct_peer_contexts_elect_a_leader_and_refuse_an_unadmitted_identity() {
    let group = super::super::tests::active_group();
    let (mut listeners, endpoints) = distinct_context_listeners().await;
    let [listener_a, listener_b, listener_c] = VOTERS.map(|node| listeners.remove(node).expect("voter listener"));
    let mut node_a = setup::build_node(&group, VOTERS[0], listener_a, &endpoints).await.expect("node A");
    let mut node_b = setup::build_node(&group, VOTERS[1], listener_b, &endpoints).await.expect("node B");
    let mut node_c = setup::build_node(&group, VOTERS[2], listener_c, &endpoints).await.expect("node C");
    workflow::elect_node_a(&mut node_a, &mut node_b, &mut node_c)
        .await
        .expect("election across distinct contexts");
    assert_eq!(node_a.service.state().role, ReplicaRole::Leader);
    assert_eq!(node_a.service.state().current_term, LIVE_TERM);

    // A key that is no admitted voter's descriptor identity is refused, whatever it sends.
    let outsider = crate::fabric_transport::cross_process::tests::client_input_for(
        endpoints[super::super::tests::NODE_A].clone(),
        crate::fabric_transport::cross_process::tests::CLIENT_PEER_ID,
    );
    let session_a = node_a.session_ref.clone();
    let (sent, received) = tokio::join!(
        crate::fabric_transport::exchange_frame(outsider, b"unadmitted-voter-frame", timeout()),
        receive_replica_event(&mut node_a.listener, &session_a, timeout()),
    );
    assert!(sent.is_err(), "an unadmitted identity obtains no delivered-frame evidence");
    let refused = received.expect_err("an unadmitted identity is refused");
    assert!(refused.to_string().contains("unadmitted"));

    setup::close_node(node_a).await;
    setup::close_node(node_b).await;
    setup::close_node(node_c).await;
}
