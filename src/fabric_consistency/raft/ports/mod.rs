use super::*;

pub trait BoundLiveReplicaEffectPorts: LiveReplicaEffectPorts {
    fn validate_start(&self, plan: &ReplicaStartPlan) -> crate::error::Result<()>;
}

impl<D, N, T, A, C> BoundLiveReplicaEffectPorts for ReplicaPortBundle<D, N, T, A, C>
where Self: LiveReplicaEffectPorts
{
    fn validate_start(&self, plan: &ReplicaStartPlan) -> crate::error::Result<()> {
        validate_replica_runtime_identity_for_start(self.identity(), plan).map_err(replica_failure)
    }
}

pub type ConcreteReplicaPortBundle<S, H> = ReplicaPortBundle<
    RedbReplicaPort,
    ReplicaTransportPort,
    TokioReplicaPort<S>,
    AdmittedReplicaPort<H>,
    ChannelReplicaPort,
>;

/// The concrete live adapter set bound by a scoped replica.
pub type ConcreteReplicaAdapterSet<S, H> = ReplicaAdapterSet<
    RedbReplicaPort,
    ReplicaTransportPort,
    TokioReplicaPort<S>,
    AdmittedReplicaPort<H>,
    ChannelReplicaPort,
>;

pub fn assemble_scoped_concrete_replica<S, H>(
    identity: ReplicaRuntimePortIdentity,
    adapters: ConcreteReplicaAdapterSet<S, H>,
) -> crate::error::Result<ConcreteReplicaPortBundle<S, H>>
where
    S: crate::fabric_time::CryptographicEntropySource,
    H: CommittedBatchHandler,
{
    validate_concrete_replica_port_identity(&identity, &adapters)?;
    ReplicaPortBundle::new(identity, adapters).map_err(replica_failure)
}

pub fn validate_concrete_replica_port_identity<S, H>(
    identity: &ReplicaRuntimePortIdentity,
    adapters: &ConcreteReplicaAdapterSet<S, H>,
) -> crate::error::Result<()>
where
    S: crate::fabric_time::CryptographicEntropySource,
    H: CommittedBatchHandler,
{
    let ReplicaAdapterSet {
        durability,
        transport,
        time,
        application,
        control,
    } = adapters;
    let is_exact = identity.durable_log_ref == durability.durable_log_ref()
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
    if !is_exact {
        return Err(crate::error::Failure::invalid_harness(
            "live Raft concrete adapter identity does not match the runtime port cohort",
        ));
    }
    Ok(())
}

impl<D: ReplicaDurabilityEffects, N, T, A, C> ReplicaDurabilityEffects for ReplicaPortBundle<D, N, T, A, C> {
    fn persist_hard_state(
        &mut self,
        term: u64,
        voted_for: Option<&str>,
        is_recovery_required: bool,
    ) -> crate::error::Result<String> {
        self.durability.persist_hard_state(term, voted_for, is_recovery_required)
    }

    fn persist_entries(
        &mut self,
        truncate_from: Option<u64>,
        entries: &[ReplicatedEntry],
    ) -> crate::error::Result<String> {
        self.durability.persist_entries(truncate_from, entries)
    }

    fn flush_log(&mut self, through_index: u64) -> crate::error::Result<String> {
        self.durability.flush_log(through_index)
    }

    fn persist_commit(&mut self, through_index: u64) -> crate::error::Result<String> {
        self.durability.persist_commit(through_index)
    }

    fn persist_snapshot(
        &mut self,
        snapshot: &ReplicaSnapshot,
        is_suffix_retained: bool,
        retained: &ReplicaRetainedState<'_>,
    ) -> crate::error::Result<String> {
        self.durability.persist_snapshot(snapshot, is_suffix_retained, retained)
    }
}

impl<D, N: ReplicaTransportEffects, T, A, C> ReplicaTransportEffects for ReplicaPortBundle<D, N, T, A, C> {
    fn send<'a>(&'a mut self, envelope: &'a ReplicaMessageEnvelope) -> ReplicaTransportFuture<'a> {
        self.transport.send(envelope)
    }
}

impl<D, N, T: ReplicaTimeEffects, A, C> ReplicaTimeEffects for ReplicaPortBundle<D, N, T, A, C> {
    fn arm_election_timer(&mut self, timer_ref: &str) -> crate::error::Result<String> {
        self.time.arm_election_timer(timer_ref)
    }

    fn arm_heartbeat_timer(&mut self) -> crate::error::Result<String> {
        self.time.arm_heartbeat_timer()
    }
}

impl<D, N, T, A: ReplicaApplicationEffects, C> ReplicaApplicationEffects for ReplicaPortBundle<D, N, T, A, C> {
    fn restore_snapshot(&mut self, snapshot: &ReplicaSnapshot) -> crate::error::Result<String> {
        self.application.restore_snapshot(snapshot)
    }

    fn apply_committed(&mut self, entries: &[ReplicatedEntry]) -> crate::error::Result<String> {
        self.application.apply_committed(entries)
    }
}

impl<D, N, T, A, C: ReplicaControlEffects> ReplicaControlEffects for ReplicaPortBundle<D, N, T, A, C> {
    fn proposal_outcome(
        &mut self,
        request_ref: &str,
        disposition: ProposalDisposition,
        committed_index: Option<u64>,
    ) -> crate::error::Result<String> {
        self.control.proposal_outcome(request_ref, disposition, committed_index)
    }

    fn read_outcome(
        &mut self,
        request_ref: &str,
        mode: ReadMode,
        disposition: ReadDisposition,
        observed_index: u64,
    ) -> crate::error::Result<String> {
        self.control.read_outcome(request_ref, mode, disposition, observed_index)
    }

    fn lifecycle_changed(&mut self, lifecycle: ReplicaLifecycle) -> crate::error::Result<String> {
        self.control.lifecycle_changed(lifecycle)
    }
}
