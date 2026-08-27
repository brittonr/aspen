use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use super::CanonicalExecutionProfile;
use super::CanonicalExecutionReceipt;
use super::CanonicalExecutionRequest;
use super::ExecutionLifecycleState;
use super::ExecutionNonClaim;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedExecutionContext {
    pub executable_path: PathBuf,
    pub executable_artifact_ref: String,
    pub executable_identity_ref: String,
    pub workspace_path: PathBuf,
    pub workspace_ref: String,
    pub stdin_ref: Option<String>,
    pub stdin_bytes: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionObservedDisposition {
    ExitPolicyAccepted,
    ExitPolicyRejected,
    OutputPolicyRejected,
    TimedOut,
    Cancelled,
}

impl ExecutionObservedDisposition {
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
pub struct RetainedExecutionStream {
    pub role: String,
    pub retained_bytes: Vec<u8>,
    pub observed_bytes: u64,
    pub retained_byte_count: u64,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionProcessObservation {
    pub lifecycle: ExecutionLifecycleState,
    pub start_observed: bool,
    pub terminal_observed: bool,
    pub teardown_observed: bool,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub disposition: ExecutionObservedDisposition,
    pub stdout: RetainedExecutionStream,
    pub stderr: RetainedExecutionStream,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedExecutionStream {
    pub content_ref: String,
    pub publication_receipt_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionStreamPublication {
    Published(PublishedExecutionStream),
    Failed { diagnostic_code: &'static str },
}

pub trait ExecutionOutputPublisher {
    fn publish(
        &mut self,
        operation_ref: &str,
        stream: &RetainedExecutionStream,
    ) -> Result<PublishedExecutionStream, ExecutionOutputPublicationError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionOutputPublicationError {
    Unavailable,
    Denied,
    Storage,
    IdentityMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionPortFailureKind {
    ProfileUnavailable,
    ResolutionDenied,
    RejectedBeforeStart,
    UnknownAfterStart,
    OutputPublication,
    ReceiptConstruction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionPortFailure {
    pub kind: ExecutionPortFailureKind,
    pub operation_ref: String,
    pub diagnostic_code: &'static str,
    pub detail: String,
    pub process_observation: Option<ExecutionProcessObservation>,
    pub receipt: Option<CanonicalExecutionReceipt>,
    pub non_claims: Vec<ExecutionNonClaim>,
}

pub type ExecutionPortResult<T> = Result<T, Box<ExecutionPortFailure>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionReconciliationStatus {
    DefinitePreStartFailure,
    Terminal { receipt_ref: String },
    UnknownRequiresReconciliation,
    NotFound,
}

// r[impl molten.fabric_execution.port_contract]
pub trait ExecutionFabricPort {
    fn profile(&self) -> &CanonicalExecutionProfile;

    fn execute(
        &mut self,
        request: &CanonicalExecutionRequest,
        resolved: &ResolvedExecutionContext,
        cancellation: Option<&AtomicBool>,
    ) -> ExecutionPortResult<CanonicalExecutionReceipt>;

    fn reconcile(&self, operation_ref: &str, generation: u64) -> ExecutionReconciliationStatus;
}
