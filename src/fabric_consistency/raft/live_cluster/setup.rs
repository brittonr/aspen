use super::*;

/// A live node publishes at most one observation per client Propose/Read event plus one per
/// committed entry: 2 proposals + 2 reads + 2 committed entries = 6 in the bounded
/// live-cluster/live-process workflows, which never drain.
const LIVE_CONTROL_OBSERVATION_CAPACITY: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeStartupMode {
    Fresh,
    Recover,
}

pub(in crate::fabric_consistency::raft) async fn build_node(
    group: &crate::fabric_consistency::ConsistencyGroupBinding,
    node_id: &str,
    listener: crate::fabric_transport::IrohCrossProcessListener,
    endpoints: &std::collections::BTreeMap<String, crate::fabric_transport::CanonicalCrossProcessEndpoint>,
) -> crate::error::Result<LiveNode> {
    let root = crate::test_support::process_workspace(&format!("live-cluster-{node_id}"))?;
    let root_owner = root.clone();
    build_node_with_root(group, node_id, listener, endpoints, &root, Some(root_owner), NodeStartupMode::Fresh).await
}

pub(in crate::fabric_consistency::raft) async fn build_node_at_root(
    group: &crate::fabric_consistency::ConsistencyGroupBinding,
    node_id: &str,
    listener: crate::fabric_transport::IrohCrossProcessListener,
    endpoints: &std::collections::BTreeMap<String, crate::fabric_transport::CanonicalCrossProcessEndpoint>,
    durability_root: &std::path::Path,
) -> crate::error::Result<LiveNode> {
    build_node_with_root(group, node_id, listener, endpoints, durability_root, None, NodeStartupMode::Fresh).await
}

pub(in crate::fabric_consistency::raft) async fn recover_node_at_root(
    group: &crate::fabric_consistency::ConsistencyGroupBinding,
    node_id: &str,
    listener: crate::fabric_transport::IrohCrossProcessListener,
    endpoints: &std::collections::BTreeMap<String, crate::fabric_transport::CanonicalCrossProcessEndpoint>,
    durability_root: &std::path::Path,
) -> crate::error::Result<LiveNode> {
    build_node_with_root(group, node_id, listener, endpoints, durability_root, None, NodeStartupMode::Recover).await
}

async fn build_node_with_root(
    group: &crate::fabric_consistency::ConsistencyGroupBinding,
    node_id: &str,
    listener: crate::fabric_transport::IrohCrossProcessListener,
    endpoints: &std::collections::BTreeMap<String, crate::fabric_transport::CanonicalCrossProcessEndpoint>,
    durability_root: &std::path::Path,
    workspace: Option<crate::test_support::ProcessWorkspace>,
    startup_mode: NodeStartupMode,
) -> crate::error::Result<LiveNode> {
    let protocol_ref = super::super::tests::test_ref("live-cluster-protocol");
    let timer = crate::fabric_time::tests::live_profile().profile;
    let timer_profile_ref = timer.profile_ref.clone();
    let entropy_profile_ref = super::super::tests::test_ref("live-cluster-entropy");
    let supervision_ref = super::super::tests::test_ref("live-cluster-supervision");
    let durable_log_ref = super::super::tests::test_ref(&format!("live-cluster-{node_id}-log"));
    let snapshot_store_ref = super::super::tests::test_ref(&format!("live-cluster-{node_id}-snapshots"));
    let fabric_binding_refs = (0..LIVE_FABRIC_BINDING_COUNT)
        .map(|index| super::super::tests::test_ref(&format!("live-cluster-fabric-binding-{index}")))
        .collect::<Vec<_>>();
    let mut state = super::super::tests::started_state(group, node_id);
    bind_state_profile(
        &mut state,
        &protocol_ref,
        &durable_log_ref,
        &snapshot_store_ref,
        &timer_profile_ref,
        &entropy_profile_ref,
        &supervision_ref,
    );
    let identity = runtime_identity(group, &state, fabric_binding_refs.clone());
    let mut plan = start_plan(group, state, fabric_binding_refs);
    let (transport, session_ref) = transport_for(node_id, endpoints, protocol_ref)?;
    let durability = durability_for_root(node_id, durable_log_ref, snapshot_store_ref, durability_root)?;
    let recovery_ref = if startup_mode == NodeStartupMode::Recover {
        let recovery = durability.plan_recovery(plan)?;
        plan = recovery.start_plan;
        Some(recovery.recovery_ref)
    } else {
        None
    };
    let (time, inbox) = time_for(group, node_id, timer, entropy_profile_ref)?;
    let application = application_for(group)?;
    let (control, control_receiver) = control_for(group, supervision_ref)?;
    let ports = assemble_scoped_concrete_replica_ports(identity, durability, transport, time, application, control)?;
    let service = ScopedLiveReplicaService::start(plan, ports, inbox).await?;
    Ok(LiveNode {
        service,
        listener: Some(listener),
        session_ref,
        recovery_ref,
        _workspace: workspace,
        _control_receiver: control_receiver,
    })
}

