mod model;

pub use model::*;

use alloc::collections::BTreeSet;
use alloc::vec::Vec;

use crate::{
    CloneRef, CohortPlan, CohortRef, EffectKind, MAX_CLEANUP_OBLIGATIONS, MAX_OBSERVATIONS,
    OperationRef, ReceiptRef, ReferenceError, ResourceRef,
};

/// Creates an untouched lifecycle state for a complete plan.
#[must_use]
pub fn new_cohort_state(plan: CohortPlan) -> CohortState {
    CohortState {
        plan,
        phase: CohortPhase::Planned,
        completed_operations: Vec::new(),
        activated_clones: Vec::new(),
        cleanup_obligations: Vec::new(),
        released_cleanup_operations: Vec::new(),
        observation_refs: Vec::new(),
    }
}

/// Reduces one exact creation or activation observation.
///
/// # Errors
///
/// Returns a lifecycle issue for crossed, duplicate, unplanned, overbound, or terminal observations.
// r[impl vm_cohort.transactional_creation]
// r[impl vm_cohort.cleanup]
pub fn reduce_effect_observation(
    state: &CohortState,
    observation: &EffectObservation,
) -> Result<CohortState, LifecycleIssue> {
    validate_effect_observation(state, observation)?;
    let mut next = state.clone();
    retain_observation(&mut next, &observation.observation_ref)?;
    match observation.outcome {
        OperationOutcome::Success => apply_success(&mut next, observation),
        OperationOutcome::TransientFailure
        | OperationOutcome::PermanentFailure
        | OperationOutcome::Cancelled => {
            add_cleanup_for_observed_clones(&mut next, Some(&observation.clone_ref))?;
            next.phase = if next.cleanup_obligations.is_empty() {
                CohortPhase::Failed
            } else {
                CohortPhase::CleanupRequired
            };
        }
        OperationOutcome::OutcomeUnknown => {
            add_cleanup_for_observed_clones(&mut next, Some(&observation.clone_ref))?;
            next.phase = CohortPhase::CleanupUncertain;
        }
    }
    Ok(next)
}

/// Cancels the cohort and conservatively requires cleanup for every clone whose effects may have started.
///
/// # Errors
///
/// Returns [`LifecycleIssue::Terminal`] for an already terminal state.
pub fn cancel_cohort(state: &CohortState) -> Result<CohortState, LifecycleIssue> {
    if is_terminal(state.phase) {
        return Err(LifecycleIssue::Terminal);
    }
    let mut next = state.clone();
    add_cleanup_for_observed_clones(&mut next, None)?;
    next.phase = if next.cleanup_obligations.is_empty() {
        CohortPhase::Failed
    } else {
        CohortPhase::CleanupRequired
    };
    Ok(next)
}

/// Reduces one exact cleanup observation.
///
/// # Errors
///
/// Returns a lifecycle issue for crossed, duplicate, overbound, or unmatched cleanup observations.
// r[impl vm_cohort.cleanup.uncertain]
pub fn reduce_cleanup_observation(
    state: &CohortState,
    observation: &CleanupObservation,
) -> Result<CohortState, LifecycleIssue> {
    if observation.cohort_ref != state.plan.cohort_ref {
        return Err(LifecycleIssue::WrongCohort);
    }
    let is_match = state.cleanup_obligations.iter().any(|obligation| {
        obligation.clone_ref == observation.clone_ref
            && obligation.resource_ref == observation.resource_ref
            && obligation.cleanup_operation_ref == observation.cleanup_operation_ref
    });
    if !is_match {
        return Err(LifecycleIssue::WrongCleanup);
    }
    let mut next = state.clone();
    retain_observation(&mut next, &observation.observation_ref)?;
    match observation.outcome {
        CleanupOutcome::Released => {
            if !next
                .released_cleanup_operations
                .contains(&observation.cleanup_operation_ref)
            {
                next.released_cleanup_operations
                    .push(observation.cleanup_operation_ref.clone());
                next.released_cleanup_operations.sort();
            }
            next.phase = if next.released_cleanup_operations.len() == next.cleanup_obligations.len()
            {
                CohortPhase::Cleaned
            } else {
                CohortPhase::CleanupRequired
            };
        }
        CleanupOutcome::Failed => next.phase = CohortPhase::CleanupRequired,
        CleanupOutcome::OutcomeUnknown => next.phase = CohortPhase::CleanupUncertain,
    }
    Ok(next)
}

/// Marks operator abandonment without claiming cleanup.
///
/// # Errors
///
/// Returns [`LifecycleIssue::Terminal`] unless cleanup is required or uncertain.
pub fn abandon_cleanup(state: &CohortState) -> Result<CohortState, LifecycleIssue> {
    if !matches!(
        state.phase,
        CohortPhase::CleanupRequired | CohortPhase::CleanupUncertain
    ) {
        return Err(LifecycleIssue::Terminal);
    }
    let mut next = state.clone();
    next.phase = CohortPhase::Abandoned;
    Ok(next)
}

