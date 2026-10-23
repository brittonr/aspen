use super::*;

/// How a transition changed the participant's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TransitionClass {
    /// The participant's state before its first transition.
    Initial,
    Applied,
    Conflict,
    Denied,
    /// The state was rebuilt from durable storage after a crash.
    Recovered,
}

impl TransitionClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Initial => "initial",
            Self::Applied => "applied",
            Self::Conflict => "conflict",
            Self::Denied => "denied",
            Self::Recovered => "recovered",
        }
    }
}

/// What the participant knows about the durability of the projected state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DurabilityFact {
    Volatile,
    Durable,
    Recovered,
}

impl DurabilityFact {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Volatile => "volatile",
            Self::Durable => "durable",
            Self::Recovered => "recovered",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationCompleteness {
    Complete,
    /// The participant could not observe every fact the contract requires.
    Partial,
}

/// One canonical projection of a participant's admitted state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolProjection {
    pub protocol: String,
    pub service: String,
    pub participant: String,
    pub generation: u64,
    /// The participant's own ordinal, from zero, with no gaps when lossless.
    pub source_sequence: u64,
    /// The run-wide observation sequence that orders projections across participants.
    pub run_sequence: u64,
    pub scheduler_choice_ref: String,
    pub transition_class: TransitionClass,
    pub logical_position: u64,
    pub state: CanonicalValue,
    pub state_ref: String,
    /// The physical layout ref; compared only under a canonical-physical profile.
    pub physical_ref: Option<String>,
    pub history_ref: String,
    pub durability: DurabilityFact,
    pub progress: u64,
    pub completeness: ObservationCompleteness,
    /// The transition's own report. Carried for diagnosis; never evidence.
    pub reported_invariants: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProjectionIssue {
    UnknownService(String),
    UnknownParticipant(String),
    ServiceMismatch {
        participant: String,
        service: String,
    },
    /// The state ref is not the canonical ref of the projected state, as when
    /// it was derived from debug text.
    NonCanonicalStateIdentity,
    MalformedRef(&'static str),
    UnboundedPayload,
}

/// The field names of a projection record, in encoding order.
pub const PROJECTION_FIELDS: &[&str] = &[
    "schema",
    "protocol",
    "service",
    "participant",
    "generation",
    "source-sequence",
    "run-sequence",
    "scheduler-choice-ref",
    "transition-class",
    "logical-position",
    "state",
    "state-ref",
    "physical-ref",
    "history-ref",
    "durability",
    "progress",
    "completeness",
    "reported-invariants",
];

/// The canonical ref of a projected state.
pub fn state_ref_of(state: &CanonicalValue) -> String {
    domain_ref(STATE_DOMAIN, &encode_canonical(state))
}

/// The canonical value of a projection.
pub fn projection_value(projection: &ProtocolProjection) -> CanonicalValue {
    CanonicalValue::record(PROJECTION_LABEL, vec![
        CanonicalValue::string(PROJECTION_SCHEMA),
        CanonicalValue::string(&projection.protocol),
        CanonicalValue::symbol(&projection.service),
        CanonicalValue::string(&projection.participant),
        CanonicalValue::unsigned(projection.generation),
        CanonicalValue::unsigned(projection.source_sequence),
        CanonicalValue::unsigned(projection.run_sequence),
        CanonicalValue::string(&projection.scheduler_choice_ref),
        CanonicalValue::symbol(projection.transition_class.as_str()),
        CanonicalValue::unsigned(projection.logical_position),
        projection.state.clone(),
        CanonicalValue::string(&projection.state_ref),
        CanonicalValue::optional(projection.physical_ref.as_deref().map(CanonicalValue::string)),
        CanonicalValue::string(&projection.history_ref),
        CanonicalValue::symbol(projection.durability.as_str()),
        CanonicalValue::unsigned(projection.progress),
        CanonicalValue::symbol(match projection.completeness {
            ObservationCompleteness::Complete => "complete",
            ObservationCompleteness::Partial => "partial",
        }),
        CanonicalValue::Sequence(
            projection.reported_invariants.iter().map(|name| CanonicalValue::string(name)).collect(),
        ),
    ])
}

/// The domain-separated canonical ref of a projection.
// r[impl molten.fabric_simulation.protocol_projection]
pub fn projection_ref(projection: &ProtocolProjection) -> String {
    domain_ref(PROJECTION_DOMAIN, &encode_canonical(&projection_value(projection)))
}

/// Admit one projection under a profile.
///
/// The participant and service must be declared, every ref must be canonical,
/// the state must fit the profile's bound, and the state ref must be the
/// canonical ref of the projected state: a ref derived any other way, such as
/// from debug text, is refused.
// r[impl molten.fabric_simulation.protocol_projection]
pub fn admit_projection(
    profile: &ProtocolOracleProfile,
    projection: &ProtocolProjection,
) -> Result<u64, ProjectionIssue> {
    if profile.service(&projection.service).is_none() {
        return Err(ProjectionIssue::UnknownService(projection.service.clone()));
    }
    let participant = profile
        .participant(&projection.participant)
        .ok_or_else(|| ProjectionIssue::UnknownParticipant(projection.participant.clone()))?;
    if participant.service != projection.service {
        return Err(ProjectionIssue::ServiceMismatch {
            participant: projection.participant.clone(),
            service: projection.service.clone(),
        });
    }
    for (field, reference) in [
        ("scheduler-choice", projection.scheduler_choice_ref.as_str()),
        ("history", projection.history_ref.as_str()),
        ("state", projection.state_ref.as_str()),
    ] {
        if !is_content_ref(reference) {
            return Err(ProjectionIssue::MalformedRef(field));
        }
    }
    if projection.physical_ref.as_deref().is_some_and(|reference| !is_content_ref(reference)) {
        return Err(ProjectionIssue::MalformedRef("physical"));
    }
    let state_bytes = encode_canonical(&projection.state);
    let is_bounded = u32::try_from(state_bytes.len()).is_ok_and(|length| length <= profile.bounds.max_state_bytes);
    if !is_bounded {
        return Err(ProjectionIssue::UnboundedPayload);
    }
    if projection.state_ref != domain_ref(STATE_DOMAIN, &state_bytes) {
        return Err(ProjectionIssue::NonCanonicalStateIdentity);
    }
    u64::try_from(encode_canonical(&projection_value(projection)).len()).map_err(|_| ProjectionIssue::UnboundedPayload)
}
