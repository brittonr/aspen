use super::core;
use super::schema;
use crate::preserves_rail;

type HarnessDivergence = crate::error::HarnessDivergence;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;
type RuntimeObserver = crate::runtime::RuntimeObserver;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessRun {
    pub report_value: preserves::IOValue,
    pub report_ref: String,
    pub suite_ref: String,
    pub initial_state_hash: String,
    pub final_state_hash: String,
    pub status: String,
}

pub fn run_suite_value(value: &preserves::IOValue) -> Result<HarnessRun> {
    run_suite(&schema::parse_suite(value)?)
}

pub fn run_suite(suite: &schema::HarnessSuite) -> Result<HarnessRun> {
    run_suite_inner(suite, None)
}

pub fn run_suite_with_effect_log(
    suite: &schema::HarnessSuite,
    effect_log: &[schema::EffectLogEntry],
) -> Result<HarnessRun> {
    run_suite_inner(suite, Some(effect_log))
}

fn run_suite_inner(
    suite: &schema::HarnessSuite,
    replay_effect_log: Option<&[schema::EffectLogEntry]>,
) -> Result<HarnessRun> {
    let material = prepare_suite_run(suite)?;
    let trace = collect_trace(suite, replay_effect_log, &material)?;
    let report_value = build_report_value(suite, &material, &trace)?;
    let report_ref = preserves_rail::canonical_hash(&report_value)?;
    Ok(HarnessRun {
        report_value,
        report_ref,
        suite_ref: material.suite_ref,
        initial_state_hash: trace.initial_state_hash,
        final_state_hash: trace.final_state_hash,
        status: "pass".to_string(),
    })
}

struct SuiteRunMaterial {
    suite_ref: String,
    policy_gate: preserves::IOValue,
    capability_gate: preserves::IOValue,
    budget_gate: preserves::IOValue,
    policy_ref: String,
    capability_ref: String,
    budget_ref: String,
    budget: schema::HarnessBudget,
}

fn prepare_suite_run(suite: &schema::HarnessSuite) -> Result<SuiteRunMaterial> {
    if !suite.actors_explicit {
        return Err(MoltenError::invalid_harness(
            "missing explicit actor registry fixture; inferred actors cannot execute evidence-bearing suites",
        ));
    }
    validate_actor_registry(suite)?;
    schema::validate_executor_preflight_inputs(suite)?;
    if !suite.capabilities_explicit {
        return Err(MoltenError::invalid_harness(
            "missing explicit capability fixture; implicit authority cannot execute evidence-bearing suites",
        ));
    }
    if !suite.budget_explicit {
        return Err(MoltenError::invalid_harness(
            "missing explicit budget fixture; default resource policy cannot execute evidence-bearing suites",
        ));
    }
    let budget = suite.budget.clone();
    if suite.steps.len() as u64 > budget.max_steps {
        return Err(divergence(
            "resource",
            None,
            budget.max_steps.to_string(),
            suite.steps.len().to_string(),
            "suite step count exceeds budget",
        ));
    }
    Ok(SuiteRunMaterial {
        suite_ref: schema::suite_ref(suite)?,
        policy_gate: schema::policy_gate_value(&suite.policy)?,
        capability_gate: schema::capability_gate_value(&suite.capabilities)?,
        budget_gate: schema::budget_gate_value(&suite.budget)?,
        policy_ref: preserves_rail::canonical_hash(&schema::policy_value(&suite.policy))?,
        capability_ref: preserves_rail::canonical_hash(&schema::capabilities_value(&suite.capabilities))?,
        budget_ref: preserves_rail::canonical_hash(&schema::budget_limits_value(&suite.budget))?,
        budget,
    })
}

struct RunTrace {
    initial_state_hash: String,
    final_state_hash: String,
    observations: Vec<preserves::IOValue>,
    effect_log: Vec<schema::EffectLogEntry>,
    total_events: u64,
}

