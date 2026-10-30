/// Explicit admitted activation observation; the runtime never assigns lifecycle fields.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Activation {
    pub request_ref: String,
    pub authority_refs: Vec<String>,
    pub result_ref: String,
    pub evidence_refs: Vec<String>,
}

pub(super) fn activate(
    input: &crate::live_reliability::service::Group,
    activation: &Activation,
) -> crate::error::Result<crate::fabric_consistency::Group> {
    let group = declared(input)?;
    let plan = plan(&group, activation)?;
    if !plan.admitted() {
        return Err(super::codec::invalid("group activation denied"));
    }
    let outcome = crate::fabric_consistency::normalize(&group, &plan, crate::fabric_consistency::Input {
        request_ref: activation.request_ref.clone(),
        binding_ref: group.binding_ref.clone(),
        service_generation: group.service_generation,
        config_epoch: group.config_epoch,
        fencing_epoch: group.fencing_epoch,
        kind: crate::fabric_consistency::Kind::Opened,
        result_ref: Some(activation.result_ref.clone()),
        evidence_refs: activation.evidence_refs.clone(),
        diagnostics: Vec::new(),
    })?;
    if outcome.kind != crate::fabric_consistency::Kind::Opened {
        return Err(super::codec::invalid("group activation not opened"));
    }
    crate::fabric_consistency::apply_outcome(&group, &plan, &outcome)
}

fn declared(input: &crate::live_reliability::service::Group) -> crate::error::Result<crate::fabric_consistency::Group> {
    crate::fabric_consistency::canonical_group(crate::fabric_consistency::GroupInput {
        group_id: input.group_id.clone(),
        extension_id: input.extension_id.clone(),
        service_id: input.service_id.clone(),
        service_generation: input.service_generation,
        application_manifest_ref: input.application_manifest_ref.clone(),
        engine_algorithm_profile: input.engine_algorithm_profile.clone(),
        engine_implementation_profile: input.engine_implementation_profile.clone(),
        membership_ref: input.membership_ref.clone(),
        config_epoch: input.config_epoch,
        placement_ref: input.placement_ref.clone(),
        fencing_ref: input.fencing_ref.clone(),
        fencing_epoch: input.fencing_epoch,
        resource_profile_ref: input.resource_profile_ref.clone(),
        policy_refs: input.policy_refs.clone(),
        non_claims: input.non_claims.clone(),
        supported_read_modes: vec![crate::fabric_consistency::ReadMode::Linearizable],
        max_command_bytes: input.max_command_bytes,
        max_in_flight_operations: input.max_in_flight_operations,
    })
}

fn plan(
    group: &crate::fabric_consistency::Group,
    activation: &Activation,
) -> crate::error::Result<crate::fabric_consistency::PortPlan> {
    crate::fabric_consistency::plan(group, crate::fabric_consistency::PortCommandInput {
        request_ref: activation.request_ref.clone(),
        binding_ref: group.binding_ref.clone(),
        group_id: group.group_id.clone(),
        extension_id: group.extension_id.clone(),
        service_id: group.service_id.clone(),
        service_generation: group.service_generation,
        application_manifest_ref: group.application_manifest_ref.clone(),
        engine_algorithm_profile: group.engine_algorithm_profile.clone(),
        engine_implementation_profile: group.engine_implementation_profile.clone(),
        membership_ref: group.membership_ref.clone(),
        config_epoch: group.config_epoch,
        placement_ref: group.placement_ref.clone(),
        fencing_ref: group.fencing_ref.clone(),
        fencing_epoch: group.fencing_epoch,
        resource_profile_ref: group.resource_profile_ref.clone(),
        policy_refs: group.policy_refs.clone(),
        authority_refs: activation.authority_refs.clone(),
        observed_in_flight_operations: 0,
        operation: crate::fabric_consistency::Request::Open {
            mode: crate::fabric_consistency::GroupOpenMode::Attach,
        },
    })
}
