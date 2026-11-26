//! Assembly of one peer's production scoped live replica: listener, peer
//! clients, durable log (and startup recovery), time, application and control
//! ports, and the ingress pump.

const TRANSPORT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
const INGRESS_ACCEPT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
const INGRESS_EVENT_CAPACITY: usize = 32;
const CONTROL_OBSERVATION_CAPACITY: usize = 32;

pub(super) async fn open(
    node_id: &str,
    mode: super::Mode,
    run_directory: &std::path::Path,
    deadline: &mut super::Deadline,
) -> crate::error::Result<super::Node> {
    let group = super::super::fixture::group()?;
    let listener =
        crate::fabric_transport::Listener::bind(super::super::fixture::iroh::listener_input(&group, node_id)?).await?;
    let endpoint_identity = listener.handoff().descriptor.public_endpoint_identity.clone();
    super::super::super::peer::write_value(
        &super::super::receipt::node_file(run_directory, node_id, "endpoint"),
        &listener.handoff().value,
    )?;
    let peers = peer_clients(&group, node_id, run_directory, deadline).await?;
    let time = super::super::fixture::time_profile()?;
    let plan = super::super::fixture::start_plan(&group, node_id, &time.profile.profile_ref)?;
    let durability = durability_port(run_directory, node_id, &plan)?;
    let (plan, recovery) = recover(mode, &durability, plan)?;
    let capacity = usize::try_from(time.profile.max_scheduler_queue_depth)
        .ok()
        .filter(|capacity| *capacity > 0)
        .ok_or_else(|| {
            crate::capability_qualification::invalid("live Raft inbox needs a positive admitted queue depth")
        })?;
    let (timer_sender, inbox) = tokio::sync::mpsc::channel(capacity);
    let time_port = time_adapter(&group, node_id, &plan, time, timer_sender)?;
    let application = application_port(&group)?;
    let (control_sender, control) = tokio::sync::mpsc::channel(CONTROL_OBSERVATION_CAPACITY);
    let control_port = observation_port(&group, &plan, control_sender)?;
    let transport = crate::fabric_consistency::raft::ReplicaTransportPort::new(
        plan.state.profile.protocol_ref.clone(),
        peers,
        TRANSPORT_TIMEOUT,
    )?;
    let ports = crate::fabric_consistency::raft::assemble_scoped_concrete_replica(
        super::super::fixture::runtime_identity(&plan),
        crate::fabric_consistency::raft::ReplicaAdapterSet {
            durability,
            transport,
            time: time_port,
            application,
            control: control_port,
        },
    )?;
    let service = crate::fabric_consistency::raft::ScopedLiveReplica::start(plan, ports, inbox).await?;
    let ingress = crate::fabric_consistency::raft::ReplicaIngressPump::spawn(
        listener,
        crate::fabric_consistency::raft::ReplicaIngressConfig {
            session_ref: super::super::fixture::iroh::session_ref(),
            accept_timeout: INGRESS_ACCEPT_TIMEOUT,
            event_capacity: INGRESS_EVENT_CAPACITY,
            delivery_limit: super::INGRESS_DELIVERY_LIMIT,
            delivery_window: super::INGRESS_DELIVERY_WINDOW,
        },
    )?;
    Ok(super::Node {
        node_id: node_id.to_string(),
        endpoint_identity,
        service,
        ingress: Some(ingress),
        recovery,
        _control: control,
    })
}

/// The admitted client towards every other static voter's published endpoint,
/// in membership order.
async fn peer_clients(
    group: &crate::fabric_consistency::Group,
    node_id: &str,
    run_directory: &std::path::Path,
    deadline: &mut super::Deadline,
) -> crate::error::Result<std::collections::BTreeMap<String, crate::fabric_transport::ClientInput>> {
    let mut peers = Vec::with_capacity(super::super::fixture::NODES.len());
    for peer_id in super::super::fixture::NODES.into_iter().filter(|peer_id| *peer_id != node_id) {
        let value =
            super::await_value(&super::super::receipt::node_file(run_directory, peer_id, "endpoint"), deadline).await?;
        let endpoint = crate::fabric_transport::parse_endpoint(&value)?;
        peers
            .push((peer_id.to_string(), super::super::fixture::iroh::client_input(group, node_id, peer_id, endpoint)?));
    }
    Ok(peers.into_iter().collect())
}

