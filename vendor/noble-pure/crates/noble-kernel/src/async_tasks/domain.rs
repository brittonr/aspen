/// Host-assigned namespace, never reused while any old callback can exist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TableId(pub u64);

/// Independent native operation identity chosen and authenticated by the shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeId(pub u64);

/// Untrusted descriptive task identity, not a result-delivery capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handle {
    pub table: TableId,
    pub slot: usize,
    pub generation: u64,
    pub context: super::Context,
}

/// Full callback binding; every native event checks both fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Callback {
    pub task: Handle,
    pub native: NativeId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[tigerstyle::state_machine_enum]
pub enum State {
    Pending,
    Ready,
    Delivered,
    Retiring,
    Retired,
}

/// Primary abnormal outcome is never replaced by cleanup or late completion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Cancelled,
    Trap,
    Deadline,
    Budget,
    Internal,
}

/// Local result classification, not an authenticated external observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Success,
    DomainError,
}

/// Bit n names input n. These masks must partition the admitted input mask.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Disposition {
    pub returned: u64,
    pub consumed: u64,
    pub retired: u64,
}

/// Result metadata, checked before the shell allocates or retains result bytes.
/// `produced` names newly acquired result owners, independently of input owners.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Completion {
    pub inputs: Disposition,
    pub produced: u64,
    pub bytes: usize,
}

/// Exact shell-owned cleanup identities. Buffer bits are input=1, result=2,
/// parked=4. A cleanup event acknowledges work already completed by the shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Obligations {
    pub inputs: u64,
    pub results: u64,
    pub buffers: u8,
}

impl Obligations {
    pub const fn empty() -> Self {
        Self {
            inputs: 0,
            results: 0,
            buffers: 0,
        }
    }
}

/// Complete semantic record. There is deliberately no snapshot-import API.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub handle: Handle,
    pub native: NativeId,
    pub state: State,
    pub reservation: super::Request,
    pub completion: Option<Completion>,
    pub outcome: Option<Outcome>,
    pub failure: Option<Failure>,
    pub completion_closed: bool,
    pub native_stopped: bool,
    pub pins: u64,
    pub returned_inputs: u64,
    pub retiring_inputs: u64,
    pub results: u64,
    pub buffers: u8,
    pub wake_pending: bool,
    pub wakeups_remaining: u32,
    pub finalized: bool,
}

/// The exact lifecycle event schema. Explicit named pairs live in transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[tigerstyle::state_machine_enum]
pub enum Event {
    Inspect,
    CompleteSuccess {
        native: NativeId,
        completion: Completion,
    },
    CompleteDomainError {
        native: NativeId,
        completion: Completion,
    },
    Deliver,
    Cancel,
    Trap,
    Deadline,
    Budget,
    InternalFailure,
    NativeStopped {
        native: NativeId,
    },
    SettlePins {
        native: NativeId,
        pins: u64,
    },
    Cleanup(Obligations),
    Wake {
        native: NativeId,
    },
    TakeWake,
    Finish,
}

/// Decision classification; duplicates carry no release/delivery accounting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[tigerstyle::state_machine_enum]
pub enum Action {
    Admitted,
    Inspected,
    ResultReady(Outcome),
    ResultDelivered,
    CancellationAcknowledged,
    FailureRecorded(Failure),
    CompletionRetired(Outcome),
    OversizedResult,
    NativeStopObserved,
    PinsSettled,
    CleanupSettled,
    WakeQueued,
    WakeCoalesced,
    WakeTaken,
    RetirementCompleted,
    DeliverySettled,
    Duplicate,
}

/// Exact one-time local deltas. No field asserts a native destructor executed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Accounting {
    pub acquired: Obligations,
    pub consumed_inputs: u64,
    pub retirement_withdrawn_inputs: u64,
    pub retired: Obligations,
    pub delivered: Obligations,
    pub cleaned: Obligations,
    pub pins_acquired: u64,
    pub pins_released: u64,
    pub rejected_result_bytes: usize,
    pub completion_accepted: bool,
    pub wake_queued: bool,
    pub wake_removed: bool,
    pub reservation_released: bool,
}

/// The same decision is returned by the pure core and retained production table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decision {
    pub record: Snapshot,
    pub action: Action,
    pub accounting: Accounting,
}

/// Rejections preserve retained ownership and release no native pin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidLimits,
    InvalidRequest,
    StorageUnavailable,
    TaskCapacity,
    TerminalCapacity,
    ByteCapacity,
    ParkedCapacity,
    PinCapacity,
    WakeCapacity,
    RetirementCapacity,
    GenerationExhausted,
    InvalidHandle,
    WrongContext,
    WrongGeneration,
    WrongNative,
    InvalidRecord,
    InvalidDisposition,
    ResultCapacity,
    Pending,
    Ready,
    Delivered,
    Retiring,
    Retired,
    NativeStillRunning,
    PinsOutstanding,
    InvalidPins,
    InvalidCleanup,
    CleanupOutstanding,
    NoWakeup,
}
