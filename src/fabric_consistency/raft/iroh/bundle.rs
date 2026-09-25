use super::*;

const ASSEMBLY_GENERATION: u64 = 1;
const ASSEMBLY_HEARTBEAT_TICKS: u64 = 1;
const ASSEMBLY_ELECTION_MIN_TICKS: u64 = 2;
const ASSEMBLY_ELECTION_MAX_TICKS: u64 = 3;
const ASSEMBLY_TICK_MILLISECONDS: u64 = 1;
const ASSEMBLY_FABRIC_BINDING_COUNT: usize = 7;
const ASSEMBLY_STARTUP_OBSERVATIONS: usize = 2;
const ASSEMBLY_EVENT_CAPACITY: usize = 4;
const ASSEMBLY_CONTROL_CAPACITY: usize = 4;

#[derive(Debug, Default)]
struct AssemblyApplicationHandler;

impl CommittedBatchHandler for AssemblyApplicationHandler {
    fn restore_snapshot(&mut self, _snapshot: &ApplicationSnapshotRestore) -> crate::error::Result<String> {
        Ok(super::tests::test_ref("assembled-application-snapshot-evidence"))
    }

    fn apply_batch(&mut self, _commands: &[ApplicationCommand]) -> crate::error::Result<String> {
        Ok(super::tests::test_ref("assembled-application-evidence"))
    }
}

// r[verify molten.fabric_consistency.live_service_ports]
#[tokio::test]
async fn concrete_port_assembly_denies_substitution_then_executes_bound_startup() {
    let group = super::tests::active_group();
    let mut replica_state = super::tests::started_state(&group, super::tests::NODE_A);
    assert_eq!(replica_state.profile.service_generation, ASSEMBLY_GENERATION);
    let listener = crate::fabric_transport::cross_process::tests::listener().await;
    let endpoint = listener.handoff().clone();
    let canonical_time = crate::fabric_time::tests::live_profile().profile;
    let refs = AssemblyRefs::new(canonical_time.profile_ref.clone());
    let mut peers = std::collections::BTreeMap::new();
    peers.insert(
        super::iroh_tests::NODE_B.to_string(),
        crate::fabric_transport::cross_process::tests::client_input(endpoint),
    );
    let transport = IrohReplicaTransportPort::new(
        refs.protocol_ref.clone(),
        peers,
        std::time::Duration::from_secs(super::iroh_tests::POSITIVE_TIMEOUT_SECONDS),
    )
    .expect("assembled Iroh transport");

    let root = crate::test_support::process_workspace("assembled-live-raft-redb").expect("workspace");
    let durability = assembly_durability_port(&root, &refs);
    let (event_sender, event_receiver) = tokio::sync::mpsc::channel(ASSEMBLY_EVENT_CAPACITY);
    let time = assembly_time_port(canonical_time, &group.service_id, &refs.entropy_profile_ref, event_sender);
    let application = assembly_application_port(&group);
    let (control_sender, _control_receiver) = tokio::sync::mpsc::channel(ASSEMBLY_CONTROL_CAPACITY);
    let control = ChannelReplicaControlPort::new(
        ReplicaControlConfig {
            service_id: group.service_id.clone(),
            service_generation: ASSEMBLY_GENERATION,
            supervision_ref: refs.supervision_ref.clone(),
        },
        control_sender,
    )
    .expect("assembled control port");

    refs.bind(&mut replica_state.profile);
    let fabric_binding_refs = (0..ASSEMBLY_FABRIC_BINDING_COUNT)
        .map(|index| super::tests::test_ref(&format!("assembled-fabric-binding-{index}")))
        .collect::<Vec<_>>();
    let identity = refs.identity(&group, &replica_state, fabric_binding_refs.clone());
    let mut mismatched = identity.clone();
    mismatched.protocol_ref = super::tests::test_ref("substituted-assembled-protocol");
    let adapters = ReplicaAdapterSet {
        durability,
        transport,
        time,
        application,
        control,
    };
    let error = validate_concrete_replica_port_identity(&mismatched, &adapters)
        .expect_err("concrete protocol substitution must deny");
    assert!(error.to_string().contains("concrete adapter identity"));

    let plan = assembly_start_plan(&group, replica_state, fabric_binding_refs);
    let bundle = assemble_scoped_concrete_replica_ports(identity, adapters).expect("concrete port assembly");
    let service = ScopedLiveReplicaService::start(plan, bundle, event_receiver)
        .await
        .expect("concrete scoped service startup");
    assert_eq!(service.startup_observations().len(), ASSEMBLY_STARTUP_OBSERVATIONS);
    assert!(!service.production_admitted());
    assert!(!service.ports().durability.adapter().state().durable_log.is_empty());
    drop(service);
    listener
        .drain_and_close(crate::fabric_transport::ListenerDrainReason::OperatorRequest)
        .await
        .expect("listener cleanup");
}

/// A live Redb durability port under `root` bound to the assembly's durable log and snapshot store
/// refs.
fn assembly_durability_port(root: &std::path::Path, refs: &AssemblyRefs) -> RedbReplicaDurabilityPort {
    let redb = crate::fabric_durability::RedbDurableStateAdapter::open(
        root,
        crate::fabric_durability::tests::profile(crate::fabric_durability::DurableAdapterKind::LiveRedb),
        crate::fabric_durability::tests::descriptor(),
    )
    .expect("assembled Redb adapter");
    RedbReplicaDurabilityPort::new(redb, refs.durable_log_ref.clone(), refs.snapshot_store_ref.clone())
        .expect("assembled durability port")
}

