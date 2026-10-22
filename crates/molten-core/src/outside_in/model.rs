/// The most processes one scenario may own at once.
pub const MAX_SCENARIO_PROCESSES: u32 = 8;
/// The longest wall-clock budget one scenario may declare.
pub const MAX_SCENARIO_WALL_CLOCK_MS: u64 = 300_000;
/// The most output bytes the harness may retain for one process.
pub const MAX_RETAINED_OUTPUT_BYTES: u64 = 1 << 20;
/// The most client operations one scenario may issue.
pub const MAX_CLIENT_OPERATIONS: u32 = 64;

/// The non-claims every outside-in result carries.
pub const OUTSIDE_IN_NON_CLAIMS: [&str; 3] = [
    "no-production-admission",
    "no-simulation-replacement",
    "no-performance-claim",
];

/// The node path a scenario drives.
///
/// The consensus path is declarable only so that its exclusion is checked:
/// the ChaosControl and live consensus campaigns own it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScenarioPath {
    /// Node control: live ingress, the durable inbox, and dispatch.
    NodeControl,
    /// The consensus path, which this track excludes.
    Consensus,
}

/// The fault a scenario applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FaultProfile {
    NoFault,
    KillRestart,
    PauseResume,
    PartitionHeal,
    LostResponse,
}

impl FaultProfile {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoFault => "no-fault",
            Self::KillRestart => "kill-restart",
            Self::PauseResume => "pause-resume",
            Self::PartitionHeal => "partition-heal",
            Self::LostResponse => "lost-response",
        }
    }

    /// The one boundary at which the profile applies its fault.
    pub const fn boundary(self) -> FaultBoundary {
        match self {
            Self::NoFault => FaultBoundary::None,
            Self::KillRestart | Self::PauseResume => FaultBoundary::ListenerAwaitingRequest,
            Self::PartitionHeal => FaultBoundary::TransportBetweenPeers,
            Self::LostResponse => FaultBoundary::ClientResponse,
        }
    }
}

/// Where a fault applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FaultBoundary {
    /// No fault applies.
    None,
    /// The listener has published its bound ticket and waits for a request.
    ListenerAwaitingRequest,
    /// The transport path between the client and the listener.
    TransportBetweenPeers,
    /// The client's response after the client handed the request over.
    ClientResponse,
}

impl FaultBoundary {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::ListenerAwaitingRequest => "listener-awaiting-request",
            Self::TransportBetweenPeers => "transport-between-peers",
            Self::ClientResponse => "client-response",
        }
    }
}

/// A scenario budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BudgetKind {
    Processes,
    WallClock,
    RetainedOutput,
    ClientOperations,
}

impl BudgetKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Processes => "processes",
            Self::WallClock => "wall-clock",
            Self::RetainedOutput => "retained-output",
            Self::ClientOperations => "client-operations",
        }
    }
}

/// The budgets a scenario declares before it starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScenarioBudgets {
    /// The most owned processes alive at once.
    pub max_processes: u32,
    /// The wall-clock budget of the whole scenario, teardown excluded.
    pub wall_clock_ms: u64,
    /// The most output bytes retained for each owned process.
    pub max_retained_output_bytes: u64,
    /// The most client operations the scenario issues.
    pub max_client_operations: u32,
}

/// One declared outside-in scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioDeclaration {
    pub scenario_id: String,
    pub path: ScenarioPath,
    pub profile: FaultProfile,
    pub boundary: FaultBoundary,
    pub budgets: ScenarioBudgets,
    /// How long the fault holds before the harness lifts it: the pause, or
    /// the partition. Zero for profiles that hold no fault.
    pub fault_hold_ms: u64,
    /// How long the harness lets the system settle before it reads histories.
    pub stabilization_ms: u64,
    /// The requests the receiver must process exactly once, in any order.
    pub expected_processed: Vec<String>,
}
