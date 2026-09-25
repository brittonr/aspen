
fn execute_prepared_distinct_process_transport_run(
    input: &DistinctProcessTransportRunInput,
) -> crate::error::Result<DistinctProcessTransportRun> {
    let timeout = std::time::Duration::from_millis(input.child_timeout_ms);
    write_transport_input(&input.run_directory, &input.request_ref, &input.payload)?;
    let listener_invocation_ref = invocation_ref(LISTENER_ROLE);
    let client_invocation_ref = invocation_ref(CLIENT_ROLE);

    let mut listener = ReapingChild::spawn(
        &input.process_binary,
        "fabric-transport-listener-child",
        &input.run_directory,
        &input.run_directory.join(LISTENER_LOG_FILE),
    )?;
    let listener_start = start_artifact(EndpointParticipantRole::Listener, &listener_invocation_ref)?;
    write_preserves(&input.run_directory.join(LISTENER_START_FILE), &listener_start.value)?;
    wait_for_handoff(&mut listener, &input.run_directory.join(HANDOFF_FILE), timeout)?;
    let handoff = read_endpoint_handoff(&input.run_directory.join(HANDOFF_FILE))?;
    validate_fixture_endpoint(&handoff)?;

    let mut client = ReapingChild::spawn(
        &input.process_binary,
        "fabric-transport-client-child",
        &input.run_directory,
        &input.run_directory.join(CLIENT_LOG_FILE),
    )?;
    let is_child_handles_distinct = listener.id() != client.id();
    let client_start = start_artifact(EndpointParticipantRole::Client, &client_invocation_ref)?;
    write_preserves(&input.run_directory.join(CLIENT_START_FILE), &client_start.value)?;

    let client_status = client.wait_bounded(timeout, CLIENT_ROLE)?;
    if !client_status.success() {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "distinct-process client child exited with {client_status}"
        )));
    }
    let listener_status = listener.wait_bounded(timeout, LISTENER_ROLE)?;
    if !listener_status.success() {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "distinct-process listener child exited with {listener_status}"
        )));
    }

    let listener_terminal = read_participant(&input.run_directory.join(LISTENER_TERMINAL_FILE))?;
    let client_terminal = read_participant(&input.run_directory.join(CLIENT_TERMINAL_FILE))?;
    let cleanup = cleanup_artifact(&listener_terminal, &client_terminal, true, true, true)?;
    write_preserves(&input.run_directory.join(CLEANUP_FILE), &cleanup.value)?;
    let run_artifacts = RunArtifacts {
        listener_start: &listener_start,
        client_start: &client_start,
        listener: &listener_terminal,
        client: &client_terminal,
        cleanup: &cleanup,
    };
    let assessment_input = assessment_input(run_artifacts, is_child_handles_distinct);
    let assessment = assess_distinct_process_transport_evidence(&assessment_input);
    let parent_value = parent_run_value(run_artifacts, &assessment_input, &assessment)?;
    let parent_ref = crate::preserves_rail::canonical_hash(&parent_value)?;
    write_preserves(&input.run_directory.join(PARENT_RUN_FILE), &parent_value)?;
    write_index(&input.run_directory)?;
    let verification = verify_distinct_process_run_directory_inner(&input.run_directory, false)?;
    write_preserves(&input.run_directory.join(VERIFICATION_FILE), &verification.value)?;
    Ok(DistinctProcessTransportRun {
        decision: verification.decision,
        parent_ref,
        verification_ref: verification.verification_ref,
        diagnostics: verification.diagnostics,
        run_directory: input.run_directory.clone(),
    })
}