/// The protocol, durable log, snapshot store, timer, entropy, and supervision refs the assembled
/// ports bind.
struct AssemblyRefs {
    protocol_ref: String,
    durable_log_ref: String,
    snapshot_store_ref: String,
    timer_profile_ref: String,
    entropy_profile_ref: String,
    supervision_ref: String,
}

impl AssemblyRefs {
    fn new(timer_profile_ref: String) -> Self {
        Self {
            protocol_ref: super::tests::test_ref("assembled-Iroh-protocol"),
            durable_log_ref: super::tests::test_ref("assembled-durable-log"),
            snapshot_store_ref: super::tests::test_ref("assembled-snapshot-store"),
            timer_profile_ref,
            entropy_profile_ref: super::tests::test_ref("assembled-entropy-profile"),
            supervision_ref: super::tests::test_ref("assembled-supervision"),
        }
    }

    fn bind(&self, profile: &mut ReplicaProfile) {
        profile.protocol_ref.clone_from(&self.protocol_ref);
        profile.durable_log_ref.clone_from(&self.durable_log_ref);
        profile.snapshot_store_ref.clone_from(&self.snapshot_store_ref);
        profile.timer_profile_ref.clone_from(&self.timer_profile_ref);
        profile.entropy_profile_ref.clone_from(&self.entropy_profile_ref);
        profile.supervision_ref.clone_from(&self.supervision_ref);
    }

    fn identity(
        self,
        group: &crate::fabric_consistency::ConsistencyGroupBinding,
        replica_state: &ReplicaState,
        fabric_binding_refs: Vec<String>,
    ) -> ReplicaRuntimePortIdentity {
        ReplicaRuntimePortIdentity {
            service_id: group.service_id.clone(),
            service_generation: ASSEMBLY_GENERATION,
            group_binding_ref: group.binding_ref.clone(),
            application_manifest_ref: group.application_manifest_ref.clone(),
            protocol_ref: self.protocol_ref,
            durable_log_ref: self.durable_log_ref,
            snapshot_store_ref: self.snapshot_store_ref,
            timer_profile_ref: self.timer_profile_ref,
            entropy_profile_ref: self.entropy_profile_ref,
            membership_ref: replica_state.membership.membership_ref.clone(),
            placement_ref: replica_state.profile.placement_ref.clone(),
            fencing_ref: replica_state.profile.fencing_ref.clone(),
            supervision_ref: self.supervision_ref,
            resource_profile_ref: replica_state.profile.resource_profile_ref.clone(),
            fabric_binding_refs,
        }
    }
}

fn assembly_time_port(
    profile: crate::fabric_time::AdmittedTimeProfile,
    service_id: &str,
    entropy_binding_ref: &str,
    event_sender: tokio::sync::mpsc::Sender<ReplicaEvent>,
) -> TokioReplicaTimePort<crate::fabric_time::OperatingSystemEntropySource> {
    TokioReplicaTimePort::new_operating_system(
        TokioReplicaTimeConfig {
            profile,
            generation: ASSEMBLY_GENERATION,
            service_id: service_id.to_string(),
            capability_ref: super::tests::test_ref("assembled-time-capability"),
            entropy_binding_ref: entropy_binding_ref.to_string(),
            tick_duration: std::time::Duration::from_millis(ASSEMBLY_TICK_MILLISECONDS),
            heartbeat_ticks: ASSEMBLY_HEARTBEAT_TICKS,
            election_min_ticks: ASSEMBLY_ELECTION_MIN_TICKS,
            election_max_ticks: ASSEMBLY_ELECTION_MAX_TICKS,
        },
        event_sender,
    )
    .expect("assembled time port")
}

fn assembly_application_port(
    group: &crate::fabric_consistency::ConsistencyGroupBinding,
) -> AdmittedReplicaApplicationPort<AssemblyApplicationHandler> {
    AdmittedReplicaApplicationPort::new(
        ReplicaApplicationConfig {
            group_binding_ref: group.binding_ref.clone(),
            application_manifest_ref: group.application_manifest_ref.clone(),
            handler_ref: super::tests::test_ref("assembled-application-handler"),
            command_schema_refs: std::collections::BTreeSet::from([super::tests::test_ref("assembled-command-schema")]),
            initial_applied_index: INITIAL_COMMIT_INDEX,
        },
        AssemblyApplicationHandler,
    )
    .expect("assembled application port")
}

/// A start plan that persists the initial hard state and arms the replica's current election timer.
fn assembly_start_plan(
    group: &crate::fabric_consistency::ConsistencyGroupBinding,
    replica_state: ReplicaState,
    port_binding_refs: Vec<String>,
) -> ReplicaStartPlan {
    let startup_timer_ref = replica_state.active_election_timer_ref.clone();
    ReplicaStartPlan {
        state: replica_state,
        service_id: group.service_id.clone(),
        application_manifest_ref: group.application_manifest_ref.clone(),
        initial_effects: vec![
            ReplicaEffect::PersistHardState {
                term: INITIAL_TERM,
                voted_for: None,
            },
            ReplicaEffect::ArmElectionTimer {
                timer_ref: startup_timer_ref,
            },
        ],
        port_binding_refs,
        production_admitted: false,
    }
}
