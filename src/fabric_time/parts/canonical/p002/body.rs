
// r[impl molten.fabric_time.evidence]
pub fn parse_fabric_time_run_readback(value: &preserves::IOValue) -> crate::error::Result<FabricTimeRunReadback> {
    const RUN_REPORT_FIELD_COUNT: usize = 20;
    const PROFILE_KIND_FIELD_INDEX: usize = 2;
    const GENERATION_FIELD_INDEX: usize = 3;
    const INITIAL_STATE_FIELD_INDEX: usize = 4;
    const SCHEDULER_TRACE_FIELD_INDEX: usize = 5;
    const ENTROPY_TRACE_FIELD_INDEX: usize = 6;
    const FAULT_PLAN_FIELD_INDEX: usize = 7;
    const TERMINAL_OUTCOME_FIELD_INDEX: usize = 8;
    const FINAL_TIME_FIELD_INDEX: usize = 9;
    const TIMER_EVENTS_FIELD_INDEX: usize = 10;
    const SCHEDULER_EVENTS_FIELD_INDEX: usize = 11;
    const ENTROPY_EVENTS_FIELD_INDEX: usize = 12;
    const DEADLINE_LEASE_EVENTS_FIELD_INDEX: usize = 13;
    const FAULT_EVENTS_FIELD_INDEX: usize = 14;
    const LIVE_CLOCK_FIELD_INDEX: usize = 15;
    const CONFORMANCE_FIELD_INDEX: usize = 16;
    let fields = value
        .collect_simple_record(FABRIC_TIME_RUN_RECORD, Some(RUN_REPORT_FIELD_COUNT))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("expected canonical fabric-time run report"))?;
    let schema = required_string(&fields[0], "fabric-time report schema")?;
    if schema != super::FABRIC_TIME_RUN_REPORT_SCHEMA {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "fabric-time report schema mismatch: {schema}"
        )));
    }
    let profile_ref = record_string_field(&fields[1], "profile-ref")?;
    let profile_kind = record_string_field(&fields[PROFILE_KIND_FIELD_INDEX], "profile-kind")?;
    let initial_state_ref = record_string_field(&fields[INITIAL_STATE_FIELD_INDEX], "initial-state-ref")?;
    let scheduler_trace_ref = record_string_field(&fields[SCHEDULER_TRACE_FIELD_INDEX], "scheduler-trace-ref")?;
    let entropy_trace_ref = record_string_field(&fields[ENTROPY_TRACE_FIELD_INDEX], "entropy-trace-ref")?;
    let fault_plan_ref = record_string_field(&fields[FAULT_PLAN_FIELD_INDEX], "fault-plan-ref")?;
    let terminal_outcome_ref = record_string_field(&fields[TERMINAL_OUTCOME_FIELD_INDEX], "terminal-outcome-ref")?;
    for content_ref in [
        &profile_ref,
        &initial_state_ref,
        &scheduler_trace_ref,
        &entropy_trace_ref,
        &fault_plan_ref,
        &terminal_outcome_ref,
    ] {
        crate::preserves_rail::validate_content_ref(content_ref)?;
    }
    if !matches!(profile_kind.as_str(), "live" | "deterministic-simulation" | "both") {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "unsupported fabric-time profile kind: {profile_kind}"
        )));
    }
    Ok(FabricTimeRunReadback {
        profile_ref,
        profile_kind,
        generation: record_u64_field(&fields[GENERATION_FIELD_INDEX], "generation")?,
        initial_state_ref,
        scheduler_trace_ref,
        entropy_trace_ref,
        fault_plan_ref,
        terminal_outcome_ref,
        final_time_ticks: record_u64_field(&fields[FINAL_TIME_FIELD_INDEX], "final-time-ticks")?,
        timer_events: record_u64_field(&fields[TIMER_EVENTS_FIELD_INDEX], "timer-events")?,
        scheduler_events: record_u64_field(&fields[SCHEDULER_EVENTS_FIELD_INDEX], "scheduler-events")?,
        entropy_events: record_u64_field(&fields[ENTROPY_EVENTS_FIELD_INDEX], "entropy-events")?,
        deadline_lease_events: record_u64_field(&fields[DEADLINE_LEASE_EVENTS_FIELD_INDEX], "deadline-lease-events")?,
        fault_events: record_u64_field(&fields[FAULT_EVENTS_FIELD_INDEX], "fault-events")?,
        live_clock_observed: record_bool_field(&fields[LIVE_CLOCK_FIELD_INDEX], "live-clock-observed")?,
        shared_conformance_passed: record_bool_field(&fields[CONFORMANCE_FIELD_INDEX], "shared-conformance-passed")?,
        report_ref: crate::preserves_rail::canonical_hash(value)?,
    })
}