fn bind_state_profile(
    state: &mut ReplicaState,
    protocol_ref: &str,
    durable_log_ref: &str,
    snapshot_store_ref: &str,
    timer_profile_ref: &str,
    entropy_profile_ref: &str,
    supervision_ref: &str,
) {
    state.profile.protocol_ref = protocol_ref.to_string();
    state.profile.durable_log_ref = durable_log_ref.to_string();
    state.profile.snapshot_store_ref = snapshot_store_ref.to_string();
    state.profile.timer_profile_ref = timer_profile_ref.to_string();
    state.profile.entropy_profile_ref = entropy_profile_ref.to_string();
    state.profile.supervision_ref = supervision_ref.to_string();
}

fn runtime_identity(
    group: &crate::fabric_consistency::ConsistencyGroupBinding,
    state: &ReplicaState,
    fabric_binding_refs: Vec<String>,
) -> ReplicaRuntimePortIdentity {
    ReplicaRuntimePortIdentity {
        service_id: group.service_id.clone(),
        service_generation: state.profile.service_generation,
        group_binding_ref: state.profile.group_binding_ref.clone(),
        application_manifest_ref: group.application_manifest_ref.clone(),
        protocol_ref: state.profile.protocol_ref.clone(),
        durable_log_ref: state.profile.durable_log_ref.clone(),
        snapshot_store_ref: state.profile.snapshot_store_ref.clone(),
        timer_profile_ref: state.profile.timer_profile_ref.clone(),
        entropy_profile_ref: state.profile.entropy_profile_ref.clone(),
        membership_ref: state.membership.membership_ref.clone(),
        placement_ref: state.profile.placement_ref.clone(),
        fencing_ref: state.profile.fencing_ref.clone(),
        supervision_ref: state.profile.supervision_ref.clone(),
        resource_profile_ref: state.profile.resource_profile_ref.clone(),
        fabric_binding_refs,
    }
}

fn start_plan(
    group: &crate::fabric_consistency::ConsistencyGroupBinding,
    state: ReplicaState,
    port_binding_refs: Vec<String>,
) -> ReplicaStartPlan {
    let timer_ref = state.active_election_timer_ref.clone();
    ReplicaStartPlan {
        state,
        service_id: group.service_id.clone(),
        application_manifest_ref: group.application_manifest_ref.clone(),
        initial_effects: vec![
            ReplicaEffect::PersistHardState {
                term: INITIAL_TERM,
                voted_for: None,
            },
            ReplicaEffect::ArmElectionTimer { timer_ref },
        ],
        port_binding_refs,
        production_admitted: false,
    }
}

