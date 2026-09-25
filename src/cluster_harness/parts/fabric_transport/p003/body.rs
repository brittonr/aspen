
#[derive(Clone, Copy)]
struct RunArtifacts<'a> {
    listener_start: &'a StartArtifact,
    client_start: &'a StartArtifact,
    listener: &'a ParticipantArtifact,
    client: &'a ParticipantArtifact,
    cleanup: &'a CleanupArtifact,
}

fn assessment_input(
    artifacts: RunArtifacts<'_>,
    child_handles_distinct: bool,
) -> DistinctProcessTransportEvidenceInput {
    let RunArtifacts {
        listener_start,
        client_start,
        listener,
        client,
        cleanup,
    } = artifacts;
    DistinctProcessTransportEvidenceInput {
        listener: participant_evidence(listener_start, listener, cleanup.listener_exited),
        client: participant_evidence(client_start, client, cleanup.client_exited),
        handoff_ref: listener.handoff_ref.clone(),
        child_handles_distinct,
        handoff_observed_before_client_start: true,
        cleanup_succeeded: cleanup.listener_exited && cleanup.client_exited && cleanup.no_orphans,
        same_process_loopback: false,
        child_only_separation_claim: false,
        default_readback_redacted: true,
        payloads_excluded: true,
        accepted_sessions: GENERATION,
        max_sessions: PROFILE_LIMIT,
        exchanged_bytes: listener.payload_bytes,
        max_frame_bytes: FRAME_LIMIT,
    }
}

fn participant_evidence(
    start: &StartArtifact,
    participant: &ParticipantArtifact,
    exited: bool,
) -> DistinctProcessParticipantEvidence {
    let expected_command_profile_ref = command_profile_ref(participant.role.as_str());
    let is_parent_observed_start = start.parent_observed
        && start.role == participant.role
        && start.invocation_ref == participant.invocation_ref
        && start.command_profile_ref == expected_command_profile_ref;
    DistinctProcessParticipantEvidence {
        role: participant.role,
        invocation_ref: participant.invocation_ref.clone(),
        parent_start_ref: start.artifact_ref.clone(),
        terminal_ref: participant.artifact_ref.clone(),
        cleanup_ref: participant.endpoint_cleanup_ref.clone(),
        descriptor_ref: participant.descriptor_ref.clone(),
        profile_id: participant.profile_id.clone(),
        protocol_id: participant.protocol_id.clone(),
        alpn: participant.alpn.clone(),
        service_id: participant.service_id.clone(),
        generation: participant.generation,
        request_ref: participant.request_ref.clone(),
        payload_ref: participant.payload_ref.clone(),
        acknowledgement_ref: participant.acknowledgement_ref.clone(),
        parent_observed_start: is_parent_observed_start,
        parent_observed_terminal: true,
        parent_observed_exit: exited,
        automatic_retry_count: participant.automatic_retry_count,
    }
}

fn parent_run_value(
    artifacts: RunArtifacts<'_>,
    input: &DistinctProcessTransportEvidenceInput,
    assessment: &DistinctProcessTransportAssessment,
) -> crate::error::Result<preserves::IOValue> {
    let RunArtifacts {
        listener_start,
        client_start,
        listener,
        client,
        cleanup,
    } = artifacts;
    if listener.handoff_ref != client.handoff_ref {
        return Err(crate::error::MoltenError::invalid_harness("participant handoff refs do not match"));
    }
    if listener.drain_reason == NOT_APPLICABLE || client.drain_reason != NOT_APPLICABLE {
        return Err(crate::error::MoltenError::invalid_harness(
            "participant drain reason does not match its listener/client role",
        ));
    }
    let decision = if assessment.admitted {
        PASS_DECISION
    } else {
        DENY_DECISION
    };
    Ok(crate::preserves_rail::record("fabric-transport-distinct-process-run-v1", vec![
        crate::preserves_rail::string(RUN_SCHEMA),
        crate::preserves_rail::string(decision),
        crate::preserves_rail::string(&listener_start.artifact_ref),
        crate::preserves_rail::string(&client_start.artifact_ref),
        crate::preserves_rail::string(&listener.artifact_ref),
        crate::preserves_rail::string(&client.artifact_ref),
        crate::preserves_rail::string(&cleanup.artifact_ref),
        crate::preserves_rail::string(&listener.handoff_ref),
        crate::preserves_rail::string(&listener.descriptor_ref),
        crate::preserves_rail::string(&listener.profile_id),
        crate::preserves_rail::string(&listener.protocol_id),
        crate::preserves_rail::string(&listener.alpn),
        crate::preserves_rail::string(&listener.service_id),
        crate::preserves_rail::u64_value(listener.generation),
        crate::preserves_rail::string(&listener.request_ref),
        crate::preserves_rail::string(&listener.payload_ref),
        crate::preserves_rail::string(&listener.acknowledgement_ref),
        crate::preserves_rail::u64_value(input.accepted_sessions),
        crate::preserves_rail::u64_value(input.max_sessions),
        crate::preserves_rail::u64_value(input.exchanged_bytes),
        crate::preserves_rail::u64_value(input.max_frame_bytes),
        strings_value(REQUIRED_TRANSPORT_NON_CLAIMS.iter().map(|claim| claim.as_str())),
        strings_value(assessment.issues.iter().map(|issue| issue.code())),
        checks(&[
            "parent-observed-distinct-child-handles",
            "handoff-precedes-client-start",
            "participant-bindings-match",
            "terminal-cleanup-and-exits-observed",
            "same-process-loopback-insufficient",
            "connectivity-only-non-claims",
        ]),
    ]))
}

