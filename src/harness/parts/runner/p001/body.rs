
fn actor_execution_events(input: ActorExecutionInput<'_>) -> Result<ActorExecutionEvents> {
    if !input.admission_decision.is_allowed() {
        return Ok(ActorExecutionEvents {
            steel: None,
            wasm: None,
        });
    }
    Ok(ActorExecutionEvents {
        steel: super::steel_executor::execute_steel_actor_step(
            input.suite,
            input.step,
            input.actor_input,
            input.hostcall_request,
        )?,
        wasm: super::wasm_executor::execute_wasm_actor_step(&super::wasm_executor::WasmActorStepInput {
            suite: input.suite,
            step: input.step,
            sequence: input.step_index,
            step_ref: input.step_ref,
            actor_input: input.actor_input,
            hostcall_request: input.hostcall_request,
            hostcall_decision: input.hostcall_decision,
        })?,
    })
}

fn boundary_events(
    execution: ActorExecutionEvents,
    runtime_events: Vec<preserves::IOValue>,
) -> Vec<preserves::IOValue> {
    let mut events = Vec::with_capacity(
        runtime_events.len() + usize::from(execution.steel.is_some()) + usize::from(execution.wasm.is_some()),
    );
    if let Some(steel) = execution.steel {
        events.push(steel);
    }
    if let Some(wasm) = execution.wasm {
        events.push(wasm);
    }
    events.extend(runtime_events);
    events
}

fn check_event_budget(step_index: u64, total_events: u64, budget: &super::schema::Budget) -> Result<()> {
    if total_events > budget.max_events {
        return Err(divergence(
            "resource",
            Some(step_index),
            budget.max_events.to_string(),
            total_events.to_string(),
            "event count exceeds budget",
        ));
    }
    Ok(())
}

fn check_effect_budget(step_index: u64, effects: u64, budget: &super::schema::Budget) -> Result<()> {
    if effects > budget.max_effects {
        return Err(divergence(
            "resource",
            Some(step_index),
            budget.max_effects.to_string(),
            effects.to_string(),
            "effect count exceeds budget",
        ));
    }
    Ok(())
}

fn check_replay_consumed(
    replay_effect_log: Option<&[super::schema::EffectLogEntry]>,
    replay_effect_index: usize,
) -> Result<()> {
    if let Some(replay_effect_log) = replay_effect_log
        && replay_effect_index != replay_effect_log.len()
    {
        return Err(divergence(
            "effect-log",
            None,
            replay_effect_index.to_string(),
            replay_effect_log.len().to_string(),
            "recorded effect log has unused entries",
        ));
    }
    Ok(())
}

fn build_report_value(
    suite: &super::schema::Suite,
    material: &SuiteRunMaterial,
    trace: &RunTrace,
) -> Result<preserves::IOValue> {
    let mut usage = super::schema::BudgetUsage {
        steps: suite.steps.len() as u64,
        effects: trace.effect_log.len() as u64,
        events: trace.total_events,
        report_bytes: 0,
    };
    let mut report_value = report_value_with_usage(suite, material, trace, &usage);
    for _ in 0..4 {
        let report_bytes = crate::preserves_rail::canonical_bytes(&report_value)?.len() as u64;
        if report_bytes == usage.report_bytes {
            break;
        }
        usage.report_bytes = report_bytes;
        report_value = report_value_with_usage(suite, material, trace, &usage);
    }
    usage.report_bytes = crate::preserves_rail::canonical_bytes(&report_value)?.len() as u64;
    if usage.report_bytes > material.budget.max_report_bytes {
        return Err(divergence(
            "resource",
            None,
            material.budget.max_report_bytes.to_string(),
            usage.report_bytes.to_string(),
            "report byte size exceeds budget",
        ));
    }
    Ok(report_value_with_usage(suite, material, trace, &usage))
}

fn report_value_with_usage(
    suite: &super::schema::Suite,
    material: &SuiteRunMaterial,
    trace: &RunTrace,
    usage: &super::schema::BudgetUsage,
) -> preserves::IOValue {
    super::schema::report_value(super::schema::ReportValueInput {
        suite,
        suite_ref: material.suite_ref.clone(),
        initial_state_hash: trace.initial_state_hash.clone(),
        final_state_hash: trace.final_state_hash.clone(),
        policy_gate: material.policy_gate.clone(),
        capability_gate: material.capability_gate.clone(),
        budget_gate: material.budget_gate.clone(),
        observations: trace.observations.clone(),
        effect_log: trace.effect_log.clone(),
        budget: &material.budget,
        usage,
    })
}

fn validate_actor_registry(suite: &super::schema::Suite) -> Result<()> {
    let mut ids = std::collections::BTreeSet::new();
    super::executor::ensure_supported_actor_executors(&suite.actors)?;
    for actor in &suite.actors {
        ids.insert(actor.id.as_str());
    }
    for step in &suite.steps {
        for actor in super::schema::actor_ids_for_step(step) {
            if !ids.contains(actor) {
                return Err(MoltenError::invalid_harness(format!("unknown actor {actor} in harness step")));
            }
        }
    }
    Ok(())
}

