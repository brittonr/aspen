use preserves::IOValue;

use super::core::AdmissionRequest;
use super::core::RuntimeState;
use super::executor::ensure_supported_actor_executors;
use super::schema::BudgetUsage;
use super::schema::EffectLogEntry;
use super::schema::HarnessSuite;
use super::schema::HostcallEvidenceContext;
use super::schema::ReportValueInput;
use super::schema::actor_ids_for_step;
use super::schema::actor_input_value;
use super::schema::actor_output_value;
use super::schema::admission_authority_evidence;
use super::schema::admission_decision_event_value_with_authority;
use super::schema::append_effect_entries_from_events;
use super::schema::budget_gate_value;
use super::schema::budget_limits_value;
use super::schema::capabilities_value;
use super::schema::capability_gate_value;
use super::schema::effect_request_sequence;
use super::schema::effect_response_sequence_and_value;
use super::schema::event_value;
use super::schema::hostcall_decision_value;
use super::schema::hostcall_request_value;
use super::schema::observation_value;
use super::schema::parse_suite;
use super::schema::policy_gate_value;
use super::schema::policy_value;
use super::schema::snapshot_value;
use super::schema::step_value;
use super::schema::suite_ref;
use super::schema::validate_executor_preflight_inputs;
use super::steel_executor::execute_steel_actor_step;
use super::wasm_executor::WasmActorStepInput;
use super::wasm_executor::execute_wasm_actor_step;
use crate::error::HarnessDivergence;
use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::canonical_bytes;
use crate::preserves_rail::canonical_hash;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessRun {
    pub report_value: IOValue,
    pub report_ref: String,
    pub suite_ref: String,
    pub initial_state_hash: String,
    pub final_state_hash: String,
    pub status: String,
}

pub fn run_suite_value(value: &IOValue) -> Result<HarnessRun> {
    run_suite(&parse_suite(value)?)
}

pub fn run_suite(suite: &HarnessSuite) -> Result<HarnessRun> {
    run_suite_inner(suite, None)
}

pub fn run_suite_with_effect_log(suite: &HarnessSuite, effect_log: &[EffectLogEntry]) -> Result<HarnessRun> {
    run_suite_inner(suite, Some(effect_log))
}

