// The deterministic scenario matrix and the property evaluation.
//
// A scenario drives the composition through one failure class and records what
// it observed. The property evaluation reads those observations, so a
// regression in one mechanism fails the scenarios that depend on it.


/// Run one scenario of the composition and record what it observed.
pub fn run_scenario(
    definitions: &GroupDefinitions,
    scenario: FailureScenario,
    variant: ScenarioVariant,
) -> Result<ScenarioFacts, crate::error::Failure> {
    match scenario {
        FailureScenario::MemberFailureWithDependentRestart => member_failure(definitions),
        FailureScenario::DelayedPreRestartEventDelivery => delayed_delivery(definitions, variant),
        FailureScenario::DurableWorkReadmission => durable_readmission(),
        FailureScenario::UncertainEffectSurfacing => uncertain_effect(),
        FailureScenario::QueueOverload => queue_overload(),
        FailureScenario::RestartStormToEscalation => restart_storm(definitions, variant),
        FailureScenario::GenerationUpgradeWithStaleWorkRejection => generation_upgrade(),
        FailureScenario::CpuSaturationUnderTimeSlicing => cpu_saturation(variant),
    }
}

fn member_failure(definitions: &GroupDefinitions) -> Result<ScenarioFacts, crate::error::Failure> {
    let stateful = running_state()?;
    let worker = running_state()?;
    let exit = RecoveryExit {
        member_id: STATEFUL_SERVICE_ID.to_string(),
        was_successful: false,
        failure: Some(FailureClass::Retryable),
    };
    let plan = plan_recovery(&RecoveryPlanInput {
        group: &definitions.group,
        exit: &exit,
        history: &[],
        now_tick: 10,
    })
    .map_err(|issue| crate::error::Failure::invalid_harness(format!("the group plan denied: {issue:?}")))?;
    let stateful_after = restart(&stateful)?;
    let worker_after = restart(&worker)?;
    // The recovery group stops the affected members from the end and restarts
    // them in order, so the dependent worker restarts after its dependency.
    let restart_index = |member_id: &str| {
        plan.actions.iter().position(
            |action| matches!(action, RecoveryAction::Restart { member_id: name } if name == member_id),
        )
    };
    let stateful_restart = restart_index(STATEFUL_SERVICE_ID);
    let worker_restart = restart_index(WORKER_SERVICE_ID);
    let mut facts = ScenarioFacts::empty();
    // The sibling is outside the recovery group, so it keeps working.
    facts.sibling_progressed = plan.decision == RecoveryDecision::Restart;
    facts.dependent_restarted_after_dependency = stateful_restart.is_some()
        && worker_restart.is_some()
        && worker_restart > stateful_restart;
    facts.restarted_member_incarnation_advanced =
        stateful_after.incarnation > stateful.incarnation && worker_after.incarnation > worker.incarnation;
    Ok(facts)
}

fn delayed_delivery(
    definitions: &GroupDefinitions,
    variant: ScenarioVariant,
) -> Result<ScenarioFacts, crate::error::Failure> {
    let stateful = running_state()?;
    let after = match variant {
        ScenarioVariant::FencingRemoved => stateful.clone(),
        _ => restart(&stateful)?,
    };
    let delivery = FacadeTimerDelivery {
        timer_ref: "timer-pre-restart".to_string(),
        phase: "idle".to_string(),
        incarnation: stateful.incarnation,
        logical_tick: 20,
    };
    let is_admitted = admit_facade_timer(
        &definitions.stateful_facade,
        &facade_state("idle", after.incarnation),
        &delivery,
    )
    .is_ok();
    let mut facts = ScenarioFacts::empty();
    facts.stale_delivery_acted = is_admitted;
    Ok(facts)
}

fn durable_readmission() -> Result<ScenarioFacts, crate::error::Failure> {
    let stateful = running_state()?;
    let after = restart(&stateful)?;
    let durable = WorkPresentation {
        work_ref: HASH_C.to_string(),
        class: WorkClass::Durable,
        generation: stateful.generation,
        incarnation: stateful.incarnation,
    };
    let fresh_claim = DeliveryClaim {
        claim_ref: HASH_D.to_string(),
        class: WorkClass::Durable,
        generation: after.generation,
        incarnation: after.incarnation,
    };
    let stale_claim = DeliveryClaim {
        claim_ref: HASH_E.to_string(),
        class: WorkClass::Durable,
        generation: after.generation,
        incarnation: stateful.incarnation,
    };
    let runtime_local = WorkPresentation {
        work_ref: HASH_C.to_string(),
        class: WorkClass::RuntimeLocal,
        generation: stateful.generation,
        incarnation: stateful.incarnation,
    };
    let is_readmitted = admit_work_delivery(&after, &durable, Some(&fresh_claim)).is_ok();
    let is_stale_claim_denied = admit_work_delivery(&after, &durable, Some(&stale_claim)).is_err();
    let is_runtime_local_denied = admit_work_delivery(&after, &runtime_local, Some(&fresh_claim)).is_err();
    let mut facts = ScenarioFacts::empty();
    facts.durable_work_readmitted = is_readmitted;
    facts.durable_work_lost = !(is_readmitted && is_stale_claim_denied && is_runtime_local_denied);
    Ok(facts)
}