fn transport_for(
    node_id: &str,
    endpoints: &std::collections::BTreeMap<String, crate::fabric_transport::CanonicalCrossProcessEndpoint>,
    protocol_ref: String,
) -> crate::error::Result<(IrohReplicaTransportPort, String)> {
    let peers = endpoints
        .iter()
        .filter(|(peer_id, _endpoint)| peer_id.as_str() != node_id)
        .map(|(peer_id, endpoint)| {
            (peer_id.clone(), crate::fabric_transport::cross_process::tests::client_input(endpoint.clone()))
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let session_ref = peers
        .values()
        .next()
        .map(|input| input.session_ref.clone())
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("live test node has no peers"))?;
    Ok((IrohReplicaTransportPort::new(protocol_ref, peers, live_timeout())?, session_ref))
}

fn durability_for_root(
    node_id: &str,
    durable_log_ref: String,
    snapshot_store_ref: String,
    root: &std::path::Path,
) -> crate::error::Result<RedbReplicaDurabilityPort> {
    let mut namespace = crate::fabric_durability::tests::descriptor();
    namespace.adapter_id = format!("live-cluster-{node_id}-adapter");
    namespace.namespace_id = format!("live-cluster-{node_id}-namespace");
    namespace.atomicity_domain.adapter_id.clone_from(&namespace.adapter_id);
    namespace.atomicity_domain.namespace_id.clone_from(&namespace.namespace_id);
    let adapter = crate::fabric_durability::RedbDurableStateAdapter::open(
        root,
        crate::fabric_durability::tests::profile(crate::fabric_durability::DurableAdapterKind::LiveRedb),
        namespace,
    )?;
    RedbReplicaDurabilityPort::new(adapter, durable_log_ref, snapshot_store_ref)
}

fn time_for(
    group: &crate::fabric_consistency::ConsistencyGroupBinding,
    node_id: &str,
    time_profile: crate::fabric_time::AdmittedTimeProfile,
    entropy_binding_ref: String,
) -> crate::error::Result<(
    TokioReplicaTimePort<crate::fabric_time::OperatingSystemEntropySource>,
    tokio::sync::mpsc::Receiver<ReplicaEvent>,
)> {
    // The replica inbox is the time profile's scheduler queue: its admitted depth bounds undrained
    // timer events. At most one election and one heartbeat timer task is armed per port; a full
    // inbox parks the timer task, so events are delayed, never dropped.
    let capacity = usize::try_from(time_profile.max_scheduler_queue_depth)
        .ok()
        .filter(|capacity| *capacity > 0)
        .ok_or_else(|| {
            crate::error::MoltenError::invalid_harness("live Raft inbox needs a positive admitted queue depth")
        })?;
    let (sender, receiver) = tokio::sync::mpsc::channel(capacity);
    let config = TokioReplicaTimeConfig {
        profile: time_profile,
        generation: group.service_generation,
        service_id: format!("{}-{node_id}", group.service_id),
        capability_ref: super::super::tests::test_ref(&format!("live-cluster-{node_id}-time-capability")),
        entropy_binding_ref,
        tick_duration: std::time::Duration::from_secs(LIVE_TICK_SECONDS),
        heartbeat_ticks: LIVE_HEARTBEAT_TICKS,
        election_min_ticks: LIVE_ELECTION_MIN_TICKS,
        election_max_ticks: LIVE_ELECTION_MAX_TICKS,
    };
    Ok((TokioReplicaTimePort::new_operating_system(config, sender)?, receiver))
}

fn application_for(
    group: &crate::fabric_consistency::ConsistencyGroupBinding,
) -> crate::error::Result<AdmittedReplicaApplicationPort<LiveApplicationHandler>> {
    AdmittedReplicaApplicationPort::new(
        ReplicaApplicationConfig {
            group_binding_ref: group.binding_ref.clone(),
            application_manifest_ref: group.application_manifest_ref.clone(),
            handler_ref: super::super::tests::test_ref("live-cluster-application-handler"),
            command_schema_refs: std::collections::BTreeSet::from([super::super::tests::test_ref(
                "live-cluster-command-schema",
            )]),
            initial_applied_index: INITIAL_COMMIT_INDEX,
        },
        LiveApplicationHandler::default(),
    )
}

fn control_for(
    group: &crate::fabric_consistency::ConsistencyGroupBinding,
    supervision_ref: String,
) -> crate::error::Result<(ChannelReplicaControlPort, tokio::sync::mpsc::Receiver<ReplicaControlObservation>)> {
    let (sender, receiver) = tokio::sync::mpsc::channel(LIVE_CONTROL_OBSERVATION_CAPACITY);
    let port = ChannelReplicaControlPort::new(
        ReplicaControlConfig {
            service_id: group.service_id.clone(),
            service_generation: group.service_generation,
            supervision_ref,
        },
        sender,
    )?;
    Ok((port, receiver))
}

pub(in crate::fabric_consistency::raft) async fn close_node(node: LiveNode) {
    let LiveNode {
        service,
        listener,
        session_ref: _,
        recovery_ref: _,
        _workspace: _,
        _control_receiver: _,
    } = node;
    drop(service);
    listener
        .expect("live node listener")
        .drain_and_close(crate::fabric_transport::ListenerDrainReason::OperatorRequest)
        .await
        .expect("live listener cleanup");
}
