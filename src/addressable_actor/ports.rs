use molten_core::addressable_actor::*;

const BLAKE3_REFERENCE_PREFIX: &str = "blake3:";
const BLAKE3_HEX_LENGTH: usize = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortError {
    pub code: &'static str,
    pub detail: String,
    pub outcome_unknown: bool,
}

impl PortError {
    pub fn new(code: &'static str, detail: impl Into<String>, outcome_unknown: bool) -> Self {
        Self {
            code,
            detail: detail.into(),
            outcome_unknown,
        }
    }
}

pub type PortResult<T> = std::result::Result<T, PortError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublishedState {
    pub state: ActorState,
    pub state_ref: String,
    pub revision: u64,
}

impl PublishedState {
    #[must_use]
    pub fn from_state(state: ActorState) -> Self {
        let state_ref = identify_actor_state(&state);
        let revision = state.revision;
        Self {
            state,
            state_ref,
            revision,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpectedState {
    pub state_ref: Option<String>,
    pub revision: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitRequest {
    pub actor_key_ref: String,
    pub expected: ExpectedState,
    pub next: PublishedState,
    pub requested_engine_epoch: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommitDisposition {
    Applied,
    AlreadyApplied,
    Stale,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommitCurrentness {
    Linearizable,
    Stale,
    Unknown,
}

impl CommitCurrentness {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Linearizable => "linearizable",
            Self::Stale => "stale",
            Self::Unknown => "unknown",
        }
    }

    pub const fn is_current(self) -> bool {
        matches!(self, Self::Linearizable)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DurabilityOutcome {
    Durable,
    Buffered,
    Unknown,
}

impl DurabilityOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Durable => "durable",
            Self::Buffered => "buffered",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitObservation {
    pub disposition: CommitDisposition,
    pub currentness: CommitCurrentness,
    pub durability: DurabilityOutcome,
    pub engine_epoch: u64,
    pub observed_state_ref: Option<String>,
}

pub trait ActorCommitPort {
    fn load(&self, actor_key_ref: &str) -> PortResult<Option<PublishedState>>;

    fn compare_and_commit(&mut self, request: &CommitRequest) -> PortResult<CommitObservation>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectAdmissionObservation {
    pub admission_ref: String,
    pub actor_key_ref: String,
    pub profile_ref: String,
    pub system_extension_manifest_ref: String,
    pub placement_ref: String,
    pub extension_generation: u64,
    pub policy_current: bool,
    pub capability_current: bool,
    pub placement_current: bool,
    pub generation_current: bool,
    pub resources_admitted: bool,
    pub adapter_admitted: bool,
}

impl EffectAdmissionObservation {
    #[must_use]
    pub fn admits(&self, effect: &ActorEffectIntent) -> bool {
        valid_reference(&self.admission_ref)
            && self.actor_key_ref == effect.actor_key_ref
            && self.profile_ref == effect.profile_ref
            && self.system_extension_manifest_ref == effect.system_extension_manifest_ref
            && self.placement_ref == effect.placement_ref
            && self.extension_generation == effect.extension_generation
            && self.policy_current
            && self.capability_current
            && self.placement_current
            && self.generation_current
            && self.resources_admitted
            && self.adapter_admitted
            && effect.requires_fresh_admission
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EffectDisposition {
    Succeeded,
    Failed,
    Unknown,
    AdmissionDenied,
}

impl EffectDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Unknown => "unknown",
            Self::AdmissionDenied => "admission-denied",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectObservation {
    pub effect_ref: String,
    pub admission_ref: String,
    pub disposition: EffectDisposition,
    pub outcome_ref: Option<String>,
}

pub trait ActorEffectPort {
    fn observe_admission(&mut self, effect: &ActorEffectIntent) -> PortResult<EffectAdmissionObservation>;

    fn execute(
        &mut self,
        effect: &ActorEffectIntent,
        admission: &EffectAdmissionObservation,
    ) -> PortResult<EffectObservation>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatusObservation {
    pub status_ref: Option<String>,
    pub outcome_unknown: bool,
}

pub trait ActorStatusPort {
    fn publish_status(&mut self, status: &ActorStatus) -> PortResult<StatusObservation>;
}

pub(crate) fn valid_reference(value: &str) -> bool {
    let Some(hex) = value.strip_prefix(BLAKE3_REFERENCE_PREFIX) else {
        return false;
    };
    hex.len() == BLAKE3_HEX_LENGTH && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}
