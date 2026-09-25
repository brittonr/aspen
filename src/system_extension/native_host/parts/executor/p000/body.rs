use crate::fabric_execution::*;

const CALLBACK_DIAGNOSTIC_CODE: &str = "native-callback-process-denied";
const CALLBACK_WIRE_CODE: &str = "native-callback-wire-denied";
const CALLBACK_VALUE_CODE: &str = "native-callback-value-denied";
const CALLBACK_JOURNAL_CODE: &str = "native-callback-journal-failed";
const CALLBACK_LOCK_CODE: &str = "native-callback-state-poisoned";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeExecutionTemplate {
    pub host_profile: AdmittedNativeHostProfile,
    pub executable: AdmittedNativeExecutable,
    pub admitted: CanonicalAdmittedSystemExtensionManifest,
    pub request: ExecutionRequest,
    pub authority: ExecutionAuthorityFacts,
    pub resources: ExecutionResourceGrant,
    pub resolved: ResolvedExecutionContext,
    pub context: NativeCallbackContext,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeInvocationObservation {
    pub envelope_ref: String,
    pub operation_ref: String,
    pub execution_receipt_ref: Option<String>,
    pub lifecycle: ExecutionLifecycleState,
    pub diagnostic_code: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeExecutorError {
    Journal(NativeJournalError),
    StatePoisoned,
    Execution(Box<ExecutionPortFailure>),
    ProcessObservation(&'static str),
    Value(NativeValuePortFailure),
    Wire(String),
    Admission(String),
}

impl NativeExecutorError {
    pub const fn diagnostic_code(&self) -> &'static str {
        match self {
            Self::Journal(_) => CALLBACK_JOURNAL_CODE,
            Self::StatePoisoned => CALLBACK_LOCK_CODE,
            Self::Execution(_) | Self::ProcessObservation(_) => CALLBACK_DIAGNOSTIC_CODE,
            Self::Value(_) => CALLBACK_VALUE_CODE,
            Self::Wire(_) | Self::Admission(_) => CALLBACK_WIRE_CODE,
        }
    }
}

pub struct NativeProcessSystemExtensionExecutor<P, J>
where
    P: ExecutionFabricPort,
    J: NativeHostJournal,
{
    port: P,
    journal: std::sync::Arc<std::sync::Mutex<J>>,
    values: SharedNativeCallbackValuePort,
    instance: std::sync::Arc<std::sync::Mutex<NativeInstanceRecord>>,
    template: NativeExecutionTemplate,
    cancellation: std::sync::Arc<std::sync::atomic::AtomicBool>,
    observations: Vec<NativeInvocationObservation>,
}

struct CallbackCompletionInput<'a> {
    operation_ref: &'a str,
    envelope_ref: &'a str,
    invocation: &'a CallbackInvocation,
    state: NativeOperationState,
    terminal_ref: Option<String>,
    execution_receipt_ref: Option<String>,
    lifecycle: ExecutionLifecycleState,
    diagnostic_code: Option<&'static str>,
}
