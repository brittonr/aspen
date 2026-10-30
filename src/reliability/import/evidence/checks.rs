//! Field-level admission checks for external live reliability evidence; each failure maps to one
//! `Issue`.

pub(super) fn validate_identity(expected: &super::Expected, evidence: &super::Observed) -> Result<(), super::Issue> {
    let profile = expected.admitted.profile();
    if expected.endpoints != profile.endpoints.iter().cloned().collect()
        || expected.max_events as u64 > u64::from(profile.evidence.max_events)
        || expected.max_witness_bytes as u64 > profile.evidence.max_bytes
        || expected.max_recovery_ms > profile.recovery.timeout_ms
        || expected.identities.get("generator") != Some(&profile.generator_ref)
        || expected.identities.get("adapter") != Some(&profile.adapter_ref)
        || expected.identities.get("model") != Some(&profile.model_ref)
    {
        return Err(super::Issue::Identity);
    }
    if !(1..=4096).contains(&expected.max_events)
        || !(1..=8_388_608).contains(&expected.max_witness_bytes)
        || !(1..=120_000).contains(&expected.max_recovery_ms)
        || expected.endpoints.is_empty()
        || expected.endpoints.len() > 64
        || evidence.faults.len() > 64
    {
        return Err(super::Issue::Bounds);
    }
    if expected.identities != evidence.identities
        || expected.identities.len() > 32
        || super::IDENTITIES
            .iter()
            .any(|key| !expected.identities.get(*key).is_some_and(|v| crate::live_reliability::checker::reference(v)))
        || evidence.profile_ref != expected.admitted.profile_ref()
        || expected.identities.get("profile") != Some(&evidence.profile_ref)
        || expected.identities.get("plan") != Some(&evidence.plan_ref)
    {
        return Err(super::Issue::Identity);
    }
    let plan = serde_json::to_value(&expected.plan).map_err(|_| super::Issue::Plan)?;
    if crate::live_reliability::checker::canonical_ref(&plan).map_err(|_| super::Issue::Plan)? != evidence.plan_ref {
        return Err(super::Issue::Plan);
    }
    Ok(())
}

pub(super) fn validate_faults(evidence: &super::Observed, events: &[serde_json::Value]) -> Result<(), super::Issue> {
    let mut ids = std::collections::BTreeSet::new();
    for fault in &evidence.faults {
        if !ids.insert(&fault.id)
            || !matches!(fault.mechanism.as_str(), "process-restart" | "temporary-partition")
            || !crate::live_reliability::checker::reference(&fault.effect_ref)
            || !(fault.selected < fault.applied
                && fault.applied < fault.observed
                && fault.observed < fault.healed
                && fault.healed <= fault.recovered)
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
    }
    let fault_events = events.iter().filter(|e| e["kind"] == "fault").count();
    if fault_events != evidence.faults.len() * 3 {
        return Err(super::Issue::Fault);
    }
    Ok(())
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
    let observed_stability = stable_membership_observations(events, read.stable_membership_event, last_recovery);
    if (observed_stability as u64) < u64::from(read.stable_membership_observations) {
        return Err(super::Issue::Recovery);
    }
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
    let start = ready["controller_time_ns"].as_u64().ok_or(super::Issue::Recovery)?;
    let end = complete["controller_time_ns"].as_u64().ok_or(super::Issue::Recovery)?;
    let elapsed_ns = end.checked_sub(start).ok_or(super::Issue::Recovery)?;
    if elapsed_ns / 1_000_000 != read.elapsed_ms
        || elapsed_ns > expected.max_recovery_ms * 1_000_000
        || elapsed_ns > expected.admitted.profile().recovery.timeout_ms * 1_000_000
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

/// Stable-membership lifecycle events observed after the last recovery, up to and including
/// `through`.
fn stable_membership_observations(events: &[serde_json::Value], through: u64, last_recovery: Option<u64>) -> usize {
    events
        .iter()
        .zip(0_u64..)
        .take_while(|(_, index)| *index <= through)
        .filter(|(event, index)| {
            event["kind"] == "lifecycle"
                && event["phase"] == "stable-membership"
                && last_recovery.is_none_or(|recovery| *index > recovery)
        })
        .count()
}

pub(super) fn validate_report<'a>(
    expected: &super::Expected,
    evidence: &'a super::Observed,
) -> Result<&'a str, super::Issue> {
    let report = &evidence.report;
    let verdict = report["verdict"].as_str().ok_or(super::Issue::Checker)?;
    if report["schema_version"] != 1
        || report["model"] != "register"
        || report["checker_id"] != "chaoscontrol-semantic-checker.v1"
        || report["profile_id"] != evidence.history["profile"]["profile_id"]
        || report["bounds"] != evidence.history["profile"]["bounds"]
        || report["completeness"] != evidence.history["completeness"]
        || !report["history_id"].as_str().is_some_and(crate::live_reliability::checker::reference)
        || !matches!(verdict, "valid" | "invalid" | "unknown")
    {
        return Err(super::Issue::Checker);
    }
    let size = serde_json::to_vec(report).map_err(|_| super::Issue::Witness)?.len();
    if size > expected.max_witness_bytes
        || !report["witness"].is_array()
        || (verdict == "invalid") != report["invalid"].is_object()
        || (verdict == "unknown") != report["unknown"].is_object()
        || (verdict != "valid" && report["witness"].as_array().is_some_and(|w| !w.is_empty()))
    {
        return Err(super::Issue::Witness);
    }
    if let Some(reference) = &evidence.reference_report
        && (reference["history_id"] != report["history_id"]
            || reference["model"] != report["model"]
            || reference["bounds"] != report["bounds"]
            || !matches!(reference["verdict"].as_str(), Some("valid" | "invalid" | "unknown"))
            || !expected
                .identities
                .get("reference-checker")
                .is_some_and(|r| crate::live_reliability::checker::reference(r)))
    {
        return Err(super::Issue::Checker);
    }
    Ok(verdict)
}

/// Event lookup by producer-recorded index; indices beyond the platform address space are absent.
fn event_at(events: &[serde_json::Value], index: u64) -> Option<&serde_json::Value> {
    usize::try_from(index).ok().and_then(|index| events.get(index))
}

pub(super) fn validate_checked(expected: &super::Expected, evidence: &super::Observed) -> Result<(), super::Issue> {
    let history_ref =
        crate::live_reliability::checker::canonical_ref(&evidence.history).map_err(|_| super::Issue::History)?;
    let report_ref =
        crate::live_reliability::checker::canonical_ref(&evidence.report).map_err(|_| super::Issue::Checker)?;
    if expected.checked.history_ref != history_ref
        || expected.checked.report_ref != report_ref
        || expected.identities.get("checker") != Some(&expected.checked.checker_ref)
        || expected.checked.report != evidence.report
    {
        return Err(super::Issue::Checker);
    }
    match (&evidence.reference_report, &expected.reference_checked) {
        (None, None) => {}
        (Some(report), Some(checked))
            if checked.history_ref == history_ref
                && checked.report == *report
                && expected.identities.get("reference-checker") == Some(&checked.checker_ref) => {}
        _ => return Err(super::Issue::Checker),
    }
    Ok(())
}
