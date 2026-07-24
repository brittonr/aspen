use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::time::Duration;

use super::tests::NODE_A;
use super::tests::active_group;
use super::tests::sent_envelope_to;
use super::tests::started_state;
use super::tests::test_ref;
use super::*;
use crate::error::Result;
use crate::fabric_durability::DurableAdapterKind;
use crate::fabric_durability::RedbDurableStateAdapter;
use crate::fabric_durability::tests::descriptor;
use crate::fabric_durability::tests::profile;
use crate::fabric_time::tests::live_profile;
use crate::fabric_transport::ListenerDrainReason;
use crate::fabric_transport::cross_process::tests::TEST_TIMEOUT_SECONDS;
use crate::fabric_transport::cross_process::tests::client_input;
use crate::fabric_transport::cross_process::tests::listener;
use crate::fabric_transport::exchange_cross_process_frame;

const NODE_B: &str = "node-b";
const POSITIVE_TIMEOUT_SECONDS: u64 = 1;
const ASSEMBLY_GENERATION: u64 = 1;
const ASSEMBLY_HEARTBEAT_TICKS: u64 = 1;
const ASSEMBLY_ELECTION_MIN_TICKS: u64 = 2;
const ASSEMBLY_ELECTION_MAX_TICKS: u64 = 3;
const ASSEMBLY_TICK_MILLISECONDS: u64 = 1;
const ASSEMBLY_FABRIC_BINDING_COUNT: usize = 7;
const ASSEMBLY_STARTUP_OBSERVATIONS: usize = 2;
const INGRESS_TEST_CAPACITY: usize = 4;
const INGRESS_TEST_DELIVERY_LIMIT: u64 = 4;

#[derive(Debug, Default)]
struct AssemblyApplicationHandler;

impl CommittedBatchHandler for AssemblyApplicationHandler {
    fn restore_snapshot(&mut self, _snapshot: &ApplicationSnapshotRestore) -> Result<String> {
        Ok(test_ref("assembled-application-snapshot-evidence"))
    }

    fn apply_batch(&mut self, _commands: &[ApplicationCommand]) -> Result<String> {
        Ok(test_ref("assembled-application-evidence"))
    }
}

// r[verify molten.fabric_consistency.live_service_ports]
#[test]
fn raft_iroh_transport_denies_empty_peer_registry() {
    let error = IrohReplicaTransportPort::new(
        test_ref("empty-Iroh-peer-registry"),
        BTreeMap::new(),
        Duration::from_secs(POSITIVE_TIMEOUT_SECONDS),
    )
    .expect_err("empty peer registry must deny");
    assert!(error.to_string().contains("at least one admitted peer"));
}

