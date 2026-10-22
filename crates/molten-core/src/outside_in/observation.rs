use super::*;

/// What the client reported for one attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClientOutcome {
    /// The client reported that it handed the request to the transport.
    Acknowledged,
    /// The client reported that it could not hand the request over.
    Failed,
    /// No client report is available.
    Unresolved,
}

/// Whether the harness observed the client's response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResponseFate {
    Observed,
    /// The response was dropped at the declared client boundary.
    Lost,
}

/// One client attempt, in execution order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientAttempt {
    pub request_ref: String,
    pub fate: ResponseFate,
    pub outcome: ClientOutcome,
}

/// The latest delivery outcome the receiver reports for one envelope.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObservedDelivery {
    pub envelope_ref: String,
    pub request_ref: String,
    /// The receiver accepted the delivery rather than denying it.
    pub is_accepted: bool,
}

/// One request the receiver reports as processed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObservedProcessed {
    pub request_ref: String,
    /// The processing decision was a pass.
    pub is_passed: bool,
}

/// The receiver history read through a public interface.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReceiverHistory {
    pub deliveries: Vec<ObservedDelivery>,
    pub pending_requests: Vec<String>,
    pub processed: Vec<ObservedProcessed>,
}

/// When the harness read a receiver history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CheckpointKind {
    /// Read after the fault was lifted, before any further client work.
    AfterFault,
    /// Read after the declared stabilization.
    Final,
}

/// One history read. `None` records a read that did not produce a history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryCheckpoint {
    pub kind: CheckpointKind,
    pub receiver: Option<ReceiverHistory>,
}

/// Everything the harness observed through public interfaces.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ObservedHistory {
    pub attempts: Vec<ClientAttempt>,
    /// The history reads, in the order the harness made them.
    pub checkpoints: Vec<HistoryCheckpoint>,
    /// The bound endpoint each listener run reported, in run order.
    pub listener_endpoints: Vec<String>,
}

/// How a scenario execution ended before judgment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScenarioExecution {
    /// The scenario ran to its observation phase.
    Completed(ObservedHistory),
    /// An owned process exited before it reported readiness.
    StartupFailed { process: String },
    /// The scenario exceeded a declared budget and the harness stopped it.
    BudgetExhausted(BudgetKind),
    /// The harness cannot execute a declared behavior on this host.
    Unsupported { behavior: String },
}

/// Whether the harness released everything it owned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TeardownStatus {
    Complete,
    /// An owned process could not be reaped.
    OrphanedProcess {
        process: String,
    },
    /// An owned scratch path could not be removed.
    ScratchPathRetained {
        path_label: String,
    },
}