// r[impl molten.fabric_transport.cross_process_listener]
pub fn run_distinct_process_listener_child(run_directory: &std::path::Path) -> crate::error::Result<()> {
    validate_child_directory(run_directory)?;
    let runtime = tokio::runtime::Runtime::new().map_err(|error| {
        crate::error::MoltenError::invalid_harness(format!("listener runtime creation failed: {error}"))
    })?;
    runtime.block_on(async {
        let mut listener = IrohCrossProcessListener::bind(IrohCrossProcessListenerInput {
            profile: fixture_profile()?,
            protocol: fixture_protocol(),
            capability: fixture_capability(LISTENER_SECRET_BYTE, LISTENER_CAPABILITY_REF)?,
            bind_addr: std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 0)),
            listener_identity_ref: LISTENER_IDENTITY_REF.to_string(),
            expected_peer_context_ref: PEER_CONTEXT_REF.to_string(),
            locator_cohort_ref: LOCATOR_COHORT_REF.to_string(),
            disclosure: fixture_disclosure(),
            validity: fixture_validity(),
            admission: EndpointAdmissionState::fully_active(),
            observed_tick: OBSERVED_TICK,
        })
        .await?;
        write_preserves_atomic(&run_directory.join(HANDOFF_FILE), &listener.handoff().value)?;
        let (request_ref, _payload) = read_transport_input(run_directory)?;
        let frame = listener
            .accept_one(
                SESSION_REF,
                &request_ref,
                std::time::Duration::from_millis(DEFAULT_DISTINCT_PROCESS_TIMEOUT_MS),
            )
            .await?;
        let endpoint_cleanup = listener.drain_and_close(ListenerDrainReason::OperatorRequest).await?;
        let participant = participant_artifact(ParticipantInput {
            role: EndpointParticipantRole::Listener,
            invocation_ref: &invocation_ref(LISTENER_ROLE),
            frame: &frame,
            endpoint_cleanup_ref: &endpoint_cleanup.cleanup_evidence_ref,
            drain_reason: Some(endpoint_cleanup.drain_reason),
            profile: &fixture_profile()?.profile,
            protocol: &fixture_protocol(),
            handoff_ref: &crate::preserves_rail::canonical_hash(
                &read_endpoint_handoff(&run_directory.join(HANDOFF_FILE))?.value,
            )?,
        })?;
        write_preserves(&run_directory.join(LISTENER_TERMINAL_FILE), &participant.value)
    })
}

// r[impl molten.fabric_transport.cross_process_session]
pub fn run_distinct_process_client_child(run_directory: &std::path::Path) -> crate::error::Result<()> {
    validate_child_directory(run_directory)?;
    let handoff = read_endpoint_handoff(&run_directory.join(HANDOFF_FILE))?;
    validate_fixture_endpoint(&handoff)?;
    let (request_ref, payload) = read_transport_input(run_directory)?;
    let runtime = tokio::runtime::Runtime::new().map_err(|error| {
        crate::error::MoltenError::invalid_harness(format!("client runtime creation failed: {error}"))
    })?;
    runtime.block_on(async {
        let frame = exchange_cross_process_frame(
            IrohCrossProcessClientInput {
                profile: fixture_profile()?,
                protocol: fixture_protocol(),
                capability: fixture_capability(CLIENT_SECRET_BYTE, CLIENT_CAPABILITY_REF)?,
                bind_addr: std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 0)),
                expected: expected_binding(&handoff),
                endpoint: handoff.clone(),
                admission: EndpointAdmissionState::fully_active(),
                session_ref: SESSION_REF.to_string(),
                request_ref,
            },
            &payload,
            std::time::Duration::from_millis(DEFAULT_DISTINCT_PROCESS_TIMEOUT_MS),
        )
        .await?;
        let participant = participant_artifact(ParticipantInput {
            role: EndpointParticipantRole::Client,
            invocation_ref: &invocation_ref(CLIENT_ROLE),
            frame: &frame,
            endpoint_cleanup_ref: &frame.cleanup_evidence_ref,
            drain_reason: None,
            profile: &fixture_profile()?.profile,
            protocol: &fixture_protocol(),
            handoff_ref: &handoff.handoff_ref,
        })?;
        write_preserves(&run_directory.join(CLIENT_TERMINAL_FILE), &participant.value)
    })
}

// r[impl molten.fabric_transport.distinct_process_evidence]
pub fn verify_distinct_process_transport_run(
    run_directory: &std::path::Path,
) -> crate::error::Result<DistinctProcessTransportVerification> {
    verify_distinct_process_run_directory_inner(run_directory, true)
}