/// The capability-rooted Redb durable log of `node_id`, reconciled after any
/// repaired storage.
fn durability_port(
    run_directory: &std::path::Path,
    node_id: &str,
    plan: &crate::fabric_consistency::raft::ReplicaStartPlan,
) -> crate::error::Result<crate::fabric_consistency::raft::RedbReplicaPort> {
    let durability_root = run_directory.join(format!("{node_id}-durability"));
    std::fs::create_dir_all(&durability_root).map_err(crate::error::Failure::from)?;
    let adapter = crate::fabric_durability::RedbDurableStateAdapter::open(
        &durability_root,
        super::super::fixture::durable_profile()?,
        super::super::fixture::durable_namespace(node_id),
    )?;
    crate::fabric_consistency::raft::RedbReplicaPort::new(
        adapter,
        plan.state.profile.durable_log_ref.clone(),
        plan.state.profile.snapshot_store_ref.clone(),
    )?
    .reconcile_repaired_storage()
}

/// A fresh replica must find no durable state; a recovered replica must find
/// some, and starts from the plan its durable log recovers.
fn recover(
    mode: super::Mode,
    durability: &crate::fabric_consistency::raft::RedbReplicaPort,
    plan: crate::fabric_consistency::raft::ReplicaStartPlan,
) -> crate::error::Result<(crate::fabric_consistency::raft::ReplicaStartPlan, Option<super::super::receipt::Recovery>)>
{
    let has_durable_state =
        !durability.adapter().state().durable_log.is_empty() || !durability.adapter().state().snapshots.is_empty();
    match mode {
        super::Mode::Fresh if has_durable_state => {
            Err(crate::capability_qualification::invalid("fresh live Raft replica found existing durable state"))
        }
        super::Mode::Fresh => Ok((plan, None)),
        super::Mode::Recover if !has_durable_state => {
            Err(crate::capability_qualification::invalid("recovered live Raft replica found no durable state"))
        }
        super::Mode::Recover => {
            let recovered = durability.plan_recovery(plan)?;
            let recovery = super::super::receipt::Recovery {
                recovery_ref: recovered.recovery_ref,
                durable_commit_index: recovered.durable_commit_index,
                replay_entry_count: u64::try_from(recovered.replay_entry_count)
                    .map_err(|_| crate::capability_qualification::invalid("live Raft replay count exceeds u64"))?,
            };
            Ok((recovered.start_plan, Some(recovery)))
        }
    }
}

fn time_adapter(
    group: &crate::fabric_consistency::Group,
    node_id: &str,
    plan: &crate::fabric_consistency::raft::ReplicaStartPlan,
    time: crate::fabric_time::Profile,
    timer_sender: tokio::sync::mpsc::Sender<crate::fabric_consistency::raft::ReplicaEvent>,
) -> crate::error::Result<
    crate::fabric_consistency::raft::TokioReplicaPort<crate::fabric_time::OperatingSystemEntropySource>,
> {
    crate::fabric_consistency::raft::TokioReplicaPort::new_operating_system(
        crate::fabric_consistency::raft::TokioReplicaConfig {
            generation: group.service_generation,
            service_id: group.service_id.clone(),
            capability_ref: super::super::fixture::reference(&format!("{node_id}/time-capability")),
            entropy_binding_ref: plan.state.profile.entropy_profile_ref.clone(),
            tick_duration: super::super::fixture::TICK_DURATION,
            heartbeat_ticks: plan.state.profile.heartbeat_ticks,
            election_min_ticks: plan.state.profile.election_min_ticks,
            election_max_ticks: plan.state.profile.election_max_ticks,
            profile: time.profile,
        },
        timer_sender,
    )
}

fn application_port(
    group: &crate::fabric_consistency::Group,
) -> crate::error::Result<crate::fabric_consistency::raft::AdmittedReplicaPort<super::AppliedCommands>> {
    crate::fabric_consistency::raft::AdmittedReplicaPort::new(
        crate::fabric_consistency::raft::ServiceReplicaConfig {
            group_binding_ref: group.binding_ref.clone(),
            application_manifest_ref: group.application_manifest_ref.clone(),
            handler_ref: super::super::fixture::reference("application-handler"),
            command_schema_refs: std::collections::BTreeSet::from([super::super::fixture::command_schema_ref()]),
            initial_applied_index: crate::fabric_consistency::raft::INITIAL_COMMIT_INDEX,
        },
        super::AppliedCommands::default(),
    )
}

fn observation_port(
    group: &crate::fabric_consistency::Group,
    plan: &crate::fabric_consistency::raft::ReplicaStartPlan,
    control_sender: tokio::sync::mpsc::Sender<crate::fabric_consistency::raft::ReplicaObservation>,
) -> crate::error::Result<crate::fabric_consistency::raft::ChannelReplicaPort> {
    crate::fabric_consistency::raft::ChannelReplicaPort::new(
        crate::fabric_consistency::raft::ReplicaConfig {
            service_id: group.service_id.clone(),
            service_generation: group.service_generation,
            supervision_ref: plan.state.profile.supervision_ref.clone(),
        },
        control_sender,
    )
}
