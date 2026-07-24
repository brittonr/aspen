use std::collections::BTreeSet;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaRuntimePortIdentity {
    pub service_id: String,
    pub service_generation: u64,
    pub group_binding_ref: String,
    pub application_manifest_ref: String,
    pub protocol_ref: String,
    pub durable_log_ref: String,
    pub snapshot_store_ref: String,
    pub timer_profile_ref: String,
    pub entropy_profile_ref: String,
    pub membership_ref: String,
    pub placement_ref: String,
    pub fencing_ref: String,
    pub supervision_ref: String,
    pub resource_profile_ref: String,
    pub fabric_binding_refs: Vec<String>,
}

pub trait BoundLiveReplicaEffectPorts: LiveReplicaEffectPorts {
    fn validate_start(&self, plan: &ReplicaStartPlan) -> Result<()>;
}

pub struct ReplicaPortBundle<D, N, T, A, C> {
    identity: ReplicaRuntimePortIdentity,
    pub durability: D,
    pub transport: N,
    pub time: T,
    pub application: A,
    pub control: C,
}

impl<D, N, T, A, C> ReplicaPortBundle<D, N, T, A, C> {
    pub fn new(
        identity: ReplicaRuntimePortIdentity,
        durability: D,
        transport: N,
        time: T,
        application: A,
        control: C,
    ) -> Result<Self> {
        validate_runtime_identity(&identity)?;
        Ok(Self {
            identity,
            durability,
            transport,
            time,
            application,
            control,
        })
    }

    pub const fn identity(&self) -> &ReplicaRuntimePortIdentity {
        &self.identity
    }
}

impl<D, N, T, A, C> BoundLiveReplicaEffectPorts for ReplicaPortBundle<D, N, T, A, C>
where Self: LiveReplicaEffectPorts
{
    fn validate_start(&self, plan: &ReplicaStartPlan) -> Result<()> {
        validate_replica_runtime_identity_for_start(&self.identity, plan)
    }
}

