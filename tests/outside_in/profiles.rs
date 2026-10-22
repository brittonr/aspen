//! The no-fault baseline and the four fault profiles over the real stack.

use molten_core::outside_in::*;

use super::client::*;
use super::node::*;
use super::shell::*;

const WALL_CLOCK_MS: u64 = 60_000;
const FAULT_HOLD_MS: u64 = 500;
const STABILIZATION_MS: u64 = 200;
const SEND_JOIN_TIMEOUT_MS: u64 = 10_000;
/// The partitioned attempt gives up inside the hold, before the heal.
const PARTITIONED_JOIN_TIMEOUT_MS: u64 = 1_500;
/// The partition lasts this long from its start, then heals.
const PARTITION_HOLD_MS: u64 = 8_000;

pub(crate) fn budgets() -> ScenarioBudgets {
    ScenarioBudgets {
        max_processes: 3,
        wall_clock_ms: WALL_CLOCK_MS,
        max_retained_output_bytes: 64 * 1024,
        max_client_operations: 4,
    }
}

pub(crate) fn declaration(profile: FaultProfile, fault_hold_ms: u64, expected: &[&str]) -> ScenarioDeclaration {
    ScenarioDeclaration {
        scenario_id: format!("outside-in-{}", profile.as_str()),
        path: ScenarioPath::NodeControl,
        profile,
        boundary: profile.boundary(),
        budgets: budgets(),
        fault_hold_ms,
        stabilization_ms: STABILIZATION_MS,
        expected_processed: expected.iter().map(ToString::to_string).collect(),
    }
}

/// An observation with nothing observed yet.
pub(crate) fn empty_observation() -> ObservedHistory {
    ObservedHistory {
        attempts: Vec::new(),
        checkpoints: Vec::new(),
        listener_endpoints: Vec::new(),
    }
}

/// Run one declared scenario: validate, execute, tear down, and judge.
pub(crate) fn run_scenario(
    declaration: &ScenarioDeclaration,
    body: impl FnOnce(&mut Harness) -> StepResult<ObservedHistory>,
) -> ScenarioResult {
    if !validate_scenario(declaration).is_empty() {
        let unexecuted = ScenarioExecution::Completed(empty_observation());
        return judge_scenario(declaration, &unexecuted, TeardownStatus::Complete);
    }
    let mut harness = match Harness::new(declaration) {
        Ok(harness) => harness,
        Err(stop) => return judge_scenario(declaration, &stop.into_execution(), TeardownStatus::Complete),
    };
    let execution = match body(&mut harness) {
        Ok(observed) => ScenarioExecution::Completed(observed),
        Err(stop) => stop.into_execution(),
    };
    let (teardown, logs) = harness.teardown();
    let result = judge_scenario(declaration, &execution, teardown);
    if !result.is_pass() {
        for (label, log) in logs {
            eprintln!("--- retained output of {label}\n{log}");
        }
    }
    result
}

fn settle_and_read(harness: &mut Harness, receiver: &Receiver, observed: &mut ObservedHistory) -> StepResult<()> {
    harness.hold(std::time::Duration::from_millis(STABILIZATION_MS))?;
    observed.checkpoints.push(read_history(harness, receiver, CheckpointKind::Final)?);
    Ok(())
}

/// Send one request to a fresh listener and let the listener drain it.
fn exchange(
    harness: &mut Harness,
    receiver: &mut Receiver,
    request: &Request,
    fate: ResponseFate,
    observed: &mut ObservedHistory,
) -> StepResult<()> {
    let listener = start_listener(harness, receiver)?;
    observed.listener_endpoints.push(listener.endpoint.clone());
    let send = start_send(harness, receiver, request, &listener.ticket, SEND_JOIN_TIMEOUT_MS)?;
    observed.attempts.push(finish_send(harness, request, send, fate)?);
    finish_listener(harness, &listener)
}

pub(crate) fn baseline_body(harness: &mut Harness) -> StepResult<ObservedHistory> {
    let mut receiver = prepare_receiver(harness)?;
    let request = build_request(harness, &mut receiver, 1)?;
    let mut observed = empty_observation();
    exchange(harness, &mut receiver, &request, ResponseFate::Observed, &mut observed)?;
    settle_and_read(harness, &receiver, &mut observed)?;
    Ok(observed)
}

fn kill_restart_body(harness: &mut Harness) -> StepResult<ObservedHistory> {
    let mut receiver = prepare_receiver(harness)?;
    let first = build_request(harness, &mut receiver, 1)?;
    let second = build_request(harness, &mut receiver, 2)?;
    let mut observed = empty_observation();
    exchange(harness, &mut receiver, &first, ResponseFate::Observed, &mut observed)?;
    let waiting = start_listener(harness, &mut receiver)?;
    observed.listener_endpoints.push(waiting.endpoint.clone());
    harness.kill(listener_handle(&waiting))?;
    let restarted = start_listener(harness, &mut receiver)?;
    observed.listener_endpoints.push(restarted.endpoint.clone());
    observed.checkpoints.push(read_history(harness, &receiver, CheckpointKind::AfterFault)?);
    let send = start_send(harness, &receiver, &second, &restarted.ticket, SEND_JOIN_TIMEOUT_MS)?;
    observed.attempts.push(finish_send(harness, &second, send, ResponseFate::Observed)?);
    finish_listener(harness, &restarted)?;
    settle_and_read(harness, &receiver, &mut observed)?;
    Ok(observed)
}

