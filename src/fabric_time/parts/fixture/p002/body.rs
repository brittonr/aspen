
fn run_deterministic_entropy_scenario(
    profile: &CanonicalTimeProfile,
    events: &mut impl crate::bounded::VecSink<CanonicalTimeEvent>,
    counters: &mut ScenarioCounters,
) -> crate::error::Result<()> {
    let mut stream = open_entropy_stream(
        &profile.profile,
        FIXTURE_GENERATION,
        &entropy_stream_request(
            &profile.profile,
            "simulation-stream",
            "scheduler-choice",
            EntropyMode::DeterministicSimulation,
            Some(ENTROPY_SEED),
        ),
    )
    .map_err(|error| core_error("open deterministic entropy stream", error))?;
    for request in [
        EntropyRequest::Bytes {
            count: ENTROPY_BYTE_COUNT,
        },
        EntropyRequest::BoundedChoice {
            upper_exclusive: ENTROPY_CHOICE_BOUND,
        },
    ] {
        let transition = draw_deterministic_entropy(&profile.profile, FIXTURE_GENERATION, &stream, request)
            .map_err(|error| core_error("draw deterministic entropy", error))?;
        let metadata = entropy_evidence_metadata(&stream, &transition);
        events.push_item(canonical_entropy_event(&metadata)?);
        stream = transition.next;
        counters.entropy_events = checked_increment(counters.entropy_events, "entropy event count")?;
    }
    Ok(())
}

fn run_production_entropy_scenario(
    profile: &CanonicalTimeProfile,
    events: &mut impl crate::bounded::VecSink<CanonicalTimeEvent>,
) -> crate::error::Result<String> {
    let stream = open_entropy_stream(
        &profile.profile,
        FIXTURE_GENERATION,
        &entropy_stream_request(
            &profile.profile,
            "production-stream",
            "session-token",
            EntropyMode::ProductionCryptographic,
            None,
        ),
    )
    .map_err(|error| core_error("open production entropy stream", error))?;
    let mut adapter = ProductionEntropyAdapter::new(OperatingSystemEntropySource);
    let (_, metadata) = adapter.draw(&profile.profile, FIXTURE_GENERATION, &stream, EntropyRequest::Bytes {
        count: ENTROPY_BYTE_COUNT,
    })?;
    events.push_item(canonical_entropy_event(&metadata)?);
    Ok(adapter.source_id().to_string())
}

fn run_deadline_lease_scenario(
    profile: &CanonicalTimeProfile,
    events: &mut impl crate::bounded::VecSink<CanonicalTimeEvent>,
    counters: &mut ScenarioCounters,
) -> crate::error::Result<()> {
    let target = virtual_value(&profile.profile, DEADLINE_TARGET);
    let deadline = Deadline {
        profile_ref: profile.profile.profile_ref.clone(),
        subject_id: "fixture-deadline".to_string(),
        generation: FIXTURE_GENERATION,
        target,
        uncertainty_ticks: 1,
    };
    let decision = evaluate_deadline(
        &profile.profile,
        FIXTURE_GENERATION,
        &deadline,
        &virtual_value(&profile.profile, DEADLINE_OBSERVATION),
    )
    .map_err(|error| core_error("evaluate fixture deadline", error))?;
    events.push_item(canonical_deadline_event(&profile.profile_ref, &decision)?);
    counters.deadline_lease_events = checked_increment(counters.deadline_lease_events, "deadline/lease event count")?;

    events.extend_items(retry_events(
        profile,
        &virtual_value(&profile.profile, DEADLINE_OBSERVATION),
        1,
        RetryPolicy {
            maximum_attempts: RETRY_ATTEMPTS,
            base_delay_ticks: RETRY_BASE,
            maximum_delay_ticks: RETRY_MAXIMUM,
            backoff: RetryBackoff::Exponential,
            jitter: RetryJitter::Bounded {
                maximum_ticks: RETRY_JITTER,
            },
        },
        Some(RETRY_JITTER),
    )?);
    counters.deadline_lease_events = checked_increment(counters.deadline_lease_events, "deadline/lease event count")?;
    counters.deadline_lease_events = checked_increment(counters.deadline_lease_events, "deadline/lease event count")?;

    let lease = evaluate_lease(&profile.profile, FIXTURE_GENERATION, &LeaseRequest {
        lease_id: "fixture-lease".to_string(),
        owner_id: "fixture-owner".to_string(),
        generation: FIXTURE_GENERATION,
        now: virtual_value(&profile.profile, DEADLINE_OBSERVATION),
        expires_at: virtual_value(&profile.profile, LEASE_EXPIRY),
        uncertainty_ticks: 0,
        consistency: LeaseConsistency::FencedExclusive,
        action: LeaseAction::AcquireExclusive,
        fencing_token: Some(FENCING_TOKEN),
        previous_fencing_token: Some(PREVIOUS_FENCING_TOKEN),
    })
    .map_err(|error| core_error("evaluate fixture lease", error))?;
    events.push_item(canonical_lease_event(&profile.profile_ref, &lease)?);
    counters.deadline_lease_events = checked_increment(counters.deadline_lease_events, "deadline/lease event count")?;
    Ok(())
}

