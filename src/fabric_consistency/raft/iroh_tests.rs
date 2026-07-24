use std::collections::BTreeMap;
use std::time::Duration;

use super::tests::NODE_A;
use super::tests::active_group;
use super::tests::sent_envelope_to;
use super::tests::started_state;
use super::tests::test_ref;
use super::*;
use crate::fabric_transport::ListenerDrainReason;
use crate::fabric_transport::cross_process::tests::TEST_TIMEOUT_SECONDS;
use crate::fabric_transport::cross_process::tests::client_input;
use crate::fabric_transport::cross_process::tests::listener;

const NODE_B: &str = "node-b";
const POSITIVE_TIMEOUT_SECONDS: u64 = 1;

// r[verify molten.fabric_consistency.live_service_ports]
#[test]
fn raft_iroh_transport_denies_empty_peer_registry() {
    let error = IrohReplicaTransportPort::new(BTreeMap::new(), Duration::from_secs(POSITIVE_TIMEOUT_SECONDS))
        .expect_err("empty peer registry must deny");
    assert!(error.to_string().contains("at least one admitted peer"));
}

// r[verify molten.fabric_consistency.live_service_ports]
// r[verify molten.fabric_consistency.live_raft]
#[tokio::test]
async fn canonical_raft_envelope_crosses_admitted_iroh_listener() {
    let group = active_group();
    let node_a = started_state(&group, NODE_A);
    let election = apply_replica_event(&node_a, ReplicaEvent::ElectionTimeout {
        entropy_ref: test_ref("iroh-election-entropy"),
    })
    .expect("election transition");
    let envelope = sent_envelope_to(&election, NODE_B);

    let mut listener = listener().await;
    let endpoint = listener.handoff().clone();
    let mut peers = BTreeMap::new();
    peers.insert(NODE_B.to_string(), client_input(endpoint));
    let timeout = Duration::from_secs(TEST_TIMEOUT_SECONDS);
    let mut transport = IrohReplicaTransportPort::new(peers, timeout).expect("Raft Iroh transport");
    let refs = replica_transport_refs(&envelope).expect("transport refs");

    let send = transport.send(&envelope);
    let receive = receive_replica_event(&mut listener, &refs.session_ref, &refs.request_ref, timeout);
    let (send, receive) = tokio::join!(send, receive);
    let acknowledgement_ref = send.expect("sent Raft frame");
    let received = receive.expect("received Raft frame");

    assert_eq!(acknowledgement_ref, received.transport_evidence.acknowledgement_ref);
    assert_eq!(received.event, ReplicaEvent::Message { envelope });
    listener.drain_and_close(ListenerDrainReason::OperatorRequest).await.expect("listener cleanup");
}
