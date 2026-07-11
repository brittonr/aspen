use preserves::IOValue;

use super::AdmittedTimeProfile;
use super::CheckedDuration;
use super::DeadlineDecision;
use super::EntropyEvidenceMetadata;
use super::EntropyMode;
use super::LeaseDecision;
use super::SchedulerAction;
use super::SchedulerSelection;
use super::SchedulerTransition;
use super::TimeNonClaim;
use super::TimeProfileDescriptor;
use super::TimeValue;
use super::TimerAction;
use super::TimerTransition;
use super::WallClockAnomalyDecision;
use super::admit_time_profile;
use super::validate_duration;
use super::validate_time_value;
use crate::error::MoltenError;
use crate::error::Result;
use crate::fabric::DeterminismClass;
use crate::fabric::FABRIC_PORT_DESCRIPTOR_SCHEMA;
use crate::fabric::FabricAuthority;
use crate::fabric::FabricPortClass;
use crate::fabric::FabricPortDescriptor;
use crate::fabric::FabricResource;
use crate::fabric::REQUIRED_FABRIC_NON_CLAIMS;
use crate::fabric::ReplayClass;
use crate::preserves_rail::bool_value;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;

pub const FABRIC_CLOCK_PORT_ID: &str = "molten.fabric.time.clock";
pub const FABRIC_TIMER_PORT_ID: &str = "molten.fabric.time.timer";
pub const FABRIC_SCHEDULER_PORT_ID: &str = "molten.fabric.scheduler.runnable";
pub const FABRIC_ENTROPY_PORT_ID: &str = "molten.fabric.entropy.stream";
pub const FABRIC_TIME_PORT_VERSION: &str = "v1";