pub(crate) fn pause_resume_body(harness: &mut Harness) -> StepResult<ObservedHistory> {
    let mut receiver = prepare_receiver(harness)?;
    let request = build_request(harness, &mut receiver, 1)?;
    let mut observed = empty_observation();
    let listener = start_listener(harness, &mut receiver)?;
    observed.listener_endpoints.push(listener.endpoint.clone());
    harness.signal(listener_handle(&listener), "STOP")?;
    let send = start_send(harness, &receiver, &request, &listener.ticket, SEND_JOIN_TIMEOUT_MS)?;
    harness.hold(std::time::Duration::from_millis(FAULT_HOLD_MS))?;
    harness.signal(listener_handle(&listener), "CONT")?;
    observed.attempts.push(finish_send(harness, &request, send, ResponseFate::Observed)?);
    finish_listener(harness, &listener)?;
    settle_and_read(harness, &receiver, &mut observed)?;
    Ok(observed)
}

fn partition_heal_body(harness: &mut Harness) -> StepResult<ObservedHistory> {
    let mut receiver = prepare_receiver(harness)?;
    let request = build_request(harness, &mut receiver, 1)?;
    let mut observed = empty_observation();
    let listener = start_listener(harness, &mut receiver)?;
    observed.listener_endpoints.push(listener.endpoint.clone());
    let forwarder = Forwarder::start(ticket_address(&listener.ticket)?)
        .map_err(|_| ScenarioStop::Unsupported("loopback-forwarder".to_string()))?;
    let forwarded = readdressed_ticket(&listener.ticket, forwarder.front_addr())?;
    forwarder.partition();
    let mut heal_at = hold_deadline(std::time::Duration::from_millis(PARTITION_HOLD_MS))?;
    let partitioned = start_send(harness, &receiver, &request, &forwarded, PARTITIONED_JOIN_TIMEOUT_MS)?;
    observed.attempts.push(finish_send(harness, &request, partitioned, ResponseFate::Observed)?);
    if heal_at.is_expired().map_err(|_| ScenarioStop::Unsupported("supervision-clock".to_string()))? {
        return Err(ScenarioStop::Budget(BudgetKind::WallClock));
    }
    harness.hold_until(&mut heal_at)?;
    forwarder.heal();
    let healed = start_send(harness, &receiver, &request, &forwarded, SEND_JOIN_TIMEOUT_MS)?;
    observed.attempts.push(finish_send(harness, &request, healed, ResponseFate::Observed)?);
    finish_listener(harness, &listener)?;
    drop(forwarder);
    settle_and_read(harness, &receiver, &mut observed)?;
    Ok(observed)
}

fn lost_response_body(harness: &mut Harness) -> StepResult<ObservedHistory> {
    let mut receiver = prepare_receiver(harness)?;
    let request = build_request(harness, &mut receiver, 1)?;
    let mut observed = empty_observation();
    exchange(harness, &mut receiver, &request, ResponseFate::Lost, &mut observed)?;
    exchange(harness, &mut receiver, &request, ResponseFate::Observed, &mut observed)?;
    settle_and_read(harness, &receiver, &mut observed)?;
    Ok(observed)
}

// r[verify molten.outside_in.real_stack]
// r[verify molten.outside_in.observation]
#[test]
fn live_outside_in_no_fault_baseline_agrees() {
    let result = run_scenario(&declaration(FaultProfile::NoFault, 0, &["request-1"]), baseline_body);
    assert_eq!(result.outcome, ScenarioOutcome::Agreement, "{result:?}");
    assert!(result.is_pass());
}

// r[verify molten.outside_in.fault_profiles]
#[test]
fn live_outside_in_kill_restart_keeps_the_committed_request_and_identity() {
    let result =
        run_scenario(&declaration(FaultProfile::KillRestart, 0, &["request-1", "request-2"]), kill_restart_body);
    assert_eq!(result.outcome, ScenarioOutcome::Agreement, "{result:?}");
    assert!(result.is_pass());
}

// r[verify molten.outside_in.fault_profiles]
#[test]
fn live_outside_in_pause_resume_delivers_once_after_resume() {
    let result =
        run_scenario(&declaration(FaultProfile::PauseResume, FAULT_HOLD_MS, &["request-1"]), pause_resume_body);
    assert_eq!(result.outcome, ScenarioOutcome::Agreement, "{result:?}");
    assert!(result.is_pass());
}

// r[verify molten.outside_in.fault_profiles]
#[test]
fn live_outside_in_partition_heal_fails_during_the_partition_and_delivers_once_after_heal() {
    let result =
        run_scenario(&declaration(FaultProfile::PartitionHeal, PARTITION_HOLD_MS, &["request-1"]), partition_heal_body);
    assert_eq!(
        result.outcome,
        ScenarioOutcome::PermittedDivergence(vec![DivergenceReason::RetriedAfterFault]),
        "{result:?}"
    );
    assert!(result.is_pass());
}

// r[verify molten.outside_in.fault_profiles]
#[test]
fn live_outside_in_lost_response_stays_unresolved_and_the_retry_is_suppressed() {
    let result = run_scenario(&declaration(FaultProfile::LostResponse, 0, &["request-1"]), lost_response_body);
    assert_eq!(
        result.outcome,
        ScenarioOutcome::PermittedDivergence(vec![
            DivergenceReason::LostResponseApplied,
            DivergenceReason::DuplicateSuppressed
        ]),
        "{result:?}"
    );
    assert!(result.is_pass());
}
