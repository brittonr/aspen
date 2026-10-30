pub(super) type Endpoints = std::collections::BTreeMap<String, crate::fabric_transport::cross_process::Endpoint>;

pub(super) fn endpoints(
    input: &super::Input,
    mechanisms: &super::super::profiles::Experimental,
) -> crate::error::Result<Endpoints> {
    let profile = mechanisms.transport.clone().into_profile()?;
    let protocol = mechanisms.protocol.clone().into_descriptor(&profile)?;
    input
        .nodes
        .iter()
        .map(|node| {
            let public: iroh::PublicKey =
                node.transport_public_key.parse().map_err(|_| super::invalid("invalid transport public key"))?;
            let endpoint = crate::fabric_transport::cross_process::endpoint(
                &profile.profile,
                &protocol,
                &crate::fabric_transport::cross_process::EndpointDescriptorBindings {
                    public_endpoint_identity: format!("iroh:{public}"),
                    listener_identity_ref: node.listener_identity_ref.clone(),
                    expected_peer_context_ref: node.peer_context_ref.clone(),
                    locator_cohort_ref: node.locator_cohort_ref.clone(),
                    locators: vec![crate::fabric_transport::EndpointLocator {
                        class: crate::fabric_transport::EndpointLocatorClass::Ip,
                        value: iroh::TransportAddr::Ip(node.bind_addr).to_string(),
                    }],
                    disclosure: crate::fabric_transport::EndpointDisclosurePolicy {
                        explicit_handoff_classes: vec![crate::fabric_transport::EndpointLocatorClass::Ip],
                        default_readback_redacted: true,
                    },
                    resources: crate::fabric_transport::EndpointResourceBounds {
                        max_sessions: profile.profile.limits.max_sessions,
                        max_frame_bytes: protocol.framing.max_frame_bytes,
                        max_queued_bytes: profile.profile.limits.max_queued_bytes,
                        max_inflight_bytes: profile.profile.limits.max_inflight_bytes,
                    },
                    validity: node.validity.clone().into_cohort()?,
                },
            )?;
            Ok((node.node_id.clone(), endpoint))
        })
        .collect()
}

pub(super) fn configuration(
    input: &super::Input,
    local: &super::NodeInput,
    mechanisms: &super::super::profiles::Experimental,
    endpoints: &Endpoints,
) -> crate::error::Result<super::super::network::Transport> {
    let peers = input
        .nodes
        .iter()
        .filter(|node| node.node_id != local.node_id)
        .map(|node| {
            let endpoint = &endpoints[&node.node_id];
            let descriptor = &endpoint.descriptor;
            let expected = super::super::profiles::ExpectedEndpointBindingDto {
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
                observed_tick: local.observed_tick,
            };
            let profile = mechanisms.transport.clone().into_profile()?;
            let protocol = mechanisms.protocol.clone().into_descriptor(&profile)?;
            crate::fabric_transport::admit_endpoint_import(
                &profile.profile,
                &protocol,
                descriptor,
                &expected.clone().into_binding()?,
                local.transport_admission.clone().into_state()?,
            )
            .map_err(|issues| super::invalid(format!("peer endpoint admission: {issues:?}")))?;
            let request_ref = super::identity(&serde_json::json!({ "schema": "molten.experimental-peer-request.v1",
            "local": local.node_id, "remote": node.node_id, "endpoint": endpoint.descriptor_ref,
            "session": node.transport_session_ref }))?;
            Ok((node.node_id.clone(), super::super::network::Peer {
                endpoint: crate::preserves_rail::canonical_bytes(&endpoint.value)?,
                expected,
                session_ref: node.transport_session_ref.clone(),
                request_ref,
            }))
        })
        .collect::<crate::error::Result<std::collections::BTreeMap<_, _>>>()?;
    Ok(super::super::network::Transport {
        profile: mechanisms.transport.clone(),
        protocol: mechanisms.protocol.clone(),
        key_file: local.transport_key_file.clone(),
        capability_ref: local.transport_capability_ref.clone(),
        bind_addr: local.bind_addr,
        client_bind_addr: local.client_bind_addr,
        listener_identity_ref: local.listener_identity_ref.clone(),
        peer_context_ref: local.peer_context_ref.clone(),
        locator_cohort_ref: local.locator_cohort_ref.clone(),
        disclosure: super::super::profiles::EndpointDisclosurePolicyDto {
            explicit_handoff_classes: vec!["ip".into()],
            default_readback_redacted: true,
        },
        validity: local.validity.clone(),
        admission: local.transport_admission.clone(),
        observed_tick: local.observed_tick,
        local_descriptor_ref: endpoints[&local.node_id].descriptor_ref.clone(),
        peers,
        timeout_ms: 5000,
    })
}