fn run_suite_inner(suite: &HarnessSuite, replay_effect_log: Option<&[EffectLogEntry]>) -> Result<HarnessRun> {
    if !suite.actors_explicit {
        return Err(MoltenError::invalid_harness(
            "missing explicit actor registry fixture; inferred actors cannot execute evidence-bearing suites",
        ));
    }
    validate_actor_registry(suite)?;
    validate_executor_preflight_inputs(suite)?;
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
    let policy_gate = policy_gate_value(&suite.policy)?;
    let capability_gate = capability_gate_value(&suite.capabilities)?;
    let budget_gate = budget_gate_value(&suite.budget)?;
    let policy_ref = canonical_hash(&policy_value(&suite.policy))?;
    let capability_ref = canonical_hash(&capabilities_value(&suite.capabilities))?;
    let budget_ref = canonical_hash(&budget_limits_value(&suite.budget))?;
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

    let suite_ref = suite_ref(suite)?;
    let mut state = RuntimeState::new(suite.seed);
    let initial_state_hash = canonical_hash(&snapshot_value(&state.snapshot()))?;
    let mut observations = Vec::with_capacity(suite.steps.len());
    let mut effect_log = Vec::new();
    let mut total_events = 0u64;
    let mut replay_effect_index = 0usize;

    for (index, step) in suite.steps.iter().enumerate() {
        let before_state_hash = canonical_hash(&snapshot_value(&state.snapshot()))?;
        let step_ref = canonical_hash(&step_value(step))?;
        let admission_request = AdmissionRequest::from_step(step);
        let admission_authority = admission_authority_evidence(&suite.capabilities, &admission_request)?;
        let admission_decision = suite.policy.decide_with_capabilities(&suite.capabilities, &admission_request);
        let admission_event = admission_decision_event_value_with_authority(
            &admission_request,
            &admission_authority,
            &admission_decision,
        );
        let hostcall_context = HostcallEvidenceContext {
            sequence: index as u64,
            suite_ref: &suite_ref,
            step_ref: &step_ref,
            policy_ref: &policy_ref,
            capability_ref: &capability_ref,
            budget_ref: &budget_ref,
        };
        let actor_input = actor_input_value(suite, step, hostcall_context)?;
        let hostcall_request = hostcall_request_value(suite, step, hostcall_context, &admission_decision)?;
        let hostcall_decision =
            hostcall_decision_value(hostcall_context, &admission_event, &admission_authority, &admission_decision)?;
        let mut events = vec![
            admission_event.clone(),
            actor_input.clone(),
            hostcall_request,
            hostcall_decision.clone(),
        ];
        let steel_execution_event = if admission_decision.is_allowed() {
            execute_steel_actor_step(suite, step, &actor_input)?
        } else {
            None
        };
        let wasm_execution_event = if admission_decision.is_allowed() {
            execute_wasm_actor_step(&WasmActorStepInput {
                suite,
                step,
                sequence: index as u64,
                step_ref: &step_ref,
                actor_input: &actor_input,
                hostcall_decision: &hostcall_decision,
            })?
        } else {
            None
        };
        let runtime_events = runtime_events_for_step(RuntimeStepInput {
            state: &mut state,
            step,
            admission_decision: &admission_decision,
            step_index: index as u64,
            replay_effect_log,
            replay_effect_index: &mut replay_effect_index,
        })?;
        let mut boundary_runtime_events = Vec::with_capacity(
            runtime_events.len()
                + usize::from(steel_execution_event.is_some())
                + usize::from(wasm_execution_event.is_some()),
        );
        if let Some(steel_execution_event) = steel_execution_event {
            boundary_runtime_events.push(steel_execution_event);
        }
        if let Some(wasm_execution_event) = wasm_execution_event {
            boundary_runtime_events.push(wasm_execution_event);
        }
        boundary_runtime_events.extend(runtime_events.clone());
        events.extend(boundary_runtime_events.clone());
        events.push(actor_output_value(step, hostcall_context, &admission_decision, &boundary_runtime_events)?);
        total_events += events.len() as u64;
        if total_events > budget.max_events {
            return Err(divergence(
                "resource",
                Some(index as u64),
                budget.max_events.to_string(),
                total_events.to_string(),
                "event count exceeds budget",
            ));
        }
        append_effect_entries_from_events(&events, &mut effect_log)?;
        if effect_log.len() as u64 > budget.max_effects {
            return Err(divergence(
                "resource",
                Some(index as u64),
                budget.max_effects.to_string(),
                effect_log.len().to_string(),
                "effect count exceeds budget",
            ));
        }
        let after_state_hash = canonical_hash(&snapshot_value(&state.snapshot()))?;
        observations.push(observation_value(index as u64, step_ref, before_state_hash, after_state_hash, events)?);
    }

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

    let final_state_hash = canonical_hash(&snapshot_value(&state.snapshot()))?;
    let mut usage = BudgetUsage {
        steps: suite.steps.len() as u64,
        effects: effect_log.len() as u64,
        events: total_events,
        report_bytes: 0,
    };
    let mut report_value = super::schema::report_value(ReportValueInput {
        suite,
        suite_ref: suite_ref.clone(),
        initial_state_hash: initial_state_hash.clone(),
        final_state_hash: final_state_hash.clone(),
        policy_gate: policy_gate.clone(),
        capability_gate: capability_gate.clone(),
        budget_gate: budget_gate.clone(),
        observations: observations.clone(),
        effect_log: effect_log.clone(),
        budget: &budget,
        usage: &usage,
    });
    for _ in 0..4 {
        let report_bytes = canonical_bytes(&report_value)?.len() as u64;
        if report_bytes == usage.report_bytes {
            break;
        }
        usage.report_bytes = report_bytes;
        report_value = super::schema::report_value(ReportValueInput {
            suite,
            suite_ref: suite_ref.clone(),
            initial_state_hash: initial_state_hash.clone(),
            final_state_hash: final_state_hash.clone(),
            policy_gate: policy_gate.clone(),
            capability_gate: capability_gate.clone(),
            budget_gate: budget_gate.clone(),
            observations: observations.clone(),
            effect_log: effect_log.clone(),
            budget: &budget,
            usage: &usage,
        });
    }
    usage.report_bytes = canonical_bytes(&report_value)?.len() as u64;
    if usage.report_bytes > budget.max_report_bytes {
        return Err(divergence(
            "resource",
            None,
            budget.max_report_bytes.to_string(),
            usage.report_bytes.to_string(),
            "report byte size exceeds budget",
        ));
    }
    report_value = super::schema::report_value(ReportValueInput {
        suite,
        suite_ref: suite_ref.clone(),
        initial_state_hash: initial_state_hash.clone(),
        final_state_hash: final_state_hash.clone(),
        policy_gate,
        capability_gate,
        budget_gate,
        observations,
        effect_log,
        budget: &budget,
        usage: &usage,
    });
    let report_ref = canonical_hash(&report_value)?;
    Ok(HarnessRun {
        report_value,
        report_ref,
        suite_ref,
        initial_state_hash,
        final_state_hash,
        status: "pass".to_string(),
    })
}

