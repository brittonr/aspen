pub(super) fn validate(input: &super::Input) -> crate::error::Result<()> {
    let raft = durable(&input.raft_store, input.group.service_generation)?;
    let scope = durable(&input.scope_store, input.group.service_generation)?;
    if raft.namespace_id == scope.namespace_id
        || raft.profile_ref == scope.profile_ref
        || raft.atomicity_domain.domain_id == scope.atomicity_domain.domain_id
    {
        return Err(super::codec::invalid("Raft and scope require distinct durable profiles, namespaces and domains"));
    }
    replay(input, &raft, &scope)
}

fn durable(
    input: &crate::live_reliability::service::Durability,
    generation: u64,
) -> crate::error::Result<crate::fabric_durability::DurableNamespaceDescriptor> {
    let profile = input.profile.clone().into_profile()?;
    let namespace = input.namespace.clone().into_descriptor(&profile)?;
    if profile.profile.adapter_kind != crate::fabric_durability::DurableAdapterKind::LiveRedb
        || namespace.generation != generation
        || !profile.profile.supported_levels.contains(&crate::fabric_durability::DurabilityLevel::MachineLoss)
    {
        return Err(super::codec::invalid("durable profile must be live, machine-loss admitted and generation-bound"));
    }
    Ok(namespace)
}

fn replay(
    input: &super::Input,
    raft: &crate::fabric_durability::DurableNamespaceDescriptor,
    scope: &crate::fabric_durability::DurableNamespaceDescriptor,
) -> crate::error::Result<()> {
    let replay = durable(&input.replay_store, input.group.service_generation)?;
    crate::local_store::Locator::parse(&input.replay_store.storage_namespace)?;
    for descriptor in [raft, scope] {
        if replay.namespace_id == descriptor.namespace_id
            || replay.profile_ref == descriptor.profile_ref
            || replay.atomicity_domain.domain_id == descriptor.atomicity_domain.domain_id
        {
            return Err(super::codec::invalid("replay requires an independent admitted profile, namespace and domain"));
        }
    }
    for locator in [
        &input.raft_store.storage_namespace,
        &input.scope_store.storage_namespace,
        &input.callback_namespace,
    ] {
        if &input.replay_store.storage_namespace == locator {
            return Err(super::codec::invalid("replay storage aliases another runtime namespace"));
        }
    }
    for descriptor in [scope, &replay] {
        if !descriptor
            .atomicity_domain
            .object_classes
            .contains(&crate::fabric_durability::DurableObjectClass::OrderedValue)
            || !descriptor
                .atomicity_domain
                .supported_levels
                .contains(&crate::fabric_durability::DurabilityLevel::MachineLoss)
        {
            return Err(super::codec::invalid("scope and replay stores require machine-loss ordered-value atomicity"));
        }
    }
    Ok(())
}