fn verify_distinct_process_run_directory_inner(
    run_directory: &std::path::Path,
    require_companion: bool,
) -> crate::error::Result<DistinctProcessTransportVerification> {
    let mut diagnostics = validate_run_membership(run_directory, require_companion)?;
    let listener_start = read_start(&run_directory.join(LISTENER_START_FILE))?;
    let client_start = read_start(&run_directory.join(CLIENT_START_FILE))?;
    let listener_terminal = read_participant(&run_directory.join(LISTENER_TERMINAL_FILE))?;
    let client_terminal = read_participant(&run_directory.join(CLIENT_TERMINAL_FILE))?;
    let actual_cleanup = read_cleanup(&run_directory.join(CLEANUP_FILE))?;
    let expected_cleanup = cleanup_artifact(&listener_terminal, &client_terminal, true, true, true)?;
    if actual_cleanup.value != expected_cleanup.value {
        diagnostics.push("cleanup-artifact-mismatch".to_string());
    }
    let run_artifacts = RunArtifacts {
        listener_start: &listener_start,
        client_start: &client_start,
        listener: &listener_terminal,
        client: &client_terminal,
        cleanup: &actual_cleanup,
    };
    let assessment_input = assessment_input(run_artifacts, true);
    let assessment = assess_distinct_process_transport_evidence(&assessment_input);
    diagnostics.extend(assessment.issues.iter().map(|issue| issue.code().to_string()));
    let expected_parent = parent_run_value(run_artifacts, &assessment_input, &assessment)?;
    let actual_parent = read_preserves(&run_directory.join(PARENT_RUN_FILE))?;
    if actual_parent != expected_parent {
        diagnostics.push("parent-run-artifact-mismatch".to_string());
    }
    let parent_ref = crate::preserves_rail::canonical_hash(&actual_parent)?;
    let index_text =
        std::fs::read_to_string(run_directory.join(INDEX_FILE)).map_err(crate::error::MoltenError::from)?;
    let expected_index = render_index(&collect_indexed_artifacts(run_directory)?);
    if index_text != expected_index {
        diagnostics.push("artifact-index-mismatch".to_string());
    }
    diagnostics.sort();
    diagnostics.dedup();
    let index_ref = text_ref(RUN_INDEX_DOMAIN, &index_text);
    let initial_decision = if diagnostics.is_empty() {
        PASS_DECISION
    } else {
        DENY_DECISION
    };
    let initial_value = verification_value(initial_decision, &index_ref, &parent_ref, &diagnostics);
    if require_companion {
        let companion = read_preserves(&run_directory.join(VERIFICATION_FILE))?;
        if companion != initial_value {
            diagnostics.push("verification-companion-mismatch".to_string());
        }
    }
    diagnostics.sort();
    diagnostics.dedup();
    let decision = if diagnostics.is_empty() {
        PASS_DECISION
    } else {
        DENY_DECISION
    }
    .to_string();
    let value = verification_value(&decision, &index_ref, &parent_ref, &diagnostics);
    let verification_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(DistinctProcessTransportVerification {
        decision,
        parent_ref,
        verification_ref,
        diagnostics,
        value,
    })
}

fn validate_run_input(input: &DistinctProcessTransportRunInput) -> crate::error::Result<()> {
    if input.child_timeout_ms == 0 || input.child_timeout_ms > MAX_DISTINCT_PROCESS_TIMEOUT_MS {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "distinct-process child timeout must be between 1 and {MAX_DISTINCT_PROCESS_TIMEOUT_MS} milliseconds"
        )));
    }
    if input.run_directory.as_os_str().is_empty() || input.process_binary.as_os_str().is_empty() {
        return Err(crate::error::MoltenError::invalid_harness(
            "distinct-process run requires explicit run directory and process binary",
        ));
    }
    crate::preserves_rail::validate_content_ref(&input.request_ref)?;
    let payload_bytes = u64::try_from(input.payload.len())
        .map_err(|_| crate::error::MoltenError::invalid_harness("distinct-process payload length exceeds u64"))?;
    if input.payload.is_empty() || payload_bytes > FRAME_LIMIT {
        return Err(crate::error::MoltenError::invalid_harness(
            "distinct-process payload must be nonempty and within the frame bound",
        ));
    }
    Ok(())
}

fn write_transport_input(
    run_directory: &std::path::Path,
    request_ref: &str,
    payload: &[u8],
) -> crate::error::Result<()> {
    std::fs::write(run_directory.join(REQUEST_INPUT_FILE), request_ref.as_bytes())
        .map_err(crate::error::MoltenError::from)?;
    std::fs::write(run_directory.join(PAYLOAD_INPUT_FILE), payload).map_err(crate::error::MoltenError::from)
}

fn read_transport_input(run_directory: &std::path::Path) -> crate::error::Result<(String, Vec<u8>)> {
    let request_ref =
        std::fs::read_to_string(run_directory.join(REQUEST_INPUT_FILE)).map_err(crate::error::MoltenError::from)?;
    crate::preserves_rail::validate_content_ref(&request_ref)?;
    let payload = std::fs::read(run_directory.join(PAYLOAD_INPUT_FILE)).map_err(crate::error::MoltenError::from)?;
    let payload_bytes = u64::try_from(payload.len())
        .map_err(|_| crate::error::MoltenError::invalid_harness("distinct-process payload length exceeds u64"))?;
    if payload.is_empty() || payload_bytes > FRAME_LIMIT {
        return Err(crate::error::MoltenError::invalid_harness(
            "distinct-process payload input is empty or over-bound",
        ));
    }
    Ok((request_ref, payload))
}
