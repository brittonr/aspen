
fn finalize_successful_session(input: SessionCloseInput<'_>) -> crate::error::Result<CrossProcessFrameEvidence> {
    let SessionCloseInput {
        mut session,
        role,
        descriptor_ref,
        session_ref,
        request_ref,
        remote_transport_identity_ref,
        frame,
    } = input;
    session = apply_cross_process_session_command(&session, &CrossProcessSessionCommand::Close)
        .map_err(|issues| shell_validation_error("cross-process session close", &issues))?
        .next;
    session = apply_cross_process_session_command(&session, &CrossProcessSessionCommand::BeginCleanup)
        .map_err(|issues| shell_validation_error("cross-process session cleanup", &issues))?
        .next;
    let cleanup_evidence_ref = cleanup_ref(session_ref, descriptor_ref, session.identity.generation);
    session = apply_cross_process_session_command(&session, &CrossProcessSessionCommand::CompleteCleanup {
        cleanup_evidence_ref: cleanup_evidence_ref.clone(),
    })
    .map_err(|issues| shell_validation_error("cross-process session cleanup completion", &issues))?
    .next;
    Ok(CrossProcessFrameEvidence {
        role,
        descriptor_ref: descriptor_ref.to_string(),
        session_ref: session_ref.to_string(),
        request_ref: request_ref.to_string(),
        payload_ref: frame.payload_ref,
        acknowledgement_ref: frame.acknowledgement_ref,
        remote_transport_identity_ref: remote_transport_identity_ref.to_string(),
        payload_bytes: frame.payload_bytes,
        delivery: session.delivery,
        retry: session.retry,
        automatic_retry_count: session.automatic_retry_count,
        terminal_class: session.terminal_class.unwrap_or(SessionTerminalClass::Clean),
        cleanup_evidence_ref,
    })
}

fn finalize_failed_session(
    mut session: CrossProcessSessionState,
    class: SessionTerminalClass,
) -> crate::error::Result<CrossProcessSessionState> {
    session = apply_cross_process_session_command(&session, &CrossProcessSessionCommand::Fail {
        class,
        delivery_definitive: false,
    })
    .map_err(|issues| shell_validation_error("cross-process session failure", &issues))?
    .next;
    session = apply_cross_process_session_command(&session, &CrossProcessSessionCommand::BeginCleanup)
        .map_err(|issues| shell_validation_error("cross-process failed-session cleanup", &issues))?
        .next;
    let cleanup_evidence_ref =
        cleanup_ref(&session.identity.session_ref, &session.identity.descriptor_ref, session.identity.generation);
    session = apply_cross_process_session_command(&session, &CrossProcessSessionCommand::CompleteCleanup {
        cleanup_evidence_ref,
    })
    .map_err(|issues| shell_validation_error("cross-process failed-session cleanup completion", &issues))?
    .next;
    Ok(session)
}

fn validate_listener_shell_input(input: &IrohCrossProcessListenerInput) -> crate::error::Result<()> {
    crate::preserves_rail::validate_content_ref(input.capability.capability_ref())?;
    crate::preserves_rail::validate_content_ref(&input.listener_identity_ref)?;
    crate::preserves_rail::validate_content_ref(&input.expected_peer_context_ref)?;
    crate::preserves_rail::validate_content_ref(&input.locator_cohort_ref)?;
    if !input.bind_addr.ip().is_loopback() {
        return Err(crate::error::MoltenError::invalid_harness(
            "initial cross-process Iroh profile requires an explicit loopback bind address",
        ));
    }
    if !input.admission.registration_active
        || !input.admission.transport_capability_active
        || !input.admission.protocol_capability_active
        || !input.admission.profile_active
        || !input.admission.listener_ready
    {
        return Err(crate::error::MoltenError::invalid_harness(
            "cross-process listener capability admission is not fully active",
        ));
    }
    Ok(())
}

fn validate_client_shell_input(input: &IrohCrossProcessClientInput, payload: &[u8]) -> crate::error::Result<()> {
    crate::preserves_rail::validate_content_ref(input.capability.capability_ref())?;
    validate_exchange_refs(&input.session_ref, &input.request_ref)?;
    if payload.is_empty() {
        return Err(crate::error::MoltenError::invalid_harness("cross-process payload must not be empty"));
    }
    if !input.bind_addr.ip().is_loopback() {
        return Err(crate::error::MoltenError::invalid_harness(
            "initial cross-process Iroh client profile requires an explicit loopback bind address",
        ));
    }
    Ok(())
}

fn validate_exchange_refs(session_ref: &str, request_ref: &str) -> crate::error::Result<()> {
    crate::preserves_rail::validate_content_ref(session_ref)?;
    crate::preserves_rail::validate_content_ref(request_ref)
}

