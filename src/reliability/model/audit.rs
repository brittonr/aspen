//! Event-by-event replay of a submitted history against the admitted plan and profile bounds.

/// Borrowed admission inputs shared by every replayed event.
pub(super) struct Context<'a> {
    pub(super) admitted: &'a crate::live_reliability::profile::Admitted,
    pub(super) plan: &'a crate::live_reliability::generator::OperationPlan,
    pub(super) history: &'a super::Document,
}

#[derive(Default)]
pub(super) struct ValidationState<'a> {
    time: u64,
    pub(super) attempts: std::collections::BTreeMap<
        String,
        (&'a crate::live_reliability::generator::PlannedOperation, Option<super::Outcome>),
    >,
    pub(super) logical: std::collections::BTreeMap<String, u32>,
    active: std::collections::BTreeMap<String, String>,
    pub(super) uncertain: u64,
    faults: std::collections::BTreeMap<String, u8>,
    pub(super) applied: u64,
    pub(super) observed: u64,
}

/// Invoke fields checked against the planned operation and retry chain.
struct Invocation<'e> {
    logical_operation_id: &'e String,
    id: &'e String,
    retry_of_attempt: &'e Option<String>,
    process: &'e String,
    object_key: &'e String,
    input: &'e super::Input,
    source_artifact: &'e String,
}

pub(super) fn validate_header(
    admitted: &crate::live_reliability::profile::Admitted,
    history: &super::Document,
) -> Result<(), super::Issue> {
    let profile = admitted.profile();
    let bounds = &history.profile.bounds;
    let attempts = u64::from(profile.operations) * u64::from(profile.evidence.max_attempts_per_operation);
    if history.schema_version != 2
        || history.workload_profile != admitted.profile_ref()
        || history.profile.profile_id != admitted.profile_ref()
        || history.profile.model != "register"
        || history.profile.independent_key_decomposition
        || !crate::live_reliability::profile::is_ref(&history.source_artifact)
        || bounds.max_operations != attempts
        || bounds.max_depth != attempts
        || bounds.max_states != profile.evidence.checker_states
        || bounds.max_branches != profile.evidence.checker_branches
        || bounds.max_memo_bytes != profile.evidence.checker_memo_bytes
        || bounds.max_reductions != profile.evidence.checker_reductions
    {
        return Err(super::Issue::Profile);
    }
    if history.events.len() as u64 > u64::from(profile.evidence.max_events)
        || serde_json::to_vec(history).map_err(|_| super::Issue::Bounds)?.len() as u64 > profile.evidence.max_bytes
    {
        return Err(super::Issue::Bounds);
    }
    Ok(())
}

pub(super) fn validate_event<'a>(
    context: &Context<'a>,
    state: &mut ValidationState<'a>,
    indexed: (usize, &super::Event),
) -> Result<(), super::Issue> {
    let (index, event) = indexed;
    let (event_index, time) = event_clock(event);
    if event_index != index as u64 || time < state.time {
        return Err(super::Issue::Order);
    }
    state.time = time;
    match event {
        super::Event::Invoke {
            logical_operation_id,
            attempt_id: id,
            retry_of_attempt,
            process,
            object_key,
            input,
            source_artifact,
            ..
        } => validate_invoke(context, state, &Invocation {
            logical_operation_id,
            id,
            retry_of_attempt,
            process,
            object_key,
            input,
            source_artifact,
        })?,
        super::Event::Complete {
            attempt_id,
            outcome,
            output,
            finalization,
            ..
        } => {
            let (operation, previous) = state.attempts.get_mut(attempt_id).ok_or(super::Issue::Incomplete)?;
            if previous.is_some() || finalization.is_some() {
                return Err(super::Issue::Outcome);
            }
            if *outcome == super::Outcome::Ok {
                super::validate_output(context.admitted, operation, *output)?;
            } else if output.is_some() {
                return Err(super::Issue::Outcome);
            }
            if *outcome == super::Outcome::Info {
                state.uncertain += 1;
            }
            *previous = Some(*outcome);
            state.active.remove(&operation.process);
        }
        super::Event::Fault {
            fault_attempt_id,
            phase,
            effect_record_ref,
            ..
        } => validate_fault(state, fault_attempt_id, phase, effect_record_ref)?,
        super::Event::Lifecycle { phase, .. } => {
            if phase.is_empty() || phase.len() > 128 {
                return Err(super::Issue::Identity);
            }
        }
    }
    Ok(())
}

/// Producer-recorded `(event_index, controller_time_ns)` of any event kind.
fn event_clock(event: &super::Event) -> (u64, u64) {
    match event {
        super::Event::Invoke {
            event_index,
            controller_time_ns,
            ..
        }
        | super::Event::Complete {
            event_index,
            controller_time_ns,
            ..
        }
        | super::Event::Fault {
            event_index,
            controller_time_ns,
            ..
        }
        | super::Event::Lifecycle {
            event_index,
            controller_time_ns,
            ..
        } => (*event_index, *controller_time_ns),
    }
}

fn validate_invoke<'a>(
    context: &Context<'a>,
    state: &mut ValidationState<'a>,
    invocation: &Invocation<'_>,
) -> Result<(), super::Issue> {
    let operation = context
        .plan
        .operations
        .iter()
        .find(|operation| &operation.id == invocation.logical_operation_id)
        .ok_or(super::Issue::Identity)?;
    let ordinal = state.logical.get(invocation.logical_operation_id).copied().unwrap_or(0);
    if ordinal >= context.admitted.profile().evidence.max_attempts_per_operation {
        return Err(super::Issue::Bounds);
    }
    let previous = ordinal.checked_sub(1).map(|previous| super::attempt_id(operation, previous));
    if invocation.id != &super::attempt_id(operation, ordinal)
        || invocation.retry_of_attempt != &previous
        || invocation.process != &operation.process
        || invocation.object_key != &operation.object_key
        || invocation.input != &super::operation_input(operation)
        || invocation.source_artifact != &context.history.source_artifact
    {
        return Err(super::Issue::Identity);
    }
    if let Some(previous) = previous
        && state.attempts.get(&previous).map(|(_, outcome)| *outcome) != Some(Some(super::Outcome::Info))
    {
        return Err(super::Issue::Retry);
    }
    if state.active.contains_key(invocation.process)
        || state.active.len() as u64 >= u64::from(context.admitted.profile().concurrency)
    {
        return Err(super::Issue::Bounds);
    }
    state.logical.insert(invocation.logical_operation_id.clone(), ordinal + 1);
    state.active.insert(invocation.process.clone(), invocation.id.clone());
    state.attempts.insert(invocation.id.clone(), (operation, None));
    Ok(())
}

fn validate_fault(
    state: &mut ValidationState<'_>,
    id: &str,
    phase: &str,
    reference: &Option<String>,
) -> Result<(), super::Issue> {
    if id.is_empty() || id.len() > 512 {
        return Err(super::Issue::Identity);
    }
    let stage = match phase {
        "selected" => 1,
        "applied" => 2,
        "observed" => 3,
        _ => return Err(super::Issue::Order),
    };
    if state.faults.get(id).copied().unwrap_or(0) + 1 != stage {
        return Err(super::Issue::Order);
    }
    if stage > 1 && !reference.as_deref().is_some_and(crate::live_reliability::profile::is_ref) {
        return Err(super::Issue::Identity);
    }
    if stage == 2 {
        state.applied += 1;
    }
    if stage == 3 {
        state.observed += 1;
    }
    state.faults.insert(id.into(), stage);
    Ok(())
}
