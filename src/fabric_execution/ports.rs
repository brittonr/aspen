#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedContext {
    pub executable_path: std::path::PathBuf,
    pub executable_artifact_ref: String,
    pub executable_identity_ref: String,
    pub workspace_path: std::path::PathBuf,
    pub workspace_ref: String,
    pub stdin_ref: Option<String>,
    pub stdin_bytes: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservedDisposition {
    ExitPolicyAccepted,
    ExitPolicyRejected,
    OutputPolicyRejected,
    TimedOut,
    Cancelled,
}

impl ObservedDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExitPolicyAccepted => "exit-policy-accepted",
            Self::ExitPolicyRejected => "exit-policy-rejected",
            Self::OutputPolicyRejected => "output-policy-rejected",
            Self::TimedOut => "timed-out",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedStream {
    pub role: String,
    pub retained_bytes: Vec<u8>,
    pub observed_bytes: u64,
    pub retained_byte_count: u64,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessObservation {
    pub lifecycle: super::ExecutionLifecycleState,
    pub start_observed: bool,
    pub terminal_observed: bool,
    pub teardown_observed: bool,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub disposition: ObservedDisposition,
    pub stdout: RetainedStream,
    pub stderr: RetainedStream,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedStream {
    pub content_ref: String,
    pub publication_receipt_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamPublication {
    Published(PublishedStream),
    Failed { diagnostic_code: &'static str },
}

pub trait ExecutionOutputPublisher {
    fn publish(
        &mut self,
        operation_ref: &str,
        stream: &RetainedStream,
    ) -> Result<PublishedStream, OutputPublicationError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputPublicationError {
    Unavailable,
    Denied,
    Storage,
    IdentityMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortFailureKind {
    ProfileUnavailable,
    ResolutionDenied,
    RejectedBeforeStart,
    UnknownAfterStart,
    OutputPublication,
    ReceiptConstruction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortFailure {
    pub kind: PortFailureKind,
    pub operation_ref: String,
    pub diagnostic_code: &'static str,
    pub detail: String,
    pub process_observation: Option<ProcessObservation>,
    pub receipt: Option<super::Receipt>,
    pub non_claims: Vec<super::ExecutionNonClaim>,
}

pub type PortResult<T> = Result<T, Box<PortFailure>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconciliationStatus {
    DefinitePreStartFailure,
    Terminal { receipt_ref: String },
    UnknownRequiresReconciliation,
    NotFound,
}

// r[impl molten.fabric_execution.port_contract]
pub trait ExecutionFabricPort {
    fn profile(&self) -> &super::Profile;

    fn execute(
        &mut self,
        request: &super::Request,
        resolved: &ResolvedContext,
        cancellation: Option<&std::sync::atomic::AtomicBool>,
    ) -> PortResult<super::Receipt>;

    fn reconcile(&self, operation_ref: &str, generation: u64) -> ReconciliationStatus;
}