pub fn canonical_time_trace_ref(trace_kind: &str, evidence_refs: &[String]) -> crate::error::Result<String> {
    let value = crate::preserves_rail::record("fabric-time-trace-v1", vec![
        field("trace-kind", crate::preserves_rail::string(trace_kind)),
        field("evidence-refs", strings_value(evidence_refs.iter().map(String::as_str))),
        checks(&["ordered-canonical-refs", "bounded-trace-summary"]),
    ]);
    crate::preserves_rail::canonical_hash(&value)
}

fn time_profile_value(profile: &super::AdmittedTimeProfile) -> preserves::IOValue {
    crate::preserves_rail::record(FABRIC_TIME_PROFILE_RECORD, vec![
        crate::preserves_rail::string(super::FABRIC_TIME_PROFILE_SCHEMA),
        field("profile-id", crate::preserves_rail::string(&profile.profile_id)),
        field("declared-profile-ref", crate::preserves_rail::string(&profile.profile_ref)),
        field("kind", crate::preserves_rail::string(profile.kind.as_str())),
        field("domains", strings_value(profile.supported_domains.iter().map(|domain| domain.as_str()))),
        field("max-duration-ticks", crate::preserves_rail::u64_value(profile.max_duration_ticks)),
        field("max-uncertainty-ticks", crate::preserves_rail::u64_value(profile.max_uncertainty_ticks)),
        field("max-timers", crate::preserves_rail::u64_value(profile.max_timers)),
        field("max-runnables", crate::preserves_rail::u64_value(profile.max_runnables)),
        field("max-entropy-request-bytes", crate::preserves_rail::u64_value(profile.max_entropy_request_bytes)),
        field("max-entropy-total-bytes", crate::preserves_rail::u64_value(profile.max_entropy_total_bytes)),
        field("max-scheduler-concurrency", crate::preserves_rail::u64_value(profile.max_scheduler_concurrency)),
        field("max-scheduler-queue-depth", crate::preserves_rail::u64_value(profile.max_scheduler_queue_depth)),
        field("fairness-bound-turns", optional_u64(profile.fairness_bound_turns)),
        field("scheduler-ordering", crate::preserves_rail::string(profile.scheduler_policy.ordering.as_str())),
        field("scheduler-replay", crate::preserves_rail::string(profile.scheduler_policy.replay.as_str())),
        field("scheduler-overload", crate::preserves_rail::string(profile.scheduler_policy.overload.as_str())),
        field("evidence-mode", crate::preserves_rail::string(profile.evidence_mode.as_str())),
        field("non-claims", strings_value(profile.non_claims.iter().map(|claim| claim.as_str()))),
        checks(&[
            "canonical-profile",
            "exact-mode",
            "bounded-resources",
            "non-claims-complete",
        ]),
    ])
}

struct PortDescriptorInput<'a> {
    port_id: &'a str,
    class: crate::fabric::FabricPortClass,
    operations: &'a [&'a str],
    input_schemas: &'a [&'a str],
    output_schemas: &'a [&'a str],
    authorities: &'a [crate::fabric::FabricAuthority],
    resources: &'a [crate::fabric::FabricResource],
    determinism: crate::fabric::DeterminismClass,
    replay: crate::fabric::ReplayClass,
    profile: &'a CanonicalTimeProfile,
}

fn port_descriptor(input: PortDescriptorInput<'_>) -> crate::fabric::FabricPortDescriptor {
    let PortDescriptorInput {
        port_id,
        class,
        operations,
        input_schemas,
        output_schemas,
        authorities,
        resources,
        determinism,
        replay,
        profile,
    } = input;
    crate::fabric::FabricPortDescriptor {
        schema: crate::fabric::FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
        port_id: port_id.to_string(),
        version: FABRIC_TIME_PORT_VERSION.to_string(),
        class,
        operation_classes: operations.iter().map(|value| (*value).to_string()).collect(),
        input_schema_refs: input_schemas.iter().map(|value| (*value).to_string()).collect(),
        output_schema_refs: output_schemas.iter().map(|value| (*value).to_string()).collect(),
        authority_requirements: authorities.to_vec(),
        resource_requirements: resources.to_vec(),
        determinism,
        replay,
        implementation_profile: profile.profile.profile_id.clone(),
        conformance_refs: vec![profile.profile_ref.clone()],
        non_claims: crate::fabric::REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
        enabled: true,
    }
}

