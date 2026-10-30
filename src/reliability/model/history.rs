// r[impl molten.consensus.live_operation_history]
//! Public client projection; no internal consensus observations enter model outcomes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublicOutcome {
    Acknowledged(Option<crate::live_reliability::generator::Value>),
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
    Write {
        value: crate::live_reliability::generator::Value,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum Event {
    Invoke {
        event_index: u64,
        logical_operation_id: String,
        attempt_id: String,
        retry_of_attempt: Option<String>,
        process: String,
        object_key: String,
        input: Input,
        controller_time_ns: u64,
        source_artifact: String,
    },
    Complete {
        event_index: u64,
        attempt_id: String,
        outcome: Outcome,
        output: Option<crate::live_reliability::generator::Value>,
        controller_time_ns: u64,
        finalization: Option<String>,
    },
    Fault {
        event_index: u64,
        fault_attempt_id: String,
        phase: String,
        effect_record_ref: Option<String>,
        controller_time_ns: u64,
    },
    Lifecycle {
        event_index: u64,
        phase: String,
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
    pub profile_id: String,
    pub model: String,
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
    pub workload_profile: String,
    pub source_artifact: String,
    pub profile: Profile,
    pub completeness: Completeness,
    pub events: Vec<Event>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptObservation {
    pub ordinal: u32,
    pub event_index: u64,
    pub started_ns: u64,
    pub completed_ns: u64,
    pub source_artifact: String,
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
    Value,
}

pub fn attempt_id(operation: &crate::live_reliability::generator::PlannedOperation, ordinal: u32) -> String {
    format!("{}:attempt:{ordinal}", operation.id)
}

pub fn project_attempt(
    admitted: &crate::live_reliability::profile::Admitted,
    operation: &crate::live_reliability::generator::PlannedOperation,
    observation: &AttemptObservation,
) -> Result<Vec<Event>, Issue> {
    if observation.ordinal >= admitted.profile().evidence.max_attempts_per_operation
        || observation.completed_ns < observation.started_ns
    {
        return Err(Issue::Bounds);
    }
    let expected = crate::live_reliability::generator::generate(admitted).map_err(|_| Issue::Plan)?;
    if !expected.operations.contains(operation) {
        return Err(Issue::Plan);
    }
    if !crate::live_reliability::profile::is_ref(&observation.source_artifact) {
        return Err(Issue::Identity);
    }
    let id = attempt_id(operation, observation.ordinal);
    let mut events = vec![Event::Invoke {
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

fn operation_input(operation: &crate::live_reliability::generator::PlannedOperation) -> Input {
    match operation.function {
        crate::live_reliability::generator::Function::Read => Input::Read,
        crate::live_reliability::generator::Function::Write => Input::Write { value: operation.value },
    }
}

fn validate_output(
    admitted: &crate::live_reliability::profile::Admitted,
    operation: &crate::live_reliability::generator::PlannedOperation,
    output: Option<crate::live_reliability::generator::Value>,
) -> Result<(), Issue> {
    match operation.function {
        crate::live_reliability::generator::Function::Write if output.is_none() => Ok(()),
        crate::live_reliability::generator::Function::Write => Err(Issue::Value),
        crate::live_reliability::generator::Function::Read => match output {
            Some(crate::live_reliability::generator::Value::Null) => Ok(()),
            Some(crate::live_reliability::generator::Value::Integer { value })
                if admitted.profile().values.contains(&value) =>
            {
                Ok(())
            }
            Some(crate::live_reliability::generator::Value::Integer { .. }) | None => Err(Issue::Value),
        },
    }
}

pub fn validate(
    admitted: &crate::live_reliability::profile::Admitted,
    plan: &crate::live_reliability::generator::OperationPlan,
    history: &Document,
) -> Result<(), Issue> {
    crate::live_reliability::generator::validate_plan(admitted, plan).map_err(|_| Issue::Plan)?;
    audit::validate_header(admitted, history)?;
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

#[path = "audit.rs"]
mod audit;