fn collect_trace(
    suite: &schema::HarnessSuite,
    replay_effect_log: Option<&[schema::EffectLogEntry]>,
    material: &SuiteRunMaterial,
) -> Result<RunTrace> {
    let mut state = core::RuntimeState::new(suite.seed);
    let initial_state_hash = preserves_rail::canonical_hash(&schema::snapshot_value(&state.snapshot()))?;
    let mut observations = Vec::with_capacity(suite.steps.len());
    let mut effect_log = Vec::new();
    let mut total_events = 0u64;
    let mut replay_effect_index = 0usize;

    for (index, step) in suite.steps.iter().enumerate() {
        let outcome = run_step(StepRunInput {
            state: &mut state,
            suite,
            step,
            material,
            step_index: index as u64,
            replay_effect_log,
            replay_effect_index: &mut replay_effect_index,
        })?;
        total_events += outcome.events.len() as u64;
        check_event_budget(index as u64, total_events, &material.budget)?;
        schema::append_effect_entries_from_events(&outcome.events, &mut effect_log)?;
        check_effect_budget(index as u64, effect_log.len() as u64, &material.budget)?;
        observations.push(schema::observation_value(
            index as u64,
            outcome.step_ref,
            outcome.before_state_hash,
            outcome.after_state_hash,
            outcome.events,
        )?);
    }
    check_replay_consumed(replay_effect_log, replay_effect_index)?;
    Ok(RunTrace {
        initial_state_hash,
        final_state_hash: preserves_rail::canonical_hash(&schema::snapshot_value(&state.snapshot()))?,
        observations,
        effect_log,
        total_events,
    })
}

struct StepRunInput<'a> {
    state: &'a mut core::RuntimeState,
    suite: &'a schema::HarnessSuite,
    step: &'a core::CoreStep,
    material: &'a SuiteRunMaterial,
    step_index: u64,
    replay_effect_log: Option<&'a [schema::EffectLogEntry]>,
    replay_effect_index: &'a mut usize,
}

struct StepOutcome {
    step_ref: String,
    before_state_hash: String,
    after_state_hash: String,
    events: Vec<preserves::IOValue>,
}

fn run_step(input: StepRunInput<'_>) -> Result<StepOutcome> {
    let before_state_hash = preserves_rail::canonical_hash(&schema::snapshot_value(&input.state.snapshot()))?;
    let step_ref = preserves_rail::canonical_hash(&schema::step_value(input.step))?;
    let admission = admission_step(input.suite, input.step, input.material, input.step_index, &step_ref)?;
    let execution = actor_execution_events(ActorExecutionInput {
        suite: input.suite,
        step: input.step,
        step_index: input.step_index,
        step_ref: &step_ref,
        actor_input: &admission.actor_input,
        hostcall_request: &admission.hostcall_request,
        hostcall_decision: &admission.hostcall_decision,
        admission_decision: &admission.decision,
    })?;
    let runtime_events = runtime_events_for_step(RuntimeStepInput {
        state: input.state,
        step: input.step,
        admission_decision: &admission.decision,
        step_index: input.step_index,
        replay_effect_log: input.replay_effect_log,
        replay_effect_index: input.replay_effect_index,
    })?;
    let boundary_runtime_events = boundary_events(execution, runtime_events);
    let mut events = admission.events;
    events.extend(boundary_runtime_events.clone());
    events.push(schema::actor_output_value(
        input.step,
        admission.context,
        &admission.decision,
        &boundary_runtime_events,
    )?);
    let after_state_hash = preserves_rail::canonical_hash(&schema::snapshot_value(&input.state.snapshot()))?;
    events.push(turn_journal_value(TurnJournalInput {
        index: input.step_index,
        step_ref: &step_ref,
        before_state_hash: &before_state_hash,
        after_state_hash: &after_state_hash,
        policy_ref: &input.material.policy_ref,
        capability_ref: &input.material.capability_ref,
        budget_ref: &input.material.budget_ref,
        events: &events,
    })?);
    Ok(StepOutcome {
        step_ref,
        before_state_hash,
        after_state_hash,
        events,
    })
}

struct AdmissionStep<'a> {
    decision: crate::runtime::AdmissionDecision,
    context: schema::HostcallEvidenceContext<'a>,
    actor_input: preserves::IOValue,
    hostcall_request: preserves::IOValue,
    hostcall_decision: preserves::IOValue,
    events: Vec<preserves::IOValue>,
}

