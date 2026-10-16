use molten_core::coordination_delivery::*;

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
    pub state: DeliveryState,
    pub state_ref: String,
    pub revision: u64,
}

impl PublishedState {
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
pub struct ExpectedState {
    pub state_ref: Option<String>,
    pub revision: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitRequest {
    pub queue_id: String,
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
    pub currentness: DeliveryCurrentness,
    pub durability: DurabilityOutcome,
    pub engine_epoch: u64,
    pub observed_state_ref: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimerObservation {
    pub accepted_timer_refs: Vec<String>,
    pub failed_timer_refs: Vec<String>,
    pub outcome_unknown: bool,
}

impl TimerObservation {
    pub const fn empty() -> Self {
        Self {
            accepted_timer_refs: Vec::new(),
            failed_timer_refs: Vec::new(),
            outcome_unknown: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatusObservation {
    pub published_status_ref: Option<String>,
    pub outcome_unknown: bool,
}

pub trait DeliveryCommitPort {
    fn load(&self, queue_id: &str) -> PortResult<Option<PublishedState>>;

    fn compare_and_commit(&mut self, request: &CommitRequest) -> PortResult<CommitObservation>;
}

pub trait DeliveryTimerPort {
    fn apply_timer_intents(&mut self, intents: &[DeliveryTimerIntent]) -> PortResult<TimerObservation>;
}

pub trait DeliveryStatusPort {
    fn publish_status(&mut self, status: &DeliveryStatus) -> PortResult<StatusObservation>;
}
