// r[impl molten.fabric_consistency.extension_port]
// r[impl molten.fabric_consistency.group_isolation]
pub fn apply_consistency_outcome(
    binding: &super::ConsistencyGroupBinding,
    plan: &super::ConsistencyPortPlan,
    outcome: &super::ConsistencyPortOutcome,
) -> crate::error::Result<super::ConsistencyGroupBinding> {
    validate_application_binding(binding, plan, outcome)?;
    if outcome.kind.is_non_mutating_failure() {
        return Ok(binding.clone());
    }
    let mut input = binding_input(binding);
    match (&plan.operation, outcome.kind) {
        (super::ConsistencyOperation::Open { .. }, super::ConsistencyOutcomeKind::Opened) => {}
        (super::ConsistencyOperation::Drain, super::ConsistencyOutcomeKind::Drained) => {}
        (super::ConsistencyOperation::Remove, super::ConsistencyOutcomeKind::Removed) => {}
        (
            super::ConsistencyOperation::Configure {
                transition:
                    super::ConfigurationTransition::StaticMembershipRefresh {
                        next_membership_ref,
                        next_config_epoch,
                    },
            },
            super::ConsistencyOutcomeKind::ConfigurationApplied,
        ) => {
            input.membership_ref.clone_from(next_membership_ref);
            input.config_epoch = *next_config_epoch;
        }
        _ => return Ok(binding.clone()),
    }
    let value = super::canonical::binding_value(&input, plan.lifecycle_after);
    let binding_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(binding_from_input(input, binding_ref, plan.lifecycle_after, value))
}

fn validate_application_binding(
    binding: &super::ConsistencyGroupBinding,
    plan: &super::ConsistencyPortPlan,
    outcome: &super::ConsistencyPortOutcome,
) -> crate::error::Result<()> {
    if plan.binding_ref != binding.binding_ref
        || plan.lifecycle_before != binding.lifecycle
        || outcome.plan_ref != plan.plan_ref
        || outcome.request_ref != plan.request_ref
        || outcome.binding_ref != binding.binding_ref
    {
        return Err(crate::error::MoltenError::invalid_harness(
            "stale or mismatched consistency outcome cannot change group state",
        ));
    }
    Ok(())
}

fn binding_input(binding: &super::ConsistencyGroupBinding) -> super::ConsistencyGroupBindingInput {
    super::ConsistencyGroupBindingInput {
        group_id: binding.group_id.clone(),
        extension_id: binding.extension_id.clone(),
        service_id: binding.service_id.clone(),
        service_generation: binding.service_generation,
        application_manifest_ref: binding.application_manifest_ref.clone(),
        engine_algorithm_profile: binding.engine_algorithm_profile.clone(),
        engine_implementation_profile: binding.engine_implementation_profile.clone(),
        membership_ref: binding.membership_ref.clone(),
        config_epoch: binding.config_epoch,
        placement_ref: binding.placement_ref.clone(),
        fencing_ref: binding.fencing_ref.clone(),
        fencing_epoch: binding.fencing_epoch,
        resource_profile_ref: binding.resource_profile_ref.clone(),
        policy_refs: binding.policy_refs.clone(),
        non_claims: binding.non_claims.clone(),
        supported_read_modes: binding.supported_read_modes.clone(),
        max_command_bytes: binding.max_command_bytes,
        max_in_flight_operations: binding.max_in_flight_operations,
    }
}

fn binding_from_input(
    input: super::ConsistencyGroupBindingInput,
    binding_ref: String,
    lifecycle: super::ConsistencyGroupLifecycle,
    value: preserves::IOValue,
) -> super::ConsistencyGroupBinding {
    super::ConsistencyGroupBinding {
        binding_ref,
        group_id: input.group_id,
        extension_id: input.extension_id,
        service_id: input.service_id,
        service_generation: input.service_generation,
        application_manifest_ref: input.application_manifest_ref,
        engine_algorithm_profile: input.engine_algorithm_profile,
        engine_implementation_profile: input.engine_implementation_profile,
        membership_ref: input.membership_ref,
        config_epoch: input.config_epoch,
        placement_ref: input.placement_ref,
        fencing_ref: input.fencing_ref,
        fencing_epoch: input.fencing_epoch,
        resource_profile_ref: input.resource_profile_ref,
        policy_refs: input.policy_refs,
        non_claims: input.non_claims,
        supported_read_modes: input.supported_read_modes,
        max_command_bytes: input.max_command_bytes,
        max_in_flight_operations: input.max_in_flight_operations,
        lifecycle,
        value,
    }
}
