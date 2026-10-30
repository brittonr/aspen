use super::*;

/// Deterministic counterpart of the node's drain/checkpoint/recover/activate sequence.
pub(super) fn run(variant: ScenarioVariant) -> Result<ScenarioFacts, crate::error::Failure> {
    let before = running_state()?;
    let draining = step(&before, LifecycleEventKind::BeginDrain)?;
    let drained = step(&draining, LifecycleEventKind::DrainSucceeded)?;
    let checkpointing = step(&drained, LifecycleEventKind::BeginCheckpoint)?;
    let checkpoint = crate::preserves_rail::record("composition-state-v1", vec![crate::preserves_rail::u64_value(7)]);
    let checkpoint_ref = crate::preserves_rail::canonical_hash(&checkpoint)?;
    let checkpointed = transition(&checkpointing, LifecycleEvent {
        kind: LifecycleEventKind::CheckpointSucceeded,
        generation: before.generation,
        incarnation: before.incarnation,
        next_generation: None,
        checkpoint_ref: Some(checkpoint_ref.clone()),
        failure_class: None,
    })?;
    let replacing = transition(&checkpointed, LifecycleEvent {
        kind: LifecycleEventKind::BeginUpgrade,
        generation: before.generation,
        incarnation: before.incarnation,
        next_generation: Some(before.generation + 1),
        checkpoint_ref: None,
        failure_class: None,
    })?;
    let transformed = crate::preserves_rail::record("composition-state-v2", vec![
        crate::preserves_rail::u64_value(7),
        crate::preserves_rail::string(&checkpoint_ref),
    ]);
    let materialized = if variant == ScenarioVariant::MigrationSkipped {
        checkpoint
    } else {
        transformed.clone()
    };
    let after = step(&replacing, LifecycleEventKind::UpgradeSucceeded)?;
    let durable = WorkPresentation {
        work_ref: HASH_C.to_string(),
        class: WorkClass::Durable,
        generation: after.generation,
        incarnation: after.incarnation,
    };
    let claim = DeliveryClaim {
        claim_ref: HASH_D.to_string(),
        class: WorkClass::Durable,
        generation: after.generation,
        incarnation: after.incarnation,
    };
    let old_work = WorkPresentation {
        work_ref: HASH_E.to_string(),
        class: WorkClass::RuntimeLocal,
        generation: before.generation,
        incarnation: before.incarnation,
    };
    let sibling = running_state()?;
    let sibling_work = WorkPresentation {
        work_ref: HASH_B.to_string(),
        class: WorkClass::RuntimeLocal,
        generation: sibling.generation,
        incarnation: sibling.incarnation,
    };
    let mut facts = ScenarioFacts::empty();
    facts.upgrade_completed = drained.phase == LifecyclePhase::Drained
        && checkpointed.checkpoint_ref.as_deref() == Some(checkpoint_ref.as_str())
        && materialized == transformed
        && after.phase == LifecyclePhase::Running;
    facts.stale_generation_acted = admit_work_delivery(&after, &old_work, None).is_ok();
    facts.durable_work_readmitted = admit_work_delivery(&after, &durable, Some(&claim)).is_ok();
    facts.durable_work_lost = !facts.durable_work_readmitted;
    facts.sibling_progressed = admit_work_delivery(&sibling, &sibling_work, None).is_ok();
    Ok(facts)
}

fn step(state: &LifecycleState, kind: LifecycleEventKind) -> Result<LifecycleState, crate::error::Failure> {
    transition(state, LifecycleEvent::simple(kind, state.generation, state.incarnation))
}

fn transition(state: &LifecycleState, event: LifecycleEvent) -> Result<LifecycleState, crate::error::Failure> {
    plan_lifecycle_transition(state, &event, idle_usage(), MAX_RESTART_ATTEMPTS)
        .map_err(|issues| crate::error::Failure::invalid_harness(format!("composition upgrade denied: {issues:?}")))
}

#[cfg(test)]
mod tests {
    // r[verify molten.testing.service_recovery_composition]
    #[test]
    fn skipped_transformation_is_not_successful_even_when_stale_work_is_denied() {
        let correct = super::run(super::ScenarioVariant::Correct).expect("complete deterministic upgrade");
        let broken = super::run(super::ScenarioVariant::MigrationSkipped).expect("broken deterministic upgrade");
        assert!(!correct.stale_generation_acted);
        assert!(!broken.stale_generation_acted);
        let scenario = super::FailureScenario::GenerationUpgradeWithStaleWorkRejection;
        assert!(super::evaluate_properties(scenario, &correct).iter().all(|outcome| outcome.holds));
        let outcomes = super::evaluate_properties(scenario, &broken);
        assert!(
            outcomes.iter().any(
                |outcome| !outcome.holds && outcome.property == super::ServiceProperty::StaleInstancesLoseInfluence
            )
        );
    }
}
