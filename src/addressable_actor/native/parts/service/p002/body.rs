struct Status;
impl crate::addressable_actor::ActorStatusPort for Status {
    fn publish_status(&mut self, status: &ActorStatus) -> crate::addressable_actor::PortResult<crate::addressable_actor::StatusObservation> {
        Ok(crate::addressable_actor::StatusObservation {
            status_ref: Some(
                crate::addressable_actor::identify_canonical_status(status).map_err(|error| super::port_error(error, false))?,
            ),
            outcome_unknown: false,
        })
    }
}

fn problem(error: impl std::fmt::Debug) -> crate::error::Failure {
    super::invalid(format!("native actor consumer: {error:?}"))
}

fn operation_input(fence: &super::Fence, operation_id: &str, operation: &ActorOperation) -> crate::error::Result<String> {
    let input_ref =
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("native-actor-operation-v1", vec![
            crate::preserves_rail::string(operation_id),
            crate::preserves_rail::string(operation.kind()),
            crate::preserves_rail::string(&fence.actor_key_ref),
            crate::preserves_rail::string(&fence.placement_ref),
            crate::preserves_rail::u64_value(fence.generation),
            crate::preserves_rail::u64_value(fence.lifecycle_sequence),
        ]))?;
    let input_ref = identify_actor_operation(&input_ref, operation);
    Ok(input_ref)
}

fn admission(
    configuration: &super::Configuration,
    host: &crate::addressable_actor::HostBindingFacts,
    fence: &super::Fence,
    observed: super::binding::Observation,
) -> ActorAdmissionFacts {
    ActorAdmissionFacts {
        profile_ref: host.profile_ref.clone(),
        system_extension_manifest_ref: host.system_extension_manifest_ref.clone(),
        authority_ref: observed.authority_ref,
        resource_ref: configuration.profile.resource_profile_ref.clone(),
        adapter_ref: observed.instance.profile_ref,
        policy_current: host.policy_current,
        capability_current: host.capability_current,
        placement_current: host.placement_current,
        generation_current: host.extension_generation == fence.generation,
        resources_admitted: host.resources_admitted,
        adapter_admitted: host.adapter_admitted,
    }
}
