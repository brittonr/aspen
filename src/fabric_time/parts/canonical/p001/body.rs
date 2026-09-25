
// r[impl molten.fabric_time.scheduler]
pub fn canonical_scheduler_transition(
    profile_ref: &str,
    transition: &super::SchedulerTransition,
) -> crate::error::Result<CanonicalTimeEvent> {
    canonical_event(
        EventHeader {
            profile_ref,
            kind: CanonicalTimeEventKind::Scheduler,
            generation: transition.runnable.generation,
            subject: &transition.runnable.runnable_id,
            action: scheduler_action(transition.action),
            ticks: 0,
        },
        vec![field(
            "service-id",
            crate::preserves_rail::string(&transition.runnable.service_id),
        )],
        &["generation-fenced", "queue-bounded", "wake-transition-checked"],
    )
}

pub fn canonical_scheduler_selection(
    profile_ref: &str,
    selection: &super::SchedulerSelection,
) -> crate::error::Result<CanonicalTimeEvent> {
    canonical_event(
        EventHeader {
            profile_ref,
            kind: CanonicalTimeEventKind::Scheduler,
            generation: selection.selected.generation,
            subject: &selection.selected.runnable_id,
            action: "selected",
            ticks: selection.choice_sequence,
        },
        vec![
            field("service-id", crate::preserves_rail::string(&selection.selected.service_id)),
            field("eligible-count", crate::preserves_rail::u64_value(selection.eligible_count)),
        ],
        &["choice-recorded", "replay-choice-checked", "concurrency-bounded"],
    )
}

// Entropy output bytes are intentionally absent. Only purpose, bounds, stream
// position, mode, and generation are evidence-bearing.
// r[impl molten.fabric_time.entropy]
pub fn canonical_entropy_event(metadata: &super::EntropyEvidenceMetadata) -> crate::error::Result<CanonicalTimeEvent> {
    let expected_replay = match metadata.mode {
        super::EntropyMode::DeterministicSimulation => super::EntropyReplayClass::RecomputeFromExplicitSeed,
        super::EntropyMode::ProductionCryptographic => super::EntropyReplayClass::SecretInputRequired,
    };
    if metadata.replay_class != expected_replay {
        return Err(crate::error::MoltenError::invalid_harness("entropy evidence mode and replay class mismatch"));
    }
    match metadata.mode {
        super::EntropyMode::DeterministicSimulation => {
            let input_ref = metadata.deterministic_input_ref.as_deref().ok_or_else(|| {
                crate::error::MoltenError::invalid_harness("deterministic entropy evidence requires an input ref")
            })?;
            crate::preserves_rail::validate_content_ref(input_ref)?;
        }
        super::EntropyMode::ProductionCryptographic if metadata.deterministic_input_ref.is_some() => {
            return Err(crate::error::MoltenError::invalid_harness(
                "production entropy evidence must not contain a deterministic input ref",
            ));
        }
        super::EntropyMode::ProductionCryptographic => {}
    }
    canonical_event(
        EventHeader {
            profile_ref: &metadata.profile_ref,
            kind: CanonicalTimeEventKind::Entropy,
            generation: metadata.generation,
            subject: &metadata.stream_id,
            action: metadata.mode.as_str(),
            ticks: metadata.end_position_bytes,
        },
        vec![
            field("purpose", crate::preserves_rail::string(&metadata.purpose)),
            field("start-position-bytes", crate::preserves_rail::u64_value(metadata.start_position_bytes)),
            field("request-bytes", crate::preserves_rail::u64_value(metadata.request_bytes)),
            field("replay-class", crate::preserves_rail::string(metadata.replay_class.as_str())),
            field("deterministic-input-ref", optional_string(metadata.deterministic_input_ref.as_deref())),
        ],
        &["purpose-bound", "generation-fenced", "secret-output-omitted"],
    )
}

// r[impl molten.fabric_time.deadline_lease]
pub fn canonical_deadline_event(
    profile_ref: &str,
    decision: &super::DeadlineDecision,
) -> crate::error::Result<CanonicalTimeEvent> {
    canonical_event(
        EventHeader {
            profile_ref,
            kind: CanonicalTimeEventKind::Deadline,
            generation: decision.generation,
            subject: &decision.subject_id,
            action: deadline_status(decision.status),
            ticks: decision.observed_ticks,
        },
        vec![
            field("domain", crate::preserves_rail::string(decision.domain.as_str())),
            field("target-ticks", crate::preserves_rail::u64_value(decision.target_ticks)),
            field("uncertainty-ticks", crate::preserves_rail::u64_value(decision.uncertainty_ticks)),
        ],
        &["domain-checked", "uncertainty-explicit", "local-decision-only"],
    )
}

pub fn canonical_lease_event(
    profile_ref: &str,
    decision: &super::LeaseDecision,
) -> crate::error::Result<CanonicalTimeEvent> {
    canonical_event(
        EventHeader {
            profile_ref,
            kind: CanonicalTimeEventKind::Lease,
            generation: decision.generation,
            subject: &decision.lease_id,
            action: lease_decision(decision.kind),
            ticks: 0,
        },
        vec![
            field("owner-id", crate::preserves_rail::string(&decision.owner_id)),
            field("fencing-token", optional_u64(decision.fencing_token)),
        ],
        &[
            "generation-fenced",
            "fencing-explicit",
            "no-distributed-exclusivity-claim",
        ],
    )
}

