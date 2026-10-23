use super::*;

impl NoveltyField {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Service => "service",
            Self::Participant => "participant",
            Self::TransitionClass => "transition-class",
            Self::LogicalPosition => "logical-position",
            Self::State => "state",
            Self::Durability => "durability",
            Self::Progress => "progress",
        }
    }
}

/// The stable novelty identity of a projection over the profile's fields.
///
/// The identity is a domain-separated BLAKE3 ref over the canonical encoding
/// of the selected fields, so equal selected protocol state yields equal
/// identity across runs, processes, and builds. A process-local hash or
/// coverage slot is never a durable novelty identity.
// r[impl molten.fabric_simulation.protocol_novelty]
pub fn novelty_identity(fields: &[NoveltyField], projection: &ProtocolProjection) -> String {
    let mut selected = fields.to_vec();
    selected.sort_unstable();
    selected.dedup();
    let values = selected
        .iter()
        .map(|field| {
            let value = match field {
                NoveltyField::Service => CanonicalValue::symbol(&projection.service),
                NoveltyField::Participant => CanonicalValue::string(&projection.participant),
                NoveltyField::TransitionClass => CanonicalValue::symbol(projection.transition_class.as_str()),
                NoveltyField::LogicalPosition => CanonicalValue::unsigned(projection.logical_position),
                NoveltyField::State => CanonicalValue::string(&projection.state_ref),
                NoveltyField::Durability => CanonicalValue::symbol(projection.durability.as_str()),
                NoveltyField::Progress => CanonicalValue::unsigned(projection.progress),
            };
            CanonicalValue::record("field", vec![CanonicalValue::symbol(field.as_str()), value])
        })
        .collect();
    let value = CanonicalValue::record("protocol-novelty", vec![
        CanonicalValue::string(PROJECTION_SCHEMA),
        CanonicalValue::Sequence(values),
    ]);
    domain_ref(NOVELTY_DOMAIN, &encode_canonical(&value))
}

/// The distinct novelty identities of a cohort, in first-seen order.
pub fn cohort_novelty(fields: &[NoveltyField], cohort: &ProtocolCohort) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    let mut identities = Vec::with_capacity(cohort.projections.len());
    for admitted in &cohort.projections {
        let identity = novelty_identity(fields, &admitted.projection);
        if seen.insert(identity.clone()) {
            identities.push(identity);
        }
    }
    identities
}

/// How much new protocol state one run reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoveltyScore {
    pub new: u64,
    pub repeated: u64,
}

/// Scheduler guidance over durable novelty identities.
///
/// An exploration driver records each run's novelty identities and prefers
/// schedules that reach protocol state no earlier run reached. The identities
/// are the full canonical refs, so guidance replays across processes.
// r[impl molten.fabric_simulation.protocol_novelty]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NoveltyGuidance {
    seen: std::collections::BTreeSet<String>,
}

impl NoveltyGuidance {
    pub const fn new() -> Self {
        Self {
            seen: std::collections::BTreeSet::new(),
        }
    }

    /// Record one run's identities and score how many were new.
    pub fn record(&mut self, identities: &[String]) -> NoveltyScore {
        let mut score = NoveltyScore { new: 0, repeated: 0 };
        for identity in identities {
            if self.seen.insert(identity.clone()) {
                score.new = score.new.saturating_add(1);
            } else {
                score.repeated = score.repeated.saturating_add(1);
            }
        }
        score
    }

    pub fn seen(&self) -> &std::collections::BTreeSet<String> {
        &self.seen
    }
}