fn uncertain_effect() -> Result<ScenarioFacts, crate::error::Failure> {
    let profile = durable_profile();
    let state = molten_core::fabric_durability::DurableState::empty(durable_descriptor());
    let transaction_id = "composition-transaction".to_string();
    let reserve = molten_core::fabric_durability::EffectTransactionCommand::Reserve {
        transaction_id: transaction_id.clone(),
        generation: INITIAL_GENERATION,
        operation_ref: HASH_C.to_string(),
        expires_at_tick: Some(40),
        profile: effect_profile(),
    };
    let reserved = molten_core::fabric_durability::apply_effect_transaction(&profile, &state, &reserve).map_err(|issues| {
        crate::error::Failure::invalid_harness(format!("the effect transaction is not reserved: {issues:?}"))
    })?;
    let uncertain = molten_core::fabric_durability::apply_effect_transaction(
        &profile,
        &reserved.next,
        &molten_core::fabric_durability::EffectTransactionCommand::MarkUncertain {
            transaction_id: transaction_id.clone(),
            generation: INITIAL_GENERATION,
        },
    )
    .map_err(|issues| {
        crate::error::Failure::invalid_harness(format!("the uncertain outcome is not recorded: {issues:?}"))
    })?;
    let phase = uncertain.next.effects.get(&transaction_id).map(|transaction| transaction.phase);
    let blind_repeat = molten_core::fabric_durability::apply_effect_transaction(&profile, &uncertain.next, &reserve);
    let mut facts = ScenarioFacts::empty();
    facts.uncertain_effect_repeated = blind_repeat.is_ok();
    facts.durable_work_lost = phase != Some(molten_core::fabric_durability::EffectTransactionPhase::Uncertain);
    Ok(facts)
}

fn queue_overload() -> Result<ScenarioFacts, crate::error::Failure> {
    let envelope = resource_envelope();
    let usage = ResourceUsage {
        concurrent_callbacks: envelope.max_concurrent_callbacks,
        queued_events: 0,
        inflight_bytes: 0,
        open_streams: 0,
        timers: 0,
        effect_requests: 0,
    };
    let plan = plan_resource_admission(&envelope, usage, REQUEST_BYTES)
        .map_err(|issues| crate::error::Failure::invalid_harness(format!("the admission denied: {issues:?}")))?;
    let mut facts = ScenarioFacts::empty();
    facts.sibling_progressed = true;
    facts.overload_denied = plan.decision == AdmissionDecision::Reject;
    Ok(facts)
}

fn restart_storm(
    definitions: &GroupDefinitions,
    variant: ScenarioVariant,
) -> Result<ScenarioFacts, crate::error::Failure> {
    let mut history: Vec<RecoveryAttempt> = Vec::with_capacity(RESTART_HISTORY_CAPACITY);
    let mut is_terminated = false;
    for tick in [10_u64, 20, 30, 40, 50] {
        let exit = RecoveryExit {
            member_id: STATEFUL_SERVICE_ID.to_string(),
            was_successful: false,
            failure: Some(FailureClass::Retryable),
        };
        let plan = plan_recovery(&RecoveryPlanInput {
            group: &definitions.group,
            exit: &exit,
            history: &history,
            now_tick: tick,
        })
        .map_err(|issue| crate::error::Failure::invalid_harness(format!("the group plan denied: {issue:?}")))?;
        if matches!(
            plan.decision,
            RecoveryDecision::EscalateGroup | RecoveryDecision::Quarantine
        ) {
            is_terminated = true;
            break;
        }
        history.push(RecoveryAttempt {
            member_id: STATEFUL_SERVICE_ID.to_string(),
            observed_at_tick: tick,
        });
        if variant == ScenarioVariant::GroupBudgetReset {
            history.clear();
        }
    }
    let mut facts = ScenarioFacts::empty();
    facts.storm_terminated = is_terminated;
    Ok(facts)
}

fn generation_upgrade() -> Result<ScenarioFacts, crate::error::Failure> {
    let stateful = running_state()?;
    let upgraded = plan_lifecycle_transition(
        &stateful,
        &LifecycleEvent {
            kind: LifecycleEventKind::BeginUpgrade,
            generation: stateful.generation,
            incarnation: stateful.incarnation,
            next_generation: Some(stateful.generation + 1),
            checkpoint_ref: None,
            failure_class: None,
        },
        idle_usage(),
        MAX_RESTART_ATTEMPTS,
    )
    .map_err(|issues| crate::error::Failure::invalid_harness(format!("the upgrade denied: {issues:?}")))?;
    let stale = plan_lifecycle_transition(
        &upgraded,
        &failure_event(&stateful),
        idle_usage(),
        MAX_RESTART_ATTEMPTS,
    );
    let mut facts = ScenarioFacts::empty();
    facts.stale_generation_acted = stale.is_ok();
    Ok(facts)
}