const RETRY_EVENT_COUNT: usize = 2;

// r[impl molten.audit_f12.compatibility]
// r[impl molten.audit_f12.validation]
pub(super) fn retry_events(
    profile: &CanonicalTimeProfile,
    now: &TimeValue,
    attempt: u64,
    policy: RetryPolicy,
    jitter: Option<u64>,
) -> crate::error::Result<[CanonicalTimeEvent; RETRY_EVENT_COUNT]> {
    let retry = plan_retry(
        &profile.profile,
        FIXTURE_GENERATION,
        "fixture-retry",
        FIXTURE_GENERATION,
        now,
        attempt,
        policy,
        jitter,
    )
    .map_err(|error| core_error("plan fixture retry", error))?;
    let deadline = canonical_named_event(EventHeader {
        profile_ref: &profile.profile_ref,
        kind: CanonicalTimeEventKind::Deadline,
        generation: FIXTURE_GENERATION,
        subject: "fixture-retry",
        action: "retry-planned",
        ticks: retry.deadline.target.ticks(),
    })?;
    let delay = canonical_named_event(EventHeader {
        profile_ref: &profile.profile_ref,
        kind: CanonicalTimeEventKind::Deadline,
        generation: FIXTURE_GENERATION,
        subject: "fixture-retry",
        action: "retry-delay",
        ticks: retry.delay.ticks,
    })?;
    Ok([deadline, delay])
}

fn run_clock_partition_faults(
    profile: &CanonicalTimeProfile,
    clock: &mut VirtualClockAdapter,
    events: &mut impl crate::bounded::VecSink<CanonicalTimeEvent>,
    counters: &mut ScenarioCounters,
) -> crate::error::Result<()> {
    let previous = clock.observe_wall()?;
    let backward = FabricTimeFault::BackwardWallJump { ticks: WALL_JUMP_FAULT };
    if !apply_clock_fault(clock, &backward)? {
        return Err(crate::error::MoltenError::invalid_harness("backward clock fault was not applied"));
    }
    clock.advance(1)?;
    let observed = clock.observe_wall()?;
    let anomaly = classify_wall_clock_observation(&previous, &observed, WallClockAnomalyPolicy {
        max_forward_jump_nanos: PROFILE_MAX_TICKS,
        max_uncertainty_nanos: PROFILE_MAX_TICKS,
    })
    .map_err(|error| core_error("classify injected wall jump", error))?;
    events.push_item(canonical_clock_anomaly_event(&profile.profile_ref, FIXTURE_GENERATION, &anomaly)?);
    events.push_item(canonical_named_event(EventHeader {
        profile_ref: &profile.profile_ref,
        kind: CanonicalTimeEventKind::Fault,
        generation: FIXTURE_GENERATION,
        subject: "wall-clock",
        action: "backward-jump-injected",
        ticks: WALL_JUMP_FAULT,
    })?);
    counters.fault_events = checked_increment(counters.fault_events, "fault event count")?;

    run_partition_fault(profile, clock, events, counters)
}

