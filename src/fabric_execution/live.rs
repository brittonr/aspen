use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;

use bounded_exec::Operation;
use bounded_exec::RunError;

use super::mechanics::*;
use super::*;
const RESOLUTION_DENIED_CODE: &str = "execution-resolution-denied";
const PROFILE_UNAVAILABLE_CODE: &str = "execution-profile-unavailable";
const PRESTART_FAILURE_CODE: &str = "execution-failed-before-start";
const UNKNOWN_FAILURE_CODE: &str = "execution-outcome-unknown";
const RECEIPT_FAILURE_CODE: &str = "execution-receipt-construction-failed";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveExecutionAdapterBuildError {
    WrongProfileKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OperationStatus {
    DefinitePreStartFailure,
    Terminal(String),
    Unknown,
}

// r[impl molten.fabric_execution.port_contract]
// r[impl molten.fabric_execution.environment]
pub struct LiveExecutionAdapter<P: ExecutionOutputPublisher> {
    profile: CanonicalExecutionProfile,
    publisher: P,
    operations: BTreeMap<String, (u64, OperationStatus)>,
}

impl<P: ExecutionOutputPublisher> LiveExecutionAdapter<P> {
    pub fn new(profile: CanonicalExecutionProfile, publisher: P) -> Result<Self, LiveExecutionAdapterBuildError> {
        if profile.profile.descriptor.kind != ExecutionProfileKind::LiveBoundedProcess {
            return Err(LiveExecutionAdapterBuildError::WrongProfileKind);
        }
        Ok(Self {
            profile,
            publisher,
            operations: BTreeMap::new(),
        })
    }

    pub fn publisher(&self) -> &P {
        &self.publisher
    }

    pub fn publisher_mut(&mut self) -> &mut P {
        &mut self.publisher
    }
}

impl<P: ExecutionOutputPublisher> ExecutionFabricPort for LiveExecutionAdapter<P> {
    fn profile(&self) -> &CanonicalExecutionProfile {
        &self.profile
    }

    // r[impl molten.fabric_execution.lifecycle]
    // r[impl molten.fabric_execution.output]
    // r[impl molten.fabric_execution.uncertainty]
    fn execute(
        &mut self,
        request: &CanonicalExecutionRequest,
        resolved: &ResolvedExecutionContext,
        cancellation: Option<&AtomicBool>,
    ) -> ExecutionPortResult<CanonicalExecutionReceipt> {
        validate_resolved_context(request, resolved).map_err(|detail| {
            failure(request, ExecutionPortFailureKind::ResolutionDenied, RESOLUTION_DENIED_CODE, detail, None, None)
        })?;
        let run_request = bounded_request(request, resolved).map_err(|detail| {
            failure(request, ExecutionPortFailureKind::RejectedBeforeStart, PRESTART_FAILURE_CODE, detail, None, None)
        })?;
        let run_result = match cancellation {
            Some(flag) => bounded_exec::run_with_cancellation(run_request, flag),
            None => bounded_exec::run(run_request),
        };
        let output = match run_result {
            Ok(output) => output,
            Err(error) => return Err(self.record_run_failure(request, error)),
        };
        let process = process_observation(output).map_err(|detail| {
            self.operations.insert(
                request.plan.request.operation_ref.clone(),
                (request.plan.request.generation, OperationStatus::Unknown),
            );
            failure(request, ExecutionPortFailureKind::UnknownAfterStart, UNKNOWN_FAILURE_CODE, detail, None, None)
        })?;
        self.publish_and_record(request, process)
    }

    fn reconcile(&self, operation_ref: &str, generation: u64) -> ExecutionReconciliationStatus {
        let Some((recorded_generation, status)) = self.operations.get(operation_ref) else {
            return ExecutionReconciliationStatus::NotFound;
        };
        if *recorded_generation != generation {
            return ExecutionReconciliationStatus::NotFound;
        }
        match status {
            OperationStatus::DefinitePreStartFailure => ExecutionReconciliationStatus::DefinitePreStartFailure,
            OperationStatus::Terminal(receipt_ref) => ExecutionReconciliationStatus::Terminal {
                receipt_ref: receipt_ref.clone(),
            },
            OperationStatus::Unknown => ExecutionReconciliationStatus::UnknownRequiresReconciliation,
        }
    }
}

impl<P: ExecutionOutputPublisher> LiveExecutionAdapter<P> {
    fn record_run_failure(
        &mut self,
        request: &CanonicalExecutionRequest,
        error: RunError,
    ) -> Box<ExecutionPortFailure> {
        let before_start = matches!(
            error,
            RunError::InvalidLimits(_)
                | RunError::EmptyProgram
                | RunError::ProgramNotAbsolute
                | RunError::EmptyCurrentDirectory
                | RunError::CurrentDirectoryNotAbsolute
                | RunError::DuplicateEnvironmentName
                | RunError::InputLimitExceeded
                | RunError::Io {
                    operation: Operation::Spawn,
                    ..
                }
        );
        let (kind, code, status) = if before_start {
            (
                ExecutionPortFailureKind::RejectedBeforeStart,
                PRESTART_FAILURE_CODE,
                OperationStatus::DefinitePreStartFailure,
            )
        } else {
            (ExecutionPortFailureKind::UnknownAfterStart, UNKNOWN_FAILURE_CODE, OperationStatus::Unknown)
        };
        self.operations
            .insert(request.plan.request.operation_ref.clone(), (request.plan.request.generation, status));
        failure(request, kind, code, error.to_string(), None, None)
    }

    fn publish_and_record(
        &mut self,
        request: &CanonicalExecutionRequest,
        process: ExecutionProcessObservation,
    ) -> ExecutionPortResult<CanonicalExecutionReceipt> {
        let stdout_publication =
            publish_stream(&mut self.publisher, &request.plan.request.operation_ref, &process.stdout);
        let stderr_publication =
            publish_stream(&mut self.publisher, &request.plan.request.operation_ref, &process.stderr);
        let publication_failed = matches!(stdout_publication, ExecutionStreamPublication::Failed { .. })
            || matches!(stderr_publication, ExecutionStreamPublication::Failed { .. });
        let receipt = canonical_execution_receipt(
            request,
            &self.profile,
            process.clone(),
            stdout_publication,
            stderr_publication,
        )
        .map_err(|error| {
            self.operations.insert(
                request.plan.request.operation_ref.clone(),
                (request.plan.request.generation, OperationStatus::Unknown),
            );
            failure(
                request,
                ExecutionPortFailureKind::ReceiptConstruction,
                RECEIPT_FAILURE_CODE,
                error.to_string(),
                Some(process.clone()),
                None,
            )
        })?;
        self.operations.insert(
            request.plan.request.operation_ref.clone(),
            (request.plan.request.generation, OperationStatus::Terminal(receipt.receipt_ref.clone())),
        );
        if publication_failed {
            return Err(failure(
                request,
                ExecutionPortFailureKind::OutputPublication,
                PUBLICATION_FAILURE_CODE,
                "one or more retained execution streams were not published".to_string(),
                Some(process),
                Some(receipt),
            ));
        }
        Ok(receipt)
    }
}

fn failure(
    request: &CanonicalExecutionRequest,
    kind: ExecutionPortFailureKind,
    diagnostic_code: &'static str,
    detail: String,
    process_observation: Option<ExecutionProcessObservation>,
    receipt: Option<CanonicalExecutionReceipt>,
) -> Box<ExecutionPortFailure> {
    Box::new(ExecutionPortFailure {
        kind,
        operation_ref: request.plan.request.operation_ref.clone(),
        diagnostic_code,
        detail,
        process_observation,
        receipt,
        non_claims: REQUIRED_EXECUTION_NON_CLAIMS.to_vec(),
    })
}

pub fn unavailable_execution_port_failure(operation_ref: &str) -> ExecutionPortFailure {
    ExecutionPortFailure {
        kind: ExecutionPortFailureKind::ProfileUnavailable,
        operation_ref: operation_ref.to_string(),
        diagnostic_code: PROFILE_UNAVAILABLE_CODE,
        detail: "the exact admitted execution profile is unavailable; no fallback was selected".to_string(),
        process_observation: None,
        receipt: None,
        non_claims: REQUIRED_EXECUTION_NON_CLAIMS.to_vec(),
    }
}
