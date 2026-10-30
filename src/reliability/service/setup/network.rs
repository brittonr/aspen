pub struct Runtime {
    pub listener: crate::fabric_transport::Listener,
    pub port: crate::fabric_consistency::raft::ReplicaTransportPort,
    pub registration: crate::fabric_transport::IrohAdapter,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transport {
    pub profile: super::profiles::TransportProfileDto,
    pub protocol: super::profiles::ProtocolDescriptorDto,
    pub key_file: String,
    pub capability_ref: String,
    pub bind_addr: std::net::SocketAddr,
    pub client_bind_addr: std::net::SocketAddr,
    pub listener_identity_ref: String,
    pub peer_context_ref: String,
    pub locator_cohort_ref: String,
    pub disclosure: super::profiles::EndpointDisclosurePolicyDto,
    pub validity: super::profiles::EndpointValidityCohortDto,
    pub admission: super::profiles::EndpointAdmissionStateDto,
    pub observed_tick: u64,
    pub local_descriptor_ref: String,
    pub peers: std::collections::BTreeMap<String, Peer>,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Peer {
    pub endpoint: Vec<u8>,
    pub expected: super::profiles::ExpectedEndpointBindingDto,
    pub session_ref: String,
    pub request_ref: String,
}

impl Transport {
    pub async fn open(
        &self,
        authority: &super::access::Authorization,
        protocol_ref: &str,
    ) -> crate::error::Result<Runtime> {
        authority.require("serve")?;
        if self.peers.len() != 2 || !(1..=30_000).contains(&self.timeout_ms) {
            return Err(crate::error::Failure::invalid_harness("transport requires exactly two bounded peers"));
        }
        let capability = self.capability(authority)?;
        let profile = self.profile.clone().into_profile()?;
        let protocol = self.protocol.clone().into_descriptor(&profile)?;
        let admission = self.admission.clone().into_state()?;
        let mut registration = crate::fabric_transport::IrohAdapter::new(profile.clone())?;
        crate::fabric_transport::TransportCommandShell::execute_command(
            &mut registration,
            &crate::fabric_transport::TransportCommand::Register {
                operation_id: authority.grant.grant_ref().to_string(),
                descriptor: protocol.clone(),
            },
        )
        .map_err(|error| crate::error::Failure::invalid_harness(format!("protocol registration failed: {error}")))?;
        if !registration.state().protocols.values().any(|registered| {
            registered.descriptor == protocol
                && registered.phase == crate::fabric_transport::ProtocolRegistrationPhase::Active
        }) || !admission.registration_active
            || !admission.transport_capability_active
            || !admission.protocol_capability_active
            || !admission.profile_active
            || !admission.listener_ready
        {
            return Err(crate::error::Failure::invalid_harness("protocol registration is not actively admitted"));
        }
        let listener = crate::fabric_transport::Listener::bind(crate::fabric_transport::ListenerInput {
            profile: profile.clone(),
            protocol: protocol.clone(),
            capability: capability.clone(),
            bind_addr: self.bind_addr,
            listener_identity_ref: self.listener_identity_ref.clone(),
            expected_peer_context_ref: self.peer_context_ref.clone(),
            locator_cohort_ref: self.locator_cohort_ref.clone(),
            disclosure: self.disclosure.clone().into_policy()?,
            validity: self.validity.clone().into_cohort()?,
            admission,
            observed_tick: self.observed_tick,
        })
        .await?;
        if listener.handoff().descriptor_ref != self.local_descriptor_ref {
            return Err(crate::error::Failure::invalid_harness(
                "live listener descriptor differs from signed configuration",
            ));
        }
        if listener.state().phase != crate::fabric_transport::CrossProcessListenerPhase::Ready {
            return Err(crate::error::Failure::invalid_harness("bound transport listener is not ready"));
        }
        let peers = self
            .peers
            .iter()
            .map(|(node, peer)| {
                if peer.endpoint.len() > 65_536 {
                    return Err(crate::error::Failure::invalid_harness("peer endpoint frame bound"));
                }
                let value = crate::preserves_rail::parse_canonical_bytes(&peer.endpoint)?;
                let endpoint = crate::fabric_transport::cross_process::parse_endpoint(&value)?;
                Ok((node.clone(), crate::fabric_transport::ClientInput {
                    profile: profile.clone(),
                    protocol: protocol.clone(),
                    capability: capability.clone(),
                    bind_addr: self.client_bind_addr,
                    endpoint,
                    expected: peer.expected.clone().into_binding()?,
                    admission,
                    session_ref: peer.session_ref.clone(),
                    request_ref: peer.request_ref.clone(),
                }))
            })
            .collect::<crate::error::Result<std::collections::BTreeMap<_, _>>>()?;
        let transport = crate::fabric_consistency::raft::ReplicaTransportPort::new(
            protocol_ref.to_string(),
            peers,
            std::time::Duration::from_millis(self.timeout_ms),
        )?;
        Ok(Runtime {
            listener,
            port: transport,
            registration,
        })
    }

    fn capability(
        &self,
        authority: &super::access::Authorization,
    ) -> crate::error::Result<crate::fabric_transport::EndpointCapability> {
        let root = authority.root.secrets()?;
        let path = crate::node_state::RelativePath::parse(&self.key_file)?;
        if root.unix_mode(&path)?.is_none_or(|mode| mode & 0o777 != 0o600) {
            return Err(crate::error::Failure::invalid_harness("transport key must be owner-only mode 0600"));
        }
        let bytes: [u8; 32] = serde_json::from_slice(&root.read(&path, 4096)?)
            .map_err(|error| crate::error::Failure::invalid_harness(error.to_string()))?;
        let key = iroh::SecretKey::from_bytes(&bytes);
        let grant = authority.grant.grant();
        if key.public().to_string() == grant.probe_public_key
            || key.public().to_string() == grant.client_public_key
            || key.public().to_string() == authority.grant.operator_public_key()
        {
            return Err(crate::error::Failure::invalid_harness(
                "transport key must be purpose-separated from probe and client",
            ));
        }
        crate::fabric_transport::EndpointCapability::from_secret_bytes(bytes, self.capability_ref.clone())
    }
}
