pub(super) mod sizing;

pub(super) struct Cohort {
    pub reservations: Vec<std::net::UdpSocket>,
    pub keys: Vec<iroh::SecretKey>,
    pub addresses: Vec<std::net::SocketAddr>,
    pub nodes: Vec<String>,
    pub mechanisms: molten::live_reliability::service::profiles::Experimental,
    endpoints: Vec<molten::fabric_transport::Endpoint>,
}

impl Cohort {
    pub(super) fn reserve(max_frame_bytes: Option<u64>) -> super::Result<Self> {
        let mechanisms = super::mechanisms::bounded_transport(super::SERVICE, max_frame_bytes)?;
        let profile = mechanisms.transport.clone().into_profile()?;
        let protocol = mechanisms.protocol.clone().into_descriptor(&profile)?;
        let mut cohort = Self {
            reservations: Vec::with_capacity(3),
            keys: Vec::with_capacity(3),
            addresses: Vec::with_capacity(3),
            nodes: Vec::with_capacity(3),
            endpoints: Vec::with_capacity(3),
            mechanisms,
        };
        for _ in 0..3 {
            let socket = std::net::UdpSocket::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
            let address = socket.local_addr()?;
            let key = iroh::SecretKey::generate();
            let node = super::reference(&format!("collaboration-native-node:{}", key.public()));
            let endpoint = endpoint(&profile, &protocol, &key, address)?;
            cohort.reservations.push(socket);
            cohort.addresses.push(address);
            cohort.keys.push(key);
            cohort.nodes.push(node);
            cohort.endpoints.push(endpoint);
        }
        Ok(cohort)
    }

    pub(super) fn configuration(
        &self,
        source: &super::Source,
        index: usize,
    ) -> super::Result<molten::live_reliability::service::Transport> {
        source.root.secrets()?.write_restricted(
            &molten::node_state::RelativePath::parse("collaboration-transport.json")?,
            &serde_json::to_vec(&self.keys[index].to_bytes())?,
            0o600,
        )?;
        let mechanisms = &self.mechanisms;
        let local = &self.endpoints[index].descriptor;
        let mut peers = std::collections::BTreeMap::new();
        for remote in 0..3 {
            if remote == index {
                continue;
            }
            if peers.len() >= super::MAX_PEERS {
                return Err("transport peer set exceeds the admitted bound".into());
            }
            let endpoint = &self.endpoints[remote];
            peers.insert(self.nodes[remote].clone(), molten::live_reliability::service::Peer {
                endpoint: molten::preserves_rail::canonical_bytes(&endpoint.value)?,
                expected: expected(endpoint),
                session_ref: super::reference(&format!("{}:{}:session", self.nodes[index], self.nodes[remote])),
                request_ref: super::reference(&format!("{}:{}:request", self.nodes[index], self.nodes[remote])),
            });
        }
        Ok(molten::live_reliability::service::Transport {
            profile: mechanisms.transport.clone(),
            protocol: mechanisms.protocol.clone(),
            key_file: "collaboration-transport.json".into(),
            capability_ref: super::reference(&format!("transport:{}", self.keys[index].public())),
            bind_addr: self.addresses[index],
            client_bind_addr: (std::net::Ipv4Addr::LOCALHOST, 0).into(),
            listener_identity_ref: local.listener_identity_ref.clone(),
            peer_context_ref: local.expected_peer_context_ref.clone(),
            locator_cohort_ref: local.locator_cohort_ref.clone(),
            disclosure: molten::live_reliability::service::profiles::EndpointDisclosurePolicyDto {
                explicit_handoff_classes: vec!["ip".into()],
                default_readback_redacted: true,
            },
            validity: molten::live_reliability::service::profiles::EndpointValidityCohortDto {
                cohort_ref: local.validity.cohort_ref.clone(),
                not_before_tick: 1,
                expires_at_tick: 100_000,
            },
            admission: molten::live_reliability::service::profiles::EndpointAdmissionStateDto {
                registration_active: true,
                transport_capability_active: true,
                protocol_capability_active: true,
                profile_active: true,
                listener_ready: true,
            },
            observed_tick: 10,
            local_descriptor_ref: self.endpoints[index].descriptor_ref.clone(),
            peers,
            timeout_ms: 5000,
        })
    }
}

fn endpoint(
    profile: &molten::fabric_transport::Profile,
    protocol: &molten::fabric_transport::ProtocolDescriptor,
    key: &iroh::SecretKey,
    address: std::net::SocketAddr,
) -> super::Result<molten::fabric_transport::Endpoint> {
    Ok(molten::fabric_transport::endpoint(
        &profile.profile,
        protocol,
        &molten::fabric_transport::EndpointDescriptorBindings {
            public_endpoint_identity: format!("iroh:{}", key.public()),
            listener_identity_ref: super::reference(&format!("listener:{}", key.public())),
            expected_peer_context_ref: super::reference("collaboration-native-peer-context"),
            locator_cohort_ref: super::reference("collaboration-native-loopback-cohort"),
            locators: vec![molten::fabric_transport::EndpointLocator {
                class: molten::fabric_transport::EndpointLocatorClass::Ip,
                value: iroh::TransportAddr::Ip(address).to_string(),
            }],
            disclosure: molten::fabric_transport::EndpointDisclosurePolicy {
                explicit_handoff_classes: vec![molten::fabric_transport::EndpointLocatorClass::Ip],
                default_readback_redacted: true,
            },
            resources: molten::fabric_transport::EndpointResourceBounds {
                max_sessions: profile.profile.limits.max_sessions,
                max_frame_bytes: protocol.framing.max_frame_bytes,
                max_queued_bytes: profile.profile.limits.max_queued_bytes,
                max_inflight_bytes: profile.profile.limits.max_inflight_bytes,
            },
            validity: molten::fabric_transport::EndpointValidityCohort {
                cohort_ref: super::reference("collaboration-native-validity"),
                not_before_tick: 1,
                expires_at_tick: 100_000,
            },
        },
    )?)
}

fn expected(
    endpoint: &molten::fabric_transport::Endpoint,
) -> molten::live_reliability::service::profiles::ExpectedEndpointBindingDto {
    let descriptor = &endpoint.descriptor;
    molten::live_reliability::service::profiles::ExpectedEndpointBindingDto {
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
        observed_tick: 10,
    }
}
