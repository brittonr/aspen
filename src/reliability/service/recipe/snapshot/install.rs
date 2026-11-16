// Snapshot installation and transport-frame bounds share the recipe test fixtures.
fn start_replica(
    config: &super::super::Configuration,
    group: &crate::fabric_consistency::Group,
) -> crate::fabric_consistency::raft::ReplicaState {
    crate::fabric_consistency::raft::plan_live_replica_start(crate::fabric_consistency::raft::ReplicaStartInput {
        group: group.clone(),
        node_id: config.node_id.clone(),
        membership: config.membership.decode(),
        profile: config.replica.decode(),
        port_bindings: config
            .bindings
            .iter()
            .map(|binding| crate::fabric_consistency::raft::ReplicaPortBinding {
                port_id: binding.port_id.clone(),
                version: binding.version.clone(),
                implementation_profile: binding.implementation_profile.clone(),
                binding_ref: binding.binding_ref.clone(),
            })
            .collect(),
    })
    .unwrap()
    .state
}

fn sent_to(
    transition: &crate::fabric_consistency::raft::ReplicaTransition,
    recipient: &str,
) -> crate::fabric_consistency::raft::ReplicaMessageEnvelope {
    transition
        .effects
        .iter()
        .find_map(|effect| match effect {
            crate::fabric_consistency::raft::ReplicaEffect::Send { envelope } if envelope.to == recipient => {
                Some(envelope.clone())
            }
            _ => None,
        })
        .expect("real transition sends to peer")
}

fn elected_pair(
    config: &super::super::Configuration,
    node_2: &super::super::Configuration,
    group: &crate::fabric_consistency::Group,
) -> (crate::fabric_consistency::raft::ReplicaState, crate::fabric_consistency::raft::ReplicaState) {
    let leader = start_replica(config, group);
    let follower = start_replica(node_2, group);
    let election = crate::fabric_consistency::raft::apply_replica_event(
        &leader,
        crate::fabric_consistency::raft::ReplicaEvent::ElectionTimeout {
            timer_ref: leader.active_election_timer_ref.clone(),
        },
    )
    .unwrap();
    let vote = crate::fabric_consistency::raft::apply_replica_event(
        &follower,
        crate::fabric_consistency::raft::ReplicaEvent::Message {
            envelope: sent_to(&election, "node-2"),
        },
    )
    .unwrap();
    let leader = crate::fabric_consistency::raft::apply_replica_event(
        &election.next,
        crate::fabric_consistency::raft::ReplicaEvent::Message {
            envelope: sent_to(&vote, "node-1"),
        },
    )
    .unwrap()
    .next;
    (leader, vote.next)
}

fn committed_snapshot(
    config: &super::super::Configuration,
    node_2: &super::super::Configuration,
    group: &crate::fabric_consistency::Group,
) -> (crate::fabric_consistency::raft::ReplicaState, std::collections::BTreeMap<String, u64>) {
    let (mut leader, mut follower) = elected_pair(config, node_2, group);
    let mut committed = std::collections::BTreeMap::new();
    let limit = config.replica.snapshot_threshold_entries;
    for position in 1..=config.replica.snapshot_threshold_entries {
        let request_ref = reference(&format!("distinct-request-{position}"));
        let proposal = crate::fabric_consistency::raft::apply_replica_event(
            &leader,
            crate::fabric_consistency::raft::ReplicaEvent::Propose {
                request_ref: request_ref.clone(),
                command_ref: reference("bounded-command"),
                command_schema_ref: config.registry.command_schema_ref.clone(),
            },
        )
        .unwrap();
        let append = crate::fabric_consistency::raft::apply_replica_event(
            &follower,
            crate::fabric_consistency::raft::ReplicaEvent::Message {
                envelope: sent_to(&proposal, "node-2"),
            },
        )
        .unwrap();
        leader = crate::fabric_consistency::raft::apply_replica_event(
            &proposal.next,
            crate::fabric_consistency::raft::ReplicaEvent::Message {
                envelope: sent_to(&append, "node-1"),
            },
        )
        .unwrap()
        .next;
        follower = append.next;
        if committed.len() < limit {
            committed.insert(request_ref, leader.last_applied);
        } else {
            panic!("snapshot fixture exceeds the admitted request bound");
        }
    }
    assert!(crate::fabric_consistency::raft::is_snapshot_due(&leader));
    committed.retain(|_, index| *index > leader.last_applied - config.replica.request_retention_entries);
    assert_eq!(u64::try_from(committed.len()).unwrap(), config.replica.request_retention_entries);
    assert_eq!(leader.completed_requests, committed);
    let leader = crate::fabric_consistency::raft::apply_replica_event(
        &leader,
        crate::fabric_consistency::raft::ReplicaEvent::CreateSnapshot {
            application_state_ref: reference("registry-state"),
        },
    )
    .unwrap()
    .next;
    (leader, committed)
}