fn validate_actor_registry(suite: &HarnessSuite) -> Result<()> {
    let mut ids = std::collections::BTreeSet::new();
    ensure_supported_actor_executors(&suite.actors)?;
    for actor in &suite.actors {
        ids.insert(actor.id.as_str());
    }
    for step in &suite.steps {
        for actor in actor_ids_for_step(step) {
            if !ids.contains(actor) {
                return Err(MoltenError::invalid_harness(format!("unknown actor {actor} in harness step")));
            }
        }
    }
    Ok(())
}

struct RuntimeStepInput<'a> {
    state: &'a mut RuntimeState,
    step: &'a super::core::CoreStep,
    admission_decision: &'a crate::runtime::AdmissionDecision,
    step_index: u64,
    replay_effect_log: Option<&'a [EffectLogEntry]>,
    replay_effect_index: &'a mut usize,
}

fn runtime_events_for_step(input: RuntimeStepInput<'_>) -> Result<Vec<IOValue>> {
    let RuntimeStepInput {
        state,
        step,
        admission_decision,
        step_index,
        replay_effect_log,
        replay_effect_index,
    } = input;

    if !admission_decision.is_allowed() {
        let turn = state.begin_turn(step);
        return Ok(state
            .rollback_turn(turn, step.primary_actor(), admission_decision.reason())
            .iter()
            .map(event_value)
            .collect());
    }

    let Some(replay_effect_log) = replay_effect_log else {
        return Ok(state.apply_step(step).iter().map(event_value).collect());
    };

    replay_effect_events(state, step, step_index, replay_effect_log, replay_effect_index)
}

fn replay_effect_events(
    state: &mut RuntimeState,
    step: &super::core::CoreStep,
    step_index: u64,
    replay_effect_log: &[EffectLogEntry],
    replay_effect_index: &mut usize,
) -> Result<Vec<IOValue>> {
    let Some(request) = state.begin_effect_for_step(step) else {
        return Ok(state.apply_step(step).iter().map(event_value).collect());
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
    let request_value = event_value(&request);
    let request_hash = canonical_hash(&request_value)?;
    let recorded_request_hash = canonical_hash(&entry.request)?;
    if request_hash != recorded_request_hash {
        return Err(divergence(
            "effect-request",
            Some(step_index),
            recorded_request_hash,
            request_hash,
            "effect request does not match recorded log",
        ));
    }

    let (response_sequence, response_value) = effect_response_sequence_and_value(&entry.response)?;
    let request_sequence = effect_request_sequence(&entry.request)?;
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
    let response_value = event_value(&response);
    let response_hash = canonical_hash(&response_value)?;
    let recorded_response_hash = canonical_hash(&entry.response)?;
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
    Ok(vec![request_value, response_value])
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
