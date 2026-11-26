
fn finalize_successful_session(input: SessionCloseInput<'_>) -> crate::error::Result<FrameEvidence> {
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
        .map_err(|issues| validation_error("cross-process session close", &issues))?
        .next;
    session = apply_cross_process_session_command(&session, &CrossProcessSessionCommand::BeginCleanup)
        .map_err(|issues| validation_error("cross-process session cleanup", &issues))?
        .next;
    let cleanup_evidence_ref = cleanup_ref(session_ref, descriptor_ref, session.identity.generation);
    session = apply_cross_process_session_command(&session, &CrossProcessSessionCommand::CompleteCleanup {
        cleanup_evidence_ref: cleanup_evidence_ref.clone(),
    })
    .map_err(|issues| validation_error("cross-process session cleanup completion", &issues))?
    .next;
    Ok(FrameEvidence {
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
    .map_err(|issues| validation_error("cross-process session failure", &issues))?
    .next;
    session = apply_cross_process_session_command(&session, &CrossProcessSessionCommand::BeginCleanup)
        .map_err(|issues| validation_error("cross-process failed-session cleanup", &issues))?
        .next;
    let cleanup_evidence_ref =
        cleanup_ref(&session.identity.session_ref, &session.identity.descriptor_ref, session.identity.generation);
    session = apply_cross_process_session_command(&session, &CrossProcessSessionCommand::CompleteCleanup {
        cleanup_evidence_ref,
    })
    .map_err(|issues| validation_error("cross-process failed-session cleanup completion", &issues))?
    .next;
    Ok(session)
}

fn validate_listener_input(input: &ListenerInput) -> crate::error::Result<()> {
    crate::preserves_rail::validate_content_ref(input.capability.capability_ref())?;
    crate::preserves_rail::validate_content_ref(&input.listener_identity_ref)?;
    crate::preserves_rail::validate_content_ref(&input.expected_peer_context_ref)?;
    crate::preserves_rail::validate_content_ref(&input.locator_cohort_ref)?;
    if !input.bind_addr.ip().is_loopback() {
        return Err(crate::error::Failure::invalid_harness(
            "initial cross-process Iroh profile requires an explicit loopback bind address",
        ));
    }
    if !input.admission.registration_active
        || !input.admission.transport_capability_active
        || !input.admission.protocol_capability_active
        || !input.admission.profile_active
        || !input.admission.listener_ready
    {
        return Err(crate::error::Failure::invalid_harness(
            "cross-process listener capability admission is not fully active",
        ));
    }
    validate_admitted_peers(&input.admitted_peers)
}

/// Admitted peers are a bounded table of distinct canonical `iroh:<EndpointId>` identities. An empty
/// table refuses every connection until `Listener::admit_peer` adds one.
fn validate_admitted_peers(peers: &std::collections::BTreeMap<String, AdmittedPeer>) -> crate::error::Result<()> {
    if peers.len() > MAX_LISTENER_ADMITTED_PEERS {
        return Err(crate::error::Failure::invalid_harness(
            "cross-process listener admitted peer table exceeds its static bound",
        ));
    }
    let mut identities = std::collections::BTreeSet::new();
    for (peer_id, peer) in peers {
        if peer_id.is_empty() || peer_id.len() > MAX_ADMITTED_PEER_ID_BYTES {
            return Err(crate::error::Failure::invalid_harness("cross-process admitted peer id is empty or oversized"));
        }
        crate::preserves_rail::validate_content_ref(&peer.peer_context_ref)?;
        let is_canonical = peer
            .public_endpoint_identity
            .strip_prefix("iroh:")
            .and_then(|id| id.parse::<iroh::EndpointId>().ok())
            .is_some_and(|id| format!("iroh:{id}") == peer.public_endpoint_identity);
        if !is_canonical {
            return Err(crate::error::Failure::invalid_harness(
                "cross-process admitted peer identity is not a canonical Iroh endpoint identity",
            ));
        }
        if !identities.insert(peer.public_endpoint_identity.as_str()) {
            return Err(crate::error::Failure::invalid_harness(
                "cross-process admitted peers must have distinct endpoint identities",
            ));
        }
    }
    Ok(())
}

const MAX_ADMITTED_PEER_ID_BYTES: usize = 256;

fn rejected(kind: ConnectionRejectionKind, failure: crate::error::Failure) -> ListenerPoll {
    ListenerPoll::Rejected(ConnectionRejection { kind, failure })
}

impl AdmittedPeer {
    /// Admit the peer behind a remote endpoint descriptor, for peers that dial with their listener
    /// key, under this listener's own `local_peer_context_ref`. The remote descriptor's expected
    /// context names what *that* listener requires of its peers, so it is never copied here.
    pub fn for_endpoint(endpoint: &Endpoint, local_peer_context_ref: &str) -> Self {
        Self {
            public_endpoint_identity: endpoint.descriptor.public_endpoint_identity.clone(),
            peer_context_ref: local_peer_context_ref.to_string(),
        }
    }
}

impl EndpointCapability {
    /// The public endpoint identity (`iroh:<EndpointId>`) this capability authenticates as.
    pub fn public_endpoint_identity(&self) -> String {
        format!("iroh:{}", self.secret_key.public())
    }
}

fn validate_client_input(input: &ClientInput, payload: &[u8]) -> crate::error::Result<()> {
    crate::preserves_rail::validate_content_ref(input.capability.capability_ref())?;
    validate_exchange_refs(&input.session_ref, &input.request_ref)?;
    if payload.is_empty() {
        return Err(crate::error::Failure::invalid_harness("cross-process payload must not be empty"));
    }
    if !input.bind_addr.ip().is_loopback() {
        return Err(crate::error::Failure::invalid_harness(
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
        if !crate::fabric_transport::is_exportable_hint(address) {
            continue;
        }
        let class = match address {
            iroh::TransportAddr::Ip(_) => EndpointLocatorClass::Ip,
            iroh::TransportAddr::Relay(_) => EndpointLocatorClass::Relay,
            iroh::TransportAddr::Custom(_) => {
                return Err(crate::error::Failure::invalid_harness(
                    "custom Iroh transport addresses are outside the admitted profile",
                ));
            }
            _ => {
                return Err(crate::error::Failure::invalid_harness(
                    "unknown Iroh transport address is outside the admitted profile",
                ));
            }
        };
        if locators.len() >= MAX_ENDPOINT_LOCATORS {
            return Err(crate::error::Failure::invalid_harness("Iroh endpoint locator count exceeds bound"));
        }
        locators.push(EndpointLocator {
            class,
            value: address.to_string(),
        });
    }
    Ok(locators)
}

fn endpoint_addr(plan: &EndpointDialPlan) -> crate::error::Result<iroh::EndpointAddr> {
    let endpoint_id = plan
        .public_endpoint_identity
        .strip_prefix("iroh:")
        .ok_or_else(|| {
            crate::error::Failure::invalid_harness("cross-process endpoint identity must use iroh prefix")
        })?
        .parse::<iroh::EndpointId>()
        .map_err(error)?;
    let mut addresses = Vec::with_capacity(plan.locators.len());
    for locator in &plan.locators {
        let address = match locator.class {
            EndpointLocatorClass::Ip => {
                let address = locator.value.strip_prefix("ip:").ok_or_else(|| {
                    crate::error::Failure::invalid_harness("cross-process IP locator prefix mismatch")
                })?;
                iroh::TransportAddr::Ip(crate::fabric_transport::parse_ip_hint(address)?)
            }
            EndpointLocatorClass::Relay => {
                let relay = locator
                    .value
                    .strip_prefix("relay:")
                    .ok_or_else(|| {
                        crate::error::Failure::invalid_harness("cross-process relay locator prefix mismatch")
                    })?
                    .parse::<iroh::RelayUrl>()
                    .map_err(error)?;
                iroh::TransportAddr::Relay(relay)
            }
            EndpointLocatorClass::Custom | EndpointLocatorClass::Private => {
                return Err(crate::error::Failure::invalid_harness(
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

pub fn frame_ref(request_ref: &str, payload: &[u8]) -> String {
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

fn error(error: impl std::fmt::Display) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(format!("cross-process Iroh transport failed: {error}"))
}

fn validation_error(label: &str, issues: &impl std::fmt::Debug) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(format!("{label} denied: {issues:?}"))
}