/// Injects a partition window and requires a deadline coupled to it to be indeterminate while it
/// lasts.
fn run_partition_fault(
    profile: &CanonicalTimeProfile,
    clock: &mut VirtualClockAdapter,
    events: &mut impl crate::bounded::VecSink<CanonicalTimeEvent>,
    counters: &mut ScenarioCounters,
) -> crate::error::Result<()> {
    let partition_until = clock
        .now_ticks()?
        .checked_add(TIMER_PERIOD)
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("partition deadline overflow"))?;
    let partition = FabricTimeFault::PartitionWindow {
        until_ticks: partition_until,
    };
    let partition_deadline_ticks = partition_until
        .checked_add(TIMER_PERIOD)
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("partition-coupled deadline overflow"))?;
    let partition_decision = evaluate_deadline_with_fault(
        &profile.profile,
        FIXTURE_GENERATION,
        &Deadline {
            profile_ref: profile.profile.profile_ref.clone(),
            subject_id: "partition-coupled-deadline".to_string(),
            generation: FIXTURE_GENERATION,
            target: virtual_value(&profile.profile, partition_deadline_ticks),
            uncertainty_ticks: 0,
        },
        &virtual_value(&profile.profile, clock.now_ticks()?),
        Some(&partition),
    )?;
    if !matches!(partition_decision, FaultedDeadlineDecision::PartitionIndeterminate { .. }) {
        return Err(crate::error::MoltenError::invalid_harness(
            "partition fault did not make the coupled deadline indeterminate",
        ));
    }
    events.push_item(canonical_named_event(EventHeader {
        profile_ref: &profile.profile_ref,
        kind: CanonicalTimeEventKind::Fault,
        generation: FIXTURE_GENERATION,
        subject: "partition-window",
        action: "deadline-indeterminate",
        ticks: partition_until,
    })?);
    events.push_item(canonical_named_event(EventHeader {
        profile_ref: &profile.profile_ref,
        kind: CanonicalTimeEventKind::Deadline,
        generation: FIXTURE_GENERATION,
        subject: "partition-coupled-deadline",
        action: "indeterminate-during-partition",
        ticks: partition_deadline_ticks,
    })?);
    counters.fault_events = checked_increment(counters.fault_events, "fault event count")?;
    counters.deadline_lease_events = checked_increment(counters.deadline_lease_events, "deadline/lease event count")?;
    Ok(())
}

fn fixture_profile(
    profile_id: &str,
    profile_ref: &str,
    kind: TimeProfileKind,
    fairness_bound_turns: Option<u64>,
) -> TimeProfileDescriptor {
    let replay = match kind {
        TimeProfileKind::Live => SchedulerReplayPolicy::RecordedChoiceRequired,
        TimeProfileKind::DeterministicSimulation => SchedulerReplayPolicy::Deterministic,
    };
    TimeProfileDescriptor {
        schema: FABRIC_TIME_PROFILE_SCHEMA.to_string(),
        profile_id: profile_id.to_string(),
        profile_ref: profile_ref.to_string(),
        kind,
        supported_domains: REQUIRED_TIME_DOMAINS.to_vec(),
        max_duration_ticks: PROFILE_MAX_TICKS,
        max_uncertainty_ticks: PROFILE_MAX_TICKS,
        max_timers: PROFILE_MAX_TIMERS,
        max_runnables: PROFILE_MAX_RUNNABLES,
        max_entropy_request_bytes: PROFILE_MAX_ENTROPY_REQUEST,
        max_entropy_total_bytes: PROFILE_MAX_ENTROPY_TOTAL,
        max_scheduler_concurrency: PROFILE_MAX_CONCURRENCY,
        max_scheduler_queue_depth: PROFILE_MAX_QUEUE,
        fairness_bound_turns,
        scheduler_policy: SchedulerPolicy {
            ordering: SchedulerOrdering::PriorityThenFifo,
            replay,
            overload: SchedulerOverloadPolicy::Reject,
        },
        evidence_mode: TimeEvidenceMode::SelectedSemanticBoundaries,
        non_claims: REQUIRED_TIME_NON_CLAIMS.to_vec(),
    }
}