fn validate_effect_observation(
    state: &CohortState,
    observation: &EffectObservation,
) -> Result<(), LifecycleIssue> {
    if is_terminal(state.phase) || state.phase == CohortPhase::Active {
        return Err(LifecycleIssue::Terminal);
    }
    if observation.cohort_ref != state.plan.cohort_ref {
        return Err(LifecycleIssue::WrongCohort);
    }
    let effect = state
        .plan
        .effects
        .iter()
        .find(|effect| effect.operation_ref == observation.operation_ref)
        .ok_or(LifecycleIssue::UnplannedOperation)?;
    if effect.clone_ref != observation.clone_ref {
        return Err(LifecycleIssue::WrongClone);
    }
    if effect.kind != observation.kind {
        return Err(LifecycleIssue::UnplannedOperation);
    }
    Ok(())
}

fn apply_success(state: &mut CohortState, observation: &EffectObservation) {
    if !state
        .completed_operations
        .contains(&observation.operation_ref)
    {
        state
            .completed_operations
            .push(observation.operation_ref.clone());
        state.completed_operations.sort();
    }
    if observation.kind == EffectKind::Activate
        && !state.activated_clones.contains(&observation.clone_ref)
    {
        state.activated_clones.push(observation.clone_ref.clone());
        state.activated_clones.sort();
    }
    state.phase = if state.activated_clones.len() == state.plan.clones.len()
        && state.completed_operations.len() == state.plan.effects.len()
    {
        CohortPhase::Active
    } else {
        CohortPhase::Preparing
    };
}

fn add_cleanup_for_observed_clones(
    state: &mut CohortState,
    current_clone: Option<&CloneRef>,
) -> Result<(), LifecycleIssue> {
    let mut affected = state
        .plan
        .effects
        .iter()
        .filter(|effect| state.completed_operations.contains(&effect.operation_ref))
        .map(|effect| effect.clone_ref.clone())
        .collect::<BTreeSet<_>>();
    if let Some(clone_ref) = current_clone {
        affected.insert(clone_ref.clone());
    }
    let obligation_capacity = affected
        .len()
        .checked_mul(crate::cohort::surface_count())
        .ok_or(LifecycleIssue::CleanupLimit)?;
    if obligation_capacity > MAX_CLEANUP_OBLIGATIONS {
        return Err(LifecycleIssue::CleanupLimit);
    }
    let mut obligations = Vec::with_capacity(obligation_capacity);
    for clone in state
        .plan
        .clones
        .iter()
        .filter(|clone| affected.contains(&clone.clone_ref))
    {
        for surface in &clone.surfaces {
            let cleanup_operation_ref = cleanup_identity(
                &state.plan.cohort_ref,
                &clone.clone_ref,
                &surface.resource_ref,
            )?;
            obligations.push(CleanupObligation {
                clone_ref: clone.clone_ref.clone(),
                resource_ref: surface.resource_ref.clone(),
                cleanup_operation_ref,
            });
        }
    }
    obligations.sort();
    obligations.dedup();
    state.cleanup_obligations = obligations;
    Ok(())
}

fn cleanup_identity(
    cohort_ref: &CohortRef,
    clone_ref: &CloneRef,
    resource_ref: &ResourceRef,
) -> Result<OperationRef, LifecycleIssue> {
    const DOMAIN: &str = "onixresearch.vm-cohort.cleanup.v1";
    let mut hasher = blake3::Hasher::new_derive_key(DOMAIN);
    crate::identity::update_text(&mut hasher, cohort_ref.as_str())
        .map_err(|ReferenceError| LifecycleIssue::Identity)?;
    crate::identity::update_text(&mut hasher, clone_ref.as_str())
        .map_err(|ReferenceError| LifecycleIssue::Identity)?;
    crate::identity::update_text(&mut hasher, resource_ref.as_str())
        .map_err(|ReferenceError| LifecycleIssue::Identity)?;
    OperationRef::new(alloc::format!("blake3:{}", hasher.finalize().to_hex()))
        .map_err(|ReferenceError| LifecycleIssue::Identity)
}

fn retain_observation(
    state: &mut CohortState,
    observation_ref: &ReceiptRef,
) -> Result<(), LifecycleIssue> {
    if state.observation_refs.len() >= MAX_OBSERVATIONS {
        return Err(LifecycleIssue::ObservationLimit);
    }
    if state.observation_refs.contains(observation_ref) {
        return Err(LifecycleIssue::DuplicateObservation);
    }
    state.observation_refs.push(observation_ref.clone());
    state.observation_refs.sort();
    Ok(())
}

const fn is_terminal(phase: CohortPhase) -> bool {
    matches!(
        phase,
        CohortPhase::Cleaned | CohortPhase::Failed | CohortPhase::Abandoned
    )
}