fn endpoint_locators(endpoint_addr: &iroh::EndpointAddr) -> crate::error::Result<Vec<EndpointLocator>> {
    let mut locators = Vec::new();
    for address in &endpoint_addr.addrs {
        let class = match address {
            iroh::TransportAddr::Ip(_) => EndpointLocatorClass::Ip,
            iroh::TransportAddr::Relay(_) => EndpointLocatorClass::Relay,
            iroh::TransportAddr::Custom(_) => {
                return Err(crate::error::MoltenError::invalid_harness(
                    "custom Iroh transport addresses are outside the admitted profile",
                ));
            }
            _ => {
                return Err(crate::error::MoltenError::invalid_harness(
                    "unknown Iroh transport address is outside the admitted profile",
                ));
            }
        };
        if locators.len() >= MAX_ENDPOINT_LOCATORS {
            return Err(crate::error::MoltenError::invalid_harness("Iroh endpoint locator count exceeds bound"));
        }
        locators.push(EndpointLocator {
            class,
            value: address.to_string(),
        });
    }
    Ok(locators)
}

fn iroh_endpoint_addr(plan: &EndpointDialPlan) -> crate::error::Result<iroh::EndpointAddr> {
    let endpoint_id = plan
        .public_endpoint_identity
        .strip_prefix("iroh:")
        .ok_or_else(|| {
            crate::error::MoltenError::invalid_harness("cross-process endpoint identity must use iroh prefix")
        })?
        .parse::<iroh::EndpointId>()
        .map_err(iroh_error)?;
    let mut addresses = Vec::with_capacity(plan.locators.len());
    for locator in &plan.locators {
        let address = match locator.class {
            EndpointLocatorClass::Ip => {
                let address = locator
                    .value
                    .strip_prefix("ip:")
                    .ok_or_else(|| {
                        crate::error::MoltenError::invalid_harness("cross-process IP locator prefix mismatch")
                    })?
                    .parse::<std::net::SocketAddr>()
                    .map_err(iroh_error)?;
                iroh::TransportAddr::Ip(address)
            }
            EndpointLocatorClass::Relay => {
                let relay = locator
                    .value
                    .strip_prefix("relay:")
                    .ok_or_else(|| {
                        crate::error::MoltenError::invalid_harness("cross-process relay locator prefix mismatch")
                    })?
                    .parse::<iroh::RelayUrl>()
                    .map_err(iroh_error)?;
                iroh::TransportAddr::Relay(relay)
            }
            EndpointLocatorClass::Custom | EndpointLocatorClass::Private => {
                return Err(crate::error::MoltenError::invalid_harness(
                    "cross-process endpoint contains an unsupported locator class",
                ));
            }
        };
        addresses.push(address);
    }
    Ok(iroh::EndpointAddr::from_parts(endpoint_id, addresses))
}

fn dial_plan_from_descriptor(descriptor: &CrossProcessEndpointDescriptor) -> EndpointDialPlan {
    EndpointDialPlan {
        descriptor_ref: descriptor.descriptor_ref.clone(),
        public_endpoint_identity: descriptor.public_endpoint_identity.clone(),
        locators: descriptor.locators.clone(),
        profile_id: descriptor.profile_id.clone(),
        protocol_id: descriptor.protocol_id.clone(),
        alpn: descriptor.alpn.clone(),
        service_id: descriptor.service_id.clone(),
        generation: descriptor.generation,
        peer_context_ref: descriptor.expected_peer_context_ref.clone(),
        resources: descriptor.resources.clone(),
    }
}

pub fn cross_process_frame_ref(request_ref: &str, payload: &[u8]) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(FRAME_DOMAIN.as_bytes());
    hasher.update(request_ref.as_bytes());
    hasher.update(payload);
    format!("blake3:{}", hasher.finalize().to_hex())
}

fn cleanup_ref(identity_ref: &str, descriptor_ref: &str, generation: u64) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(CLEANUP_DOMAIN.as_bytes());
    hasher.update(identity_ref.as_bytes());
    hasher.update(descriptor_ref.as_bytes());
    hasher.update(&generation.to_be_bytes());
    format!("blake3:{}", hasher.finalize().to_hex())
}

fn blake3_ref(bytes: &[u8]) -> String {
    format!("blake3:{}", blake3::hash(bytes).to_hex())
}

fn iroh_error(error: impl std::fmt::Display) -> crate::error::MoltenError {
    crate::error::MoltenError::invalid_harness(format!("cross-process Iroh transport failed: {error}"))
}

fn shell_validation_error(label: &str, issues: &impl std::fmt::Debug) -> crate::error::MoltenError {
    crate::error::MoltenError::invalid_harness(format!("{label} denied: {issues:?}"))
}
