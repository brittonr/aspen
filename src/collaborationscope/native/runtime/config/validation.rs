pub(super) fn validate(
    input: &super::Input,
    plan: &crate::fabric_consistency::raft::ReplicaStartPlan,
    installation: &super::super::super::Installation,
    time: &crate::fabric_time::AdmittedTimeProfile,
) -> crate::error::Result<()> {
    identities(input, plan, installation, time)?;
    resources(input)
}

fn identities(
    input: &super::Input,
    plan: &crate::fabric_consistency::raft::ReplicaStartPlan,
    installation: &super::super::super::Installation,
    time: &crate::fabric_time::AdmittedTimeProfile,
) -> crate::error::Result<()> {
    if plan.production_admitted
        || input.installation_ref != crate::preserves_rail::canonical_hash(&installation.status_value())?
        || !input.group.policy_refs.contains(&input.policy_ref)
        || input.resource_ref != input.group.resource_profile_ref
        || input.replica.timer_profile_ref != time.profile_ref
        || input.membership.voters.len() != 3
        || time.kind != crate::fabric_time::TimeProfileKind::Live
        || !time.supported_domains.contains(&crate::fabric_time::TimeDomain::Logical)
        || input.transport.peers.keys().cloned().collect::<std::collections::BTreeSet<_>>()
            != input.membership.voters.iter().filter(|node| **node != input.node_id).cloned().collect()
        || !(1..=256).contains(&input.ingress_capacity)
        || !(1..=65536).contains(&input.ingress_delivery_limit)
        || !(1..=30000).contains(&input.timeout_ms)
        || !(1..=30000).contains(&input.transport.timeout_ms)
        || !(1..=30000).contains(&input.time.tick_ms)
        || !(1..=256).contains(&time.max_scheduler_queue_depth)
        || input.raft_store.storage_namespace == input.scope_store.storage_namespace
        || input.callback_namespace == input.raft_store.storage_namespace
        || input.callback_namespace == input.scope_store.storage_namespace
    {
        return Err(super::codec::invalid("runtime identities or bounded resources differ from admitted host"));
    }
    Ok(())
}

fn resources(input: &super::Input) -> crate::error::Result<()> {
    for reference in [&input.handler_ref, &input.session_ref, &input.time.capability_ref] {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    crate::local_store::Locator::parse(&input.raft_store.storage_namespace)?;
    crate::local_store::Locator::parse(&input.scope_store.storage_namespace)?;
    crate::node_state::RelativePath::parse(&input.callback_namespace)?;
    crate::node_state::RelativePath::parse(&input.transport.key_file)?;
    super::stores::validate(input)?;
    super::transport::validate(input)?;
    if let Some(genesis) = &input.genesis {
        if genesis.consistency_epoch == 0 {
            return Err(super::codec::invalid("empty genesis requires explicit admitted currentness"));
        }
        crate::preserves_rail::validate_content_ref(&genesis.revocation_ref)?;
        crate::preserves_rail::validate_content_ref(&genesis.observation_ref)?;
    }
    Ok(())
}

pub(super) fn host<E: crate::system_extension::SystemExtensionExecutor>(
    input: &super::Input,
    host: &crate::system_extension::Dispatcher<E>,
    time: &crate::fabric_time::AdmittedTimeProfile,
) -> crate::error::Result<()> {
    let manifest = host.manifest().manifest();
    if input.group.max_command_bytes > manifest.resources.max_inflight_bytes {
        return Err(super::codec::invalid("group command bytes exceed installed resources"));
    }
    let bindings = host.manifest().required_port_bindings();
    let timer = bindings
        .iter()
        .find(|binding| binding.binding.class == crate::fabric::FabricPortClass::Time)
        .ok_or_else(|| super::codec::invalid("host time binding missing"))?;
    if timer.binding.implementation_profile != time.profile_id {
        return Err(super::codec::invalid("decoded time profile differs from installed implementation"));
    }
    measured_bindings(input, host)?;
    adapter_profiles(input, time)
}

fn measured_bindings<E: crate::system_extension::SystemExtensionExecutor>(
    input: &super::Input,
    host: &crate::system_extension::Dispatcher<E>,
) -> crate::error::Result<()> {
    if input.bindings.len() != crate::fabric_consistency::raft::REQUIRED_REPLICA_PORTS.len() {
        return Err(super::codec::invalid("exactly seven measured fabric bindings required"));
    }
    for (port_id, version) in crate::fabric_consistency::raft::REQUIRED_REPLICA_PORTS {
        let mut matches =
            input.bindings.iter().filter(|binding| binding.port_id == port_id && binding.version == version);
        let measured = matches.next().ok_or_else(|| super::codec::invalid("missing measured fabric binding"))?;
        if matches.next().is_some() {
            return Err(super::codec::invalid("duplicate measured fabric binding"));
        }
        let actual = host
            .manifest()
            .binding_for(&crate::fabric::FabricPortKey {
                port_id: port_id.into(),
                version: version.into(),
            })
            .ok_or_else(|| super::codec::invalid("missing actual host binding"))?;
        if measured.implementation_profile != actual.binding.implementation_profile
            || measured.binding_ref != actual.binding_ref
        {
            return Err(super::codec::invalid("measured binding differs from running host"));
        }
    }
    Ok(())
}

fn adapter_profiles(input: &super::Input, time: &crate::fabric_time::AdmittedTimeProfile) -> crate::error::Result<()> {
    for (port_id, profile_id) in [
        (crate::fabric_transport::FABRIC_TRANSPORT_PORT_ID, &input.transport.profile.profile_id),
        (crate::fabric_durability::FABRIC_DURABLE_LOG_PORT_ID, &input.raft_store.profile.profile_id),
        (crate::fabric_durability::FABRIC_SNAPSHOT_PORT_ID, &input.raft_store.profile.profile_id),
        (crate::fabric_time::FABRIC_TIMER_PORT_ID, &time.profile_id),
        (crate::fabric_time::FABRIC_ENTROPY_PORT_ID, &time.profile_id),
    ] {
        if !input
            .bindings
            .iter()
            .any(|binding| binding.port_id == port_id && &binding.implementation_profile == profile_id)
        {
            return Err(super::codec::invalid("actual adapter profile differs from installed fabric binding"));
        }
    }
    Ok(())
}
