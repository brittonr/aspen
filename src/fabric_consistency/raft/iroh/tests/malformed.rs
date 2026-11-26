use super::*;

// r[verify molten.consensus.long_running_service_bounds]
#[tokio::test]
async fn listener_pump_drops_an_undecodable_frame_and_delivers_the_next_one() {
    let group = super::super::tests::active_group();
    let state = super::super::tests::started_state(&group, super::super::tests::NODE_A);
    let election = apply_replica_event(&state, ReplicaEvent::ElectionTimeout {
        timer_ref: state.active_election_timer_ref.clone(),
    })
    .expect("election");
    let envelope = super::super::tests::sent_envelope_to(&election, NODE_B);
    let listener = crate::fabric_transport::cross_process::tests::listener().await;
    let input = crate::fabric_transport::cross_process::tests::client_input_for(
        listener.handoff().clone(),
        super::super::tests::NODE_A,
    );
    let timeout = std::time::Duration::from_secs(crate::fabric_transport::cross_process::tests::TEST_TIMEOUT_SECONDS);
    let mut pump = ReplicaIngressPump::spawn(listener, ReplicaIngressConfig {
        session_ref: input.session_ref.clone(),
        accept_timeout: timeout,
        event_capacity: INGRESS_TEST_CAPACITY,
        delivery_limit: INGRESS_TEST_DELIVERY_LIMIT,
        delivery_window: INGRESS_TEST_WINDOW,
    })
    .expect("pump");
    let malformed =
        crate::fabric_transport::exchange_frame(input.clone(), b"not-a-canonical-raft-frame", timeout).await;
    assert!(malformed.is_err(), "an undecodable frame is never acknowledged");
    let mut transport = ReplicaTransportPort::new(
        super::super::tests::test_ref("malformed-then-valid"),
        [(NODE_B.to_owned(), input)].into(),
        timeout,
    )
    .expect("transport");
    let (sent, received) = tokio::join!(transport.send(&envelope), pump.next());
    assert_eq!(
        sent.expect("valid frame delivered"),
        received.expect("valid frame").transport_evidence.acknowledgement_ref
    );
    assert_eq!(pump.dropped_frames(), 1);
    let listener = pump.shutdown().await.expect("ingress still running");
    listener
        .drain_and_close(crate::fabric_transport::ListenerDrainReason::OperatorRequest)
        .await
        .expect("cleanup");
}