// r[verify molten.fabric_consistency.live_service_ports]
#[test]
fn ingress_config_admits_bounded_values_and_denies_zero_capacity() {
    let valid = IrohReplicaIngressConfig {
        session_ref: test_ref("ingress-session"),
        accept_timeout: Duration::from_secs(POSITIVE_TIMEOUT_SECONDS),
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
    let listener = listener().await;
    let pump = IrohReplicaIngressPump::spawn(listener, IrohReplicaIngressConfig {
        session_ref: test_ref("ingress-cancellation-session"),
        accept_timeout: Duration::from_secs(TEST_TIMEOUT_SECONDS),
        event_capacity: INGRESS_TEST_CAPACITY,
        delivery_limit: INGRESS_TEST_DELIVERY_LIMIT,
    })
    .expect("ingress pump");
    let listener = pump.shutdown().await.expect("cancelled ingress listener");
    listener
        .drain_and_close(ListenerDrainReason::OperatorRequest)
        .await
        .expect("ingress listener cleanup");
}

// r[verify molten.fabric_consistency.live_service_ports]
#[tokio::test]
async fn concrete_port_assembly_denies_substitution_then_executes_bound_startup() {
    let group = active_group();
    let mut replica_state = started_state(&group, NODE_A);
    assert_eq!(replica_state.profile.service_generation, ASSEMBLY_GENERATION);
    let listener = listener().await;
    let endpoint = listener.handoff().clone();
    let protocol_ref = test_ref("assembled-Iroh-protocol");
    let durable_log_ref = test_ref("assembled-durable-log");
    let snapshot_store_ref = test_ref("assembled-snapshot-store");
    let entropy_profile_ref = test_ref("assembled-entropy-profile");
    let supervision_ref = test_ref("assembled-supervision");
    let service_id = group.service_id.clone();
    let application_manifest_ref = group.application_manifest_ref.clone();
    let mut peers = BTreeMap::new();
    peers.insert(NODE_B.to_string(), client_input(endpoint));
    let transport =
        IrohReplicaTransportPort::new(protocol_ref.clone(), peers, Duration::from_secs(POSITIVE_TIMEOUT_SECONDS))
            .expect("assembled Iroh transport");

    let root = crate::test_support::process_workspace("assembled-live-raft-redb").expect("workspace");
    let redb = RedbDurableStateAdapter::open(&root, profile(DurableAdapterKind::LiveRedb), descriptor())
        .expect("assembled Redb adapter");
    let durability = RedbReplicaDurabilityPort::new(redb, durable_log_ref.clone(), snapshot_store_ref.clone())
        .expect("assembled durability port");

    let canonical_time = live_profile().profile;
    let timer_profile_ref = canonical_time.profile_ref.clone();
    let (event_sender, event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let time = TokioReplicaTimePort::new_operating_system(
        TokioReplicaTimeConfig {
            profile: canonical_time,
            generation: ASSEMBLY_GENERATION,
            service_id: service_id.clone(),
            capability_ref: test_ref("assembled-time-capability"),
            entropy_binding_ref: entropy_profile_ref.clone(),
            tick_duration: Duration::from_millis(ASSEMBLY_TICK_MILLISECONDS),
            heartbeat_ticks: ASSEMBLY_HEARTBEAT_TICKS,
            election_min_ticks: ASSEMBLY_ELECTION_MIN_TICKS,
            election_max_ticks: ASSEMBLY_ELECTION_MAX_TICKS,
        },
        event_sender,
    )
    .expect("assembled time port");
    let application = AdmittedReplicaApplicationPort::new(
        ReplicaApplicationConfig {
            group_binding_ref: group.binding_ref.clone(),
            application_manifest_ref: application_manifest_ref.clone(),
            handler_ref: test_ref("assembled-application-handler"),
            command_schema_refs: BTreeSet::from([test_ref("assembled-command-schema")]),
            initial_applied_index: INITIAL_COMMIT_INDEX,
        },
        AssemblyApplicationHandler,
    )
    .expect("assembled application port");
    let (control_sender, _control_receiver) = tokio::sync::mpsc::unbounded_channel();
    let control = ChannelReplicaControlPort::new(
        ReplicaControlConfig {
            service_id: service_id.clone(),
            service_generation: ASSEMBLY_GENERATION,
            supervision_ref: supervision_ref.clone(),
        },
        control_sender,
    )
    .expect("assembled control port");

    replica_state.profile.protocol_ref.clone_from(&protocol_ref);
    replica_state.profile.durable_log_ref.clone_from(&durable_log_ref);
    replica_state.profile.snapshot_store_ref.clone_from(&snapshot_store_ref);
    replica_state.profile.timer_profile_ref.clone_from(&timer_profile_ref);
    replica_state.profile.entropy_profile_ref.clone_from(&entropy_profile_ref);
    replica_state.profile.supervision_ref.clone_from(&supervision_ref);
    let fabric_binding_refs = (0..ASSEMBLY_FABRIC_BINDING_COUNT)
        .map(|index| test_ref(&format!("assembled-fabric-binding-{index}")))
        .collect::<Vec<_>>();
    let identity = ReplicaRuntimePortIdentity {
        service_id: service_id.clone(),
        service_generation: ASSEMBLY_GENERATION,
        group_binding_ref: group.binding_ref.clone(),
        application_manifest_ref: application_manifest_ref.clone(),
        protocol_ref,
        durable_log_ref,
        snapshot_store_ref,
        timer_profile_ref,
        entropy_profile_ref,
        membership_ref: replica_state.membership.membership_ref.clone(),
        placement_ref: replica_state.profile.placement_ref.clone(),
        fencing_ref: replica_state.profile.fencing_ref.clone(),
        supervision_ref,
        resource_profile_ref: replica_state.profile.resource_profile_ref.clone(),
        fabric_binding_refs: fabric_binding_refs.clone(),
    };

    let mut mismatched = identity.clone();
    mismatched.protocol_ref = test_ref("substituted-assembled-protocol");
    let error =
        validate_concrete_replica_port_identity(&mismatched, &durability, &transport, &time, &application, &control)
            .expect_err("concrete protocol substitution must deny");
    assert!(error.to_string().contains("concrete adapter identity"));

    let startup_timer_ref = replica_state.active_election_timer_ref.clone();
    let plan = ReplicaStartPlan {
        state: replica_state,
        service_id,
        application_manifest_ref,
        initial_effects: vec![
            ReplicaEffect::PersistHardState {
                term: INITIAL_TERM,
                voted_for: None,
            },
            ReplicaEffect::ArmElectionTimer {
                timer_ref: startup_timer_ref,
            },
        ],
        port_binding_refs: fabric_binding_refs,
        production_admitted: false,
    };
    let bundle = assemble_scoped_concrete_replica_ports(identity, durability, transport, time, application, control)
        .expect("concrete port assembly");
    let service = ScopedLiveReplicaService::start(plan, bundle, event_receiver)
        .await
        .expect("concrete scoped service startup");
    assert_eq!(service.startup_observations().len(), ASSEMBLY_STARTUP_OBSERVATIONS);
    assert!(!service.production_admitted());
    assert!(!service.ports().durability.adapter().state().durable_log.is_empty());
    drop(service);
    listener.drain_and_close(ListenerDrainReason::OperatorRequest).await.expect("listener cleanup");
}

// r[verify molten.fabric_consistency.live_service_ports]
// r[verify molten.fabric_consistency.live_raft]
#[tokio::test]
async fn canonical_raft_envelope_crosses_admitted_iroh_listener() {
    let group = active_group();
    let node_a = started_state(&group, NODE_A);
    let election = apply_replica_event(&node_a, ReplicaEvent::ElectionTimeout {
        timer_ref: node_a.active_election_timer_ref.clone(),
    })
    .expect("election transition");
    let first_envelope = sent_envelope_to(&election, NODE_B);
    let second_election = apply_replica_event(&election.next, ReplicaEvent::ElectionTimeout {
        timer_ref: election.next.active_election_timer_ref.clone(),
    })
    .expect("second election transition");
    let second_envelope = sent_envelope_to(&second_election, NODE_B);

    let mut listener = listener().await;
    let endpoint = listener.handoff().clone();
    let input = client_input(endpoint);
    let session_ref = input.session_ref.clone();
    let mut peers = BTreeMap::new();
    peers.insert(NODE_B.to_string(), input);
    let timeout = Duration::from_secs(TEST_TIMEOUT_SECONDS);
    let mut transport =
        IrohReplicaTransportPort::new(test_ref("Raft-Iroh-protocol"), peers, timeout).expect("Raft Iroh transport");
    let mut request_refs = Vec::new();

    for envelope in [first_envelope, second_envelope] {
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
    listener.drain_and_close(ListenerDrainReason::OperatorRequest).await.expect("listener cleanup");
}

// r[verify molten.fabric_consistency.live_service_ports]
#[tokio::test]
async fn malformed_raft_payload_is_denied_before_transport_acknowledgement() {
    let mut listener = listener().await;
    let endpoint = listener.handoff().clone();
    let input = client_input(endpoint);
    let session_ref = input.session_ref.clone();
    let timeout = Duration::from_secs(TEST_TIMEOUT_SECONDS);

    let send = exchange_cross_process_frame(input, b"not-a-canonical-raft-frame", timeout);
    let receive = receive_replica_event(&mut listener, &session_ref, timeout);
    let (send, receive) = tokio::join!(send, receive);

    assert!(send.is_err());
    assert!(receive.is_err());
    assert_eq!(listener.state().active_sessions, 0);
    listener.drain_and_close(ListenerDrainReason::OperatorRequest).await.expect("listener cleanup");
}
