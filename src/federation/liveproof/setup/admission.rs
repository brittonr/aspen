pub fn manifest(
    policy: &crate::mesh::admission::rules::LocalPolicy,
    proofset: &crate::capability_tokens::CapabilityProofset,
    signer: &str,
    crypto: &crate::fabric_crypto_identity::Profile,
    dag_conformance_ref: &str,
) -> crate::error::Result<crate::mesh::admission::Manifest> {
    crate::mesh::admission::admit_manifest(manifest_input(policy, proofset, signer, crypto, dag_conformance_ref)?)
}

/// The live receiver's manifest input, binding the owner-admitted DAG descriptor it resolved.
pub fn manifest_input(
    policy: &crate::mesh::admission::rules::LocalPolicy,
    proofset: &crate::capability_tokens::CapabilityProofset,
    signer: &str,
    crypto: &crate::fabric_crypto_identity::Profile,
    dag_conformance_ref: &str,
) -> crate::error::Result<crate::mesh::admission::ManifestInput> {
    let time = super::profiles::time()?;
    let descriptors = descriptors(crypto, &time, dag_conformance_ref)?;
    let policy_ref = crate::preserves_rail::canonical_hash(&crate::mesh::admission::rules::policy_value(policy)?)?;
    let peer_ref = super::transport::reference("federation-live-provider");
    let peers = vec![molten_core::federation::PeerHint {
        peer_ref,
        signer_ref: signer.to_string(),
        source: molten_core::federation::LocatorSource::Static,
        observed_at: 1,
        expires_at: super::transport::UNTIL,
        available: true,
    }];
    let peer_configuration_ref = crate::mesh::admission::peer_configuration_ref(&peers)?;
    let resources = policy.resource_profile_ref.clone();
    let bounds = bounds();
    let rate = crate::mesh::admission::rate_profile_ref(&bounds)?;
    let evidence = super::transport::reference("federation-live-detached-evidence-v1");
    let policies = vec![
        policy_ref.clone(),
        peer_configuration_ref.clone(),
        resources.clone(),
        rate.clone(),
        time.profile_ref,
    ];
    let extension = admit_extension(&descriptors, proofset, policies, &evidence)?;
    let dag_binding_ref = extension
        .required_port_bindings()
        .iter()
        .find(|binding| binding.binding.key.port_id == crate::dag_sync::DAG_SYNC_PORT_ID)
        .ok_or_else(|| super::super::invalid("admitted extension omitted DAG binding"))?
        .binding_ref
        .clone();
    Ok(crate::mesh::admission::ManifestInput {
        extension,
        generation: 1,
        peers,
        bounds,
        dag_binding_ref,
        peer_configuration_ref,
        local_admission_policy_ref: policy_ref,
        resource_profile_ref: resources,
        rate_profile_ref: rate,
        evidence_profile_ref: evidence,
    })
}
fn descriptors(
    crypto: &crate::fabric_crypto_identity::Profile,
    time: &crate::fabric_time::Profile,
    dag_conformance_ref: &str,
) -> crate::error::Result<Vec<crate::fabric::FabricPortDescriptor>> {
    let mut descriptors = crate::fabric_crypto_identity::fabric_crypto_port_descriptors(crypto);
    descriptors.push(crate::fabric_transport::fabric_transport_port_descriptor(&super::transport::profile()?));
    descriptors.extend(crate::content_store_adapter::port_descriptors(&super::profiles::content()?.profile_ref));
    descriptors.extend(crate::fabric_time::port_descriptors(time));
    descriptors.extend(crate::fabric_observability::port_descriptors(&super::transport::reference(
        "federation-live-observation",
    )));
    let dag_profile =
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("federation-live-dag-profile-v1", vec![
            crate::preserves_rail::u64_value(1),
            crate::preserves_rail::u64_value(super::transport::LIMIT),
            crate::preserves_rail::string(
                "peer-partitioned; capability-authorized; durable-progress; verified-content",
            ),
        ]))?;
    descriptors.push(crate::dag_sync::dag_sync_port_descriptor(&dag_profile, &[dag_conformance_ref.to_string()])?);
    Ok(descriptors)
}
fn bounds() -> molten_core::federation::PullBounds {
    molten_core::federation::PullBounds {
        inventory_entries: 1,
        inventory_bytes: super::transport::LIMIT,
        fetch_bytes: super::transport::LIMIT,
        dag_nodes: 1,
        content_chunks: 8,
        concurrent_sessions: 1,
        query_interval: 1,
        freshness: 100,
        session_ticks: 100,
        retries: 0,
        retained_status: 16,
    }
}
fn admit_extension(
    descriptors: &[crate::fabric::FabricPortDescriptor],
    proofset: &crate::capability_tokens::CapabilityProofset,
    policies: Vec<String>,
    evidence: &str,
) -> crate::error::Result<crate::system_extension::AdmittedManifest> {
    let tier = crate::fabric::canonical_extension_tier_admission(&crate::fabric::ExtensionTierRequest {
        tier: crate::fabric::ExtensionTier::SystemExtension,
        requested_authorities: authorities(descriptors),
        admission_evidence: crate::fabric::REQUIRED_SYSTEM_EXTENSION_EVIDENCE.to_vec(),
    })?;
    let input = crate::system_extension::SystemExtensionManifestInput {
        schema: crate::system_extension::SYSTEM_EXTENSION_MANIFEST_SCHEMA.to_string(),
        extension_id: "federated-pull-live".to_string(),
        service_id: "federated-pull-live".to_string(),
        implementation_ref: super::transport::reference(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/federation/lifecycle/extension.rs"
        ))),
        callback_groups: callback_groups(),
        required_ports: descriptors.iter().map(requirement).collect(),
        optional_ports: Vec::new(),
        capability_refs: vec![crate::preserves_rail::canonical_hash(
            &crate::capability_tokens::capability_proofset_value(proofset)?,
        )?],
        policy_refs: policies,
        provenance_refs: vec![super::transport::reference(include_str!("admission.rs"))],
        resources: resources(),
        recovery_group: None,
        execution_profile: crate::system_extension::ExecutionProfile::InProcessNative,
        state_schema: "molten.federation.callback-state.v1".to_string(),
        compatible_state_schemas: vec!["molten.federation.callback-state.v1".to_string()],
        evidence_profile_ref: evidence.to_string(),
        initial_generation: 1,
        non_claims: crate::system_extension::REQUIRED_SYSTEM_EXTENSION_NON_CLAIMS.to_vec(),
    };
    crate::system_extension::admit_manifest(&input, descriptors, &tier, &[
        crate::system_extension::ExecutionProfile::InProcessNative,
    ])
}
fn authorities(descriptors: &[crate::fabric::FabricPortDescriptor]) -> Vec<crate::fabric::FabricAuthority> {
    let mut authorities = descriptors
        .iter()
        .flat_map(|descriptor| descriptor.authority_requirements.clone())
        .collect::<Vec<_>>();
    authorities.sort();
    authorities.dedup();
    for authority in [
        crate::fabric::FabricAuthority::Supervision,
        crate::fabric::FabricAuthority::Evidence,
    ] {
        if !authorities.contains(&authority) {
            authorities.push(authority);
        }
    }
    authorities
}
fn callback_groups() -> Vec<String> {
    [
        "initialize",
        "start",
        "request",
        "health",
        "checkpoint",
        "recover",
        "drain",
        "shutdown",
    ]
    .iter()
    .map(|value| value.to_string())
    .collect()
}
fn resources() -> crate::system_extension::ResourceEnvelope {
    crate::system_extension::ResourceEnvelope {
        max_concurrent_callbacks: 1,
        max_queued_events: 8,
        max_inflight_bytes: super::transport::LIMIT * 2,
        max_open_streams: 8,
        max_timers: 8,
        max_effect_requests: 8,
        callback_deadline_ticks: 100,
        shutdown_grace_ticks: 100,
        max_restart_attempts: 1,
        overload_policy: crate::system_extension::OverloadPolicy::UpstreamBackpressure,
    }
}
fn requirement(descriptor: &crate::fabric::FabricPortDescriptor) -> crate::fabric::FabricPortRequirement {
    crate::fabric::FabricPortRequirement {
        port_id: descriptor.port_id.clone(),
        version: descriptor.version.clone(),
        class: descriptor.class,
        operation_classes: descriptor.operation_classes.clone(),
        input_schema_refs: descriptor.input_schema_refs.clone(),
        output_schema_refs: descriptor.output_schema_refs.clone(),
        allowed_authorities: descriptor.authority_requirements.clone(),
        available_resources: descriptor.resource_requirements.clone(),
        expected_determinism: descriptor.determinism,
        expected_replay: descriptor.replay,
        expected_profile: descriptor.implementation_profile.clone(),
    }
}
