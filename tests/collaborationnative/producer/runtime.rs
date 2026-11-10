pub(super) fn build(
    source: &super::Source,
    service: &molten::node_service_host::codec::ExtensionInput,
    cohort: &super::transport::Cohort,
    index: usize,
) -> super::Result<(
    molten::collaboration_scope::native_runtime::Input,
    molten::node_service_host::effects::config::Configuration,
)> {
    let manifest = service.admit_manifest()?;
    let membership_ref = membership(cohort)?;
    let group = group(service, manifest.manifest_ref(), membership_ref.clone());
    let activation = molten::collaboration_scope::native_runtime::Activation {
        request_ref: super::reference("collaboration-native-group-attach-request"),
        authority_refs: vec![service.authority.authority_ref.clone()],
        result_ref: super::reference("collaboration-native-group-attach-result"),
        evidence_refs: vec![source.validation.selected_output_validation_ref.clone()],
    };
    let active = super::activation::activate(&group, &activation)?;
    let mechanisms = &cohort.mechanisms;
    let time = molten::live_reliability::service::Time {
        profile: mechanisms.time.clone(),
        capability_ref: super::reference("collaboration-logical-time-capability"),
        tick_ms: 10,
    };
    let installation_ref = installation(&manifest, service)?;
    let bindings = bindings(&manifest);
    let input = input(source, service, cohort, index, Parts {
        group,
        activation,
        membership_ref,
        active: &active,
        mechanisms,
        time,
        installation_ref,
        bindings,
    })?;
    let authority =
        molten::preserves_rail::record("collaboration-native-authority-v1", vec![molten::preserves_rail::string(
            "operator-scope-authority",
        )]);
    let bytes = molten::collaboration_scope::native_runtime::encode(&input, &authority)?;
    let runtime_ref = molten::preserves_rail::content_ref_from_bytes(&bytes);
    source.root.write_new(&molten::node_state::RelativePath::parse(super::RUNTIME)?, &bytes)?;
    let effects = effects(service, &manifest, &input, runtime_ref)?;
    Ok((input, effects))
}

/// The declared cohort's exact membership identity.
fn membership(cohort: &super::transport::Cohort) -> super::Result<String> {
    molten::preserves_rail::canonical_hash(&molten::preserves_rail::record("collaboration-native-membership-v1", vec![
        molten::preserves_rail::sequence(cohort.nodes.iter().map(molten::preserves_rail::string).collect()),
    ]))
    .map_err(Into::into)
}

/// The installation identity binds service, manifest, generation, and exact required bindings.
fn installation(
    manifest: &molten::system_extension::AdmittedManifest,
    service: &molten::node_service_host::codec::ExtensionInput,
) -> super::Result<String> {
    molten::preserves_rail::canonical_hash(&molten::preserves_rail::record("collaboration-installation-v1", vec![
        molten::preserves_rail::string(super::SERVICE),
        molten::preserves_rail::string(manifest.manifest_ref()),
        molten::preserves_rail::u64_value(1),
        molten::preserves_rail::u64_value(service.manifest.resources.max_inflight_bytes),
        molten::preserves_rail::sequence(
            manifest
                .required_port_bindings()
                .iter()
                .map(|binding| molten::preserves_rail::string(&binding.binding_ref))
                .collect(),
        ),
    ]))
    .map_err(Into::into)
}

/// The admitted port bindings, bounded by the declared manifest.
fn bindings(
    manifest: &molten::system_extension::AdmittedManifest,
) -> Vec<molten::live_reliability::service::PortBinding> {
    manifest
        .required_port_bindings()
        .iter()
        .filter(|binding| {
            [
                molten::fabric_transport::FABRIC_TRANSPORT_PORT_ID,
                molten::fabric_durability::FABRIC_DURABLE_LOG_PORT_ID,
                molten::fabric_durability::FABRIC_SNAPSHOT_PORT_ID,
                molten::fabric_time::FABRIC_TIMER_PORT_ID,
                molten::fabric_time::FABRIC_ENTROPY_PORT_ID,
                molten::fabric_membership::FABRIC_MEMBERSHIP_PORT_ID,
                molten::fabric_membership::FABRIC_PLACEMENT_PORT_ID,
            ]
            .contains(&binding.binding.key.port_id.as_str())
        })
        .map(|binding| molten::live_reliability::service::PortBinding {
            port_id: binding.binding.key.port_id.clone(),
            version: binding.binding.key.version.clone(),
            implementation_profile: binding.binding.implementation_profile.clone(),
            binding_ref: binding.binding_ref.clone(),
        })
        .collect()
}