fn canonical_event(
    header: EventHeader<'_>,
    details: Vec<preserves::IOValue>,
    event_checks: &[&str],
) -> crate::error::Result<CanonicalTimeEvent> {
    let EventHeader {
        profile_ref,
        kind,
        generation,
        subject,
        action,
        ticks,
    } = header;
    if generation == 0 {
        return Err(crate::error::MoltenError::invalid_harness("fabric time event generation must be non-zero"));
    }
    let value = crate::preserves_rail::record(FABRIC_TIME_EVENT_RECORD, vec![
        crate::preserves_rail::string(super::FABRIC_TIME_OBSERVATION_SCHEMA),
        field("profile-ref", crate::preserves_rail::string(profile_ref)),
        field("kind", crate::preserves_rail::string(kind.as_str())),
        field("generation", crate::preserves_rail::u64_value(generation)),
        field("subject", crate::preserves_rail::string(subject)),
        field("action", crate::preserves_rail::string(action)),
        field("ticks", crate::preserves_rail::u64_value(ticks)),
        field("details", crate::preserves_rail::sequence(details)),
        checks(event_checks),
    ]);
    let evidence_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalTimeEvent {
        evidence_ref,
        profile_ref: profile_ref.to_string(),
        kind,
        generation,
        value,
    })
}

fn field(name: &str, value: preserves::IOValue) -> preserves::IOValue {
    crate::preserves_rail::record("field", vec![crate::preserves_rail::string(name), value])
}

fn checks(values: &[&str]) -> preserves::IOValue {
    field("checks", strings_value(values.iter().copied()))
}

fn strings_value<'a>(values: impl Iterator<Item = &'a str>) -> preserves::IOValue {
    crate::preserves_rail::sequence(values.map(crate::preserves_rail::string).collect())
}

fn optional_u64(value: Option<u64>) -> preserves::IOValue {
    match value {
        Some(value) => crate::preserves_rail::record("some", vec![crate::preserves_rail::u64_value(value)]),
        None => crate::preserves_rail::record("none", Vec::new()),
    }
}

fn optional_string(value: Option<&str>) -> preserves::IOValue {
    match value {
        Some(value) => crate::preserves_rail::record("some", vec![crate::preserves_rail::string(value)]),
        None => crate::preserves_rail::record("none", Vec::new()),
    }
}

fn timer_action(action: super::TimerAction) -> &'static str {
    match action {
        super::TimerAction::NotDue => "not-due",
        super::TimerAction::Deliver => "deliver",
        super::TimerAction::Coalesced => "coalesced",
        super::TimerAction::DroppedLate => "dropped-late",
        super::TimerAction::DroppedOverload => "dropped-overload",
        super::TimerAction::Backpressure => "backpressure",
        super::TimerAction::RetainedOverload => "retained-overload",
        super::TimerAction::Cancelled => "cancelled",
        super::TimerAction::DiscardedStaleGeneration => "discarded-stale-generation",
    }
}

fn scheduler_action(action: super::SchedulerAction) -> &'static str {
    match action {
        super::SchedulerAction::Woken => "woken",
        super::SchedulerAction::Yielded => "yielded",
        super::SchedulerAction::Blocked => "blocked",
        super::SchedulerAction::Completed => "completed",
        super::SchedulerAction::Cancelled => "cancelled",
        super::SchedulerAction::RejectedOverload => "rejected-overload",
        super::SchedulerAction::Backpressure => "backpressure",
        super::SchedulerAction::DiscardedStaleGeneration => "discarded-stale-generation",
    }
}

fn deadline_status(status: super::DeadlineStatus) -> &'static str {
    match status {
        super::DeadlineStatus::Pending => "pending",
        super::DeadlineStatus::Expired => "expired",
        super::DeadlineStatus::IndeterminateWithinUncertainty => "indeterminate-within-uncertainty",
    }
}

fn lease_decision(decision: super::LeaseDecisionKind) -> &'static str {
    match decision {
        super::LeaseDecisionKind::LocallyActive => "locally-active",
        super::LeaseDecisionKind::LocallyExpired => "locally-expired",
        super::LeaseDecisionKind::IndeterminateWithinUncertainty => "indeterminate-within-uncertainty",
        super::LeaseDecisionKind::RenewalAllowed => "renewal-allowed",
        super::LeaseDecisionKind::ExclusiveActionAllowed => "exclusive-action-allowed",
        super::LeaseDecisionKind::DeniedWithoutFencing => "denied-without-fencing",
        super::LeaseDecisionKind::DeniedStaleFencingToken => "denied-stale-fencing-token",
        super::LeaseDecisionKind::DeniedExpired => "denied-expired",
    }
}

fn clock_anomaly(kind: super::WallClockAnomalyKind) -> &'static str {
    match kind {
        super::WallClockAnomalyKind::Stable => "stable",
        super::WallClockAnomalyKind::BackwardJump => "backward-jump",
        super::WallClockAnomalyKind::ForwardJump => "forward-jump",
        super::WallClockAnomalyKind::ExcessiveUncertainty => "excessive-uncertainty",
    }
}

fn record_string_field(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<String> {
    let field_value = named_field_value(value, label)?;
    required_string(&field_value, label)
}

fn record_u64_field(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<u64> {
    let field_value = named_field_value(value, label)?;
    field_value
        .as_u64()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected u64 for {label}")))?
        .map_err(|error| crate::error::MoltenError::invalid_harness(format!("u64 out of range for {label}: {error}")))
}
