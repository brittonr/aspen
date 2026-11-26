//! The runtime port cohort a replica binds and its exact match against the admitted start plan.
//!
//! The effect traits the adapters implement, and the async executor that drives them, stay in
//! the `molten` shell: their results carry the shell's classified I/O failures.

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

/// The five effect adapters a replica binds, before they are checked against its runtime identity.
pub struct ReplicaAdapterSet<D, N, T, A, C> {
    pub durability: D,
    pub transport: N,
    pub time: T,
    pub application: A,
    pub control: C,
}

/// The five effect adapters bound to a validated runtime identity.
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
        adapters: ReplicaAdapterSet<D, N, T, A, C>,
    ) -> crate::denial::Result<Self> {
        let ReplicaAdapterSet {
            durability,
            transport,
            time,
            application,
            control,
        } = adapters;
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

fn validate_runtime_identity(identity: &ReplicaRuntimePortIdentity) -> crate::denial::Result<()> {
    if identity.service_id.is_empty() || identity.service_generation == 0 {
        return Err(crate::denial::ReplicaError::Denied(
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
        crate::denial::refs::validate_content_ref(reference)?;
    }
    if identity.fabric_binding_refs.len() != crate::admission::REQUIRED_REPLICA_PORT_COUNT {
        return Err(crate::denial::ReplicaError::Denied(
            "live Raft runtime ports require the complete admitted fabric binding cohort",
        ));
    }
    let unique = identity.fabric_binding_refs.iter().collect::<alloc::collections::BTreeSet<_>>();
    if unique.len() != identity.fabric_binding_refs.len() {
        return Err(crate::denial::ReplicaError::Denied("live Raft runtime ports contain duplicate fabric bindings"));
    }
    for reference in &identity.fabric_binding_refs {
        crate::denial::refs::validate_content_ref(reference)?;
    }
    Ok(())
}

pub fn validate_replica_runtime_identity_for_start(
    identity: &ReplicaRuntimePortIdentity,
    plan: &crate::admission::ReplicaStartPlan,
) -> crate::denial::Result<()> {
    validate_runtime_identity(identity)?;
    let state = &plan.state;
    let is_exact = identity.service_id == plan.service_id
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
    let expected_bindings = plan.port_binding_refs.iter().collect::<alloc::collections::BTreeSet<_>>();
    let actual_bindings = identity.fabric_binding_refs.iter().collect::<alloc::collections::BTreeSet<_>>();
    if !is_exact || actual_bindings != expected_bindings {
        return Err(crate::denial::ReplicaError::Denied(
            "live Raft runtime port identity does not match the admitted start plan",
        ));
    }
    Ok(())
}