fn verification_value(decision: &str, index_ref: &str, parent_ref: &str, diagnostics: &[String]) -> preserves::IOValue {
    crate::preserves_rail::record("fabric-transport-distinct-process-verification-v1", vec![
        crate::preserves_rail::string(VERIFICATION_SCHEMA),
        crate::preserves_rail::string(decision),
        crate::preserves_rail::string(index_ref),
        crate::preserves_rail::string(parent_ref),
        strings_value(diagnostics.iter().map(String::as_str)),
        checks(&[
            "offline-canonical-artifact-verification",
            "fixed-run-directory-membership",
            "child-only-claims-denied",
        ]),
    ])
}

fn failure_value(error_ref: &str) -> preserves::IOValue {
    crate::preserves_rail::record("fabric-transport-distinct-process-failure-v1", vec![
        crate::preserves_rail::string("molten.fabric.transport.distinct-process-failure.v1"),
        crate::preserves_rail::string(DENY_DECISION),
        crate::preserves_rail::string("execution-error"),
        crate::preserves_rail::string(error_ref),
        strings_value(REQUIRED_TRANSPORT_NON_CLAIMS.iter().map(|claim| claim.as_str())),
        checks(&[
            "raw-error-excluded",
            "owned-child-lifetimes-scope-bound",
            "cleanup-success-not-claimed",
            "failure-does-not-establish-process-separation",
        ]),
    ])
}

fn read_participant(path: &std::path::Path) -> crate::error::Result<ParticipantArtifact> {
    let value = read_preserves(path)?;
    let fields = simple_record(&value, "fabric-transport-participant-terminal-v1", PARTICIPANT_FIELD_COUNT)?;
    let mut fields = fields.as_slice().iter();
    require_schema(next(&mut fields, "participant schema")?, PARTICIPANT_SCHEMA)?;
    require_decision(next(&mut fields, "participant decision")?)?;
    let role = parse_role(&required_string(next(&mut fields, "participant role")?, "participant role")?)?;
    let invocation_ref = required_ref(next(&mut fields, "invocation ref")?, "invocation ref")?;
    let descriptor_ref = required_ref(next(&mut fields, "descriptor ref")?, "descriptor ref")?;
    let handoff_ref = required_ref(next(&mut fields, "handoff ref")?, "handoff ref")?;
    let profile_id = required_string(next(&mut fields, "profile id")?, "profile id")?;
    let protocol_id = required_string(next(&mut fields, "protocol id")?, "protocol id")?;
    let alpn = required_string(next(&mut fields, "ALPN")?, "ALPN")?;
    let service_id = required_string(next(&mut fields, "service id")?, "service id")?;
    let generation = required_u64(next(&mut fields, "generation")?, "generation")?;
    let request_ref = required_ref(next(&mut fields, "request ref")?, "request ref")?;
    let payload_ref = required_ref(next(&mut fields, "payload ref")?, "payload ref")?;
    let acknowledgement_ref = required_ref(next(&mut fields, "ack ref")?, "ack ref")?;
    let remote_transport_identity_ref = required_ref(next(&mut fields, "remote ref")?, "remote ref")?;
    let payload_bytes = required_u64(next(&mut fields, "payload bytes")?, "payload bytes")?;
    let delivery = parse_delivery(&required_string(next(&mut fields, "delivery")?, "delivery")?)?;
    let retry = parse_retry(&required_string(next(&mut fields, "retry")?, "retry")?)?;
    let automatic_retry_count = required_u64(next(&mut fields, "retry count")?, "retry count")?;
    let session_cleanup_ref = required_ref(next(&mut fields, "session cleanup")?, "session cleanup")?;
    let endpoint_cleanup_ref = required_ref(next(&mut fields, "endpoint cleanup")?, "endpoint cleanup")?;
    let drain_reason = required_string(next(&mut fields, "drain reason")?, "drain reason")?;
    let _non_claims = next(&mut fields, "non-claims")?;
    let _checks = next(&mut fields, "checks")?;
    let artifact_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(ParticipantArtifact {
        role,
        invocation_ref,
        descriptor_ref,
        handoff_ref,
        profile_id,
        protocol_id,
        alpn,
        service_id,
        generation,
        request_ref,
        payload_ref,
        acknowledgement_ref,
        remote_transport_identity_ref,
        payload_bytes,
        delivery,
        retry,
        automatic_retry_count,
        session_cleanup_ref,
        endpoint_cleanup_ref,
        drain_reason,
        value,
        artifact_ref,
    })
}