const FABRIC_TIME_PROFILE_RECORD: &str = "fabric-time-profile-v1";
const FABRIC_TIME_EVENT_RECORD: &str = "fabric-time-event-v1";
const FABRIC_TIME_RUN_RECORD: &str = "fabric-time-run-v1";
const MAX_RUN_EVIDENCE_REFS: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalTimeProfile {
    pub profile: AdmittedTimeProfile,
    pub profile_ref: String,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalTimeValue {
    pub time: TimeValue,
    pub value_ref: String,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalDuration {
    pub duration: CheckedDuration,
    pub value_ref: String,
    pub value: IOValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalTimeEventKind {
    ClockAnomaly,
    Timer,
    Scheduler,
    Entropy,
    Deadline,
    Lease,
    Fault,
    Conformance,
}

impl CanonicalTimeEventKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ClockAnomaly => "clock-anomaly",
            Self::Timer => "timer",
            Self::Scheduler => "scheduler",
            Self::Entropy => "entropy",
            Self::Deadline => "deadline",
            Self::Lease => "lease",
            Self::Fault => "fault",
            Self::Conformance => "conformance",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalTimeEvent {
    pub evidence_ref: String,
    pub profile_ref: String,
    pub kind: CanonicalTimeEventKind,
    pub generation: u64,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FabricTimeRunReport {
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
    pub evidence_refs: Vec<String>,
    pub non_claims: Vec<TimeNonClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalFabricTimeRun {
    pub report_ref: String,
    pub report: FabricTimeRunReport,
    pub value: IOValue,
}

// r[impl molten.fabric_time.evidence]
pub fn canonical_admit_time_profile(descriptor: &TimeProfileDescriptor) -> Result<CanonicalTimeProfile> {
    let profile = admit_time_profile(descriptor).map_err(|issues| validation_error("fabric time profile", &issues))?;
    let value = time_profile_value(&profile);
    let profile_ref = canonical_hash(&value)?;
    Ok(CanonicalTimeProfile {
        profile,
        profile_ref,
        value,
    })
}

// r[impl molten.fabric_time.time_domains]
pub fn canonical_time_value(profile: &CanonicalTimeProfile, time: &TimeValue) -> Result<CanonicalTimeValue> {
    validate_time_value(&profile.profile, time).map_err(|error| validation_error("canonical time value", &[error]))?;
    let mut details = vec![
        field("profile-admission-ref", string(&profile.profile_ref)),
        field("profile-contract-ref", string(time.profile_ref())),
        field("domain", string(time.domain().as_str())),
        field("ticks", u64_value(time.ticks())),
    ];
    if let TimeValue::Wall(wall) = time {
        details.push(field("uncertainty-nanos", u64_value(wall.uncertainty_nanos)));
        details.push(field("observation-sequence", u64_value(wall.observation_sequence)));
    }
    details.push(checks(&["domain-explicit", "profile-bound", "checked-range"]));
    let value = record("fabric-time-value-v1", details);
    let value_ref = canonical_hash(&value)?;
    Ok(CanonicalTimeValue {
        time: time.clone(),
        value_ref,
        value,
    })
}

pub fn canonical_duration(profile: &CanonicalTimeProfile, duration: &CheckedDuration) -> Result<CanonicalDuration> {
    validate_duration(&profile.profile, duration).map_err(|error| validation_error("canonical duration", &[error]))?;
    let value = record("fabric-time-duration-v1", vec![
        field("profile-admission-ref", string(&profile.profile_ref)),
        field("profile-contract-ref", string(&duration.profile_ref)),
        field("domain", string(duration.domain.as_str())),
        field("ticks", u64_value(duration.ticks)),
        checks(&["domain-explicit", "profile-bound", "checked-range"]),
    ]);
    let value_ref = canonical_hash(&value)?;
    Ok(CanonicalDuration {
        duration: duration.clone(),
        value_ref,
        value,
    })
}

// r[impl molten.fabric_time.live_sim_parity]
pub fn fabric_time_port_descriptors(profile: &CanonicalTimeProfile) -> Vec<FabricPortDescriptor> {
    let (determinism, replay) = match profile.profile.kind {
        super::TimeProfileKind::Live => (DeterminismClass::ExternalEffect, ReplayClass::RecordedEffectRequired),
        super::TimeProfileKind::DeterministicSimulation => {
            (DeterminismClass::DeterministicWithRecordedInputs, ReplayClass::Recompute)
        }
    };
    vec![
        port_descriptor(
            FABRIC_CLOCK_PORT_ID,
            FabricPortClass::Time,
            &[
                "observe-wall",
                "observe-monotonic",
                "advance-logical",
                "advance-virtual",
                "convert-explicit",
            ],
            &[super::FABRIC_TIME_PROFILE_SCHEMA],
            &[super::FABRIC_TIME_OBSERVATION_SCHEMA],
            &[FabricAuthority::Time],
            &[FabricResource::LogicalTime],
            determinism,
            replay,
            profile,
        ),
        port_descriptor(
            FABRIC_TIMER_PORT_ID,
            FabricPortClass::Time,
            &["schedule", "poll", "cancel", "cleanup-generation"],
            &[super::FABRIC_TIME_PROFILE_SCHEMA],
            &[super::FABRIC_TIMER_EVENT_SCHEMA],
            &[FabricAuthority::Time],
            &[FabricResource::LogicalTime, FabricResource::QueueDepth],
            determinism,
            replay,
            profile,
        ),
        port_descriptor(
            FABRIC_SCHEDULER_PORT_ID,
            FabricPortClass::Scheduling,
            &["wake", "choose", "yield", "block", "cancel", "cleanup-generation"],
            &[super::FABRIC_TIME_PROFILE_SCHEMA],
            &[super::FABRIC_SCHEDULER_EVENT_SCHEMA],
            &[FabricAuthority::Scheduling],
            &[FabricResource::Concurrency, FabricResource::QueueDepth],
            determinism,
            replay,
            profile,
        ),
        port_descriptor(
            FABRIC_ENTROPY_PORT_ID,
            FabricPortClass::Time,
            &["open-purpose-stream", "draw-bytes", "draw-choice"],
            &[super::FABRIC_TIME_PROFILE_SCHEMA],
            &[super::FABRIC_ENTROPY_EVENT_SCHEMA],
            &[FabricAuthority::Time],
            &[FabricResource::Memory],
            determinism,
            replay,
            profile,
        ),
    ]
}

// r[impl molten.fabric_time.timers]
pub fn canonical_timer_event(profile_ref: &str, transition: &TimerTransition) -> Result<CanonicalTimeEvent> {
    canonical_event(
        profile_ref,
        CanonicalTimeEventKind::Timer,
        transition.next.key.generation,
        &transition.next.key.service_id,
        timer_action(transition.action),
        transition.next.next_deadline_ticks,
        vec![
            field("timer-sequence", u64_value(transition.next.key.sequence)),
            field("delivery-count", u64_value(transition.delivery_count)),
            field("skipped-count", u64_value(transition.skipped_count)),
            field("lateness-ticks", u64_value(transition.lateness_ticks)),
            field("fire-count", u64_value(transition.next.fire_count)),
            field("timer-slot-charge", u64_value(transition.next.resource_charge.timer_slots)),
            field("delivery-queue-unit-charge", u64_value(transition.next.resource_charge.delivery_queue_units)),
        ],
        &["generation-fenced", "duplicate-fire-checked", "resource-accounted"],
    )
}

// r[impl molten.fabric_time.scheduler]
pub fn canonical_scheduler_transition(
    profile_ref: &str,
    transition: &SchedulerTransition,
) -> Result<CanonicalTimeEvent> {
    canonical_event(
        profile_ref,
        CanonicalTimeEventKind::Scheduler,
        transition.runnable.generation,
        &transition.runnable.runnable_id,
        scheduler_action(transition.action),
        0,
        vec![field("service-id", string(&transition.runnable.service_id))],
        &["generation-fenced", "queue-bounded", "wake-transition-checked"],
    )
}

pub fn canonical_scheduler_selection(profile_ref: &str, selection: &SchedulerSelection) -> Result<CanonicalTimeEvent> {
    canonical_event(
        profile_ref,
        CanonicalTimeEventKind::Scheduler,
        selection.selected.generation,
        &selection.selected.runnable_id,
        "selected",
        selection.choice_sequence,
        vec![
            field("service-id", string(&selection.selected.service_id)),
            field("eligible-count", u64_value(selection.eligible_count)),
        ],
        &["choice-recorded", "replay-choice-checked", "concurrency-bounded"],
    )
}

// Entropy output bytes are intentionally absent. Only purpose, bounds, stream
// position, mode, and generation are evidence-bearing.
// r[impl molten.fabric_time.entropy]
pub fn canonical_entropy_event(metadata: &EntropyEvidenceMetadata) -> Result<CanonicalTimeEvent> {
    let expected_replay = match metadata.mode {
        EntropyMode::DeterministicSimulation => super::EntropyReplayClass::RecomputeFromExplicitSeed,
        EntropyMode::ProductionCryptographic => super::EntropyReplayClass::SecretInputRequired,
    };
    if metadata.replay_class != expected_replay {
        return Err(MoltenError::invalid_harness("entropy evidence mode and replay class mismatch"));
    }
    match metadata.mode {
        EntropyMode::DeterministicSimulation => {
            let input_ref = metadata
                .deterministic_input_ref
                .as_deref()
                .ok_or_else(|| MoltenError::invalid_harness("deterministic entropy evidence requires an input ref"))?;
            crate::preserves_rail::validate_content_ref(input_ref)?;
        }
        EntropyMode::ProductionCryptographic if metadata.deterministic_input_ref.is_some() => {
            return Err(MoltenError::invalid_harness(
                "production entropy evidence must not contain a deterministic input ref",
            ));
        }
        EntropyMode::ProductionCryptographic => {}
    }
    canonical_event(
        &metadata.profile_ref,
        CanonicalTimeEventKind::Entropy,
        metadata.generation,
        &metadata.stream_id,
        metadata.mode.as_str(),
        metadata.end_position_bytes,
        vec![
            field("purpose", string(&metadata.purpose)),
            field("start-position-bytes", u64_value(metadata.start_position_bytes)),
            field("request-bytes", u64_value(metadata.request_bytes)),
            field("replay-class", string(metadata.replay_class.as_str())),
            field("deterministic-input-ref", optional_string(metadata.deterministic_input_ref.as_deref())),
        ],
        &["purpose-bound", "generation-fenced", "secret-output-omitted"],
    )
}

// r[impl molten.fabric_time.deadline_lease]
pub fn canonical_deadline_event(profile_ref: &str, decision: &DeadlineDecision) -> Result<CanonicalTimeEvent> {
    canonical_event(
        profile_ref,
        CanonicalTimeEventKind::Deadline,
        decision.generation,
        &decision.subject_id,
        deadline_status(decision.status),
        decision.observed_ticks,
        vec![
            field("domain", string(decision.domain.as_str())),
            field("target-ticks", u64_value(decision.target_ticks)),
            field("uncertainty-ticks", u64_value(decision.uncertainty_ticks)),
        ],
        &["domain-checked", "uncertainty-explicit", "local-decision-only"],
    )
}

pub fn canonical_lease_event(profile_ref: &str, decision: &LeaseDecision) -> Result<CanonicalTimeEvent> {
    canonical_event(
        profile_ref,
        CanonicalTimeEventKind::Lease,
        decision.generation,
        &decision.lease_id,
        lease_decision(decision.kind),
        0,
        vec![
            field("owner-id", string(&decision.owner_id)),
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
    decision: &WallClockAnomalyDecision,
) -> Result<CanonicalTimeEvent> {
    canonical_event(
        profile_ref,
        CanonicalTimeEventKind::ClockAnomaly,
        generation,
        "wall-clock",
        clock_anomaly(decision.kind),
        decision.observed_unix_nanos,
        vec![
            field("previous-unix-nanos", u64_value(decision.previous_unix_nanos)),
            field("delta-nanos", u64_value(decision.delta_nanos)),
        ],
        &["wall-clock-untrusted", "anomaly-classified", "no-global-time-claim"],
    )
}

pub fn canonical_named_event(
    profile_ref: &str,
    kind: CanonicalTimeEventKind,
    generation: u64,
    subject: &str,
    action: &str,
    ticks: u64,
) -> Result<CanonicalTimeEvent> {
    canonical_event(profile_ref, kind, generation, subject, action, ticks, Vec::new(), &[
        "explicit-input",
        "generation-bound",
        "bounded-evidence",
    ])
}

// r[impl molten.fabric_time.evidence]
// r[impl molten.fabric_time.non_claims]
pub fn canonical_fabric_time_run(report: FabricTimeRunReport) -> Result<CanonicalFabricTimeRun> {
    if report.generation == 0 {
        return Err(MoltenError::invalid_harness("fabric time run generation must be non-zero"));
    }
    if !matches!(report.profile_kind.as_str(), "live" | "deterministic-simulation" | "both") {
        return Err(MoltenError::invalid_harness(format!(
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
        return Err(MoltenError::invalid_harness(format!(
            "fabric time run evidence count {} exceeds {}",
            report.evidence_refs.len(),
            MAX_RUN_EVIDENCE_REFS
        )));
    }
    if report.non_claims != super::REQUIRED_TIME_NON_CLAIMS {
        return Err(MoltenError::invalid_harness("fabric time run must preserve the complete canonical non-claim set"));
    }
    let value = record(FABRIC_TIME_RUN_RECORD, vec![
        string(super::FABRIC_TIME_RUN_REPORT_SCHEMA),
        field("profile-ref", string(&report.profile_ref)),
        field("profile-kind", string(&report.profile_kind)),
        field("generation", u64_value(report.generation)),
        field("initial-state-ref", string(&report.initial_state_ref)),
        field("scheduler-trace-ref", string(&report.scheduler_trace_ref)),
        field("entropy-trace-ref", string(&report.entropy_trace_ref)),
        field("fault-plan-ref", string(&report.fault_plan_ref)),
        field("terminal-outcome-ref", string(&report.terminal_outcome_ref)),
        field("final-time-ticks", u64_value(report.final_time_ticks)),
        field("timer-events", u64_value(report.timer_events)),
        field("scheduler-events", u64_value(report.scheduler_events)),
        field("entropy-events", u64_value(report.entropy_events)),
        field("deadline-lease-events", u64_value(report.deadline_lease_events)),
        field("fault-events", u64_value(report.fault_events)),
        field("live-clock-observed", bool_value(report.live_clock_observed)),
        field("shared-conformance-passed", bool_value(report.shared_conformance_passed)),
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
    let report_ref = canonical_hash(&value)?;
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

// r[impl molten.fabric_time.evidence]
pub fn parse_fabric_time_run_readback(value: &IOValue) -> Result<FabricTimeRunReadback> {
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
        .ok_or_else(|| MoltenError::invalid_harness("expected canonical fabric-time run report"))?;
    let schema = required_string(&fields[0], "fabric-time report schema")?;
    if schema != super::FABRIC_TIME_RUN_REPORT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!("fabric-time report schema mismatch: {schema}")));
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
        return Err(MoltenError::invalid_harness(format!("unsupported fabric-time profile kind: {profile_kind}")));
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
        report_ref: canonical_hash(value)?,
    })
}

pub fn canonical_time_trace_ref(trace_kind: &str, evidence_refs: &[String]) -> Result<String> {
    let value = record("fabric-time-trace-v1", vec![
        field("trace-kind", string(trace_kind)),
        field("evidence-refs", strings_value(evidence_refs.iter().map(String::as_str))),
        checks(&["ordered-canonical-refs", "bounded-trace-summary"]),
    ]);
    canonical_hash(&value)
}

fn time_profile_value(profile: &AdmittedTimeProfile) -> IOValue {
    record(FABRIC_TIME_PROFILE_RECORD, vec![
        string(super::FABRIC_TIME_PROFILE_SCHEMA),
        field("profile-id", string(&profile.profile_id)),
        field("declared-profile-ref", string(&profile.profile_ref)),
        field("kind", string(profile.kind.as_str())),
        field("domains", strings_value(profile.supported_domains.iter().map(|domain| domain.as_str()))),
        field("max-duration-ticks", u64_value(profile.max_duration_ticks)),
        field("max-uncertainty-ticks", u64_value(profile.max_uncertainty_ticks)),
        field("max-timers", u64_value(profile.max_timers)),
        field("max-runnables", u64_value(profile.max_runnables)),
        field("max-entropy-request-bytes", u64_value(profile.max_entropy_request_bytes)),
        field("max-entropy-total-bytes", u64_value(profile.max_entropy_total_bytes)),
        field("max-scheduler-concurrency", u64_value(profile.max_scheduler_concurrency)),
        field("max-scheduler-queue-depth", u64_value(profile.max_scheduler_queue_depth)),
        field("fairness-bound-turns", optional_u64(profile.fairness_bound_turns)),
        field("scheduler-ordering", string(profile.scheduler_policy.ordering.as_str())),
        field("scheduler-replay", string(profile.scheduler_policy.replay.as_str())),
        field("scheduler-overload", string(profile.scheduler_policy.overload.as_str())),
        field("evidence-mode", string(profile.evidence_mode.as_str())),
        field("non-claims", strings_value(profile.non_claims.iter().map(|claim| claim.as_str()))),
        checks(&[
            "canonical-profile",
            "exact-mode",
            "bounded-resources",
            "non-claims-complete",
        ]),
    ])
}

fn port_descriptor(
    port_id: &str,
    class: FabricPortClass,
    operations: &[&str],
    input_schemas: &[&str],
    output_schemas: &[&str],
    authorities: &[FabricAuthority],
    resources: &[FabricResource],
    determinism: DeterminismClass,
    replay: ReplayClass,
    profile: &CanonicalTimeProfile,
) -> FabricPortDescriptor {
    FabricPortDescriptor {
        schema: FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
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
        non_claims: REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
        enabled: true,
    }
}

fn canonical_event(
    profile_ref: &str,
    kind: CanonicalTimeEventKind,
    generation: u64,
    subject: &str,
    action: &str,
    ticks: u64,
    details: Vec<IOValue>,
    event_checks: &[&str],
) -> Result<CanonicalTimeEvent> {
    if generation == 0 {
        return Err(MoltenError::invalid_harness("fabric time event generation must be non-zero"));
    }
    let value = record(FABRIC_TIME_EVENT_RECORD, vec![
        string(super::FABRIC_TIME_OBSERVATION_SCHEMA),
        field("profile-ref", string(profile_ref)),
        field("kind", string(kind.as_str())),
        field("generation", u64_value(generation)),
        field("subject", string(subject)),
        field("action", string(action)),
        field("ticks", u64_value(ticks)),
        field("details", sequence(details)),
        checks(event_checks),
    ]);
    let evidence_ref = canonical_hash(&value)?;
    Ok(CanonicalTimeEvent {
        evidence_ref,
        profile_ref: profile_ref.to_string(),
        kind,
        generation,
        value,
    })
}

fn field(name: &str, value: IOValue) -> IOValue {
    record("field", vec![string(name), value])
}

fn checks(values: &[&str]) -> IOValue {
    field("checks", strings_value(values.iter().copied()))
}

fn strings_value<'a>(values: impl Iterator<Item = &'a str>) -> IOValue {
    sequence(values.map(string).collect())
}

fn optional_u64(value: Option<u64>) -> IOValue {
    match value {
        Some(value) => record("some", vec![u64_value(value)]),
        None => record("none", Vec::new()),
    }
}

fn optional_string(value: Option<&str>) -> IOValue {
    match value {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
}

fn timer_action(action: TimerAction) -> &'static str {
    match action {
        TimerAction::NotDue => "not-due",
        TimerAction::Deliver => "deliver",
        TimerAction::Coalesced => "coalesced",
        TimerAction::DroppedLate => "dropped-late",
        TimerAction::DroppedOverload => "dropped-overload",
        TimerAction::Backpressure => "backpressure",
        TimerAction::RetainedOverload => "retained-overload",
        TimerAction::Cancelled => "cancelled",
        TimerAction::DiscardedStaleGeneration => "discarded-stale-generation",
    }
}

fn scheduler_action(action: SchedulerAction) -> &'static str {
    match action {
        SchedulerAction::Woken => "woken",
        SchedulerAction::Yielded => "yielded",
        SchedulerAction::Blocked => "blocked",
        SchedulerAction::Completed => "completed",
        SchedulerAction::Cancelled => "cancelled",
        SchedulerAction::RejectedOverload => "rejected-overload",
        SchedulerAction::Backpressure => "backpressure",
        SchedulerAction::DiscardedStaleGeneration => "discarded-stale-generation",
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

fn record_string_field(value: &preserves::Value<IOValue>, label: &str) -> Result<String> {
    let field_value = named_field_value(value, label)?;
    required_string(&field_value, label)
}

fn record_u64_field(value: &preserves::Value<IOValue>, label: &str) -> Result<u64> {
    let field_value = named_field_value(value, label)?;
    field_value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {label}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {label}: {error}")))
}

fn record_bool_field(value: &preserves::Value<IOValue>, label: &str) -> Result<bool> {
    named_field_value(value, label)?
        .as_boolean()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected bool for {label}")))
}

fn named_field_value(value: &preserves::Value<IOValue>, label: &str) -> Result<preserves::Value<IOValue>> {
    const NAMED_FIELD_ARITY: usize = 2;
    let fields = value
        .collect_simple_record("field", Some(NAMED_FIELD_ARITY))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected named field {label}")))?;
    let actual = required_string(&fields[0], "field-name")?;
    if actual != label {
        return Err(MoltenError::invalid_harness(format!("expected field {label}, found {actual}")));
    }
    Ok(fields[1].clone())
}

fn required_string(value: &preserves::Value<IOValue>, label: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {label}")))
}

fn validation_error<T: std::fmt::Debug>(label: &str, issues: &[T]) -> MoltenError {
    MoltenError::invalid_harness(format!("{label} validation failed: {issues:?}"))
}
