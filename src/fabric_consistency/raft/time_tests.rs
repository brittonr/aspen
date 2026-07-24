use std::time::Duration;

use super::tests::test_ref;
use super::*;
use crate::error::Result;
use crate::fabric_time::CryptographicEntropySource;
use crate::fabric_time::tests::live_profile;

const GENERATION: u64 = 1;
const HEARTBEAT_TICKS: u64 = 1;
const ELECTION_MIN_TICKS: u64 = 2;
const ELECTION_MAX_TICKS: u64 = 3;
const TICK_MILLISECONDS: u64 = 1;
const EVENT_TIMEOUT_MILLISECONDS: u64 = 200;
const EVENT_QUIET_MILLISECONDS: u64 = 20;
const ENTROPY_BINDING_LABEL: &str = "admitted-entropy-binding";

#[derive(Debug, Default)]
struct FixedEntropySource;

impl CryptographicEntropySource for FixedEntropySource {
    fn source_id(&self) -> &'static str {
        "fixed-live-raft-test-source"
    }

    fn fill_secret(&mut self, output: &mut [u8]) -> Result<()> {
        output.fill(0);
        Ok(())
    }
}

// r[verify molten.fabric_consistency.live_service_ports]
#[tokio::test]
async fn tokio_replica_time_port_delivers_entropy_bound_election_and_heartbeat_events() {
    let (mut port, mut receiver) =
        time_port(HEARTBEAT_TICKS, ELECTION_MIN_TICKS, ELECTION_MAX_TICKS).expect("Tokio replica time port");

    let timer_ref = test_ref("election-timer-token");
    let election_evidence = port.arm_election_timer(&timer_ref).expect("election timer");
    assert!(election_evidence.starts_with("blake3:"));
    let election_event = tokio::time::timeout(Duration::from_millis(EVENT_TIMEOUT_MILLISECONDS), receiver.recv())
        .await
        .expect("bounded election wait")
        .expect("election event");
    let ReplicaEvent::ElectionTimeout {
        timer_ref: delivered_timer_ref,
    } = election_event
    else {
        panic!("expected election timeout");
    };
    assert_eq!(delivered_timer_ref, timer_ref);

    let heartbeat_evidence = port.arm_heartbeat_timer().expect("heartbeat timer");
    assert!(heartbeat_evidence.starts_with("blake3:"));
    let heartbeat_event = tokio::time::timeout(Duration::from_millis(EVENT_TIMEOUT_MILLISECONDS), receiver.recv())
        .await
        .expect("bounded heartbeat wait")
        .expect("heartbeat event");
    assert_eq!(heartbeat_event, ReplicaEvent::HeartbeatTimeout);
}

// r[verify molten.fabric_consistency.live_service_ports]
#[tokio::test]
async fn rearming_election_timer_cancels_the_superseded_delivery() {
    let (mut port, mut receiver) =
        time_port(HEARTBEAT_TICKS, ELECTION_MIN_TICKS, ELECTION_MAX_TICKS).expect("Tokio replica time port");

    port.arm_election_timer(&test_ref("superseded-timer-token")).expect("first timer");
    let replacement_timer_ref = test_ref("replacement-timer-token");
    port.arm_election_timer(&replacement_timer_ref).expect("replacement timer");
    let delivered = tokio::time::timeout(Duration::from_millis(EVENT_TIMEOUT_MILLISECONDS), receiver.recv())
        .await
        .expect("bounded replacement wait")
        .expect("replacement event");
    assert_eq!(delivered, ReplicaEvent::ElectionTimeout {
        timer_ref: replacement_timer_ref,
    });
    tokio::time::sleep(Duration::from_millis(EVENT_QUIET_MILLISECONDS)).await;
    assert!(receiver.try_recv().is_err());
}

// r[verify molten.fabric_consistency.live_service_ports]
#[test]
fn tokio_replica_time_port_denies_unsafe_timer_bounds_before_activation() {
    let result = time_port(HEARTBEAT_TICKS, HEARTBEAT_TICKS, ELECTION_MAX_TICKS);
    let error = result.err().expect("unsafe bounds must deny");
    assert!(error.to_string().contains("heartbeat < election minimum"));
}

// r[verify molten.fabric_consistency.live_service_ports]
#[tokio::test]
async fn tokio_replica_time_port_denies_malformed_timer_token_without_delivery() {
    let (mut port, mut receiver) =
        time_port(HEARTBEAT_TICKS, ELECTION_MIN_TICKS, ELECTION_MAX_TICKS).expect("Tokio replica time port");
    let error = port.arm_election_timer("malformed-timer-token").expect_err("malformed timer token must deny");
    assert!(error.to_string().contains("content ref"), "unexpected error: {error}");
    assert!(receiver.try_recv().is_err());
}

fn time_port(
    heartbeat_ticks: u64,
    election_min_ticks: u64,
    election_max_ticks: u64,
) -> Result<(TokioReplicaTimePort<FixedEntropySource>, tokio::sync::mpsc::UnboundedReceiver<ReplicaEvent>)> {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let profile = live_profile().profile;
    let config = TokioReplicaTimeConfig {
        profile,
        generation: GENERATION,
        service_id: "raft-service".to_string(),
        capability_ref: test_ref("raft-time-capability"),
        entropy_binding_ref: entropy_binding_ref(),
        tick_duration: Duration::from_millis(TICK_MILLISECONDS),
        heartbeat_ticks,
        election_min_ticks,
        election_max_ticks,
    };
    Ok((TokioReplicaTimePort::new(config, FixedEntropySource, sender)?, receiver))
}

fn entropy_binding_ref() -> String {
    test_ref(ENTROPY_BINDING_LABEL)
}
