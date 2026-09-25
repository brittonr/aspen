
fn prepare_run_directory(run_directory: &std::path::Path, force: bool) -> crate::error::Result<()> {
    if run_directory.exists() {
        if !force {
            return Err(crate::error::MoltenError::invalid_harness(format!(
                "distinct-process run directory already exists: {}",
                run_directory.display()
            )));
        }
        std::fs::remove_dir_all(run_directory).map_err(crate::error::MoltenError::from)?;
    }
    std::fs::create_dir_all(run_directory).map_err(crate::error::MoltenError::from)
}

fn validate_child_directory(run_directory: &std::path::Path) -> crate::error::Result<()> {
    if !run_directory.is_dir() {
        return Err(crate::error::MoltenError::invalid_harness(
            "distinct-process child requires an existing run directory",
        ));
    }
    Ok(())
}

fn wait_for_handoff(
    child: &mut ReapingChild,
    handoff_path: &std::path::Path,
    timeout: std::time::Duration,
) -> crate::error::Result<()> {
    let mut deadline = crate::fabric_time::SupervisionDeadline::after(timeout)?;
    loop {
        if handoff_path.is_file() {
            return Ok(());
        }
        if let Some(status) = child.try_wait()? {
            child.finished = true;
            return Err(crate::error::MoltenError::invalid_harness(format!(
                "listener exited before endpoint handoff with {status}"
            )));
        }
        if deadline.is_expired()? {
            return Err(crate::error::MoltenError::invalid_harness(
                "listener did not publish endpoint handoff before timeout",
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(CHILD_POLL_INTERVAL_MS));
    }
}

fn fixture_profile() -> crate::error::Result<CanonicalTransportProfile> {
    canonical_transport_profile(&TransportProfile {
        schema: TRANSPORT_PROFILE_SCHEMA.to_string(),
        profile_id: "iroh-distinct-process-v1".to_string(),
        profile_ref: PROFILE_REF.to_string(),
        adapter_kind: TransportAdapterKind::IrohLive,
        capabilities: vec![
            TransportCapability::BidirectionalStreams,
            TransportCapability::UnidirectionalStreams,
        ],
        limits: TransportLimits {
            max_listeners: PROFILE_LIMIT,
            max_sessions: PROFILE_LIMIT,
            max_streams_per_session: PROFILE_LIMIT,
            max_frame_bytes: FRAME_LIMIT,
            max_datagram_bytes: DATAGRAM_LIMIT,
            max_queued_events: PROFILE_LIMIT,
            max_queued_bytes: QUEUE_LIMIT,
            max_inflight_bytes: INFLIGHT_LIMIT,
            operation_deadline_ticks: DEADLINE_WINDOW,
        },
        non_claims: REQUIRED_TRANSPORT_NON_CLAIMS.to_vec(),
    })
}

fn fixture_protocol() -> ProtocolDescriptor {
    ProtocolDescriptor {
        schema: TRANSPORT_PROTOCOL_SCHEMA.to_string(),
        protocol_id: "distinct-process-echo".to_string(),
        version: "v1".to_string(),
        alpn: "molten/distinct-process-echo/1".to_string(),
        extension_id: "distinct-process-extension".to_string(),
        service_id: "distinct-process-service".to_string(),
        generation: GENERATION,
        listener_limit: 1,
        requested_capabilities: vec![
            TransportCapability::BidirectionalStreams,
            TransportCapability::UnidirectionalStreams,
        ],
        framing: FramingProfile {
            profile_id: "length-prefixed-blake3-v1".to_string(),
            profile_ref: FRAMING_REF.to_string(),
            max_frame_bytes: FRAME_LIMIT,
            length_prefix_bytes: LENGTH_PREFIX_BYTES,
            payload_hash_required: true,
        },
        cleanup_policy: ListenerCleanupPolicy::BoundedDrain {
            grace_ticks: DEADLINE_WINDOW,
        },
        registration_authority_ref: AUTHORITY_REF.to_string(),
        profile_ref: PROFILE_REF.to_string(),
    }
}

fn fixture_disclosure() -> EndpointDisclosurePolicy {
    EndpointDisclosurePolicy {
        explicit_handoff_classes: vec![EndpointLocatorClass::Ip],
        default_readback_redacted: true,
    }
}

fn fixture_validity() -> EndpointValidityCohort {
    EndpointValidityCohort {
        cohort_ref: VALIDITY_REF.to_string(),
        not_before_tick: VALID_FROM_TICK,
        expires_at_tick: VALID_UNTIL_TICK,
    }
}

fn fixture_capability(secret_byte: u8, capability_ref: &str) -> crate::error::Result<IrohEndpointCapability> {
    IrohEndpointCapability::from_secret_bytes([secret_byte; IROH_SECRET_KEY_BYTES_LOCAL], capability_ref.to_string())
}

fn expected_binding(endpoint: &CanonicalCrossProcessEndpoint) -> ExpectedEndpointBinding {
    let descriptor = &endpoint.descriptor;
    ExpectedEndpointBinding {
        profile_id: descriptor.profile_id.clone(),
        profile_ref: descriptor.profile_ref.clone(),
        protocol_id: descriptor.protocol_id.clone(),
        protocol_version: descriptor.protocol_version.clone(),
        alpn: descriptor.alpn.clone(),
        extension_id: descriptor.extension_id.clone(),
        service_id: descriptor.service_id.clone(),
        generation: descriptor.generation,
        public_endpoint_identity: descriptor.public_endpoint_identity.clone(),
        listener_identity_ref: descriptor.listener_identity_ref.clone(),
        peer_context_ref: descriptor.expected_peer_context_ref.clone(),
        observed_tick: OBSERVED_TICK,
    }
}

fn validate_fixture_endpoint(endpoint: &CanonicalCrossProcessEndpoint) -> crate::error::Result<()> {
    validate_cross_process_endpoint(&fixture_profile()?.profile, &fixture_protocol(), &endpoint.descriptor)
        .map_err(|issues| crate::error::MoltenError::invalid_harness(format!("fixture endpoint denied: {issues:?}")))
}

struct ParticipantInput<'a> {
    role: EndpointParticipantRole,
    invocation_ref: &'a str,
    frame: &'a CrossProcessFrameEvidence,
    endpoint_cleanup_ref: &'a str,
    drain_reason: Option<ListenerDrainReason>,
    profile: &'a TransportProfile,
    protocol: &'a ProtocolDescriptor,
    handoff_ref: &'a str,
}

const PARTICIPANT_CHECKS: [&str; 4] = [
    "canonical-frame-observed",
    "terminal-cleanup-observed",
    "payload-bytes-excluded",
    "runtime-handles-excluded",
];

fn participant_artifact(input: ParticipantInput<'_>) -> crate::error::Result<ParticipantArtifact> {
    let ParticipantInput {
        role,
        invocation_ref,
        frame,
        endpoint_cleanup_ref,
        drain_reason,
        profile,
        protocol,
        handoff_ref,
    } = input;
    for reference in [invocation_ref, endpoint_cleanup_ref, handoff_ref] {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    let value = crate::preserves_rail::record("fabric-transport-participant-terminal-v1", vec![
        crate::preserves_rail::string(PARTICIPANT_SCHEMA),
        crate::preserves_rail::string(PASS_DECISION),
        crate::preserves_rail::string(role.as_str()),
        crate::preserves_rail::string(invocation_ref),
        crate::preserves_rail::string(&frame.descriptor_ref),
        crate::preserves_rail::string(handoff_ref),
        crate::preserves_rail::string(&profile.profile_id),
        crate::preserves_rail::string(&protocol.protocol_id),
        crate::preserves_rail::string(&protocol.alpn),
        crate::preserves_rail::string(&protocol.service_id),
        crate::preserves_rail::u64_value(protocol.generation),
        crate::preserves_rail::string(&frame.request_ref),
        crate::preserves_rail::string(&frame.payload_ref),
        crate::preserves_rail::string(&frame.acknowledgement_ref),
        crate::preserves_rail::string(&frame.remote_transport_identity_ref),
        crate::preserves_rail::u64_value(frame.payload_bytes),
        crate::preserves_rail::string(frame.delivery.as_str()),
        crate::preserves_rail::string(frame.retry.as_str()),
        crate::preserves_rail::u64_value(frame.automatic_retry_count),
        crate::preserves_rail::string(&frame.cleanup_evidence_ref),
        crate::preserves_rail::string(endpoint_cleanup_ref),
        crate::preserves_rail::string(drain_reason.map_or(NOT_APPLICABLE, ListenerDrainReason::as_str)),
        strings_value(profile.non_claims.iter().map(|claim| claim.as_str())),
        checks(&PARTICIPANT_CHECKS),
    ]);
    let artifact_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(ParticipantArtifact {
        role,
        invocation_ref: invocation_ref.to_string(),
        descriptor_ref: frame.descriptor_ref.clone(),
        handoff_ref: handoff_ref.to_string(),
        profile_id: profile.profile_id.clone(),
        protocol_id: protocol.protocol_id.clone(),
        alpn: protocol.alpn.clone(),
        service_id: protocol.service_id.clone(),
        generation: protocol.generation,
        request_ref: frame.request_ref.clone(),
        payload_ref: frame.payload_ref.clone(),
        acknowledgement_ref: frame.acknowledgement_ref.clone(),
        remote_transport_identity_ref: frame.remote_transport_identity_ref.clone(),
        payload_bytes: frame.payload_bytes,
        delivery: frame.delivery,
        retry: frame.retry,
        automatic_retry_count: frame.automatic_retry_count,
        session_cleanup_ref: frame.cleanup_evidence_ref.clone(),
        endpoint_cleanup_ref: endpoint_cleanup_ref.to_string(),
        drain_reason: drain_reason.map_or(NOT_APPLICABLE, ListenerDrainReason::as_str).to_string(),
        value,
        artifact_ref,
    })
}

fn start_artifact(role: EndpointParticipantRole, invocation_ref: &str) -> crate::error::Result<StartArtifact> {
    crate::preserves_rail::validate_content_ref(invocation_ref)?;
    let command_profile_ref = command_profile_ref(role.as_str());
    let value = crate::preserves_rail::record("fabric-transport-child-start-v1", vec![
        crate::preserves_rail::string(START_SCHEMA),
        crate::preserves_rail::string(role.as_str()),
        crate::preserves_rail::string(invocation_ref),
        crate::preserves_rail::string(&command_profile_ref),
        crate::preserves_rail::bool_value(true),
        checks(&["parent-observed-start", "raw-process-id-excluded"]),
    ]);
    let artifact_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(StartArtifact {
        role,
        invocation_ref: invocation_ref.to_string(),
        command_profile_ref,
        parent_observed: true,
        value,
        artifact_ref,
    })
}

fn cleanup_artifact(
    listener: &ParticipantArtifact,
    client: &ParticipantArtifact,
    listener_exited: bool,
    client_exited: bool,
    no_orphans: bool,
) -> crate::error::Result<CleanupArtifact> {
    let value = crate::preserves_rail::record("fabric-transport-distinct-cleanup-v1", vec![
        crate::preserves_rail::string(CLEANUP_SCHEMA),
        crate::preserves_rail::string(&listener.artifact_ref),
        crate::preserves_rail::string(&client.artifact_ref),
        crate::preserves_rail::string(&listener.endpoint_cleanup_ref),
        crate::preserves_rail::string(&client.endpoint_cleanup_ref),
        crate::preserves_rail::bool_value(listener_exited),
        crate::preserves_rail::bool_value(client_exited),
        crate::preserves_rail::bool_value(no_orphans),
        checks(&[
            "listener-reaped",
            "client-reaped",
            "no-orphaned-children",
            "cleanup-refs-bound",
        ]),
    ]);
    let artifact_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CleanupArtifact {
        listener_terminal_ref: listener.artifact_ref.clone(),
        client_terminal_ref: client.artifact_ref.clone(),
        listener_cleanup_ref: listener.endpoint_cleanup_ref.clone(),
        client_cleanup_ref: client.endpoint_cleanup_ref.clone(),
        listener_exited,
        client_exited,
        no_orphans,
        value,
        artifact_ref,
    })
}
