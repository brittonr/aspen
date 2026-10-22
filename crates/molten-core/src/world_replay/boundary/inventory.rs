use super::super::*;
use super::*;

/// The kind of a nondeterministic source at the semantic host boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReplaySourceKind {
    Hostcall,
    EffectPort,
    SchedulerDecision,
    AsyncEvent,
    Clock,
    Entropy,
    ExternalRead,
    RuntimeObservation,
}

impl ReplaySourceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Hostcall => "hostcall",
            Self::EffectPort => "effect-port",
            Self::SchedulerDecision => "scheduler-decision",
            Self::AsyncEvent => "async-event",
            Self::Clock => "clock",
            Self::Entropy => "entropy",
            Self::ExternalRead => "external-read",
            Self::RuntimeObservation => "runtime-observation",
        }
    }
}

/// How replay treats a source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReplaySourceHandling {
    /// The source is a pure function of replayed state.
    Deterministic,
    /// A seeded or virtual model reproduces the source, such as virtual time.
    Simulated,
    /// Replay returns the sealed recorded observation.
    RecordedObservation,
    /// The profile cannot replay the source; reaching it blocks replay.
    Unsupported,
}

impl ReplaySourceHandling {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Deterministic => "deterministic",
            Self::Simulated => "simulated",
            Self::RecordedObservation => "recorded-observation",
            Self::Unsupported => "unsupported",
        }
    }
}

/// One inventory row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaySource {
    pub source_id: String,
    pub kind: ReplaySourceKind,
    pub handling: ReplaySourceHandling,
    /// The source writes, sends, publishes, releases, or calls outside the
    /// world. Replay never repeats it.
    pub is_external_mutation: bool,
    /// Recorded values from the source must not be stored as public plaintext.
    pub is_sensitive: bool,
}

/// The closed source inventory of one replay profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaySourceInventory {
    pub schema: String,
    pub inventory_ref: String,
    /// The replay profile the inventory belongs to.
    pub profile_ref: String,
    /// The adapter cohort that records and replays observations.
    pub adapter_cohort_ref: String,
    /// The protection profile recorded observations follow.
    pub protection_profile_ref: String,
    /// The one opaque-diagnostic cohort the profile admits, if any.
    pub opaque_diagnostic_cohort_ref: Option<String>,
    /// The sources in canonical identifier order.
    pub sources: Vec<ReplaySource>,
}

impl ReplaySourceInventory {
    pub fn source(&self, source_id: &str) -> Option<&ReplaySource> {
        self.sources
            .binary_search_by(|source| source.source_id.as_str().cmp(source_id))
            .ok()
            .map(|index| &self.sources[index])
    }
}

// r[impl molten.world_replay_boundary.inventory]
pub fn identify_replay_source_inventory(inventory: &ReplaySourceInventory) -> Result<String, WorldReplayIssue> {
    let mut hasher = blake3::Hasher::new_derive_key(REPLAY_SOURCE_INVENTORY_IDENTITY_DOMAIN);
    update_text(&mut hasher, &inventory.schema)?;
    update_text(&mut hasher, &inventory.profile_ref)?;
    update_text(&mut hasher, &inventory.adapter_cohort_ref)?;
    update_text(&mut hasher, &inventory.protection_profile_ref)?;
    update_optional_text(&mut hasher, inventory.opaque_diagnostic_cohort_ref.as_deref())?;
    update_usize(&mut hasher, inventory.sources.len())?;
    for source in &inventory.sources {
        update_text(&mut hasher, &source.source_id)?;
        update_text(&mut hasher, source.kind.as_str())?;
        update_text(&mut hasher, source.handling.as_str())?;
        update_bool(&mut hasher, source.is_external_mutation);
        update_bool(&mut hasher, source.is_sensitive);
    }
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

/// Validate that the inventory is closed, canonical, and self-identified.
// r[impl molten.world_replay_boundary.inventory]
pub fn validate_replay_source_inventory(inventory: &ReplaySourceInventory) -> Vec<ReplayBoundaryBlocker> {
    let mut blockers = Vec::with_capacity(MAX_REPLAY_BOUNDARY_BLOCKERS);
    if inventory.schema != REPLAY_SOURCE_INVENTORY_SCHEMA {
        blockers.push(ReplayBoundaryBlocker::InvalidRecord("inventory-schema"));
    }
    let references = [
        (&inventory.profile_ref, "inventory-profile-ref"),
        (&inventory.adapter_cohort_ref, "inventory-adapter-cohort-ref"),
        (&inventory.protection_profile_ref, "inventory-protection-profile-ref"),
    ];
    for (reference, label) in references {
        if !valid_reference(reference) {
            blockers.push(ReplayBoundaryBlocker::InvalidRecord(label));
        }
    }
    if inventory
        .opaque_diagnostic_cohort_ref
        .as_deref()
        .is_some_and(|reference| !valid_reference(reference))
    {
        blockers.push(ReplayBoundaryBlocker::InvalidRecord("inventory-opaque-cohort-ref"));
    }
    if inventory.sources.is_empty() || inventory.sources.len() > MAX_REPLAY_SOURCES {
        blockers.push(ReplayBoundaryBlocker::InvalidRecord("inventory-source-count"));
    }
    blockers.extend(source_blockers(&inventory.sources));
    match identify_replay_source_inventory(inventory) {
        Ok(identity) if identity == inventory.inventory_ref => {}
        _ => blockers.push(ReplayBoundaryBlocker::InventoryIdentityMismatch),
    }
    blockers
}

fn source_blockers(sources: &[ReplaySource]) -> Vec<ReplayBoundaryBlocker> {
    let mut blockers = Vec::with_capacity(MAX_REPLAY_BOUNDARY_BLOCKERS);
    for pair in sources.windows(2) {
        match pair[0].source_id.cmp(&pair[1].source_id) {
            core::cmp::Ordering::Less => {}
            core::cmp::Ordering::Equal => {
                blockers.push(ReplayBoundaryBlocker::DuplicateSource(pair[1].source_id.clone()))
            }
            core::cmp::Ordering::Greater => blockers.push(ReplayBoundaryBlocker::NonCanonicalSourceOrder),
        }
    }
    for source in sources {
        if !valid_boundary_text(&source.source_id) {
            blockers.push(ReplayBoundaryBlocker::InvalidRecord("source-id"));
        }
        let is_sealed =
            matches!(source.handling, ReplaySourceHandling::RecordedObservation | ReplaySourceHandling::Unsupported);
        if source.is_external_mutation && !is_sealed {
            blockers.push(ReplayBoundaryBlocker::MutationNotSealed(source.source_id.clone()));
        }
    }
    blockers
}

/// Bounded identifier text: lowercase ASCII letters, digits, and `:._-`.
pub(crate) fn valid_boundary_text(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_REPLAY_TEXT_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b':' | b'.' | b'_' | b'-'))
}
