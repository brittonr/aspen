use preserves::IOValue;

use super::runner::run_suite_with_effect_log;
use super::schema::EventBoundary;
use super::schema::effect_log_from_observations;
use super::schema::event_boundary;
use super::schema::parse_admission_decision_event;
use super::schema::parse_report;
use super::schema::parse_suite;
use super::schema::validate_actor_registry_evidence;
use super::schema::validate_admission_evidence;
use super::schema::validate_budget_fixture_evidence;
use super::schema::validate_budget_gate_evidence;
use super::schema::validate_capability_gate_evidence;
use super::schema::validate_executor_preflight_evidence;
use super::schema::validate_hostcall_evidence;
use super::schema::validate_policy_gate_evidence;
use super::schema::validate_runtime_predicate_evidence;
use crate::error::HarnessDivergence;
use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::canonical_bytes;
mod compare;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayOutcome {
    pub expected_report_ref: String,
    pub actual_report_ref: String,
    pub final_state_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportValidation {
    pub report_ref: String,
    pub suite_ref: String,
    pub final_state_hash: String,
    pub observations: usize,
}

pub fn validate_report_value(report_value: &IOValue) -> Result<ReportValidation> {
    let report = parse_report(report_value)?;
    let suite = parse_suite(&report.suite_value)?;
    validate_actor_registry_evidence(&suite, &report.observations)?;
    validate_budget_fixture_evidence(&suite)?;
    validate_budget_gate_evidence(&suite, report.budget_gate.as_ref())?;
    validate_policy_gate_evidence(&suite, report.policy_gate.as_ref())?;
    validate_capability_gate_evidence(&suite, report.capability_gate.as_ref())?;
    let policy_gate = report
        .policy_gate
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("missing policy gate evidence"))?;
    let capability_gate = report
        .capability_gate
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("missing capability gate evidence"))?;
    let budget_gate = report
        .budget_gate
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("missing budget gate evidence"))?;
    validate_admission_evidence(&suite, &report.observations, capability_gate)?;
    validate_executor_preflight_evidence(&suite, &report.observations, report.executor_preflights.as_ref())?;
    validate_runtime_predicate_evidence(&suite, &report.observations)?;
    validate_hostcall_evidence(&suite, &report.observations, policy_gate, capability_gate, budget_gate)?;
    let observed_effect_log = effect_log_from_observations(&report.observations)?;
    if observed_effect_log != report.effect_log {
        return Err(MoltenError::invalid_harness("effect log does not match observed effect request/response records"));
    }
    let event_count: u64 = report.observations.iter().map(|observation| observation.events.len() as u64).sum();
    let report_bytes = canonical_bytes(report_value)?.len() as u64;
    let usage = &report.budget.usage;
    let limits = &report.budget.limits;
    if usage.steps != report.observations.len() as u64 {
        return Err(MoltenError::invalid_harness("budget step usage does not match observations"));
    }
    if usage.effects != report.effect_log.len() as u64 {
        return Err(MoltenError::invalid_harness("budget effect usage does not match effect log"));
    }
    if usage.events != event_count {
        return Err(MoltenError::invalid_harness("budget event usage does not match observations"));
    }
    if usage.report_bytes != report_bytes {
        return Err(MoltenError::invalid_harness("budget report byte usage does not match canonical report bytes"));
    }
    if usage.steps > limits.max_steps
        || usage.effects > limits.max_effects
        || usage.events > limits.max_events
        || usage.report_bytes > limits.max_report_bytes
    {
        return Err(MoltenError::invalid_harness("budget usage exceeds declared limits"));
    }
    Ok(ReportValidation {
        report_ref: report.report_ref,
        suite_ref: report.suite_ref,
        final_state_hash: report.final_state_hash,
        observations: report.observations.len(),
    })
}

pub fn replay_report_value(report_value: &IOValue) -> Result<ReplayOutcome> {
    let expected = parse_report(report_value)?;
    let actual_run = run_suite_with_effect_log(&parse_suite(&expected.suite_value)?, &expected.effect_log)?;
    let actual = parse_report(&actual_run.report_value)?;
    compare::report_start(&expected, &actual)?;
    compare::observations(&expected.observations, &actual.observations)?;
    compare::report_end(&expected, &actual)?;
    Ok(ReplayOutcome {
        expected_report_ref: expected.report_ref,
        actual_report_ref: actual.report_ref,
        final_state_hash: actual.final_state_hash,
    })
}

pub fn report_summary(report_value: &IOValue) -> Result<String> {
    let report = parse_report(report_value)?;
    Ok(format!(
        "report {}\nstatus={}\nreplay_status={}\nprofile={}\nsuite={}\ninitial_state={}\nfinal_state={}\nobservations={}\neffects={}\nevents={}\nreport_bytes={}",
        report.report_ref,
        report.status,
        report.replay_status,
        report.profile,
        report.suite_ref,
        report.initial_state_hash,
        report.final_state_hash,
        report.observations.len(),
        report.effect_log.len(),
        report.budget.usage.events,
        report.budget.usage.report_bytes
    ))
}

fn event_divergence_kind(expected: &IOValue, actual: &IOValue) -> &'static str {
    if is_capability_decision_divergence(expected, actual) {
        return "capability-decision";
    }
    match (event_boundary(expected), event_boundary(actual)) {
        (EventBoundary::EffectRequest, _) | (_, EventBoundary::EffectRequest) => "effect-request",
        (EventBoundary::EffectResponse, _) | (_, EventBoundary::EffectResponse) => "effect-response",
        (EventBoundary::PolicyDecision, _) | (_, EventBoundary::PolicyDecision) => "policy-decision",
        (EventBoundary::ActorInput, _) | (_, EventBoundary::ActorInput) => "actor-input",
        (EventBoundary::HostcallRequest, _) | (_, EventBoundary::HostcallRequest) => "hostcall-request",
        (EventBoundary::HostcallDecision, _) | (_, EventBoundary::HostcallDecision) => "hostcall-decision",
        (EventBoundary::ActorOutput, _) | (_, EventBoundary::ActorOutput) => "actor-output",
        (EventBoundary::SteelExecution, _) | (_, EventBoundary::SteelExecution) => "steel-execution",
        (EventBoundary::WasmExecution, _) | (_, EventBoundary::WasmExecution) => "wasm-execution",
        (EventBoundary::RuntimePredicate, _) | (_, EventBoundary::RuntimePredicate) => "runtime-predicate",
        (EventBoundary::Trace, EventBoundary::Trace) => "trace",
    }
}

fn is_capability_decision_divergence(expected: &IOValue, actual: &IOValue) -> bool {
    let (Ok(expected), Ok(actual)) = (parse_admission_decision_event(expected), parse_admission_decision_event(actual))
    else {
        return false;
    };
    expected.authority != actual.authority
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
