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
    run_suite(&super::schema::parse_suite(value)?)
}

pub fn run_suite(suite: &super::schema::Suite) -> Result<HarnessRun> {
    run_suite_inner(suite, None)
}

pub fn run_suite_with_effect_log(
    suite: &super::schema::Suite,
    effect_log: &[super::schema::EffectLogEntry],
) -> Result<HarnessRun> {
    run_suite_inner(suite, Some(effect_log))
}

fn run_suite_inner(
    suite: &super::schema::Suite,
    replay_effect_log: Option<&[super::schema::EffectLogEntry]>,
) -> Result<HarnessRun> {
    let material = prepare_suite_run(suite)?;
    let trace = collect_trace(suite, replay_effect_log, &material)?;
    let report_value = build_report_value(suite, &material, &trace)?;
    let report_ref = crate::preserves_rail::canonical_hash(&report_value)?;
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
    budget: super::schema::Budget,
}

fn prepare_suite_run(suite: &super::schema::Suite) -> Result<SuiteRunMaterial> {
    if !suite.actors_explicit {
        return Err(MoltenError::invalid_harness(
            "missing explicit actor registry fixture; inferred actors cannot execute evidence-bearing suites",
        ));
    }
    validate_actor_registry(suite)?;
    super::schema::validate_executor_preflight_inputs(suite)?;
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
        suite_ref: super::schema::suite_ref(suite)?,
        policy_gate: super::schema::policy_gate_value(&suite.policy)?,
        capability_gate: super::schema::capability_gate_value(&suite.capabilities)?,
        budget_gate: super::schema::budget_gate_value(&suite.budget)?,
        policy_ref: crate::preserves_rail::canonical_hash(&super::schema::policy_value(&suite.policy))?,
        capability_ref: crate::preserves_rail::canonical_hash(&super::schema::capabilities_value(&suite.capabilities))?,
        budget_ref: crate::preserves_rail::canonical_hash(&super::schema::budget_limits_value(&suite.budget))?,
        budget,
    })
}

struct RunTrace {
    initial_state_hash: String,
    final_state_hash: String,
    observations: Vec<preserves::IOValue>,
    effect_log: Vec<super::schema::EffectLogEntry>,
    total_events: u64,
}

fn collect_trace(
    suite: &super::schema::Suite,
    replay_effect_log: Option<&[super::schema::EffectLogEntry]>,
    material: &SuiteRunMaterial,
) -> Result<RunTrace> {
    let mut state = super::core::RuntimeState::new(suite.seed);
    let initial_state_hash = crate::preserves_rail::canonical_hash(&super::schema::snapshot_value(&state.snapshot()))?;
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
        super::schema::append_effect_entries_from_events(&outcome.events, &mut effect_log)?;
        check_effect_budget(index as u64, effect_log.len() as u64, &material.budget)?;
        observations.push(super::schema::observation_value(
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
        final_state_hash: crate::preserves_rail::canonical_hash(&super::schema::snapshot_value(&state.snapshot()))?,
        observations,
        effect_log,
        total_events,
    })
}

struct StepRunInput<'a> {
    state: &'a mut super::core::RuntimeState,
    suite: &'a super::schema::Suite,
    step: &'a super::core::CoreStep,
    material: &'a SuiteRunMaterial,
    step_index: u64,
    replay_effect_log: Option<&'a [super::schema::EffectLogEntry]>,
    replay_effect_index: &'a mut usize,
}

struct StepOutcome {
    step_ref: String,
    before_state_hash: String,
    after_state_hash: String,
    events: Vec<preserves::IOValue>,
}

fn run_step(input: StepRunInput<'_>) -> Result<StepOutcome> {
    let before_state_hash =
        crate::preserves_rail::canonical_hash(&super::schema::snapshot_value(&input.state.snapshot()))?;
    let step_ref = crate::preserves_rail::canonical_hash(&super::schema::step_value(input.step))?;
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
    events.push(super::schema::actor_output_value(
        input.step,
        admission.context,
        &admission.decision,
        &boundary_runtime_events,
    )?);
    let after_state_hash =
        crate::preserves_rail::canonical_hash(&super::schema::snapshot_value(&input.state.snapshot()))?;
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
    context: super::schema::HostcallEvidenceContext<'a>,
    actor_input: preserves::IOValue,
    hostcall_request: preserves::IOValue,
    hostcall_decision: preserves::IOValue,
    events: Vec<preserves::IOValue>,
}

fn admission_step<'a>(
    suite: &'a super::schema::Suite,
    step: &'a super::core::CoreStep,
    material: &'a SuiteRunMaterial,
    step_index: u64,
    step_ref: &'a str,
) -> Result<AdmissionStep<'a>> {
    let request = super::core::AdmissionRequest::from_step(step);
    let authority = super::schema::admission_authority_evidence(&suite.capabilities, &request)?;
    let decision = suite.policy.decide_with_capabilities(&suite.capabilities, &request);
    let event = super::schema::admission_decision_event_value_with_authority(&request, &authority, &decision);
    let context = super::schema::HostcallEvidenceContext {
        sequence: step_index,
        suite_ref: &material.suite_ref,
        step_ref,
        policy_ref: &material.policy_ref,
        capability_ref: &material.capability_ref,
        budget_ref: &material.budget_ref,
    };
    let actor_input = super::schema::actor_input_value(suite, step, context)?;
    let hostcall_request = super::schema::hostcall_request_value(suite, step, context, &decision)?;
    let hostcall_decision = super::schema::hostcall_decision_value(context, &event, &authority, &decision)?;
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
    suite: &'a super::schema::Suite,
    step: &'a super::core::CoreStep,
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
