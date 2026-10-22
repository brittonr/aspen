use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::*;

/// The first way an observed history departs from the contract.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum HistoryMismatch {
    /// Listener runs on the same storage reported different endpoints.
    ListenerIdentityChanged,
    /// A lost response was reported as anything but unresolved.
    InventedClientOutcome(String),
    /// A request processed at an earlier read is gone at a later one.
    StateReset(String),
    /// The receiver accepted more than one envelope for one request.
    DuplicateAcceptedDelivery(String),
    ExpectedNotProcessed(String),
    ProcessingDenied(String),
    UnexpectedProcessed(String),
    PendingAfterStabilization(String),
}

/// Why an observed history differs from the client's view within the contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DivergenceReason {
    /// The client's response was lost but the receiver processed the request.
    LostResponseApplied,
    /// An attempt failed during the fault and a later attempt succeeded.
    RetriedAfterFault,
    /// The client sent a request more than once and the receiver accepted it once.
    DuplicateSuppressed,
}

/// An observation the judgment needs but the harness did not produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MissingObservation {
    FinalHistory,
    AfterFaultHistory,
    ClientAttempts,
    ListenerRuns,
}

/// The typed outcome of one scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScenarioOutcome {
    Agreement,
    PermittedDivergence(Vec<DivergenceReason>),
    Failure(HistoryMismatch),
    Incomplete(MissingObservation),
    Unsupported(String),
    BudgetExhausted(BudgetKind),
    StartupFailed(String),
    DeclarationRejected(Vec<ScenarioIssue>),
}

/// The judged result of one scenario, with the track's non-claims.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioResult {
    pub scenario_id: String,
    pub profile: FaultProfile,
    pub outcome: ScenarioOutcome,
    pub teardown: TeardownStatus,
    pub non_claims: [&'static str; 3],
}

impl ScenarioResult {
    /// The histories agree, or differ only within the contract, and the
    /// harness released everything it owned.
    pub fn is_pass(&self) -> bool {
        let is_agreeing = matches!(self.outcome, ScenarioOutcome::Agreement | ScenarioOutcome::PermittedDivergence(_));
        is_agreeing && self.teardown == TeardownStatus::Complete
    }
}

/// Judge one scenario execution against its declaration.
// r[impl molten.outside_in.observation]
// r[impl molten.outside_in.fault_profiles]
// r[impl molten.outside_in.validation]
pub fn judge_scenario(
    declaration: &ScenarioDeclaration,
    execution: &ScenarioExecution,
    teardown: TeardownStatus,
) -> ScenarioResult {
    let issues = validate_scenario(declaration);
    let outcome = if issues.is_empty() {
        execution_outcome(declaration, execution)
    } else {
        ScenarioOutcome::DeclarationRejected(issues)
    };
    ScenarioResult {
        scenario_id: declaration.scenario_id.clone(),
        profile: declaration.profile,
        outcome,
        teardown,
        non_claims: OUTSIDE_IN_NON_CLAIMS,
    }
}

fn execution_outcome(declaration: &ScenarioDeclaration, execution: &ScenarioExecution) -> ScenarioOutcome {
    match execution {
        ScenarioExecution::Completed(observed) => history_outcome(declaration, observed),
        ScenarioExecution::StartupFailed { process } => ScenarioOutcome::StartupFailed(process.clone()),
        ScenarioExecution::BudgetExhausted(kind) => ScenarioOutcome::BudgetExhausted(*kind),
        ScenarioExecution::Unsupported { behavior } => ScenarioOutcome::Unsupported(behavior.clone()),
    }
}

fn history_outcome(declaration: &ScenarioDeclaration, observed: &ObservedHistory) -> ScenarioOutcome {
    let is_over_budget =
        u32::try_from(observed.attempts.len()).map_or(true, |count| count > declaration.budgets.max_client_operations);
    if is_over_budget {
        return ScenarioOutcome::BudgetExhausted(BudgetKind::ClientOperations);
    }
    let final_history = match required_observations(declaration, observed) {
        Ok(history) => history,
        Err(missing) => return ScenarioOutcome::Incomplete(missing),
    };
    if let Some(mismatch) = first_mismatch(declaration, observed, final_history) {
        return ScenarioOutcome::Failure(mismatch);
    }
    let reasons = divergence_reasons(observed, final_history);
    if reasons.is_empty() {
        ScenarioOutcome::Agreement
    } else {
        ScenarioOutcome::PermittedDivergence(reasons)
    }
}

fn required_observations<'a>(
    declaration: &ScenarioDeclaration,
    observed: &'a ObservedHistory,
) -> Result<&'a ReceiverHistory, MissingObservation> {
    if observed.attempts.is_empty() {
        return Err(MissingObservation::ClientAttempts);
    }
    let required_runs = if declaration.profile == FaultProfile::KillRestart {
        2
    } else {
        1
    };
    if observed.listener_endpoints.len() < required_runs {
        return Err(MissingObservation::ListenerRuns);
    }
    if declaration.profile == FaultProfile::KillRestart && checkpoint(observed, CheckpointKind::AfterFault).is_none() {
        return Err(MissingObservation::AfterFaultHistory);
    }
    checkpoint(observed, CheckpointKind::Final).ok_or(MissingObservation::FinalHistory)
}

