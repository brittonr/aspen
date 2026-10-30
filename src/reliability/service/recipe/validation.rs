pub(super) fn input(input: &super::Input) -> crate::error::Result<()> {
    if input.run_id.is_empty()
        || input.run_id.len() > 256
        || input.group.service_generation != 1
        || !(1..=30000).contains(&input.tick_ms)
        || input.operator.policy_generation == 0
        || input.operator.not_before_ms >= input.operator.expires_at_ms
        || input.operator.expires_at_ms - input.operator.not_before_ms > input.operator.max_grant_lifetime_ms
        || input.operator.allowed_operations.is_empty()
        || input.operator.allowed_operations.len() > 32
    {
        return Err(super::invalid("invalid bounded experimental policy, generation, or timing"));
    }
    super::super::super::profile::admit(&input.workload)
        .map_err(|issue| super::invalid(format!("workload admission: {issue:?}")))?;
    super::super::registry::Registry::new(input.registry.clone())?;
    if input.workload.group != input.group.group_id
        || input.workload.service != input.group.service_id
        || input.workload.keys != [input.registry.object_key.clone()]
        || input.workload.values.iter().copied().collect::<std::collections::BTreeSet<_>>()
            != input.registry.endpoints.keys().copied().collect()
    {
        return Err(super::invalid("recipe workload and registry corpus differ"));
    }
    let unique = |values: Vec<String>| values.iter().collect::<std::collections::BTreeSet<_>>().len() == values.len();
    if !unique(input.nodes.iter().map(|node| node.node_id.clone()).collect())
        || !unique(input.nodes.iter().map(|node| node.endpoint.to_string()).collect())
        || !unique(input.nodes.iter().map(|node| node.bind_addr.to_string()).collect())
        || !unique(input.nodes.iter().map(|node| node.grant_id.clone()).collect())
    {
        return Err(super::invalid("recipe node IDs, endpoints, bindings, and grant IDs must be unique"));
    }
    crate::preserves_rail::validate_content_ref(&input.handler_ref)?;
    keys(input)?;
    for node in &input.nodes {
        node_input(node)?;
    }
    Ok(())
}

fn keys(input: &super::Input) -> crate::error::Result<()> {
    let operator = key(&input.operator.public_key)?;
    let mut transport_keys = std::collections::BTreeSet::new();
    let mut probe_keys = std::collections::BTreeSet::new();
    let mut client_keys = std::collections::BTreeSet::new();
    for node in &input.nodes {
        if !transport_keys.insert(key(&node.transport_public_key)?) || !probe_keys.insert(key(&node.probe_public_key)?)
        {
            return Err(super::invalid("transport and probe keys must each be unique across nodes"));
        }
        client_keys.insert(key(&node.client_public_key)?);
    }
    if transport_keys.contains(&operator)
        || probe_keys.contains(&operator)
        || client_keys.contains(&operator)
        || !transport_keys.is_disjoint(&probe_keys)
        || !transport_keys.is_disjoint(&client_keys)
        || !probe_keys.is_disjoint(&client_keys)
    {
        return Err(super::invalid("operator, transport, probe and client signing purposes must be separated"));
    }
    Ok(())
}

fn key(value: &str) -> crate::error::Result<String> {
    let key: iroh::PublicKey = value.parse().map_err(|_| super::invalid("invalid recipe public key"))?;
    if key.to_string() != value {
        return Err(super::invalid("public key must use its canonical spelling"));
    }
    Ok(key.to_string())
}

fn node_input(node: &super::NodeInput) -> crate::error::Result<()> {
    if node.node_id.is_empty()
        || node.node_id.len() > 256
        || node.grant_id.is_empty()
        || node.grant_id.len() > 256
        || node.endpoint.port() == 0
        || node.bind_addr.port() == 0
        || node.client_bind_addr.port() == 0
        || node.endpoint.ip().is_unspecified()
        || node.bind_addr.ip().is_unspecified()
        || node.client_bind_addr.ip().is_unspecified()
        || node.bind_addr == node.client_bind_addr
        || node.endpoint == node.bind_addr
        || node.endpoint == node.client_bind_addr
        || node.bindings.len() != 7
    {
        return Err(super::invalid("recipe requires explicit nonzero distinct sockets and bounded node identities"));
    }
    for reference in [
        &node.source_binding_ref,
        &node.client_scope_ref,
        &node.transport_capability_ref,
        &node.time_capability_ref,
        &node.listener_identity_ref,
        &node.peer_context_ref,
        &node.locator_cohort_ref,
        &node.transport_session_ref,
        &node.supervision_ref,
    ] {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    crate::node_state::RelativePath::parse(&node.transport_key_file)?;
    crate::node_state::RelativePath::parse(&node.health_path)?;
    crate::local_store::Locator::parse(&node.storage_namespace)?;
    node.validity.clone().into_cohort()?;
    Ok(())
}

pub(super) fn configuration(
    config: &super::super::decoder::Configuration,
    group: &crate::fabric_consistency::Group,
) -> crate::error::Result<()> {
    config.admission.validate()?;
    let bindings = config
        .bindings
        .iter()
        .map(|binding| crate::fabric_consistency::raft::ReplicaPortBinding {
            port_id: binding.port_id.clone(),
            version: binding.version.clone(),
            implementation_profile: binding.implementation_profile.clone(),
            binding_ref: binding.binding_ref.clone(),
        })
        .collect();
    let plan =
        crate::fabric_consistency::raft::plan_live_replica_start(crate::fabric_consistency::raft::ReplicaStartInput {
            group: group.clone(),
            node_id: config.node_id.clone(),
            membership: config.membership.decode(),
            profile: config.replica.decode(),
            port_bindings: bindings,
        })?;
    if plan.production_admitted {
        return Err(super::invalid("experimental recipe cannot admit production"));
    }
    Ok(())
}