struct TurnJournalInput<'a> {
    index: u64,
    step_ref: &'a str,
    before_state_hash: &'a str,
    after_state_hash: &'a str,
    policy_ref: &'a str,
    capability_ref: &'a str,
    budget_ref: &'a str,
    events: &'a [preserves::IOValue],
}

fn turn_journal_value(input: TurnJournalInput<'_>) -> Result<preserves::IOValue> {
    let mut event_refs = Vec::with_capacity(input.events.len());
    let mut effect_refs = Vec::with_capacity(input.events.len());
    let mut receipt_refs = Vec::with_capacity(input.events.len());
    for event in input.events {
        let event_ref = crate::preserves_rail::canonical_hash(event)?;
        match super::schema::event_boundary(event) {
            super::schema::EventBoundary::EffectRequest | super::schema::EventBoundary::EffectResponse => {
                effect_refs.push(event_ref.clone());
            }
            super::schema::EventBoundary::RuntimePredicate
            | super::schema::EventBoundary::HostcallDecision
            | super::schema::EventBoundary::SteelExecution
            | super::schema::EventBoundary::WasmExecution => {
                receipt_refs.push(event_ref.clone());
            }
            super::schema::EventBoundary::PolicyDecision
            | super::schema::EventBoundary::ActorInput
            | super::schema::EventBoundary::HostcallRequest
            | super::schema::EventBoundary::ActorOutput
            | super::schema::EventBoundary::Trace => {}
        }
        event_refs.push(event_ref);
    }
    Ok(crate::preserves_rail::record("turn-journal-v1", vec![
        crate::preserves_rail::string("molten.harness.turn-journal.v1"),
        crate::preserves_rail::u64_value(input.index),
        crate::preserves_rail::record("scheduler-key", vec![crate::preserves_rail::string(format!(
            "logical:0:priority:0:queue:{}",
            input.index
        ))]),
        crate::preserves_rail::record("step-ref", vec![crate::preserves_rail::string(input.step_ref)]),
        crate::preserves_rail::record("before-state-ref", vec![crate::preserves_rail::string(input.before_state_hash)]),
        crate::preserves_rail::record("after-state-ref", vec![crate::preserves_rail::string(input.after_state_hash)]),
        crate::preserves_rail::record("policy-ref", vec![crate::preserves_rail::string(input.policy_ref)]),
        crate::preserves_rail::record("capability-ref", vec![crate::preserves_rail::string(input.capability_ref)]),
        crate::preserves_rail::record("budget-ref", vec![crate::preserves_rail::string(input.budget_ref)]),
        crate::preserves_rail::record("event-refs", vec![crate::preserves_rail::sequence(
            event_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("effect-refs", vec![crate::preserves_rail::sequence(
            effect_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("receipt-refs", vec![crate::preserves_rail::sequence(
            receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
    ]))
}

struct RuntimeStepInput<'a> {
    state: &'a mut super::core::RuntimeState,
    step: &'a super::core::CoreStep,
    admission_decision: &'a crate::runtime::AdmissionDecision,
    step_index: u64,
    replay_effect_log: Option<&'a [super::schema::EffectLogEntry]>,
    replay_effect_index: &'a mut usize,
}

fn runtime_events_for_step(input: RuntimeStepInput<'_>) -> Result<Vec<preserves::IOValue>> {
    let RuntimeStepInput {
        state,
        step,
        admission_decision,
        step_index,
        replay_effect_log,
        replay_effect_index,
    } = input;

    if !admission_decision.is_allowed() {
        let before = state.snapshot();
        let turn = state.begin_turn(step);
        let (runtime_events, receipt) = state.rollback_turn_with_predicate_receipt(
            turn.clone(),
            step.primary_actor(),
            admission_decision.reason(),
        )?;
        let after = state.snapshot();
        let mut events = vec![receipt.value];
        events.extend(runtime_events.iter().map(super::schema::event_value));
        events.extend(step_predicate_receipts(step, &before, &after)?);
        return Ok(events);
    }

    let Some(replay_effect_log) = replay_effect_log else {
        if !is_dataspace_turn(step) {
            let events = state.apply_step(step).iter().map(super::schema::event_value).collect();
            return with_time_random_handler_receipt(step, step_index, events);
        }
        let before = state.snapshot();
        let turn = state.begin_turn(step);
        let (runtime_events, receipt) = state.commit_turn_with_predicate_receipt(turn)?;
        let after = state.snapshot();
        let mut events = vec![receipt.value];
        events.extend(runtime_events.iter().map(super::schema::event_value));
        events.extend(step_predicate_receipts(step, &before, &after)?);
        return with_time_random_handler_receipt(step, step_index, events);
    };

    replay_effect_events(state, step, step_index, replay_effect_log, replay_effect_index)
}

fn is_dataspace_turn(step: &super::core::CoreStep) -> bool {
    matches!(
        step,
        super::core::CoreStep::Send { .. }
            | super::core::CoreStep::Observe { .. }
            | super::core::CoreStep::Assert { .. }
            | super::core::CoreStep::Retract { .. }
    )
}