fn read_start(path: &std::path::Path) -> crate::error::Result<StartArtifact> {
    let value = read_preserves(path)?;
    let fields = simple_record(&value, "fabric-transport-child-start-v1", START_FIELD_COUNT)?;
    let mut fields = fields.as_slice().iter();
    require_schema(next(&mut fields, "start schema")?, START_SCHEMA)?;
    let role = parse_role(&required_string(next(&mut fields, "start role")?, "start role")?)?;
    let invocation_ref = required_ref(next(&mut fields, "start invocation")?, "start invocation")?;
    let command_profile_ref = required_ref(next(&mut fields, "command profile")?, "command profile")?;
    let is_parent_observed = required_bool(next(&mut fields, "parent observed")?, "parent observed")?;
    let _checks = next(&mut fields, "start checks")?;
    let artifact_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(StartArtifact {
        role,
        invocation_ref,
        command_profile_ref,
        parent_observed: is_parent_observed,
        value,
        artifact_ref,
    })
}

fn read_cleanup(path: &std::path::Path) -> crate::error::Result<CleanupArtifact> {
    let value = read_preserves(path)?;
    let fields = simple_record(&value, "fabric-transport-distinct-cleanup-v1", CLEANUP_FIELD_COUNT)?;
    let mut fields = fields.as_slice().iter();
    require_schema(next(&mut fields, "cleanup schema")?, CLEANUP_SCHEMA)?;
    let listener_terminal_ref = required_ref(next(&mut fields, "listener terminal")?, "listener terminal")?;
    let client_terminal_ref = required_ref(next(&mut fields, "client terminal")?, "client terminal")?;
    let listener_cleanup_ref = required_ref(next(&mut fields, "listener cleanup")?, "listener cleanup")?;
    let client_cleanup_ref = required_ref(next(&mut fields, "client cleanup")?, "client cleanup")?;
    let is_listener_exited = required_bool(next(&mut fields, "listener exited")?, "listener exited")?;
    let is_client_exited = required_bool(next(&mut fields, "client exited")?, "client exited")?;
    let is_no_orphans = required_bool(next(&mut fields, "no orphans")?, "no orphans")?;
    let _checks = next(&mut fields, "cleanup checks")?;
    let artifact_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CleanupArtifact {
        listener_terminal_ref,
        client_terminal_ref,
        listener_cleanup_ref,
        client_cleanup_ref,
        listener_exited: is_listener_exited,
        client_exited: is_client_exited,
        no_orphans: is_no_orphans,
        value,
        artifact_ref,
    })
}

fn read_endpoint_handoff(path: &std::path::Path) -> crate::error::Result<CanonicalCrossProcessEndpoint> {
    parse_canonical_cross_process_endpoint(&read_preserves(path)?)
}

fn read_preserves(path: &std::path::Path) -> crate::error::Result<preserves::IOValue> {
    ensure_regular_file(path)?;
    let bytes = std::fs::read(path).map_err(crate::error::MoltenError::from)?;
    crate::preserves_rail::parse_canonical_bytes(&bytes)
}

fn write_preserves(path: &std::path::Path, value: &preserves::IOValue) -> crate::error::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(crate::error::MoltenError::from)?;
    }
    let bytes = crate::preserves_rail::canonical_bytes(value)?;
    std::fs::write(path, bytes).map_err(crate::error::MoltenError::from)
}
