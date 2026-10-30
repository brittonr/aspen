use super::bounds;
use super::domain;

// r[impl molten.world_witness.currentness]
pub fn validate_currentness(
    input: &domain::CurrentnessInput,
) -> Result<domain::CurrentnessAdmission, domain::WitnessIssue> {
    bounds::validate(input)?;
    if matches!(input.retained, domain::RetainedWitnessState::Missing) {
        return Err(domain::WitnessIssue::MissingHighestState);
    }
    if input.selected_profile < input.policy.required_profile
        || (input.selected_profile == domain::AssuranceProfile::LocalGenerationV1
            && matches!(input.retained, domain::RetainedWitnessState::Present(_)))
    {
        return Err(domain::WitnessIssue::ProfileDowngrade);
    }
    if input.subject.generation < input.local_generation {
        return Err(domain::WitnessIssue::StaleGeneration);
    }
    if input.selected_profile == domain::AssuranceProfile::LocalGenerationV1 {
        return Ok(domain::CurrentnessAdmission {
            profile: input.selected_profile,
            highest: None,
        });
    }
    let observation = available(input)?;
    validate_binding(input, observation)?;
    validate_checkpoint(input, observation)?;
    validate_retained(input, observation)?;
    Ok(domain::CurrentnessAdmission {
        profile: input.selected_profile,
        highest: Some(domain::HighestWitnessState {
            subject: input.subject.clone(),
            checkpoint: observation.checkpoint.clone(),
            consistency_predecessor: observation.consistency.predecessor.clone(),
            quorum: observation.quorum.witnesses.clone(),
        }),
    })
}

/// Final receipts require committed external currentness, not a reservation.
pub fn validate_committed_currentness(
    input: &domain::CurrentnessInput,
) -> Result<domain::CurrentnessAdmission, domain::WitnessIssue> {
    let admission = validate_currentness(input)?;
    if input.selected_profile == domain::AssuranceProfile::IndependentWitnessV1
        && available(input)?.current_head.phase != domain::CurrentHeadPhase::Committed
    {
        return Err(domain::WitnessIssue::MissingCurrentness);
    }
    Ok(admission)
}

fn available(input: &domain::CurrentnessInput) -> Result<&domain::AvailableWitnessObservation, domain::WitnessIssue> {
    match input.observation.as_ref() {
        Some(domain::WitnessObservation::Available(value)) => Ok(value),
        Some(domain::WitnessObservation::Fork { .. }) => Err(domain::WitnessIssue::Fork),
        Some(domain::WitnessObservation::Unavailable {
            append_may_have_occurred: true,
        }) => Err(domain::WitnessIssue::UncertainAppend),
        Some(domain::WitnessObservation::Unavailable { .. }) => Err(domain::WitnessIssue::Unavailable),
        None => Err(domain::WitnessIssue::MissingCurrentness),
    }
}

// r[impl molten.world_witness.provider_boundary]
fn validate_binding(
    input: &domain::CurrentnessInput,
    value: &domain::AvailableWitnessObservation,
) -> Result<(), domain::WitnessIssue> {
    let contract = &input.policy.provider;
    if [
        &contract.provider_id,
        &contract.contract_id,
        &contract.source_revision,
        &contract.license,
    ]
    .iter()
    .any(|field| field.trim().is_empty())
    {
        return Err(domain::WitnessIssue::InvalidContract);
    }
    if &value.contract != contract || value.checkpoint.provider_id != contract.provider_id {
        return Err(domain::WitnessIssue::ProviderSubstitution);
    }
    if value.append.subject != input.subject
        || value.inclusion.subject != input.subject
        || value.current_head.subject != input.subject
    {
        return Err(domain::WitnessIssue::DomainSubstitution);
    }
    if input.operation_id.is_empty() || value.append.operation_id != input.operation_id {
        return Err(domain::WitnessIssue::OperationSubstitution);
    }
    if !value.append.accepted
        || !value.inclusion.verified
        || value.inclusion.checkpoint_id != value.checkpoint.checkpoint_id
    {
        return Err(domain::WitnessIssue::InvalidInclusion);
    }
    if !value.current_head.verified || value.current_head.checkpoint_id != value.checkpoint.checkpoint_id {
        return Err(domain::WitnessIssue::MissingCurrentness);
    }
    Ok(())
}

fn validate_checkpoint(
    input: &domain::CurrentnessInput,
    value: &domain::AvailableWitnessObservation,
) -> Result<(), domain::WitnessIssue> {
    let checkpoint = &value.checkpoint;
    if !checkpoint.signature_verified || checkpoint.checkpoint_id.is_empty() {
        return Err(domain::WitnessIssue::InvalidCheckpoint);
    }
    if checkpoint.observed_at > input.now || input.now - checkpoint.observed_at > input.policy.maximum_age {
        return Err(domain::WitnessIssue::StaleCheckpoint);
    }
    if !value.consistency.verified || value.consistency.checkpoint_id != checkpoint.checkpoint_id {
        return Err(domain::WitnessIssue::FailedConsistency);
    }
    if !value.quorum.verified
        || value.quorum.checkpoint_id != checkpoint.checkpoint_id
        || input.policy.minimum_quorum == 0
        || value.quorum.witnesses.len() < input.policy.minimum_quorum
        || !value.quorum.witnesses.is_subset(&input.policy.allowed_witnesses)
    {
        return Err(domain::WitnessIssue::InsufficientQuorum);
    }
    Ok(())
}

fn validate_retained(
    input: &domain::CurrentnessInput,
    value: &domain::AvailableWitnessObservation,
) -> Result<(), domain::WitnessIssue> {
    let domain::RetainedWitnessState::Present(highest) = &input.retained else {
        return if value.consistency.predecessor.is_none() {
            Ok(())
        } else {
            Err(domain::WitnessIssue::FailedConsistency)
        };
    };
    if highest.subject.branch_id != input.subject.branch_id {
        return Err(domain::WitnessIssue::DomainSubstitution);
    }
    if highest.checkpoint.provider_id != value.checkpoint.provider_id {
        return Err(domain::WitnessIssue::ProviderSubstitution);
    }
    if input.subject.generation < highest.subject.generation || value.checkpoint.revision < highest.checkpoint.revision
    {
        return Err(domain::WitnessIssue::StaleCheckpoint);
    }
    if input.subject.generation == highest.subject.generation && input.subject != highest.subject {
        return Err(domain::WitnessIssue::Fork);
    }
    if value.checkpoint.revision == highest.checkpoint.revision
        && value.checkpoint.checkpoint_id != highest.checkpoint.checkpoint_id
    {
        return Err(domain::WitnessIssue::Fork);
    }
    if !highest.quorum.is_subset(&value.quorum.witnesses) {
        return Err(domain::WitnessIssue::InsufficientQuorum);
    }
    if value.consistency.predecessor.as_ref() != Some(&highest.checkpoint.checkpoint_id) {
        return Err(domain::WitnessIssue::FailedConsistency);
    }
    Ok(())
}