/// Everything the runtime input needs besides the node and cohort positions.
struct Parts<'a> {
    group: molten::live_reliability::service::Group,
    activation: molten::collaboration_scope::native_runtime::Activation,
    membership_ref: String,
    active: &'a molten::fabric_consistency::Group,
    mechanisms: &'a molten::live_reliability::service::profiles::Experimental,
    time: molten::live_reliability::service::Time,
    installation_ref: String,
    bindings: Vec<molten::live_reliability::service::PortBinding>,
}

/// The encoded runtime input; it names this node's exact cohort position.
fn input(
    source: &super::Source,
    service: &molten::node_service_host::codec::ExtensionInput,
    cohort: &super::transport::Cohort,
    index: usize,
    parts: Parts<'_>,
) -> super::Result<molten::collaboration_scope::native_runtime::Input> {
    let Parts {
        group,
        activation,
        membership_ref,
        active,
        mechanisms,
        time,
        installation_ref,
        bindings,
    } = parts;
    Ok(molten::collaboration_scope::native_runtime::Input {
        replica: replica(active, mechanisms)?,
        group,
        activation,
        membership: molten::live_reliability::service::Membership {
            membership_ref,
            config_epoch: 1,
            voters: cohort.nodes.clone(),
        },
        bindings,
        node_id: cohort.nodes[index].clone(),
        installation_ref: installation_ref.clone(),
        policy_ref: service.executable.policy_ref.clone(),
        resource_ref: service.executable.resource_ref.clone(),
        raft_store: store(super::SERVICE, "collaboration-raft")?,
        scope_store: store("collaboration-scope", "collaboration-scope")?,
        replay_store: store("collaboration-replay", "collaboration-replay")?,
        callback_namespace: service.cache_namespace.clone(),
        transport: cohort.configuration(source, index)?,
        time,
        handler_ref: super::reference("collaboration-native-handler"),
        session_ref: super::reference(&format!("collaboration-native-session:{}", cohort.nodes[index])),
        ingress_capacity: 64,
        ingress_delivery_limit: 65_536,
        timeout_ms: 30_000,
        genesis: Some(molten::collaboration_scope::native_runtime::Genesis {
            consistency_epoch: 1,
            revocation_ref: super::reference("collaboration-empty-genesis-revocation"),
            observation_ref: super::reference("collaboration-empty-genesis-observation"),
        }),
    })
}

fn group(
    service: &molten::node_service_host::codec::ExtensionInput,
    manifest_ref: &str,
    membership_ref: String,
) -> molten::live_reliability::service::Group {
    molten::live_reliability::service::Group {
        group_id: super::GROUP_ID.into(),
        extension_id: molten::collaboration_scope::EXTENSION_ID.into(),
        service_id: super::SERVICE.into(),
        service_generation: 1,
        application_manifest_ref: manifest_ref.into(),
        engine_algorithm_profile: molten::fabric_consistency::raft::LIVE_RAFT_ALGORITHM_PROFILE.into(),
        engine_implementation_profile: molten::fabric_consistency::raft::LIVE_RAFT_IMPLEMENTATION_PROFILE.into(),
        membership_ref,
        config_epoch: 1,
        placement_ref: super::reference("collaboration-static-loopback-placement"),
        fencing_ref: super::reference("collaboration-native-fencing"),
        fencing_epoch: 1,
        resource_profile_ref: service.executable.resource_ref.clone(),
        policy_refs: service.manifest.policy_refs.clone(),
        non_claims: vec![
            "consistency-does-not-prove-extension-semantics".to_string(),
            "local-status-is-not-quorum-currentness".to_string(),
        ],
        max_command_bytes: 65536,
        max_in_flight_operations: 64,
    }
}

