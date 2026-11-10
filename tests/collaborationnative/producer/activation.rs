//! The declared group advanced to `Active` by an explicit admitted activation observation.

/// The declared group advanced to `Active` by an explicit admitted activation observation.
pub(super) fn activate(
    input: &molten::live_reliability::service::Group,
    activation: &molten::collaboration_scope::native_runtime::Activation,
) -> super::Result<molten::fabric_consistency::Group> {
    let group = molten::fabric_consistency::canonical_group(molten::fabric_consistency::GroupInput {
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
        supported_read_modes: vec![molten::fabric_consistency::ReadMode::Linearizable],
        max_command_bytes: input.max_command_bytes,
        max_in_flight_operations: input.max_in_flight_operations,
    })?;
    let plan = molten::fabric_consistency::plan(&group, molten::fabric_consistency::PortCommandInput {
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
        operation: molten::fabric_consistency::Request::Open {
            mode: molten::fabric_consistency::GroupOpenMode::Attach,
        },
    })?;
    if !plan.admitted() {
        return Err("group activation denied".into());
    }
    let outcome = molten::fabric_consistency::normalize(&group, &plan, molten::fabric_consistency::Input {
        request_ref: activation.request_ref.clone(),
        binding_ref: group.binding_ref.clone(),
        service_generation: group.service_generation,
        config_epoch: group.config_epoch,
        fencing_epoch: group.fencing_epoch,
        kind: molten::fabric_consistency::Kind::Opened,
        result_ref: Some(activation.result_ref.clone()),
        evidence_refs: activation.evidence_refs.clone(),
        diagnostics: Vec::new(),
    })?;
    if outcome.kind != molten::fabric_consistency::Kind::Opened {
        return Err("group activation not opened".into());
    }
    Ok(molten::fabric_consistency::apply_outcome(&group, &plan, &outcome)?)
}
