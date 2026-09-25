
// r[verify molten.fabric_consistency.live_service_ports]
#[test]
fn running_host_projects_exact_required_live_replica_ports() {
    let mut host = host_without(None);
    host.activate(ACTIVATION_TICK).expect("active host");
    let group = active_group_for_host(&host);
    let plan =
        plan_live_replica_start_for_host(&host, group.clone(), NODE_A.to_string(), membership(&group), profile(&group))
            .expect("host-backed replica start plan");

    assert_eq!(plan.port_binding_refs.len(), REQUIRED_REPLICA_PORTS.len());
    assert!(!plan.production_admitted);
}

// r[verify molten.fabric_consistency.live_service_ports]
#[test]
fn host_start_denies_inactive_supervision_and_missing_durable_log() {
    let inactive = host_without(None);
    let inactive_group = active_group_for_host(&inactive);
    let inactive_error = plan_live_replica_start_for_host(
        &inactive,
        inactive_group.clone(),
        NODE_A.to_string(),
        membership(&inactive_group),
        profile(&inactive_group),
    )
    .expect_err("inactive host must deny");
    assert!(inactive_error.to_string().contains("running supervised"));

    let mut missing_log = host_without(Some(crate::fabric_durability::FABRIC_DURABLE_LOG_PORT_ID));
    missing_log.activate(ACTIVATION_TICK).expect("active incomplete host");
    let missing_log_group = active_group_for_host(&missing_log);
    let missing_log_error = plan_live_replica_start_for_host(
        &missing_log,
        missing_log_group.clone(),
        NODE_A.to_string(),
        membership(&missing_log_group),
        profile(&missing_log_group),
    )
    .expect_err("missing durable log must deny");
    assert!(missing_log_error.to_string().contains(crate::fabric_durability::FABRIC_DURABLE_LOG_PORT_ID));
}

// r[verify molten.fabric_consistency.live_service_ports]
#[test]
fn host_start_denies_group_policy_not_admitted_by_manifest() {
    let mut host = host_without(None);
    host.activate(ACTIVATION_TICK).expect("active host");
    let mut policies = host.manifest().manifest().policy_refs.clone();
    policies.push(test_ref("unadmitted-policy"));
    let group = active_group_for_host_with_policies(&host, policies);
    let error =
        plan_live_replica_start_for_host(&host, group.clone(), NODE_A.to_string(), membership(&group), profile(&group))
            .expect_err("unadmitted group policy must deny");
    assert!(error.to_string().contains("policy refs are not admitted"));
}

#[derive(Debug, Default)]
struct ServiceDurabilityPort {
    hard_state_writes: usize,
}

impl ReplicaDurabilityEffects for ServiceDurabilityPort {
    fn persist_hard_state(&mut self, _term: u64, _voted_for: Option<&str>) -> crate::error::Result<String> {
        self.hard_state_writes += 1;
        Ok(test_ref("service-hard-state"))
    }

    fn persist_entries(
        &mut self,
        _truncate_from: Option<u64>,
        _entries: &[ReplicatedEntry],
    ) -> crate::error::Result<String> {
        Ok(test_ref("service-entries"))
    }

    fn flush_log(&mut self, _through_index: u64) -> crate::error::Result<String> {
        Ok(test_ref("service-flush"))
    }

    fn persist_commit(&mut self, _through_index: u64) -> crate::error::Result<String> {
        Ok(test_ref("service-commit"))
    }

    fn persist_snapshot(&mut self, _snapshot: &ReplicaSnapshot) -> crate::error::Result<String> {
        Ok(test_ref("service-snapshot"))
    }
}

#[derive(Debug, Default)]
struct ServiceTransportPort {
    sent: usize,
}

impl ReplicaTransportEffects for ServiceTransportPort {
    fn send<'a>(&'a mut self, _envelope: &'a ReplicaMessageEnvelope) -> ReplicaTransportFuture<'a> {
        self.sent += 1;
        Box::pin(async { Ok(test_ref("service-transport")) })
    }
}

#[derive(Debug, Default)]
struct ServiceTimePort {
    election_timer_refs: Vec<String>,
    heartbeat_arms: usize,
}

impl ReplicaTimeEffects for ServiceTimePort {
    fn arm_election_timer(&mut self, timer_ref: &str) -> crate::error::Result<String> {
        self.election_timer_refs.push(timer_ref.to_string());
        Ok(test_ref("service-election-timer"))
    }

    fn arm_heartbeat_timer(&mut self) -> crate::error::Result<String> {
        self.heartbeat_arms += 1;
        Ok(test_ref("service-heartbeat-timer"))
    }
}

#[derive(Debug, Default)]
struct ServiceApplicationHandler;

impl CommittedBatchHandler for ServiceApplicationHandler {
    fn restore_snapshot(&mut self, _snapshot: &ApplicationSnapshotRestore) -> crate::error::Result<String> {
        Ok(test_ref("service-application-snapshot-handler"))
    }

    fn apply_batch(&mut self, _commands: &[ApplicationCommand]) -> crate::error::Result<String> {
        Ok(test_ref("service-application-handler"))
    }
}