pub fn canonical_clock_anomaly_event(
    profile_ref: &str,
    generation: u64,
    decision: &super::WallClockAnomalyDecision,
) -> crate::error::Result<CanonicalTimeEvent> {
    canonical_event(
        EventHeader {
            profile_ref,
            kind: CanonicalTimeEventKind::ClockAnomaly,
            generation,
            subject: "wall-clock",
            action: clock_anomaly(decision.kind),
            ticks: decision.observed_unix_nanos,
        },
        vec![
            field("previous-unix-nanos", crate::preserves_rail::u64_value(decision.previous_unix_nanos)),
            field("delta-nanos", crate::preserves_rail::u64_value(decision.delta_nanos)),
        ],
        &["wall-clock-untrusted", "anomaly-classified", "no-global-time-claim"],
    )
}

/// The identity of one canonical fabric-time event: profile, kind, generation, subject, action, and
/// tick.
#[derive(Clone, Copy)]
pub struct EventHeader<'a> {
    pub profile_ref: &'a str,
    pub kind: CanonicalTimeEventKind,
    pub generation: u64,
    pub subject: &'a str,
    pub action: &'a str,
    pub ticks: u64,
}

pub fn canonical_named_event(header: EventHeader<'_>) -> crate::error::Result<CanonicalTimeEvent> {
    canonical_event(header, Vec::new(), &["explicit-input", "generation-bound", "bounded-evidence"])
}

// r[impl molten.fabric_time.evidence]
// r[impl molten.fabric_time.non_claims]
pub fn canonical_fabric_time_run(report: FabricTimeRunReport) -> crate::error::Result<CanonicalFabricTimeRun> {
    if report.generation == 0 {
        return Err(crate::error::MoltenError::invalid_harness("fabric time run generation must be non-zero"));
    }
    if !matches!(report.profile_kind.as_str(), "live" | "deterministic-simulation" | "both") {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "unsupported fabric-time profile kind: {}",
            report.profile_kind
        )));
    }
    for content_ref in [
        &report.profile_ref,
        &report.initial_state_ref,
        &report.scheduler_trace_ref,
        &report.entropy_trace_ref,
        &report.fault_plan_ref,
        &report.terminal_outcome_ref,
    ]
    .into_iter()
    .chain(report.evidence_refs.iter())
    {
        crate::preserves_rail::validate_content_ref(content_ref)?;
    }
    if report.evidence_refs.len() > MAX_RUN_EVIDENCE_REFS {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "fabric time run evidence count {} exceeds {}",
            report.evidence_refs.len(),
            MAX_RUN_EVIDENCE_REFS
        )));
    }
    if report.non_claims != super::REQUIRED_TIME_NON_CLAIMS {
        return Err(crate::error::MoltenError::invalid_harness(
            "fabric time run must preserve the complete canonical non-claim set",
        ));
    }
    let value = crate::preserves_rail::record(FABRIC_TIME_RUN_RECORD, vec![
        crate::preserves_rail::string(super::FABRIC_TIME_RUN_REPORT_SCHEMA),
        field("profile-ref", crate::preserves_rail::string(&report.profile_ref)),
        field("profile-kind", crate::preserves_rail::string(&report.profile_kind)),
        field("generation", crate::preserves_rail::u64_value(report.generation)),
        field("initial-state-ref", crate::preserves_rail::string(&report.initial_state_ref)),
        field("scheduler-trace-ref", crate::preserves_rail::string(&report.scheduler_trace_ref)),
        field("entropy-trace-ref", crate::preserves_rail::string(&report.entropy_trace_ref)),
        field("fault-plan-ref", crate::preserves_rail::string(&report.fault_plan_ref)),
        field("terminal-outcome-ref", crate::preserves_rail::string(&report.terminal_outcome_ref)),
        field("final-time-ticks", crate::preserves_rail::u64_value(report.final_time_ticks)),
        field("timer-events", crate::preserves_rail::u64_value(report.timer_events)),
        field("scheduler-events", crate::preserves_rail::u64_value(report.scheduler_events)),
        field("entropy-events", crate::preserves_rail::u64_value(report.entropy_events)),
        field("deadline-lease-events", crate::preserves_rail::u64_value(report.deadline_lease_events)),
        field("fault-events", crate::preserves_rail::u64_value(report.fault_events)),
        field("live-clock-observed", crate::preserves_rail::bool_value(report.live_clock_observed)),
        field("shared-conformance-passed", crate::preserves_rail::bool_value(report.shared_conformance_passed)),
        field("evidence-refs", strings_value(report.evidence_refs.iter().map(String::as_str))),
        field("non-claims", strings_value(report.non_claims.iter().map(|claim| claim.as_str()))),
        checks(&[
            "time-domains-not-interchangeable",
            "timer-and-scheduler-generation-fenced",
            "entropy-purpose-bound-and-secret-free",
            "deadline-and-lease-claims-local",
            "live-and-simulation-profiles-distinct",
        ]),
    ]);
    let report_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalFabricTimeRun {
        report_ref,
        report,
        value,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FabricTimeRunReadback {
    pub profile_ref: String,
    pub profile_kind: String,
    pub generation: u64,
    pub initial_state_ref: String,
    pub scheduler_trace_ref: String,
    pub entropy_trace_ref: String,
    pub fault_plan_ref: String,
    pub terminal_outcome_ref: String,
    pub final_time_ticks: u64,
    pub timer_events: u64,
    pub scheduler_events: u64,
    pub entropy_events: u64,
    pub deadline_lease_events: u64,
    pub fault_events: u64,
    pub live_clock_observed: bool,
    pub shared_conformance_passed: bool,
    pub report_ref: String,
}