/// The last successful read of the given kind. A failed read of that kind
/// leaves the observation missing.
fn checkpoint(observed: &ObservedHistory, kind: CheckpointKind) -> Option<&ReceiverHistory> {
    observed.checkpoints.iter().rev().find(|checkpoint| checkpoint.kind == kind)?.receiver.as_ref()
}

fn first_mismatch(
    declaration: &ScenarioDeclaration,
    observed: &ObservedHistory,
    final_history: &ReceiverHistory,
) -> Option<HistoryMismatch> {
    if observed.listener_endpoints.windows(2).any(|pair| pair[0] != pair[1]) {
        return Some(HistoryMismatch::ListenerIdentityChanged);
    }
    if let Some(attempt) = observed
        .attempts
        .iter()
        .find(|attempt| attempt.fate == ResponseFate::Lost && attempt.outcome != ClientOutcome::Unresolved)
    {
        return Some(HistoryMismatch::InventedClientOutcome(attempt.request_ref.clone()));
    }
    state_reset(observed)
        .or_else(|| duplicate_acceptance(final_history))
        .or_else(|| expected_mismatch(declaration, final_history))
}

fn state_reset(observed: &ObservedHistory) -> Option<HistoryMismatch> {
    let mut earlier = BTreeSet::new();
    for history in observed.checkpoints.iter().filter_map(|checkpoint| checkpoint.receiver.as_ref()) {
        let current = processed_set(history);
        if let Some(lost) = earlier.difference(&current).next() {
            return Some(HistoryMismatch::StateReset((*lost).to_string()));
        }
        earlier = current;
    }
    None
}

fn duplicate_acceptance(history: &ReceiverHistory) -> Option<HistoryMismatch> {
    let mut accepted = BTreeMap::<&str, BTreeSet<&str>>::new();
    for delivery in history.deliveries.iter().filter(|delivery| delivery.is_accepted) {
        accepted.entry(delivery.request_ref.as_str()).or_default().insert(delivery.envelope_ref.as_str());
    }
    accepted
        .into_iter()
        .find(|(_, envelopes)| envelopes.len() > 1)
        .map(|(request, _)| HistoryMismatch::DuplicateAcceptedDelivery(request.to_string()))
}

fn expected_mismatch(declaration: &ScenarioDeclaration, history: &ReceiverHistory) -> Option<HistoryMismatch> {
    let processed = history
        .processed
        .iter()
        .map(|processed| (processed.request_ref.as_str(), processed.is_passed))
        .collect::<BTreeMap<_, _>>();
    let expected = declaration.expected_processed.iter().map(String::as_str).collect::<BTreeSet<_>>();
    for request in &expected {
        match processed.get(request) {
            None => return Some(HistoryMismatch::ExpectedNotProcessed((*request).to_string())),
            Some(false) => return Some(HistoryMismatch::ProcessingDenied((*request).to_string())),
            Some(true) => {}
        }
    }
    if let Some(unexpected) = processed.keys().find(|request| !expected.contains(*request)) {
        return Some(HistoryMismatch::UnexpectedProcessed((*unexpected).to_string()));
    }
    history
        .pending_requests
        .iter()
        .min()
        .map(|pending| HistoryMismatch::PendingAfterStabilization(pending.clone()))
}

fn divergence_reasons(observed: &ObservedHistory, history: &ReceiverHistory) -> Vec<DivergenceReason> {
    let processed = processed_set(history);
    let mut attempts_by_request = BTreeMap::<&str, Vec<&ClientAttempt>>::new();
    for attempt in &observed.attempts {
        attempts_by_request.entry(attempt.request_ref.as_str()).or_default().push(attempt);
    }
    let mut reasons = BTreeSet::new();
    for (request, attempts) in attempts_by_request {
        if !processed.contains(request) {
            continue;
        }
        if attempts.iter().any(|attempt| attempt.fate == ResponseFate::Lost) {
            reasons.insert(DivergenceReason::LostResponseApplied);
        }
        if attempts.iter().any(|attempt| attempt.outcome == ClientOutcome::Failed) {
            reasons.insert(DivergenceReason::RetriedAfterFault);
        }
        let possibly_delivered = attempts.iter().filter(|attempt| attempt.outcome != ClientOutcome::Failed).count();
        if possibly_delivered > 1 {
            reasons.insert(DivergenceReason::DuplicateSuppressed);
        }
    }
    reasons.into_iter().collect()
}

fn processed_set(history: &ReceiverHistory) -> BTreeSet<&str> {
    history.processed.iter().map(|processed| processed.request_ref.as_str()).collect()
}
