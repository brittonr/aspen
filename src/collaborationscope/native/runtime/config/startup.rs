pub(super) async fn start(
    root: &crate::node_state::DirectoryRoot,
    mut admitted: super::AdmittedPlan,
) -> crate::error::Result<super::super::Parts> {
    let input = &admitted.input;
    let replay = replay(root, input)?;
    let authority = admitted.authority.bind(root, &admitted.time, replay)?;
    let adapter = durable(root, &input.raft_store)?;
    let has_raft_state = !adapter.state().durable_log.is_empty() || !adapter.state().snapshots.is_empty();
    let durability = crate::fabric_consistency::raft::RedbReplicaPort::new(
        adapter,
        input.replica.durable_log_ref.clone(),
        input.replica.snapshot_store_ref.clone(),
    )?
    .reconcile_repaired_storage()?;
    if has_raft_state {
        admitted.plan = durability.plan_recovery(admitted.plan)?.start_plan;
    }
    let application = application(root, input, has_raft_state)?;
    let identity = identity(&admitted.plan);
    let (sender, inbox) = tokio::sync::mpsc::channel(
        usize::try_from(admitted.time.max_scheduler_queue_depth).map_err(super::codec::invalid)?,
    );
    let time = super::assembly::timer(input, admitted.time, sender.clone())?;
    let (control_port, control) = super::assembly::control(input)?;
    let application = super::assembly::application(input, application)?;
    let (listener, transport) = super::transport::open(root, input).await?;
    let ports = crate::fabric_consistency::raft::assemble_scoped_concrete_replica(
        identity,
        crate::fabric_consistency::raft::ReplicaAdapterSet {
            durability,
            transport,
            time,
            application,
            control: control_port,
        },
    )?;
    let service = crate::fabric_consistency::raft::ScopedLiveReplica::start(admitted.plan, ports, inbox).await?;
    let ingress = super::assembly::ingress(input, listener)?;
    Ok(super::super::Parts {
        service,
        ingress,
        sender,
        control,
        installation: admitted.installation,
        timeout: std::time::Duration::from_millis(input.timeout_ms),
        authority,
    })
}

fn durable(
    root: &crate::node_state::DirectoryRoot,
    input: &crate::live_reliability::service::Durability,
) -> crate::error::Result<crate::fabric_durability::RedbDurableStateAdapter> {
    let profile = input.profile.clone().into_profile()?;
    let namespace = input.namespace.clone().into_descriptor(&profile)?;
    let store = crate::local_store::DurableStoreRoot::from_ledger(
        &root.ledger_store()?,
        &crate::local_store::Locator::parse(&input.storage_namespace)?,
    )?;
    crate::fabric_durability::RedbDurableStateAdapter::open_root(store, profile, namespace)
}

fn replay(
    root: &crate::node_state::DirectoryRoot,
    input: &super::Input,
) -> crate::error::Result<super::super::super::Database> {
    let adapter = durable(root, &input.replay_store)?;
    let domain = adapter.state().descriptor.atomicity_domain.clone();
    super::super::super::Database::new(adapter, domain)
}

fn application(
    root: &crate::node_state::DirectoryRoot,
    input: &super::Input,
    has_raft_state: bool,
) -> crate::error::Result<super::super::super::Application> {
    let adapter = durable(root, &input.scope_store)?;
    let domain = adapter.state().descriptor.atomicity_domain.clone();
    let mut store = super::super::super::Database::new(adapter, domain)?;
    match store.load()? {
        None if !has_raft_state => store.initialize(&genesis(input)?)?,
        None => return Err(super::codec::invalid("Raft recovery requires genuine existing scope state")),
        Some(state) => {
            if state.currentness.service_generation != input.group.service_generation
                || (!has_raft_state
                    && (!state.scopes.is_empty()
                        || !state.bindings.is_empty()
                        || !state.operations.is_empty()
                        || state.currentness.committed_index != 0))
            {
                return Err(super::codec::invalid("scope durable state differs from replica startup"));
            }
        }
    }
    let values = root.ledger()?.open_subdir(&crate::node_state::RelativePath::parse(&input.callback_namespace)?)?;
    let values = crate::system_extension::CapabilityCallbackValuePort::new(values)?;
    super::super::super::Application::open(store, Box::new(values))
}

fn genesis(input: &super::Input) -> crate::error::Result<super::super::super::State> {
    let genesis = input.genesis.as_ref().ok_or_else(|| super::codec::invalid("missing explicit empty genesis"))?;
    Ok(super::super::super::State {
        scopes: std::collections::BTreeMap::new(),
        bindings: std::collections::BTreeMap::new(),
        operations: std::collections::BTreeMap::new(),
        currentness: super::super::super::Currentness {
            service_generation: input.group.service_generation,
            consistency_epoch: genesis.consistency_epoch,
            committed_index: 0,
            logical_now: 0,
            revocation: molten_core::nominal::RevocationRef::new(genesis.revocation_ref.clone())
                .map_err(|error| super::codec::invalid(format!("{error:?}")))?,
            observation: molten_core::nominal::EvidenceRef::new(genesis.observation_ref.clone())
                .map_err(|error| super::codec::invalid(format!("{error:?}")))?,
        },
    })
}

fn identity(
    plan: &crate::fabric_consistency::raft::ReplicaStartPlan,
) -> crate::fabric_consistency::raft::ReplicaRuntimePortIdentity {
    let profile = &plan.state.profile;
    crate::fabric_consistency::raft::ReplicaRuntimePortIdentity {
        service_id: plan.service_id.clone(),
        service_generation: profile.service_generation,
        group_binding_ref: profile.group_binding_ref.clone(),
        application_manifest_ref: plan.application_manifest_ref.clone(),
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