fn admission_step<'a>(
    suite: &'a schema::HarnessSuite,
    step: &'a core::CoreStep,
    material: &'a SuiteRunMaterial,
    step_index: u64,
    step_ref: &'a str,
) -> Result<AdmissionStep<'a>> {
    let request = core::AdmissionRequest::from_step(step);
    let authority = schema::admission_authority_evidence(&suite.capabilities, &request)?;
    let decision = suite.policy.decide_with_capabilities(&suite.capabilities, &request);
    let event = schema::admission_decision_event_value_with_authority(&request, &authority, &decision);
    let context = schema::HostcallEvidenceContext {
        sequence: step_index,
        suite_ref: &material.suite_ref,
        step_ref,
        policy_ref: &material.policy_ref,
        capability_ref: &material.capability_ref,
        budget_ref: &material.budget_ref,
    };
    let actor_input = schema::actor_input_value(suite, step, context)?;
    let hostcall_request = schema::hostcall_request_value(suite, step, context, &decision)?;
    let hostcall_decision = schema::hostcall_decision_value(context, &event, &authority, &decision)?;
    Ok(AdmissionStep {
        decision,
        context,
        events: vec![
            event,
            actor_input.clone(),
            hostcall_request.clone(),
            hostcall_decision.clone(),
        ],
        actor_input,
        hostcall_request,
        hostcall_decision,
    })
}

struct ActorExecutionInput<'a> {
    suite: &'a schema::HarnessSuite,
    step: &'a core::CoreStep,
    step_index: u64,
    step_ref: &'a str,
    actor_input: &'a preserves::IOValue,
    hostcall_request: &'a preserves::IOValue,
    hostcall_decision: &'a preserves::IOValue,
    admission_decision: &'a crate::runtime::AdmissionDecision,
}

struct ActorExecutionEvents {
    steel: Option<preserves::IOValue>,
    wasm: Option<preserves::IOValue>,
}

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

