//! Admission and source-shape invariants checked before wire encoding.

pub(super) fn projection<'a>(
    protocol: &'a chaoscontrol_smr::observation::ProtocolProfile,
    plan: &chaoscontrol_smr::SmrWorkloadPlan,
    context: &super::ProjectionContext,
    observation: &super::super::super::ChaosControlChainObservation,
) -> Result<(String, &'a chaoscontrol_smr::observation::Participant), super::Issue> {
    let protocol_ref = admit_profiles(protocol, plan)?;
    if observation.chaoscontrol_profile_ref != plan.profile_ref {
        return Err(super::Issue::WorkloadProfile);
    }
    reference(&context.group_ref)?;
    if observation.group_ref != context.group_ref {
        return Err(super::Issue::Group);
    }
    if context.source_sequence == 0 {
        return Err(super::Issue::SourceSequence);
    }
    for value in [
        &context.boundary_ref,
        &context.scheduler_ref,
        &context.transition_class_ref,
    ] {
        reference(value)?;
    }
    // Check bounds before copying retained command bytes or caller references.
    command_bounds(&observation.command_bytes, plan.profile.bounds.maximum_command_bytes)?;
    for value in [
        &observation.replica_ref,
        &observation.operation_ref,
        &observation.command_ref,
        &observation.application_receipt_ref,
        &observation.prior_digest,
        &observation.next_digest,
        &observation.application_state_ref,
    ] {
        reference(value)?;
    }
    let participant = protocol
        .participants
        .iter()
        .find(|value| value.participant_ref == observation.replica_ref)
        .ok_or(super::Issue::Participant)?;
    if observation.lifecycle_generation != participant.generation {
        return Err(super::Issue::Generation);
    }
    Ok((protocol_ref, participant))
}

pub(super) fn admit_profiles(
    protocol: &chaoscontrol_smr::observation::ProtocolProfile,
    plan: &chaoscontrol_smr::SmrWorkloadPlan,
) -> Result<String, super::Issue> {
    let admitted = chaoscontrol_smr::admit_profile(&plan.profile).map_err(|_| super::Issue::WorkloadProfile)?;
    if admitted != *plan {
        return Err(super::Issue::WorkloadProfile);
    }
    protocol.admit().map_err(|_| super::Issue::ProtocolProfile)
}

pub(super) fn reference(value: &str) -> Result<(), super::Issue> {
    let hex = value.strip_prefix(chaoscontrol_smr::DIGEST_PREFIX).ok_or(super::Issue::Reference)?;
    if hex.len() != chaoscontrol_smr::DIGEST_HEX_LENGTH
        || !hex.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(super::Issue::Reference);
    }
    Ok(())
}

pub(super) fn command_bounds(bytes: &[u8], maximum: u64) -> Result<(), super::Issue> {
    let length = u64::try_from(bytes.len()).map_err(|_| super::Issue::CommandBounds)?;
    if bytes.is_empty() || length > maximum {
        return Err(super::Issue::CommandBounds);
    }
    Ok(())
}

pub(super) fn binding_shape(binding: &super::ApplyBinding) -> Result<(), super::Issue> {
    for value in [
        &binding.group_ref,
        &binding.participant_ref,
        &binding.operation_ref,
        &binding.command_ref,
        &binding.application_receipt_ref,
        &binding.chain.profile_ref,
        &binding.chain.prior_digest,
        &binding.chain.digest,
        &binding.chain.application_state_ref,
    ] {
        reference(value)?;
    }
    if Some(binding.chain.replica_id.as_str()) != binding.participant_ref.strip_prefix(chaoscontrol_smr::DIGEST_PREFIX)
    {
        return Err(super::Issue::Participant);
    }
    if binding.chain.event_sequence == 0 {
        return Err(super::Issue::SourceSequence);
    }
    if binding.chain.command_index == 0 || binding.chain.command_index > chaoscontrol_smr::SUPPORTED_MAXIMUM_COMMANDS {
        return Err(super::Issue::CommandBounds);
    }
    command_bounds(&binding.command_bytes, chaoscontrol_smr::SUPPORTED_MAXIMUM_COMMAND_BYTES)
}