fn store(profile: &str, storage: &str) -> super::Result<molten::live_reliability::service::Durability> {
    let mechanisms = super::mechanisms::durability::buffered(profile)?;
    Ok(molten::live_reliability::service::Durability {
        profile: mechanisms.durable,
        namespace: mechanisms.namespace,
        storage_namespace: storage.into(),
    })
}

fn replica(
    group: &molten::fabric_consistency::Group,
    mechanisms: &molten::live_reliability::service::profiles::Experimental,
) -> super::Result<molten::live_reliability::service::Replica> {
    let transport = mechanisms.transport.clone().into_profile()?;
    let protocol = mechanisms.protocol.clone().into_descriptor(&transport)?;
    let protocol_ref = molten::preserves_rail::canonical_hash(&molten::preserves_rail::record(
        "collaboration-native-protocol-v1",
        vec![
            molten::preserves_rail::string(&protocol.protocol_id),
            molten::preserves_rail::string(&protocol.version),
            molten::preserves_rail::string(&protocol.alpn),
            molten::preserves_rail::u64_value(protocol.generation),
        ],
    ))?;
    let time = mechanisms.time.clone().into_descriptor()?;
    let durability = mechanisms.durable.clone().into_profile()?;
    Ok(molten::live_reliability::service::Replica {
        profile_ref: super::reference("collaboration-native-replica-profile"),
        group_binding_ref: group.binding_ref.clone(),
        service_generation: 1,
        protocol_ref,
        durable_log_ref: durability.profile.profile_ref.clone(),
        snapshot_store_ref: durability.profile.profile_ref,
        timer_profile_ref: time.profile.profile_ref.clone(),
        entropy_profile_ref: time.profile.profile_ref,
        placement_ref: group.placement_ref.clone(),
        fencing_ref: group.fencing_ref.clone(),
        fencing_epoch: 1,
        supervision_ref: super::reference("collaboration-native-supervision"),
        resource_profile_ref: group.resource_profile_ref.clone(),
        // This signed-source rail performs slow independent source checks between operations.
        // At two responses per heartbeat, this schedule keeps the finite ingress budget
        // above the declared twelve-hour operator window instead of exhausting it in minutes.
        heartbeat_ticks: 200,
        election_min_ticks: 600,
        election_max_ticks: 1200,
        max_log_entries: 4096,
        max_message_entries: 64,
        max_effects_per_step: 128,
    })
}

fn effects(
    service: &molten::node_service_host::codec::ExtensionInput,
    manifest: &molten::system_extension::AdmittedManifest,
    input: &molten::collaboration_scope::native_runtime::Input,
    runtime_ref: String,
) -> super::Result<molten::node_service_host::effects::config::Configuration> {
    let binding = manifest
        .binding_for(&molten::fabric::FabricPortKey {
            port_id: molten::collaboration_scope::COMMIT_PORT_ID.into(),
            version: molten::collaboration_scope::COMMIT_PORT_VERSION.into(),
        })
        .ok_or("actual admitted native commit binding missing")?;
    Ok(molten::node_service_host::effects::config::Configuration {
        bindings: vec![molten::node_service_host::effects::config::Binding {
            service_id: super::SERVICE.into(),
            manifest_ref: manifest.manifest_ref().into(),
            port_binding_ref: binding.binding_ref.clone(),
            port_id: molten::collaboration_scope::COMMIT_PORT_ID.into(),
            port_version: molten::collaboration_scope::COMMIT_PORT_VERSION.into(),
            operation: molten::collaboration_scope::COMMIT_OPERATION.into(),
            group_ref: input.replica.group_binding_ref.clone(),
            generation: 1,
            policy_ref: service.executable.policy_ref.clone(),
            resource_ref: service.executable.resource_ref.clone(),
            installation_ref: input.installation_ref.clone(),
            runtime_artifact: super::RUNTIME.into(),
            runtime_ref,
        }],
    })
}