fn validate_runtime_identity(identity: &ReplicaRuntimePortIdentity) -> Result<()> {
    if identity.service_id.is_empty() || identity.service_generation == 0 {
        return Err(MoltenError::invalid_harness(
            "live Raft runtime ports require service identity and positive generation",
        ));
    }
    for reference in [
        &identity.group_binding_ref,
        &identity.application_manifest_ref,
        &identity.protocol_ref,
        &identity.durable_log_ref,
        &identity.snapshot_store_ref,
        &identity.timer_profile_ref,
        &identity.entropy_profile_ref,
        &identity.membership_ref,
        &identity.placement_ref,
        &identity.fencing_ref,
        &identity.supervision_ref,
        &identity.resource_profile_ref,
    ] {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    if identity.fabric_binding_refs.len() != REQUIRED_REPLICA_PORTS.len() {
        return Err(MoltenError::invalid_harness(
            "live Raft runtime ports require the complete admitted fabric binding cohort",
        ));
    }
    let unique = identity.fabric_binding_refs.iter().collect::<BTreeSet<_>>();
    if unique.len() != identity.fabric_binding_refs.len() {
        return Err(MoltenError::invalid_harness("live Raft runtime ports contain duplicate fabric bindings"));
    }
    for reference in &identity.fabric_binding_refs {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    Ok(())
}

pub fn validate_replica_runtime_identity_for_start(
    identity: &ReplicaRuntimePortIdentity,
    plan: &ReplicaStartPlan,
) -> Result<()> {
    validate_runtime_identity(identity)?;
    let state = &plan.state;
    let exact = identity.service_id == plan.service_id
        && identity.service_generation == state.profile.service_generation
        && identity.group_binding_ref == state.profile.group_binding_ref
        && identity.application_manifest_ref == plan.application_manifest_ref
        && identity.protocol_ref == state.profile.protocol_ref
        && identity.durable_log_ref == state.profile.durable_log_ref
        && identity.snapshot_store_ref == state.profile.snapshot_store_ref
        && identity.timer_profile_ref == state.profile.timer_profile_ref
        && identity.entropy_profile_ref == state.profile.entropy_profile_ref
        && identity.membership_ref == state.membership.membership_ref
        && identity.placement_ref == state.profile.placement_ref
        && identity.fencing_ref == state.profile.fencing_ref
        && identity.supervision_ref == state.profile.supervision_ref
        && identity.resource_profile_ref == state.profile.resource_profile_ref;
    let expected_bindings = plan.port_binding_refs.iter().collect::<BTreeSet<_>>();
    let actual_bindings = identity.fabric_binding_refs.iter().collect::<BTreeSet<_>>();
    if !exact || actual_bindings != expected_bindings {
        return Err(MoltenError::invalid_harness(
            "live Raft runtime port identity does not match the admitted start plan",
        ));
    }
    Ok(())
}

pub type ConcreteReplicaPortBundle<S, H> = ReplicaPortBundle<
    RedbReplicaDurabilityPort,
    IrohReplicaTransportPort,
    TokioReplicaTimePort<S>,
    AdmittedReplicaApplicationPort<H>,
    ChannelReplicaControlPort,
>;

pub fn assemble_scoped_concrete_replica_ports<S, H>(
    identity: ReplicaRuntimePortIdentity,
    durability: RedbReplicaDurabilityPort,
    transport: IrohReplicaTransportPort,
    time: TokioReplicaTimePort<S>,
    application: AdmittedReplicaApplicationPort<H>,
    control: ChannelReplicaControlPort,
) -> Result<ConcreteReplicaPortBundle<S, H>>
where
    S: crate::fabric_time::CryptographicEntropySource,
    H: CommittedBatchHandler,
{
    validate_concrete_replica_port_identity(&identity, &durability, &transport, &time, &application, &control)?;
    ReplicaPortBundle::new(identity, durability, transport, time, application, control)
}

pub fn validate_concrete_replica_port_identity<S, H>(
    identity: &ReplicaRuntimePortIdentity,
    durability: &RedbReplicaDurabilityPort,
    transport: &IrohReplicaTransportPort,
    time: &TokioReplicaTimePort<S>,
    application: &AdmittedReplicaApplicationPort<H>,
    control: &ChannelReplicaControlPort,
) -> Result<()>
where
    S: crate::fabric_time::CryptographicEntropySource,
    H: CommittedBatchHandler,
{
    let exact = identity.durable_log_ref == durability.durable_log_ref()
        && identity.snapshot_store_ref == durability.snapshot_store_ref()
        && identity.protocol_ref == transport.protocol_ref()
        && identity.timer_profile_ref == time.timer_profile_ref()
        && identity.entropy_profile_ref == time.entropy_binding_ref()
        && identity.service_generation == time.service_generation()
        && identity.group_binding_ref == application.group_binding_ref()
        && identity.application_manifest_ref == application.application_manifest_ref()
        && identity.service_id == control.service_id()
        && identity.service_generation == control.service_generation()
        && identity.supervision_ref == control.supervision_ref();
    if !exact {
        return Err(MoltenError::invalid_harness(
            "live Raft concrete adapter identity does not match the runtime port cohort",
        ));
    }
    Ok(())
}

impl<D: ReplicaDurabilityEffects, N, T, A, C> ReplicaDurabilityEffects for ReplicaPortBundle<D, N, T, A, C> {
    fn persist_hard_state(&mut self, term: u64, voted_for: Option<&str>) -> Result<String> {
        self.durability.persist_hard_state(term, voted_for)
    }

    fn persist_entries(&mut self, truncate_from: Option<u64>, entries: &[ReplicatedEntry]) -> Result<String> {
        self.durability.persist_entries(truncate_from, entries)
    }

    fn flush_log(&mut self, through_index: u64) -> Result<String> {
        self.durability.flush_log(through_index)
    }

    fn persist_commit(&mut self, through_index: u64) -> Result<String> {
        self.durability.persist_commit(through_index)
    }

    fn persist_snapshot(&mut self, snapshot: &ReplicaSnapshot) -> Result<String> {
        self.durability.persist_snapshot(snapshot)
    }
}

impl<D, N: ReplicaTransportEffects, T, A, C> ReplicaTransportEffects for ReplicaPortBundle<D, N, T, A, C> {
    fn send<'a>(&'a mut self, envelope: &'a ReplicaMessageEnvelope) -> ReplicaTransportFuture<'a> {
        self.transport.send(envelope)
    }
}

impl<D, N, T: ReplicaTimeEffects, A, C> ReplicaTimeEffects for ReplicaPortBundle<D, N, T, A, C> {
    fn arm_election_timer(&mut self, timer_ref: &str) -> Result<String> {
        self.time.arm_election_timer(timer_ref)
    }

    fn arm_heartbeat_timer(&mut self) -> Result<String> {
        self.time.arm_heartbeat_timer()
    }
}

impl<D, N, T, A: ReplicaApplicationEffects, C> ReplicaApplicationEffects for ReplicaPortBundle<D, N, T, A, C> {
    fn restore_snapshot(&mut self, snapshot: &ReplicaSnapshot) -> Result<String> {
        self.application.restore_snapshot(snapshot)
    }

    fn apply_committed(&mut self, entries: &[ReplicatedEntry]) -> Result<String> {
        self.application.apply_committed(entries)
    }
}

impl<D, N, T, A, C: ReplicaControlEffects> ReplicaControlEffects for ReplicaPortBundle<D, N, T, A, C> {
    fn proposal_outcome(
        &mut self,
        request_ref: &str,
        disposition: ProposalDisposition,
        committed_index: Option<u64>,
    ) -> Result<String> {
        self.control.proposal_outcome(request_ref, disposition, committed_index)
    }

    fn read_outcome(
        &mut self,
        request_ref: &str,
        mode: crate::fabric_consistency::ConsistencyReadMode,
        disposition: ReadDisposition,
        observed_index: u64,
    ) -> Result<String> {
        self.control.read_outcome(request_ref, mode, disposition, observed_index)
    }

    fn lifecycle_changed(&mut self, lifecycle: ReplicaLifecycle) -> Result<String> {
        self.control.lifecycle_changed(lifecycle)
    }
}
