pub(super) fn validate(input: &super::Input) -> crate::error::Result<()> {
    let transport = &input.transport;
    let profile = transport.profile.clone().into_profile()?;
    let protocol = transport.protocol.clone().into_descriptor(&profile)?;
    if profile.profile.adapter_kind != crate::fabric_transport::TransportAdapterKind::IrohLive
        || protocol.service_id != input.group.service_id
        || protocol.extension_id != input.group.extension_id
        || protocol.generation != input.group.service_generation
        || transport.peers.len() != 2
    {
        return Err(super::codec::invalid("transport is not the exact live group protocol"));
    }
    let admission = transport.admission.clone().into_state()?;
    if !admission.registration_active
        || !admission.transport_capability_active
        || !admission.protocol_capability_active
        || !admission.profile_active
        || !admission.listener_ready
    {
        return Err(super::codec::invalid("transport endpoint admissions are inactive"));
    }
    transport.disclosure.clone().into_policy()?;
    transport.validity.clone().into_cohort()?;
    for peer in transport.peers.values() {
        if peer.endpoint.len() > 65536 {
            return Err(super::codec::invalid("peer endpoint bound"));
        }
        let value = crate::preserves_rail::parse_canonical_bytes(&peer.endpoint)?;
        crate::fabric_transport::cross_process::parse_endpoint(&value)?;
        peer.expected.clone().into_binding()?;
        crate::preserves_rail::validate_content_ref(&peer.session_ref)?;
        crate::preserves_rail::validate_content_ref(&peer.request_ref)?;
    }
    Ok(())
}

pub(super) async fn open(
    root: &crate::node_state::DirectoryRoot,
    input: &super::Input,
) -> crate::error::Result<(crate::fabric_transport::Listener, crate::fabric_consistency::raft::ReplicaTransportPort)> {
    let transport = &input.transport;
    let capability = capability(root, transport)?;
    let profile = transport.profile.clone().into_profile()?;
    let protocol = transport.protocol.clone().into_descriptor(&profile)?;
    register(input, &profile, &protocol)?;
    let admission = transport.admission.clone().into_state()?;
    let listener = listener(transport, &profile, &protocol, &capability).await?;
    // One client per admitted peer: the map is exactly as large as the validated peer set.
    let peers = transport
        .peers
        .iter()
        .map(|(node, peer)| {
            let value = crate::preserves_rail::parse_canonical_bytes(&peer.endpoint)?;
            Ok((node.clone(), crate::fabric_transport::ClientInput {
                profile: profile.clone(),
                protocol: protocol.clone(),
                capability: capability.clone(),
                bind_addr: transport.client_bind_addr,
                endpoint: crate::fabric_transport::cross_process::parse_endpoint(&value)?,
                expected: peer.expected.clone().into_binding()?,
                admission,
                session_ref: peer.session_ref.clone(),
                request_ref: peer.request_ref.clone(),
            }))
        })
        .collect::<crate::error::Result<std::collections::BTreeMap<_, _>>>()?;
    let port = crate::fabric_consistency::raft::ReplicaTransportPort::new(
        input.replica.protocol_ref.clone(),
        peers,
        std::time::Duration::from_millis(transport.timeout_ms),
    )?;
    Ok((listener, port))
}

fn capability(
    root: &crate::node_state::DirectoryRoot,
    transport: &crate::live_reliability::service::Transport,
) -> crate::error::Result<crate::fabric_transport::EndpointCapability> {
    let secrets = root.secrets()?;
    let path = crate::node_state::RelativePath::parse(&transport.key_file)?;
    if secrets.unix_mode(&path)?.is_none_or(|mode| mode & 0o777 != 0o600) {
        return Err(super::codec::invalid("transport key requires owner-only mode 0600"));
    }
    let bytes: [u8; 32] = serde_json::from_slice(&secrets.read(&path, 4096)?).map_err(super::codec::invalid)?;
    crate::fabric_transport::EndpointCapability::from_secret_bytes(bytes, transport.capability_ref.clone())
}

fn register(
    input: &super::Input,
    profile: &crate::fabric_transport::Profile,
    protocol: &crate::fabric_transport::ProtocolDescriptor,
) -> crate::error::Result<()> {
    let mut registration = crate::fabric_transport::IrohAdapter::new(profile.clone())?;
    crate::fabric_transport::TransportCommandShell::execute_command(
        &mut registration,
        &crate::fabric_transport::TransportCommand::Register {
            operation_id: input.activation.request_ref.clone(),
            descriptor: protocol.clone(),
        },
    )
    .map_err(super::codec::invalid)?;
    if !registration.state().protocols.values().any(|registered| {
        registered.descriptor == *protocol
            && registered.phase == crate::fabric_transport::ProtocolRegistrationPhase::Active
    }) {
        return Err(super::codec::invalid("actual transport protocol registration denied"));
    }
    Ok(())
}

async fn listener(
    transport: &crate::live_reliability::service::Transport,
    profile: &crate::fabric_transport::Profile,
    protocol: &crate::fabric_transport::ProtocolDescriptor,
    capability: &crate::fabric_transport::EndpointCapability,
) -> crate::error::Result<crate::fabric_transport::Listener> {
    let listener = crate::fabric_transport::Listener::bind(crate::fabric_transport::ListenerInput {
        profile: profile.clone(),
        protocol: protocol.clone(),
        capability: capability.clone(),
        bind_addr: transport.bind_addr,
        listener_identity_ref: transport.listener_identity_ref.clone(),
        expected_peer_context_ref: transport.peer_context_ref.clone(),
        locator_cohort_ref: transport.locator_cohort_ref.clone(),
        disclosure: transport.disclosure.clone().into_policy()?,
        validity: transport.validity.clone().into_cohort()?,
        admission: transport.admission.clone().into_state()?,
        observed_tick: transport.observed_tick,
    })
    .await?;
    if listener.handoff().descriptor_ref != transport.local_descriptor_ref
        || listener.state().phase != crate::fabric_transport::CrossProcessListenerPhase::Ready
    {
        return Err(super::codec::invalid("actual listener differs from admitted descriptor"));
    }
    Ok(listener)
}