fn check_event_budget(step_index: u64, total_events: u64, budget: &schema::HarnessBudget) -> Result<()> {
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

fn check_effect_budget(step_index: u64, effects: u64, budget: &schema::HarnessBudget) -> Result<()> {
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
    replay_effect_log: Option<&[schema::EffectLogEntry]>,
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
    suite: &schema::HarnessSuite,
    material: &SuiteRunMaterial,
    trace: &RunTrace,
) -> Result<preserves::IOValue> {
    let mut usage = schema::BudgetUsage {
        steps: suite.steps.len() as u64,
        effects: trace.effect_log.len() as u64,
        events: trace.total_events,
        report_bytes: 0,
    };
    let mut report_value = report_value_with_usage(suite, material, trace, &usage);
    for _ in 0..4 {
        let report_bytes = preserves_rail::canonical_bytes(&report_value)?.len() as u64;
        if report_bytes == usage.report_bytes {
            break;
        }
        usage.report_bytes = report_bytes;
        report_value = report_value_with_usage(suite, material, trace, &usage);
    }
    usage.report_bytes = preserves_rail::canonical_bytes(&report_value)?.len() as u64;
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
    suite: &schema::HarnessSuite,
    material: &SuiteRunMaterial,
    trace: &RunTrace,
    usage: &schema::BudgetUsage,
) -> preserves::IOValue {
    schema::report_value(schema::ReportValueInput {
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

fn validate_actor_registry(suite: &schema::HarnessSuite) -> Result<()> {
    let mut ids = std::collections::BTreeSet::new();
    super::executor::ensure_supported_actor_executors(&suite.actors)?;
    for actor in &suite.actors {
        ids.insert(actor.id.as_str());
    }
    for step in &suite.steps {
        for actor in schema::actor_ids_for_step(step) {
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
        let event_ref = preserves_rail::canonical_hash(event)?;
        match schema::event_boundary(event) {
            schema::EventBoundary::EffectRequest | schema::EventBoundary::EffectResponse => {
                effect_refs.push(event_ref.clone());
            }
            schema::EventBoundary::RuntimePredicate
            | schema::EventBoundary::HostcallDecision
            | schema::EventBoundary::SteelExecution
            | schema::EventBoundary::WasmExecution => {
                receipt_refs.push(event_ref.clone());
            }
            schema::EventBoundary::PolicyDecision
            | schema::EventBoundary::ActorInput
            | schema::EventBoundary::HostcallRequest
            | schema::EventBoundary::ActorOutput
            | schema::EventBoundary::Trace => {}
        }
        event_refs.push(event_ref);
    }
    Ok(preserves_rail::record("turn-journal-v1", vec![
        preserves_rail::string("molten.harness.turn-journal.v1"),
        preserves_rail::u64_value(input.index),
        preserves_rail::record("scheduler-key", vec![preserves_rail::string(format!(
            "logical:0:priority:0:queue:{}",
            input.index
        ))]),
        preserves_rail::record("step-ref", vec![preserves_rail::string(input.step_ref)]),
        preserves_rail::record("before-state-ref", vec![preserves_rail::string(input.before_state_hash)]),
        preserves_rail::record("after-state-ref", vec![preserves_rail::string(input.after_state_hash)]),
        preserves_rail::record("policy-ref", vec![preserves_rail::string(input.policy_ref)]),
        preserves_rail::record("capability-ref", vec![preserves_rail::string(input.capability_ref)]),
        preserves_rail::record("budget-ref", vec![preserves_rail::string(input.budget_ref)]),
        preserves_rail::record("event-refs", vec![preserves_rail::sequence(
            event_refs.iter().map(preserves_rail::string).collect(),
        )]),
        preserves_rail::record("effect-refs", vec![preserves_rail::sequence(
            effect_refs.iter().map(preserves_rail::string).collect(),
        )]),
        preserves_rail::record("receipt-refs", vec![preserves_rail::sequence(
            receipt_refs.iter().map(preserves_rail::string).collect(),
        )]),
    ]))
}

struct RuntimeStepInput<'a> {
    state: &'a mut core::RuntimeState,
    step: &'a core::CoreStep,
    admission_decision: &'a crate::runtime::AdmissionDecision,
    step_index: u64,
    replay_effect_log: Option<&'a [schema::EffectLogEntry]>,
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
        events.extend(runtime_events.iter().map(schema::event_value));
        events.extend(step_predicate_receipts(step, &before, &after)?);
        return Ok(events);
    }

    let Some(replay_effect_log) = replay_effect_log else {
        if !is_dataspace_turn(step) {
            let events = state.apply_step(step).iter().map(schema::event_value).collect();
            return with_time_random_handler_receipt(step, step_index, events);
        }
        let before = state.snapshot();
        let turn = state.begin_turn(step);
        let (runtime_events, receipt) = state.commit_turn_with_predicate_receipt(turn)?;
        let after = state.snapshot();
        let mut events = vec![receipt.value];
        events.extend(runtime_events.iter().map(schema::event_value));
        events.extend(step_predicate_receipts(step, &before, &after)?);
        return with_time_random_handler_receipt(step, step_index, events);
    };

    replay_effect_events(state, step, step_index, replay_effect_log, replay_effect_index)
}

fn is_dataspace_turn(step: &core::CoreStep) -> bool {
    matches!(
        step,
        core::CoreStep::Send { .. }
            | core::CoreStep::Observe { .. }
            | core::CoreStep::Assert { .. }
            | core::CoreStep::Retract { .. }
    )
}

fn step_predicate_receipts(
    step: &core::CoreStep,
    before: &crate::runtime::RuntimeSnapshot,
    after: &crate::runtime::RuntimeSnapshot,
) -> Result<Vec<preserves::IOValue>> {
    match step {
        core::CoreStep::Observe { actor, pattern } => {
            let observer = RuntimeObserver {
                actor: actor.clone(),
                pattern: pattern.clone(),
            };
            let receipt = crate::runtime::evaluate_observe_initial_delivery(before, &observer)?.receipt;
            Ok(vec![receipt.value])
        }
        core::CoreStep::Assert { value, .. } | core::CoreStep::Retract { value, .. } => {
            let live_owners = after.assertions.iter().map(|assertion| assertion.actor.clone()).collect();
            let receipt = crate::runtime::evaluate_assertion_visibility(after, value, &live_owners)?.receipt;
            Ok(vec![receipt.value])
        }
        core::CoreStep::Send { .. } | core::CoreStep::Clock { .. } | core::CoreStep::Random { .. } => Ok(Vec::new()),
    }
}

fn replay_effect_events(
    state: &mut core::RuntimeState,
    step: &core::CoreStep,
    step_index: u64,
    replay_effect_log: &[schema::EffectLogEntry],
    replay_effect_index: &mut usize,
) -> Result<Vec<preserves::IOValue>> {
    let Some(request) = state.begin_effect_for_step(step) else {
        if !is_dataspace_turn(step) {
            return Ok(state.apply_step(step).iter().map(schema::event_value).collect());
        }
        let before = state.snapshot();
        let turn = state.begin_turn(step);
        let (runtime_events, receipt) = state.commit_turn_with_predicate_receipt(turn)?;
        let after = state.snapshot();
        let mut events = vec![receipt.value];
        events.extend(runtime_events.iter().map(schema::event_value));
        events.extend(step_predicate_receipts(step, &before, &after)?);
        return Ok(events);
    };

    let Some(entry) = replay_effect_log.get(*replay_effect_index) else {
        return Err(divergence(
            "effect-log",
            Some(step_index),
            format!("entry {}", *replay_effect_index),
            "missing",
            "recorded effect log ended before effect request",
        ));
    };
    let request_value = schema::event_value(&request);
    let request_hash = preserves_rail::canonical_hash(&request_value)?;
    let recorded_request_hash = preserves_rail::canonical_hash(&entry.request)?;
    if request_hash != recorded_request_hash {
        return Err(divergence(
            "effect-request",
            Some(step_index),
            recorded_request_hash,
            request_hash,
            "effect request does not match recorded log",
        ));
    }

    let (response_sequence, response_value) = schema::effect_response_sequence_and_value(&entry.response)?;
    let request_sequence = schema::effect_request_sequence(&entry.request)?;
    if response_sequence != request_sequence {
        return Err(divergence(
            "effect-log",
            Some(step_index),
            request_sequence.to_string(),
            response_sequence.to_string(),
            "recorded effect request/response sequence mismatch",
        ));
    }

    let response = state.apply_recorded_effect_response(&request, response_value)?;
    let response_value = schema::event_value(&response);
    let response_hash = preserves_rail::canonical_hash(&response_value)?;
    let recorded_response_hash = preserves_rail::canonical_hash(&entry.response)?;
    if response_hash != recorded_response_hash {
        return Err(divergence(
            "effect-response",
            Some(step_index),
            recorded_response_hash,
            response_hash,
            "effect response does not match recorded log",
        ));
    }

    *replay_effect_index += 1;
    with_time_random_handler_receipt(step, step_index, vec![request_value, response_value])
}

fn with_time_random_handler_receipt(
    step: &core::CoreStep,
    step_index: u64,
    events: Vec<preserves::IOValue>,
) -> Result<Vec<preserves::IOValue>> {
    let (effect, actor) = match step {
        core::CoreStep::Clock { actor } => ("clock", actor.as_str()),
        core::CoreStep::Random { actor, .. } => ("random", actor.as_str()),
        core::CoreStep::Send { .. }
        | core::CoreStep::Observe { .. }
        | core::CoreStep::Assert { .. }
        | core::CoreStep::Retract { .. } => return Ok(events),
    };
    if events.len() != 2 {
        return Err(MoltenError::invalid_harness(format!(
            "deterministic {effect} handler expected request and response events at step {step_index}"
        )));
    }
    let request_ref = preserves_rail::canonical_hash(&events[0])?;
    let response_ref = preserves_rail::canonical_hash(&events[1])?;
    let handler_binding = preserves_rail::record("time-random-handler-binding-v1", vec![
        preserves_rail::string("local-deterministic"),
        preserves_rail::string(effect),
        preserves_rail::string(actor),
        preserves_rail::u64_value(step_index),
    ]);
    let handler_binding_ref = preserves_rail::canonical_hash(&handler_binding)?;
    let receipt = preserves_rail::record("time-random-handler-receipt-v1", vec![
        preserves_rail::string("molten.effects.time-random-handler.v1"),
        preserves_rail::record("profile", vec![preserves_rail::string("local-deterministic")]),
        preserves_rail::record("effect", vec![preserves_rail::string(effect)]),
        preserves_rail::record("actor", vec![preserves_rail::string(actor)]),
        preserves_rail::record("request-ref", vec![preserves_rail::string(&request_ref)]),
        preserves_rail::record("handler-binding-ref", vec![preserves_rail::string(&handler_binding_ref)]),
        preserves_rail::record("response-ref", vec![preserves_rail::string(&response_ref)]),
        preserves_rail::record("decision", vec![preserves_rail::string("pass")]),
        preserves_rail::record("checks", vec![preserves_rail::record("check", vec![
            preserves_rail::string("deny-by-default-bypassed-only-by-local-test-handler"),
            preserves_rail::string("pass"),
        ])]),
    ]);
    Ok(vec![events[0].clone(), receipt, events[1].clone()])
}

fn divergence(
    kind: impl Into<String>,
    step: Option<u64>,
    expected: impl Into<String>,
    actual: impl Into<String>,
    detail: impl Into<String>,
) -> MoltenError {
    MoltenError::harness_divergence(HarnessDivergence::new(kind, step, expected, actual, detail))
}
