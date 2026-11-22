//! Fault-stage and recovery admission for external live reliability evidence.

pub(super) fn validate_faults(
    expected: &super::Expected,
    evidence: &super::Observed,
    events: &[serde_json::Value],
) -> Result<(), super::Issue> {
    let profile = &expected.fault.profile;
    crate::live_reliability::fault::admit(profile, &expected.endpoints).map_err(|_| super::Issue::Fault)?;
    let fault_events = events.iter().filter(|event| event["kind"] == "fault").count();
    let (mechanism, duration_ms) = match &profile.fault {
        crate::live_reliability::fault::Mechanism::NoFault => {
            return if expected.fault.effect_ref.is_none() && evidence.faults.is_empty() && fault_events == 0 {
                Ok(())
            } else {
                Err(super::Issue::Fault)
            };
        }
        crate::live_reliability::fault::Mechanism::ProcessRestart { down_ms, .. } => ("process-restart", *down_ms),
        crate::live_reliability::fault::Mechanism::TemporaryPartition { duration_ms, .. } => {
            ("temporary-partition", *duration_ms)
        }
    };
    let [fault] = evidence.faults.as_slice() else {
        return Err(super::Issue::Fault);
    };
    if fault_events != 3
        || fault.mechanism != mechanism
        || expected.fault.effect_ref.as_deref() != Some(fault.effect_ref.as_str())
        || !crate::live_reliability::checker::reference(&fault.effect_ref)
    {
        return Err(super::Issue::Fault);
    }
    validate_fault_stages(fault, events)?;
    let disruption = interval_ns(events, fault.applied, fault.healed).ok_or(super::Issue::Fault)?;
    let recovery = interval_ns(events, fault.healed, fault.recovered).ok_or(super::Issue::Fault)?;
    if !(duration_ms * NS_PER_MS..=profile.heal_deadline_ms * NS_PER_MS).contains(&disruption)
        || recovery > profile.recovery_deadline_ms * NS_PER_MS
    {
        return Err(super::Issue::Fault);
    }
    Ok(())
}

fn validate_fault_stages(fault: &super::FaultObservation, events: &[serde_json::Value]) -> Result<(), super::Issue> {
    if !(fault.selected < fault.applied
        && fault.applied < fault.observed
        && fault.observed < fault.healed
        && fault.healed < fault.recovered)
    {
        return Err(super::Issue::Fault);
    }
    for (phase, index) in [
        ("selected", fault.selected),
        ("applied", fault.applied),
        ("observed", fault.observed),
    ] {
        let event = event_at(events, index).ok_or(super::Issue::Fault)?;
        if event["kind"] != "fault"
            || event["phase"] != phase
            || event["fault_attempt_id"] != fault.id
            || (phase != "selected" && event["effect_record_ref"] != fault.effect_ref)
        {
            return Err(super::Issue::Fault);
        }
    }
    for (phase, index) in [("heal", fault.healed), ("recovery", fault.recovered)] {
        let event = event_at(events, index).ok_or(super::Issue::Fault)?;
        if event["kind"] != "lifecycle" || event["phase"] != phase {
            return Err(super::Issue::Fault);
        }
    }
    Ok(())
}

const NS_PER_MS: u64 = 1_000_000;

fn interval_ns(events: &[serde_json::Value], start: u64, end: u64) -> Option<u64> {
    elapsed_ns(event_at(events, start)?, event_at(events, end)?)
}

fn elapsed_ns(start: &serde_json::Value, end: &serde_json::Value) -> Option<u64> {
    end["controller_time_ns"].as_u64()?.checked_sub(start["controller_time_ns"].as_u64()?)
}

pub(super) fn validate_recovery(
    expected: &super::Expected,
    evidence: &super::Observed,
    events: &[serde_json::Value],
) -> Result<(), super::Issue> {
    let mut endpoints = std::collections::BTreeSet::new();
    let mut attempts = std::collections::BTreeSet::new();
    let last_recovery = evidence.faults.iter().map(|f| f.recovered).max();
    for read in &evidence.recovery {
        if !endpoints.insert(read.endpoint.clone())
            || !attempts.insert(&read.attempt_id)
            || !read.ready
            || !read.stable_membership
            || read.elapsed_ms > expected.max_recovery_ms
        {
            return Err(super::Issue::Recovery);
        }
        validate_final_read(expected, events, last_recovery, read)?;
    }
    if endpoints != expected.endpoints {
        return Err(super::Issue::Recovery);
    }
    if evidence.report["verdict"] == "valid" && evidence.recovery.windows(2).any(|r| r[0].value != r[1].value) {
        return Err(super::Issue::Recovery);
    }
    Ok(())
}

