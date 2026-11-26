use super::*;

fn node_a_envelope() -> ReplicaMessageEnvelope {
    let group = super::super::tests::active_group();
    let state = super::super::tests::started_state(&group, super::super::tests::NODE_A);
    let election = apply_replica_event(&state, ReplicaEvent::ElectionTimeout {
        timer_ref: state.active_election_timer_ref.clone(),
    })
    .expect("election");
    super::super::tests::sent_envelope_to(&election, NODE_B)
}

fn timeout() -> std::time::Duration {
    std::time::Duration::from_secs(crate::fabric_transport::cross_process::tests::TEST_TIMEOUT_SECONDS)
}

fn pump(
    listener: crate::fabric_transport::Listener,
    session_ref: String,
    accept_timeout: std::time::Duration,
) -> ReplicaIngressPump {
    ReplicaIngressPump::spawn(listener, ReplicaIngressConfig {
        session_ref,
        accept_timeout,
        event_capacity: INGRESS_TEST_CAPACITY,
        delivery_limit: INGRESS_TEST_DELIVERY_LIMIT,
        delivery_window: INGRESS_TEST_WINDOW,
    })
    .expect("ingress pump")
}

/// Node A's envelope sent through a transport that dials as node A.
async fn deliver_as_node_a(
    pump: &mut ReplicaIngressPump,
    endpoint: crate::fabric_transport::Endpoint,
    envelope: &ReplicaMessageEnvelope,
) {
    let input = crate::fabric_transport::cross_process::tests::client_input_for(endpoint, super::super::tests::NODE_A);
    let mut transport = ReplicaTransportPort::new(
        super::super::tests::test_ref("authenticated-ingress-protocol"),
        [(NODE_B.to_owned(), input)].into(),
        timeout(),
    )
    .expect("transport");
    let (sent, received) = tokio::join!(transport.send(envelope), pump.next());
    let received = received.expect("the authenticated voter's frame is delivered");
    assert_eq!(sent.expect("acknowledged frame"), received.transport_evidence.acknowledgement_ref);
    assert_eq!(received.event, ReplicaEvent::Message {
        envelope: envelope.clone(),
    });
}

async fn close(pump: ReplicaIngressPump) {
    let listener = pump.shutdown().await.expect("ingress survived every rejected connection");
    assert_eq!(listener.state().active_sessions, 0);
    listener
        .drain_and_close(crate::fabric_transport::ListenerDrainReason::OperatorRequest)
        .await
        .expect("listener cleanup");
}

// r[verify molten.fabric_transport.raft_peer_auth_and_isolation]
#[tokio::test]
async fn spoofed_sender_is_denied_before_acknowledgement_and_the_matching_voter_is_delivered() {
    let envelope = node_a_envelope();
    let message = replica_message(&envelope).expect("canonical message");
    let listener = crate::fabric_transport::cross_process::tests::listener().await;
    let endpoint = listener.handoff().clone();
    // Node C is an admitted voter, but it authenticates as node C while naming node A as sender.
    let mut spoofer =
        crate::fabric_transport::cross_process::tests::client_input_for(endpoint.clone(), super::super::tests::NODE_C);
    spoofer.request_ref.clone_from(&message.envelope_ref);
    let mut pump = pump(listener, spoofer.session_ref.clone(), timeout());

    let spoofed = crate::fabric_transport::exchange_frame(spoofer, &message.bytes, timeout()).await;
    assert!(spoofed.is_err(), "a voter speaking for another voter is never acknowledged");
    deliver_as_node_a(&mut pump, endpoint, &envelope).await;
    assert_eq!(pump.dropped_frames(), 1);
    assert_eq!(pump.authentication_failures(), 1);
    close(pump).await;
}

// r[verify molten.fabric_transport.raft_peer_auth_and_isolation]
#[tokio::test]
async fn unadmitted_endpoint_is_refused_after_its_handshake_and_ingress_continues() {
    let envelope = node_a_envelope();
    let message = replica_message(&envelope).expect("canonical message");
    let admitted = crate::fabric_transport::cross_process::tests::admitted_test_peers(&[super::super::tests::NODE_A]);
    let listener = crate::fabric_transport::cross_process::tests::listener_admitting(17, admitted).await;
    let endpoint = listener.handoff().clone();
    // The generic test client holds a valid key and the exact descriptor, but no admission.
    let mut outsider = crate::fabric_transport::cross_process::tests::client_input(endpoint.clone());
    outsider.request_ref.clone_from(&message.envelope_ref);
    let mut pump = pump(listener, outsider.session_ref.clone(), timeout());

    let refused = crate::fabric_transport::exchange_frame(outsider, &message.bytes, timeout()).await;
    assert!(refused.is_err(), "an unadmitted endpoint obtains no delivered-frame evidence");
    deliver_as_node_a(&mut pump, endpoint, &envelope).await;
    assert_eq!(pump.dropped_frames(), 1);
    assert_eq!(pump.authentication_failures(), 1);
    close(pump).await;
}

// r[verify molten.fabric_transport.raft_peer_auth_and_isolation]
#[tokio::test]
async fn failed_handshake_and_stalled_session_drop_only_their_connections() {
    let envelope = node_a_envelope();
    let listener = crate::fabric_transport::cross_process::tests::listener().await;
    let endpoint = listener.handoff().clone();
    let accept_timeout = std::time::Duration::from_secs(POSITIVE_TIMEOUT_SECONDS);
    let session_ref = crate::fabric_transport::cross_process::tests::client_input(endpoint.clone()).session_ref;
    let mut pump = pump(listener, session_ref, accept_timeout);

    // A handshake offering an unregistered ALPN fails during the listener's handshake.
    let wrong_protocol =
        crate::fabric_transport::cross_process::tests::client_input_for(endpoint.clone(), super::super::tests::NODE_B);
    let failed = crate::fabric_transport::cross_process::connect_without_exchange(
        wrong_protocol,
        b"molten/unregistered-protocol/1",
        timeout(),
    )
    .await;
    assert!(failed.is_err(), "the handshake for an unregistered protocol fails");

    // An admitted voter completes its handshake and then stalls without opening a stream.
    let stalled_input =
        crate::fabric_transport::cross_process::tests::client_input_for(endpoint.clone(), super::super::tests::NODE_C);
    let alpn = stalled_input.protocol.alpn.as_bytes().to_vec();
    let (_stalled_endpoint, stalled) =
        crate::fabric_transport::cross_process::connect_without_exchange(stalled_input, &alpn, timeout())
            .await
            .expect("stalled peer completes its handshake");

    deliver_as_node_a(&mut pump, endpoint, &envelope).await;
    assert_eq!(pump.dropped_frames(), 2, "the failed handshake and the stalled session were each dropped");
    assert_eq!(pump.authentication_failures(), 0);
    drop(stalled);
    close(pump).await;
}
