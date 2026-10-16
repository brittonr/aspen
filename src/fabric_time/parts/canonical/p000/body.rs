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
pub struct Profile {
    pub profile: super::AdmittedTimeProfile,
    pub profile_ref: String,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Value {
    pub time: super::TimeValue,
    pub value_ref: String,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Duration {
    pub duration: super::CheckedDuration,
    pub value_ref: String,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    ClockAnomaly,
    Timer,
    Scheduler,
    Entropy,
    Deadline,
    Lease,
    Fault,
    Conformance,
}

impl EventKind {
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
pub struct Event {
    pub evidence_ref: String,
    pub profile_ref: String,
    pub kind: EventKind,
    pub generation: u64,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunReport {
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
    pub non_claims: Vec<super::TimeNonClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub report_ref: String,
    pub report: RunReport,
    pub value: preserves::IOValue,
}

// r[impl molten.modularity.fabric_boundary.compatibility]
// r[impl molten.fabric_time.evidence]
pub fn admit_profile(
    descriptor: &super::TimeProfileDescriptor,
) -> crate::error::Result<Profile> {
    let profile =
        super::admit_time_profile(descriptor).map_err(|issues| validation_error("fabric time profile", &issues))?;
    let value = profile_value(&profile);
    let profile_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(Profile {
        profile,
        profile_ref,
        value,
    })
}

// r[impl molten.fabric_time.time_domains]
pub fn value(
    profile: &Profile,
    time: &super::TimeValue,
) -> crate::error::Result<Value> {
    super::validate_time_value(&profile.profile, time)
        .map_err(|error| validation_error("canonical time value", &[error]))?;
    let mut details = vec![
        field("profile-admission-ref", crate::preserves_rail::string(&profile.profile_ref)),
        field("profile-contract-ref", crate::preserves_rail::string(time.profile_ref())),
        field("domain", crate::preserves_rail::string(time.domain().as_str())),
        field("ticks", crate::preserves_rail::u64_value(time.ticks())),
    ];
    if let super::TimeValue::Wall(wall) = time {
        details.push(field("uncertainty-nanos", crate::preserves_rail::u64_value(wall.uncertainty_nanos)));
        details.push(field("observation-sequence", crate::preserves_rail::u64_value(wall.observation_sequence)));
    }
    details.push(checks(&["domain-explicit", "profile-bound", "checked-range"]));
    let value = crate::preserves_rail::record("fabric-time-value-v1", details);
    let value_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(Value {
        time: time.clone(),
        value_ref,
        value,
    })
}

pub fn duration(
    profile: &Profile,
    duration: &super::CheckedDuration,
) -> crate::error::Result<Duration> {
    super::validate_duration(&profile.profile, duration)
        .map_err(|error| validation_error("canonical duration", &[error]))?;
    let value = crate::preserves_rail::record("fabric-time-duration-v1", vec![
        field("profile-admission-ref", crate::preserves_rail::string(&profile.profile_ref)),
        field("profile-contract-ref", crate::preserves_rail::string(&duration.profile_ref)),
        field("domain", crate::preserves_rail::string(duration.domain.as_str())),
        field("ticks", crate::preserves_rail::u64_value(duration.ticks)),
        checks(&["domain-explicit", "profile-bound", "checked-range"]),
    ]);
    let value_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(Duration {
        duration: duration.clone(),
        value_ref,
        value,
    })
}

// r[impl molten.fabric_time.live_sim_parity]
pub fn port_descriptors(profile: &Profile) -> Vec<crate::fabric::FabricPortDescriptor> {
    let (determinism, replay) = replay_classes(profile.profile.kind);
    vec![
        port_descriptor(PortDescriptorInput {
            port_id: FABRIC_CLOCK_PORT_ID,
            class: crate::fabric::FabricPortClass::Time,
            operations: &[
                "observe-wall",
                "observe-monotonic",
                "advance-logical",
                "advance-virtual",
                "convert-explicit",
            ],
            input_schemas: &[super::FABRIC_TIME_PROFILE_SCHEMA],
            output_schemas: &[super::FABRIC_TIME_OBSERVATION_SCHEMA],
            authorities: &[crate::fabric::FabricAuthority::Time],
            resources: &[crate::fabric::FabricResource::LogicalTime],
            determinism,
            replay,
            profile,
        }),
        port_descriptor(PortDescriptorInput {
            port_id: FABRIC_TIMER_PORT_ID,
            class: crate::fabric::FabricPortClass::Time,
            operations: &["schedule", "poll", "cancel", "cleanup-generation"],
            input_schemas: &[super::FABRIC_TIME_PROFILE_SCHEMA],
            output_schemas: &[super::FABRIC_TIMER_EVENT_SCHEMA],
            authorities: &[crate::fabric::FabricAuthority::Time],
            resources: &[
                crate::fabric::FabricResource::LogicalTime,
                crate::fabric::FabricResource::QueueDepth,
            ],
            determinism,
            replay,
            profile,
        }),
        port_descriptor(PortDescriptorInput {
            port_id: FABRIC_SCHEDULER_PORT_ID,
            class: crate::fabric::FabricPortClass::Scheduling,
            operations: &["wake", "choose", "yield", "block", "cancel", "cleanup-generation"],
            input_schemas: &[super::FABRIC_TIME_PROFILE_SCHEMA],
            output_schemas: &[super::FABRIC_SCHEDULER_EVENT_SCHEMA],
            authorities: &[crate::fabric::FabricAuthority::Scheduling],
            resources: &[
                crate::fabric::FabricResource::Concurrency,
                crate::fabric::FabricResource::QueueDepth,
            ],
            determinism,
            replay,
            profile,
        }),
        port_descriptor(PortDescriptorInput {
            port_id: FABRIC_ENTROPY_PORT_ID,
            class: crate::fabric::FabricPortClass::Time,
            operations: &["open-purpose-stream", "draw-bytes", "draw-choice"],
            input_schemas: &[super::FABRIC_TIME_PROFILE_SCHEMA],
            output_schemas: &[super::FABRIC_ENTROPY_EVENT_SCHEMA],
            authorities: &[crate::fabric::FabricAuthority::Time],
            resources: &[crate::fabric::FabricResource::Memory],
            determinism,
            replay,
            profile,
        }),
    ]
}

fn replay_classes(kind: super::TimeProfileKind) -> (crate::fabric::DeterminismClass, crate::fabric::ReplayClass) {
    match kind {
        super::TimeProfileKind::Live => {
            (crate::fabric::DeterminismClass::ExternalEffect, crate::fabric::ReplayClass::RecordedEffectRequired)
        }
        super::TimeProfileKind::DeterministicSimulation => (
            crate::fabric::DeterminismClass::DeterministicWithRecordedInputs,
            crate::fabric::ReplayClass::Recompute,
        ),
    }
}

// r[impl molten.fabric_time.timers]
pub fn timer_event(
    profile_ref: &str,
    transition: &super::TimerTransition,
) -> crate::error::Result<Event> {
    event(
        EventHeader {
            profile_ref,
            kind: EventKind::Timer,
            generation: transition.next.key.generation,
            subject: &transition.next.key.service_id,
            action: timer_action(transition.action),
            ticks: transition.next.next_deadline_ticks,
        },
        vec![
            field("timer-sequence", crate::preserves_rail::u64_value(transition.next.key.sequence)),
            field("delivery-count", crate::preserves_rail::u64_value(transition.delivery_count)),
            field("skipped-count", crate::preserves_rail::u64_value(transition.skipped_count)),
            field("lateness-ticks", crate::preserves_rail::u64_value(transition.lateness_ticks)),
            field("fire-count", crate::preserves_rail::u64_value(transition.next.fire_count)),
            field("timer-slot-charge", crate::preserves_rail::u64_value(transition.next.resource_charge.timer_slots)),
            field(
                "delivery-queue-unit-charge",
                crate::preserves_rail::u64_value(transition.next.resource_charge.delivery_queue_units),
            ),
        ],
        &["generation-fenced", "duplicate-fire-checked", "resource-accounted"],
    )
}
