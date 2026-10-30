use std::collections;

use super::domain;
use super::planning;

/// Reject unbounded collections before inspecting any of their members.
pub(super) fn validate(input: &domain::CurrentnessInput) -> Result<(), domain::WitnessIssue> {
    count(input.policy.allowed_witnesses.len())?;
    count(input.policy.minimum_quorum)?;
    retained_count(&input.retained)?;
    match input.observation.as_ref() {
        Some(domain::WitnessObservation::Available(value)) => count(value.quorum.witnesses.len())?,
        Some(domain::WitnessObservation::Fork { checkpoint_ids, .. }) => count(checkpoint_ids.len())?,
        _ => {}
    }
    label(&input.operation_id)?;
    contract(&input.policy.provider)?;
    labels(&input.policy.allowed_witnesses)?;
    retained(&input.retained)?;
    match input.observation.as_ref() {
        Some(domain::WitnessObservation::Available(value)) => available(value.as_ref()),
        Some(domain::WitnessObservation::Fork {
            provider_id,
            checkpoint_ids,
        }) => {
            label(provider_id)?;
            labels(checkpoint_ids)
        }
        _ => Ok(()),
    }
}

pub(super) fn validate_stage(staged: &planning::StagedClaim) -> Result<(), domain::WitnessIssue> {
    retained_count(&staged.retained)?;
    label(&staged.operation_id)?;
    label(&staged.transition.choregraph_after_identity)?;
    if let Some(before) = &staged.transition.choregraph_before_identity {
        label(before)?;
    }
    retained(&staged.retained)
}

fn count(value: usize) -> Result<(), domain::WitnessIssue> {
    if value > domain::MAX_WITNESSES {
        return Err(domain::WitnessIssue::BoundsExceeded);
    }
    Ok(())
}

fn label(value: &str) -> Result<(), domain::WitnessIssue> {
    if value.len() > domain::MAX_WITNESS_LABEL_BYTES {
        return Err(domain::WitnessIssue::BoundsExceeded);
    }
    Ok(())
}

fn labels(values: &collections::BTreeSet<String>) -> Result<(), domain::WitnessIssue> {
    for value in values {
        label(value)?;
    }
    Ok(())
}

fn contract(value: &domain::ProviderContract) -> Result<(), domain::WitnessIssue> {
    label(&value.provider_id)?;
    for field in [&value.contract_id, &value.source_revision, &value.license] {
        if field.len() > domain::MAX_WITNESS_CONTRACT_BYTES {
            return Err(domain::WitnessIssue::BoundsExceeded);
        }
    }
    Ok(())
}

fn checkpoint(value: &domain::SignedCheckpoint) -> Result<(), domain::WitnessIssue> {
    label(&value.provider_id)?;
    label(&value.checkpoint_id)
}

fn retained_count(value: &domain::RetainedWitnessState) -> Result<(), domain::WitnessIssue> {
    if let domain::RetainedWitnessState::Present(highest) = value {
        count(highest.quorum.len())?;
    }
    Ok(())
}

fn retained(value: &domain::RetainedWitnessState) -> Result<(), domain::WitnessIssue> {
    if let domain::RetainedWitnessState::Present(highest) = value {
        checkpoint(&highest.checkpoint)?;
        if let Some(predecessor) = &highest.consistency_predecessor {
            label(predecessor)?;
        }
        labels(&highest.quorum)?;
    }
    Ok(())
}

fn available(value: &domain::AvailableWitnessObservation) -> Result<(), domain::WitnessIssue> {
    contract(&value.contract)?;
    checkpoint(&value.checkpoint)?;
    label(&value.append.operation_id)?;
    label(&value.inclusion.checkpoint_id)?;
    label(&value.consistency.checkpoint_id)?;
    if let Some(predecessor) = &value.consistency.predecessor {
        label(predecessor)?;
    }
    label(&value.quorum.checkpoint_id)?;
    labels(&value.quorum.witnesses)?;
    label(&value.current_head.checkpoint_id)
}
