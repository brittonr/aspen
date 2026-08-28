use molten_core::coordination_delivery::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeliveryPortError {
    pub code: &'static str,
    pub detail: String,
    pub outcome_unknown: bool,
}

impl DeliveryPortError {
    pub fn new(code: &'static str, detail: impl Into<String>, outcome_unknown: bool) -> Self {
        Self {
            code,
            detail: detail.into(),
            outcome_unknown,
        }
    }
}

pub type DeliveryPortResult<T> = std::result::Result<T, DeliveryPortError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublishedDeliveryState {
    pub state: DeliveryState,
    pub state_ref: String,
    pub revision: u64,
}

impl PublishedDeliveryState {
    pub fn from_state(state: DeliveryState) -> Self {
        let state_ref = identify_delivery_state(&state);
        let revision = state.revision;
        Self {
            state,
            state_ref,
            revision,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpectedDeliveryState {
    pub state_ref: Option<String>,
    pub revision: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeliveryCommitRequest {
    pub queue_id: String,
    pub expected: ExpectedDeliveryState,
    pub next: PublishedDeliveryState,
    pub requested_engine_epoch: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeliveryCommitDisposition {
    Applied,
    AlreadyApplied,
    Stale,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeliveryDurabilityOutcome {
    Durable,
    Buffered,
    Unknown,
}

impl DeliveryDurabilityOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Durable => "durable",
            Self::Buffered => "buffered",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeliveryCommitObservation {
    pub disposition: DeliveryCommitDisposition,
    pub currentness: DeliveryCurrentness,
    pub durability: DeliveryDurabilityOutcome,
    pub engine_epoch: u64,
    pub observed_state_ref: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeliveryTimerObservation {
    pub accepted_timer_refs: Vec<String>,
    pub failed_timer_refs: Vec<String>,
    pub outcome_unknown: bool,
}

impl DeliveryTimerObservation {
    pub const fn empty() -> Self {
        Self {
            accepted_timer_refs: Vec::new(),
            failed_timer_refs: Vec::new(),
            outcome_unknown: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeliveryStatusObservation {
    pub published_status_ref: Option<String>,
    pub outcome_unknown: bool,
}

pub trait DeliveryCommitPort {
    fn load(&self, queue_id: &str) -> DeliveryPortResult<Option<PublishedDeliveryState>>;

    fn compare_and_commit(&mut self, request: &DeliveryCommitRequest) -> DeliveryPortResult<DeliveryCommitObservation>;
}

pub trait DeliveryTimerPort {
    fn apply_timer_intents(&mut self, intents: &[DeliveryTimerIntent]) -> DeliveryPortResult<DeliveryTimerObservation>;
}

pub trait DeliveryStatusPort {
    fn publish_status(&mut self, status: &DeliveryStatus) -> DeliveryPortResult<DeliveryStatusObservation>;
}