// r[verify molten.fabric_consistency.live_service_ports]
#[tokio::test]
async fn scoped_service_executes_startup_and_current_timer_through_separate_ports() {
    let mut host = host_without(None);
    host.activate(ACTIVATION_TICK).expect("active host");
    let group = active_group_for_host(&host);
    let replica_profile = profile(&group);
    let plan = plan_live_replica_start_for_host(
        &host,
        group.clone(),
        NODE_A.to_string(),
        membership(&group),
        replica_profile.clone(),
    )
    .expect("host-backed replica start plan");
    let initial_timer_ref = plan.state.active_election_timer_ref.clone();
    let runtime_identity = runtime_identity(&plan);
    assert_substituted_identity_denied(&runtime_identity, &plan);
    let (event_sender, event_receiver) = tokio::sync::mpsc::channel(SERVICE_EVENT_CAPACITY);
    let (control_sender, _control_receiver) = tokio::sync::mpsc::channel(SERVICE_CONTROL_CAPACITY);
    let application = application_port_for(&group);
    let control = ChannelReplicaControlPort::new(
        ReplicaControlConfig {
            service_id: group.service_id.clone(),
            service_generation: group.service_generation,
            supervision_ref: replica_profile.supervision_ref,
        },
        control_sender,
    )
    .expect("control port");
    let ports = ReplicaPortBundle::new(runtime_identity, ReplicaAdapterSet {
        durability: ServiceDurabilityPort::default(),
        transport: ServiceTransportPort::default(),
        time: ServiceTimePort::default(),
        application,
        control,
    })
    .expect("bound runtime port bundle");
    let mut service = ScopedLiveReplicaService::start(plan, ports, event_receiver).await.expect("scoped live service");

    assert!(!service.production_admitted());
    assert_eq!(service.startup_observations().len(), EXPECTED_STARTUP_OBSERVATIONS);
    assert_eq!(service.ports().durability.hard_state_writes, 1);
    assert_eq!(service.ports().time.election_timer_refs, vec![initial_timer_ref.clone()]);

    event_sender
        .try_send(ReplicaEvent::ElectionTimeout {
            timer_ref: initial_timer_ref.clone(),
        })
        .expect("queue election timeout");
    let outcome = service
        .run_next(std::time::Duration::from_millis(SERVICE_EVENT_TIMEOUT_MILLISECONDS))
        .await
        .expect("bounded service turn");
    assert!(matches!(outcome, ReplicaExecutionOutcome::Applied(_)));
    assert_eq!(service.state().role, ReplicaRole::Candidate);
    assert_eq!(service.ports().transport.sent, EXPECTED_ELECTION_PORT_CALLS);
    assert_eq!(service.ports().durability.hard_state_writes, EXPECTED_ELECTION_PORT_CALLS);
    assert_eq!(service.ports().time.election_timer_refs.len(), EXPECTED_ELECTION_PORT_CALLS);
    assert_eq!(service.ports().time.election_timer_refs.last(), Some(&service.state().active_election_timer_ref));

    let stale = service
        .handle_event(ReplicaEvent::ElectionTimeout {
            timer_ref: initial_timer_ref,
        })
        .await;
    assert!(matches!(stale, ReplicaExecutionOutcome::Denied { .. }));
    assert_eq!(service.ports().transport.sent, EXPECTED_ELECTION_PORT_CALLS);
    assert_eq!(service.ports().durability.hard_state_writes, EXPECTED_ELECTION_PORT_CALLS);
}

fn assert_substituted_identity_denied(runtime_identity: &ReplicaRuntimePortIdentity, plan: &ReplicaStartPlan) {
    let mut mismatched_identity = runtime_identity.clone();
    mismatched_identity.protocol_ref = test_ref("substituted-runtime-protocol");
    let mismatch = validate_replica_runtime_identity_for_start(&mismatched_identity, plan)
        .expect_err("substituted runtime identity must deny before effects");
    assert!(mismatch.to_string().contains("does not match the admitted start plan"));
}

fn application_port_for(group: &ConsistencyGroupBinding) -> AdmittedReplicaApplicationPort<ServiceApplicationHandler> {
    AdmittedReplicaApplicationPort::new(
        ReplicaApplicationConfig {
            group_binding_ref: group.binding_ref.clone(),
            application_manifest_ref: group.application_manifest_ref.clone(),
            handler_ref: test_ref("service-application-binding"),
            command_schema_refs: std::collections::BTreeSet::from([test_ref("service-command-schema")]),
            initial_applied_index: INITIAL_COMMIT_INDEX,
        },
        ServiceApplicationHandler,
    )
    .expect("application port")
}

fn runtime_identity(plan: &ReplicaStartPlan) -> ReplicaRuntimePortIdentity {
    ReplicaRuntimePortIdentity {
        service_id: plan.service_id.clone(),
        service_generation: plan.state.profile.service_generation,
        group_binding_ref: plan.state.profile.group_binding_ref.clone(),
        application_manifest_ref: plan.application_manifest_ref.clone(),
        protocol_ref: plan.state.profile.protocol_ref.clone(),
        durable_log_ref: plan.state.profile.durable_log_ref.clone(),
        snapshot_store_ref: plan.state.profile.snapshot_store_ref.clone(),
        timer_profile_ref: plan.state.profile.timer_profile_ref.clone(),
        entropy_profile_ref: plan.state.profile.entropy_profile_ref.clone(),
        membership_ref: plan.state.membership.membership_ref.clone(),
        placement_ref: plan.state.profile.placement_ref.clone(),
        fencing_ref: plan.state.profile.fencing_ref.clone(),
        supervision_ref: plan.state.profile.supervision_ref.clone(),
        resource_profile_ref: plan.state.profile.resource_profile_ref.clone(),
        fabric_binding_refs: plan.port_binding_refs.clone(),
    }
}