/// One endpoint's final read must follow fresh readiness and stable membership after the last
/// recovery.
fn validate_final_read(
    expected: &super::Expected,
    events: &[serde_json::Value],
    last_recovery: Option<u64>,
    read: &super::FinalRead,
) -> Result<(), super::Issue> {
    let invoke = events
        .iter()
        .find(|e| e["kind"] == "invoke" && e["attempt_id"] == read.attempt_id)
        .ok_or(super::Issue::Recovery)?;
    let complete = events
        .iter()
        .find(|e| e["kind"] == "complete" && e["attempt_id"] == read.attempt_id)
        .ok_or(super::Issue::Recovery)?;
    let final_start = expected.plan.operations.len() - expected.endpoints.len();
    let operation = expected.plan.operations[final_start..]
        .iter()
        .find(|op| invoke["logical_operation_id"] == op.id)
        .ok_or(super::Issue::Recovery)?;
    let ready = event_at(events, read.ready_event).ok_or(super::Issue::Recovery)?;
    let membership = event_at(events, read.stable_membership_event).ok_or(super::Issue::Recovery)?;
    let invocation = invoke["event_index"].as_u64().ok_or(super::Issue::Recovery)?;
    if ready["kind"] != "lifecycle"
        || ready["phase"] != "ready"
        || membership["kind"] != "lifecycle"
        || membership["phase"] != "stable-membership"
        || read.ready_event >= invocation
        || read.stable_membership_event >= invocation
        || read.stable_membership_observations < expected.admitted.profile().recovery.stable_membership_observations
        || last_recovery.is_some_and(|i| read.ready_event <= i || read.stable_membership_event <= i)
    {
        return Err(super::Issue::Recovery);
    }
    let fault_policy = &expected.fault.profile;
    let mut samples = stable_membership_observations(events, read.stable_membership_event, last_recovery);
    let first_sample = samples.next().ok_or(super::Issue::Recovery)?;
    if samples.count() as u64 + 1 < u64::from(read.stable_membership_observations)
        || elapsed_ns(first_sample, membership).is_none_or(|span| span < fault_policy.stable_membership_ms * NS_PER_MS)
    {
        return Err(super::Issue::Recovery);
    }
    let elapsed = elapsed_ns(ready, complete).ok_or(super::Issue::Recovery)?;
    let final_read_start = if read.ready_event > read.stable_membership_event {
        ready
    } else {
        membership
    };
    if elapsed / NS_PER_MS != read.elapsed_ms
        || elapsed > expected.max_recovery_ms * NS_PER_MS
        || elapsed > expected.admitted.profile().recovery.timeout_ms * NS_PER_MS
        || elapsed_ns(final_read_start, complete)
            .is_none_or(|span| span > fault_policy.final_read_deadline_ms * NS_PER_MS)
    {
        return Err(super::Issue::Recovery);
    }
    if invoke["input"]["function"] != "read"
        || operation.endpoint != read.endpoint
        || complete["outcome"] != "ok"
        || complete["output"] != read.value
        || last_recovery.is_some_and(|index| invoke["event_index"].as_u64().is_none_or(|i| i <= index))
    {
        return Err(super::Issue::Recovery);
    }
    Ok(())
}

/// Stable-membership samples in the fresh observation interval, not proof of continuity.
fn stable_membership_observations(
    events: &[serde_json::Value],
    through: u64,
    last_recovery: Option<u64>,
) -> impl Iterator<Item = &serde_json::Value> {
    events
        .iter()
        .zip(0_u64..)
        .take_while(move |(_, index)| *index <= through)
        .filter(move |(event, index)| {
            event["kind"] == "lifecycle"
                && event["phase"] == "stable-membership"
                && last_recovery.is_none_or(|recovery| *index > recovery)
        })
        .map(|(event, _)| event)
}

/// Event lookup by producer-recorded index; indices beyond the platform address space are absent.
fn event_at(events: &[serde_json::Value], index: u64) -> Option<&serde_json::Value> {
    usize::try_from(index).ok().and_then(|index| events.get(index))
}
