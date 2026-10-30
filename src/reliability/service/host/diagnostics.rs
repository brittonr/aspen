/// Diagnostic observation, not a readiness or policy grant. Readiness needs a fresh quorum read.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HostObservation {
    pub schema: String,
    pub node_id: String,
    pub endpoint: String,
    pub group_binding_ref: String,
    pub membership_ref: String,
    pub service_generation: u64,
    pub source_binding_ref: String,
    pub source_validation_ref: String,
    pub production_admitted: bool,
    pub observation_ref: String,
}

pub(super) fn observe(
    replica: &crate::fabric_consistency::raft::ScopedLiveReplica<super::launch::Ports>,
    source: &crate::node_runtime::ValidatedNodeSourceGate,
    endpoint: std::net::SocketAddr,
) -> crate::error::Result<HostObservation> {
    let state: &crate::fabric_consistency::raft::ReplicaState = replica.state();
    let endpoint = endpoint.to_string();
    let observation_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
        "live-registry-host-observation-v1",
        vec![
            crate::preserves_rail::string(&state.node_id),
            crate::preserves_rail::string(&endpoint),
            crate::preserves_rail::string(&state.profile.group_binding_ref),
            crate::preserves_rail::string(&state.membership.membership_ref),
            crate::preserves_rail::u64_value(state.profile.service_generation),
            crate::preserves_rail::string(source.binding_ref()),
            crate::preserves_rail::string(source.validation_ref()),
            crate::preserves_rail::string("nonproduction-no-readiness-or-authority-claim"),
        ],
    ))?;
    Ok(HostObservation {
        schema: "molten.live-registry-host-observation.v1".into(),
        node_id: state.node_id.clone(),
        endpoint,
        group_binding_ref: state.profile.group_binding_ref.clone(),
        membership_ref: state.membership.membership_ref.clone(),
        service_generation: state.profile.service_generation,
        source_binding_ref: source.binding_ref().into(),
        source_validation_ref: source.validation_ref().into(),
        production_admitted: replica.production_admitted(),
        observation_ref,
    })
}
