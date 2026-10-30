pub type Ports = crate::fabric_consistency::raft::ConcreteReplicaPortBundle<
    crate::fabric_time::OperatingSystemEntropySource,
    super::registry::Registry,
>;

/// Effect authority is supplied by the node owner, never manufactured from references.
/// Durable storage must already be opened through the caller's node-root capability.
pub struct Startup {
    pub group: crate::fabric_consistency::Group,
    pub node_id: String,
    pub membership: crate::fabric_consistency::raft::StaticMembership,
    pub profile: crate::fabric_consistency::raft::ReplicaProfile,
    pub bindings: Vec<PortBinding>,
    pub durability: crate::fabric_consistency::raft::RedbReplicaPort,
    pub transport: crate::fabric_consistency::raft::ReplicaTransportPort,
    pub listener: crate::fabric_transport::Listener,
    pub ingress: crate::fabric_consistency::raft::ReplicaIngressConfig,
    pub timer: crate::fabric_consistency::raft::TokioReplicaConfig,
    pub registry: super::registry::Registry,
    pub handler_ref: String,
    pub host: super::runtime::HostAdmission,
    pub is_recovery: bool,
    pub source_gate: crate::node_runtime::ValidatedNodeSourceGate,
    pub authorization: super::access::Authorization,
    pub health_path: String,
    pub peer_probes: std::collections::BTreeMap<String, super::leader::PeerProbe>,
    pub transport_registration: crate::fabric_transport::IrohAdapter,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortBinding {
    pub port_id: String,
    pub version: String,
    pub implementation_profile: String,
    pub binding_ref: String,
}

pub async fn start(input: Startup) -> crate::error::Result<super::runtime::Host> {
    input.host.validate()?;
    input.authorization.validate_start(&input)?;
    let native = super::super::readiness::current::read(&input.authorization.root, &input.health_path)?;
    let instance_ref = super::control::instance_ref(&input.authorization.grant)?;
    admit_cohort(&input)?;
    let mut plan =
        crate::fabric_consistency::raft::plan_live_replica_start(crate::fabric_consistency::raft::ReplicaStartInput {
            group: input.group.clone(),
            node_id: input.node_id,
            membership: input.membership,
            profile: input.profile,
            port_bindings: replica_bindings(input.bindings),
        })?;
    if plan.production_admitted {
        return Err(crate::error::Failure::invalid_harness("live registry is nonproduction only"));
    }
    let durability = input.durability.reconcile_repaired_storage()?;
    if input.is_recovery {
        plan = durability.plan_recovery(plan)?.start_plan;
    }
    let identity = identity(&input.group, &plan);
    let capacity = usize::try_from(input.timer.profile.max_scheduler_queue_depth)
        .ok()
        .filter(|capacity| (1..=256).contains(capacity))
        .ok_or_else(|| crate::error::Failure::invalid_harness("registry timer queue must be bounded to 1..256"))?;
    let (sender, inbox) = tokio::sync::mpsc::channel(capacity);
    let time = crate::fabric_consistency::raft::TokioReplicaPort::new_operating_system(input.timer, sender)?;
    let (sender, observations) = tokio::sync::mpsc::channel(256);
    let control = crate::fabric_consistency::raft::ChannelReplicaPort::new(
        crate::fabric_consistency::raft::ReplicaConfig {
            service_id: input.group.service_id,
            service_generation: plan.state.profile.service_generation,
            supervision_ref: plan.state.profile.supervision_ref.clone(),
        },
        sender,
    )?;
    let application = application_port(&plan, input.handler_ref, input.registry)?;
    let ports = crate::fabric_consistency::raft::assemble_scoped_concrete_replica(
        identity,
        crate::fabric_consistency::raft::ReplicaAdapterSet {
            durability,
            transport: input.transport,
            time,
            application,
            control,
        },
    )?;
    let (_unused_sender, unused_inbox) = tokio::sync::mpsc::channel(1);
    let replica = crate::fabric_consistency::raft::ScopedLiveReplica::start(plan, ports, unused_inbox).await?;
    let ingress = crate::fabric_consistency::raft::ReplicaIngressPump::spawn(input.listener, input.ingress)?;
    Ok(super::runtime::Host::new(
        input.host,
        replica,
        ingress,
        (inbox, observations),
        super::session::Context {
            authorization: input.authorization,
            source_gate: input.source_gate,
            health_path: input.health_path,
            instance_ref,
            native_run_ref: native.run_ref,
            native_startup_ref: native.startup_ref,
            peer_probes: input.peer_probes,
            source_trust_ref: native.source_trust_ref,
            transport_registration: input.transport_registration,
        },
    ))
}

/// Registry profile, public peer cohort and durable state must match the admitted group before
/// planning.
fn admit_cohort(input: &Startup) -> crate::error::Result<()> {
    if input.host.profile.group != input.group.group_id
        || input.host.profile.service != input.group.service_id
        || input.host.profile.keys.first().map(String::as_str) != Some(input.registry.object_key())
        || input.host.profile.values.iter().copied().collect::<std::collections::BTreeSet<_>>()
            != input.registry.values().into_iter().collect()
        || input.host.peers.keys().cloned().collect::<std::collections::BTreeSet<_>>()
            != input.membership.voters.iter().cloned().collect()
    {
        return Err(crate::error::Failure::invalid_harness(
            "registry profile or public peer cohort does not match admitted group",
        ));
    }
    if input.host.peers.get(&input.node_id) != Some(&input.host.endpoint) {
        return Err(crate::error::Failure::invalid_harness("public endpoint is not the admitted local peer"));
    }
    if !input.is_recovery
        && (!input.durability.adapter().state().durable_log.is_empty()
            || !input.durability.adapter().state().snapshots.is_empty())
    {
        return Err(crate::error::Failure::invalid_harness(
            "existing registry durable state requires recovery startup",
        ));
    }
    Ok(())
}

fn replica_bindings(bindings: Vec<PortBinding>) -> Vec<crate::fabric_consistency::raft::ReplicaPortBinding> {
    bindings
        .into_iter()
        .map(|binding| crate::fabric_consistency::raft::ReplicaPortBinding {
            port_id: binding.port_id,
            version: binding.version,
            implementation_profile: binding.implementation_profile,
            binding_ref: binding.binding_ref,
        })
        .collect()
}

fn application_port(
    plan: &crate::fabric_consistency::raft::ReplicaStartPlan,
    handler_ref: String,
    registry: super::registry::Registry,
) -> crate::error::Result<crate::fabric_consistency::raft::AdmittedReplicaPort<super::registry::Registry>> {
    crate::fabric_consistency::raft::AdmittedReplicaPort::new(
        crate::fabric_consistency::raft::ServiceReplicaConfig {
            group_binding_ref: plan.state.profile.group_binding_ref.clone(),
            application_manifest_ref: plan.application_manifest_ref.clone(),
            handler_ref,
            command_schema_refs: [registry.schema_ref().to_string()].into(),
            initial_applied_index: 0,
        },
        registry,
    )
}

fn identity(
    group: &crate::fabric_consistency::Group,
    plan: &crate::fabric_consistency::raft::ReplicaStartPlan,
) -> crate::fabric_consistency::raft::ReplicaRuntimePortIdentity {
    let profile = &plan.state.profile;
    crate::fabric_consistency::raft::ReplicaRuntimePortIdentity {
        service_id: group.service_id.clone(),
        service_generation: profile.service_generation,
        group_binding_ref: profile.group_binding_ref.clone(),
        application_manifest_ref: group.application_manifest_ref.clone(),
        protocol_ref: profile.protocol_ref.clone(),
        durable_log_ref: profile.durable_log_ref.clone(),
        snapshot_store_ref: profile.snapshot_store_ref.clone(),
        timer_profile_ref: profile.timer_profile_ref.clone(),
        entropy_profile_ref: profile.entropy_profile_ref.clone(),
        membership_ref: plan.state.membership.membership_ref.clone(),
        placement_ref: profile.placement_ref.clone(),
        fencing_ref: profile.fencing_ref.clone(),
        supervision_ref: profile.supervision_ref.clone(),
        resource_profile_ref: profile.resource_profile_ref.clone(),
        fabric_binding_refs: plan.port_binding_refs.clone(),
    }
}
