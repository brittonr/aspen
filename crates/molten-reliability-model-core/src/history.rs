// r[impl molten.consensus.live_operation_history]
//! Public client projection; no internal consensus observations enter model outcomes.

mod audit;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublicOutcome {
    Acknowledged(Option<crate::generator::Value>),
    PreEffectDenied,
    Timeout,
    Disconnected,
    LostResponse,
    MalformedResponse,
    Pending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Ok,
    Fail,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "function", rename_all = "lowercase", deny_unknown_fields)]
pub enum Input {
    Read,
    Write { value: crate::generator::Value },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum Event {
    Invoke {
        event_index: u64,
        logical_operation_id: alloc::string::String,
        attempt_id: alloc::string::String,
        retry_of_attempt: Option<alloc::string::String>,
        process: alloc::string::String,
        object_key: alloc::string::String,
        input: Input,
        controller_time_ns: u64,
        source_artifact: alloc::string::String,
    },
    Complete {
        event_index: u64,
        attempt_id: alloc::string::String,
        outcome: Outcome,
        output: Option<crate::generator::Value>,
        controller_time_ns: u64,
        finalization: Option<alloc::string::String>,
    },
    Fault {
        event_index: u64,
        fault_attempt_id: alloc::string::String,
        phase: alloc::string::String,
        effect_record_ref: Option<alloc::string::String>,
        controller_time_ns: u64,
    },
    Lifecycle {
        event_index: u64,
        phase: alloc::string::String,
        controller_time_ns: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckerBounds {
    pub max_operations: u64,
    pub max_states: u64,
    pub max_branches: u64,
    pub max_depth: u64,
    pub max_memo_bytes: u64,
    pub max_reductions: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub profile_id: alloc::string::String,
    pub model: alloc::string::String,
    pub independent_key_decomposition: bool,
    pub bounds: CheckerBounds,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Completeness {
    pub invocations: u64,
    pub completions: u64,
    pub pending: u64,
    pub applied_faults: u64,
    pub observed_faults: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub schema_version: u32,
    pub workload_profile: alloc::string::String,
    pub source_artifact: alloc::string::String,
    pub profile: Profile,
    pub completeness: Completeness,
    pub events: alloc::vec::Vec<Event>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptObservation {
    pub ordinal: u32,
    pub event_index: u64,
    pub started_ns: u64,
    pub completed_ns: u64,
    pub source_artifact: alloc::string::String,
    pub outcome: PublicOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Issue {
    Profile,
    Bounds,
    Plan,
    Identity,
    Order,
    Retry,
    Outcome,
    Incomplete,
    Output,
}

impl Issue {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Profile => "profile",
            Self::Bounds => "bounds",
            Self::Plan => "plan",
            Self::Identity => "identity",
            Self::Order => "order",
            Self::Retry => "retry",
            Self::Outcome => "outcome",
            Self::Incomplete => "incomplete",
            Self::Output => "value",
        }
    }
}

impl core::fmt::Display for Issue {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "register history denied: {}", self.as_str())
    }
}

impl core::error::Error for Issue {}

pub fn attempt_id(operation: &crate::generator::PlannedOperation, ordinal: u32) -> alloc::string::String {
    alloc::format!("{}:attempt:{ordinal}", operation.id)
}

pub fn project_attempt(
    admitted: &crate::profile::Admitted,
    operation: &crate::generator::PlannedOperation,
    observation: &AttemptObservation,
) -> Result<alloc::vec::Vec<Event>, Issue> {
    if observation.ordinal >= admitted.profile().evidence.max_attempts_per_operation
        || observation.completed_ns < observation.started_ns
    {
        return Err(Issue::Bounds);
    }
    let expected = crate::generator::generate(admitted).map_err(|_| Issue::Plan)?;
    if !expected.operations.contains(operation) {
        return Err(Issue::Plan);
    }
    if !crate::profile::is_ref(&observation.source_artifact) {
        return Err(Issue::Identity);
    }
    let id = attempt_id(operation, observation.ordinal);
    let mut events = alloc::vec![Event::Invoke {
        event_index: observation.event_index,
        logical_operation_id: operation.id.clone(),
        attempt_id: id.clone(),
        retry_of_attempt: observation.ordinal.checked_sub(1).map(|previous| attempt_id(operation, previous)),
        process: operation.process.clone(),
        object_key: operation.object_key.clone(),
        input: operation_input(operation),
        controller_time_ns: observation.started_ns,
        source_artifact: observation.source_artifact.clone(),
    }];
    let (outcome, output) = match &observation.outcome {
        PublicOutcome::Pending => return Ok(events),
        PublicOutcome::Acknowledged(value) => {
            validate_output(admitted, operation, *value)?;
            (Outcome::Ok, *value)
        }
        PublicOutcome::PreEffectDenied => (Outcome::Fail, None),
        PublicOutcome::Timeout
        | PublicOutcome::Disconnected
        | PublicOutcome::LostResponse
        | PublicOutcome::MalformedResponse => (Outcome::Info, None),
    };
    events.push(Event::Complete {
        event_index: observation.event_index.checked_add(1).ok_or(Issue::Bounds)?,
        attempt_id: id,
        outcome,
        output,
        controller_time_ns: observation.completed_ns,
        finalization: None,
    });
    Ok(events)
}

fn operation_input(operation: &crate::generator::PlannedOperation) -> Input {
    match operation.function {
        crate::generator::Function::Read => Input::Read,
        crate::generator::Function::Write => Input::Write { value: operation.value },
    }
}

fn validate_output(
    admitted: &crate::profile::Admitted,
    operation: &crate::generator::PlannedOperation,
    output: Option<crate::generator::Value>,
) -> Result<(), Issue> {
    match operation.function {
        crate::generator::Function::Write if output.is_none() => Ok(()),
        crate::generator::Function::Write => Err(Issue::Output),
        crate::generator::Function::Read => match output {
            Some(crate::generator::Value::Null) => Ok(()),
            Some(crate::generator::Value::Integer { value }) if admitted.profile().values.contains(&value) => Ok(()),
            Some(crate::generator::Value::Integer { .. }) | None => Err(Issue::Output),
        },
    }
}

/// Replays `history` against the admitted plan and profile bounds.
///
/// `encoded_len` measures the submitted document in its publication encoding; the shell supplies
/// the `serde_json` byte length. It runs only after the header identity and bounds checks passed,
/// exactly where the evidence byte bound is enforced.
pub fn validate<F>(
    admitted: &crate::profile::Admitted,
    plan: &crate::generator::OperationPlan,
    history: &Document,
    encoded_len: F,
) -> Result<(), Issue>
where
    F: FnOnce(&Document) -> Result<u64, Issue>,
{
    crate::generator::validate_plan(admitted, plan).map_err(|_| Issue::Plan)?;
    audit::validate_header(admitted, history, encoded_len)?;
    let context = audit::Context {
        admitted,
        plan,
        history,
    };
    let mut state = audit::ValidationState::default();
    for (index, event) in history.events.iter().enumerate() {
        audit::validate_event(&context, &mut state, (index, event))?;
    }
    let invocations = state.attempts.len() as u64;
    let completions = state.attempts.values().filter(|(_, outcome)| outcome.is_some()).count() as u64;
    let pending = invocations - completions;
    if state.logical.len() != plan.operations.len()
        || history.completeness.invocations != invocations
        || history.completeness.completions != completions
        || history.completeness.pending != pending
    {
        return Err(Issue::Incomplete);
    }
    if history.completeness.applied_faults != state.applied || history.completeness.observed_faults != state.observed {
        return Err(Issue::Incomplete);
    }
    if state.uncertain + pending > u64::from(admitted.profile().evidence.max_uncertain) {
        return Err(Issue::Bounds);
    }
    Ok(())
}