fn admit_frame(config: &super::super::Configuration, encoded: &crate::fabric_consistency::raft::ReplicaMessage) -> u64 {
    let profile = config.transport.profile.clone().into_profile().unwrap();
    let protocol = config.transport.protocol.clone().into_descriptor(&profile).unwrap();
    let peer = &config.transport.peers["node-3"];
    let endpoint = crate::fabric_transport::cross_process::parse_endpoint(
        &crate::preserves_rail::parse_canonical_bytes(&peer.endpoint).unwrap(),
    )
    .unwrap();
    let dial = crate::fabric_transport::admit_endpoint_import(
        &profile.profile,
        &protocol,
        &endpoint.descriptor,
        &peer.expected.clone().into_binding().unwrap(),
        config.transport.admission.clone().into_state().unwrap(),
    )
    .unwrap();
    let bytes = u64::try_from(encoded.bytes.len()).unwrap();
    assert!(bytes <= dial.resources.max_frame_bytes);
    let session = crate::fabric_transport::plan_cross_process_session(
        &dial,
        &peer.session_ref,
        crate::fabric_transport::EndpointParticipantRole::Client,
    )
    .unwrap();
    let commands = [
        crate::fabric_transport::CrossProcessSessionCommand::BeginDial {
            observed_descriptor_ref: endpoint.descriptor_ref.clone(),
            callback_generation: protocol.generation,
        },
        crate::fabric_transport::CrossProcessSessionCommand::Established {
            observed_peer_context_ref: endpoint.descriptor.expected_peer_context_ref.clone(),
            callback_generation: protocol.generation,
        },
        crate::fabric_transport::CrossProcessSessionCommand::QueueFrame {
            payload_bytes: bytes,
            callback_generation: protocol.generation,
        },
        crate::fabric_transport::CrossProcessSessionCommand::FrameSubmitted {
            payload_bytes: bytes,
            callback_generation: protocol.generation,
        },
    ];
    commands
        .into_iter()
        .fold(session, |state, command| {
            crate::fabric_transport::apply_cross_process_session_command(&state, &command).unwrap().next
        })
        .inflight_bytes
}

// r[verify molten.consensus.bounded_request_dedup]
// r[verify molten.consensus.snapshot_threshold]
#[test]
fn selected_full_retention_snapshot_fits_admitted_protocol_frame_and_installs() {
    let input = input();
    let group = input.group.open(&input.mechanisms.registration_authority_ref).unwrap();
    let output = super::materialize(input).unwrap();
    let config = &output.configurations["node-1"];
    let fresh = start_replica(&output.configurations["node-3"], &group);
    let (leader, committed) = committed_snapshot(config, &output.configurations["node-2"], &group);
    let heartbeat = crate::fabric_consistency::raft::apply_replica_event(
        &leader,
        crate::fabric_consistency::raft::ReplicaEvent::HeartbeatTimeout,
    )
    .unwrap();
    let install = sent_to(&heartbeat, "node-3");
    let crate::fabric_consistency::raft::Message::InstallSnapshot { snapshot, .. } = &install.message else {
        panic!("lagging replica must receive the complete snapshot");
    };
    assert_eq!(snapshot.completed_requests, committed);
    let encoded = crate::fabric_consistency::raft::replica_message(&install).unwrap();
    let decoded = crate::fabric_consistency::raft::parse_replica_message(&encoded.bytes).unwrap();
    assert_eq!(decoded, install);
    assert_eq!(admit_frame(config, &encoded), u64::try_from(encoded.bytes.len()).unwrap());
    let installed = crate::fabric_consistency::raft::apply_replica_event(
        &fresh,
        crate::fabric_consistency::raft::ReplicaEvent::Message { envelope: decoded },
    )
    .unwrap();
    assert_eq!(installed.next.completed_requests, committed);
    assert_eq!(installed.next.last_applied, leader.last_applied);
}
