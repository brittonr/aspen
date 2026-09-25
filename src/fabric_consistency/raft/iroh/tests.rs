use super::*;

pub(super) const NODE_B: &str = "node-b";
pub(super) const POSITIVE_TIMEOUT_SECONDS: u64 = 1;
const INGRESS_TEST_CAPACITY: usize = 4;
const INGRESS_TEST_DELIVERY_LIMIT: u64 = 4;

// r[verify molten.fabric_consistency.live_service_ports]
#[test]
fn raft_iroh_transport_denies_empty_peer_registry() {
    let error = IrohReplicaTransportPort::new(
        super::tests::test_ref("empty-Iroh-peer-registry"),
        std::collections::BTreeMap::new(),
        std::time::Duration::from_secs(POSITIVE_TIMEOUT_SECONDS),
    )
    .expect_err("empty peer registry must deny");
    assert!(error.to_string().contains("at least one admitted peer"));
}

// r[verify molten.fabric_consistency.live_service_ports]
#[test]
fn ingress_config_admits_bounded_values_and_denies_zero_capacity() {
    let valid = IrohReplicaIngressConfig {
        session_ref: super::tests::test_ref("ingress-session"),
        accept_timeout: std::time::Duration::from_secs(POSITIVE_TIMEOUT_SECONDS),
        event_capacity: INGRESS_TEST_CAPACITY,
        delivery_limit: INGRESS_TEST_DELIVERY_LIMIT,
    };
    assert!(super::iroh::validate_ingress_config(&valid).is_ok());
    let mut invalid = valid;
    invalid.event_capacity = 0;
    let error = super::iroh::validate_ingress_config(&invalid).expect_err("zero ingress capacity must deny");
    assert!(error.to_string().contains("capacity"));
}

// r[verify molten.fabric_consistency.live_service_ports]
#[tokio::test]
async fn ingress_shutdown_cancels_accept_and_returns_the_listener() {
    let listener = crate::fabric_transport::cross_process::tests::listener().await;
    let pump = IrohReplicaIngressPump::spawn(listener, IrohReplicaIngressConfig {
        session_ref: super::tests::test_ref("ingress-cancellation-session"),
        accept_timeout: std::time::Duration::from_secs(
            crate::fabric_transport::cross_process::tests::TEST_TIMEOUT_SECONDS,
        ),
        event_capacity: INGRESS_TEST_CAPACITY,
        delivery_limit: INGRESS_TEST_DELIVERY_LIMIT,
    })
    .expect("ingress pump");
    let listener = pump.shutdown().await.expect("cancelled ingress listener");
    listener
        .drain_and_close(crate::fabric_transport::ListenerDrainReason::OperatorRequest)
        .await
        .expect("ingress listener cleanup");
}

// r[verify molten.fabric_consistency.live_service_ports]
// r[verify molten.fabric_consistency.live_raft]
#[tokio::test]
async fn canonical_raft_envelope_crosses_admitted_iroh_listener() {
    let group = super::tests::active_group();
    let node_a = super::tests::started_state(&group, super::tests::NODE_A);
    let election = apply_replica_event(&node_a, ReplicaEvent::ElectionTimeout {
        timer_ref: node_a.active_election_timer_ref.clone(),
    })
    .expect("election transition");
    let first_envelope = super::tests::sent_envelope_to(&election, NODE_B);
    let second_election = apply_replica_event(&election.next, ReplicaEvent::ElectionTimeout {
        timer_ref: election.next.active_election_timer_ref.clone(),
    })
    .expect("second election transition");
    let second_envelope = super::tests::sent_envelope_to(&second_election, NODE_B);

    let mut listener = crate::fabric_transport::cross_process::tests::listener().await;
    let endpoint = listener.handoff().clone();
    let input = crate::fabric_transport::cross_process::tests::client_input(endpoint);
    let session_ref = input.session_ref.clone();
    let mut peers = std::collections::BTreeMap::new();
    peers.insert(NODE_B.to_string(), input);
    let timeout = std::time::Duration::from_secs(crate::fabric_transport::cross_process::tests::TEST_TIMEOUT_SECONDS);
    let mut transport = IrohReplicaTransportPort::new(super::tests::test_ref("Raft-Iroh-protocol"), peers, timeout)
        .expect("Raft Iroh transport");
    let envelopes = [first_envelope, second_envelope];
    let mut request_refs = Vec::with_capacity(envelopes.len());
    for envelope in envelopes {
        let refs = replica_transport_refs(&envelope).expect("transport refs");
        let send = transport.send(&envelope);
        let receive = receive_replica_event(&mut listener, &session_ref, timeout);
        let (send, receive) = tokio::join!(send, receive);
        let acknowledgement_ref = send.expect("sent Raft frame");
        let received = receive.expect("received Raft frame");

        assert_eq!(acknowledgement_ref, received.transport_evidence.acknowledgement_ref);
        assert_eq!(received.transport_evidence.request_ref, refs.request_ref);
        assert_eq!(received.event, ReplicaEvent::Message { envelope });
        request_refs.push(refs.request_ref);
    }
    assert_ne!(request_refs[0], request_refs[1]);
    listener
        .drain_and_close(crate::fabric_transport::ListenerDrainReason::OperatorRequest)
        .await
        .expect("listener cleanup");
}

// r[verify molten.fabric_consistency.live_service_ports]
#[tokio::test]
async fn malformed_raft_payload_is_denied_before_transport_acknowledgement() {
    let mut listener = crate::fabric_transport::cross_process::tests::listener().await;
    let endpoint = listener.handoff().clone();
    let input = crate::fabric_transport::cross_process::tests::client_input(endpoint);
    let session_ref = input.session_ref.clone();
    let timeout = std::time::Duration::from_secs(crate::fabric_transport::cross_process::tests::TEST_TIMEOUT_SECONDS);

    let send = crate::fabric_transport::exchange_cross_process_frame(input, b"not-a-canonical-raft-frame", timeout);
    let receive = receive_replica_event(&mut listener, &session_ref, timeout);
    let (send, receive) = tokio::join!(send, receive);

    assert!(send.is_err());
    assert!(receive.is_err());
    assert_eq!(listener.state().active_sessions, 0);
    listener
        .drain_and_close(crate::fabric_transport::ListenerDrainReason::OperatorRequest)
        .await
        .expect("listener cleanup");
}
