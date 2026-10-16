use super::mechanics::*;
use super::*;
const RESOLUTION_DENIED_CODE: &str = "execution-resolution-denied";
const PROFILE_UNAVAILABLE_CODE: &str = "execution-profile-unavailable";
const PRESTART_FAILURE_CODE: &str = "execution-failed-before-start";
const UNKNOWN_FAILURE_CODE: &str = "execution-outcome-unknown";
const RECEIPT_FAILURE_CODE: &str = "execution-receipt-construction-failed";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterBuildError {
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
pub struct Adapter<P: ExecutionOutputPublisher> {
    profile: Profile,
    publisher: P,
    operations: std::collections::BTreeMap<String, (u64, OperationStatus)>,
}

impl<P: ExecutionOutputPublisher> Adapter<P> {
    pub fn new(profile: Profile, publisher: P) -> Result<Self, AdapterBuildError> {
        if profile.profile.descriptor.kind != ExecutionProfileKind::LiveBoundedProcess {
            return Err(AdapterBuildError::WrongProfileKind);
        }
        Ok(Self {
            profile,
            publisher,
            operations: std::collections::BTreeMap::new(),
        })
    }

    pub fn publisher(&self) -> &P {
        &self.publisher
    }

    pub fn publisher_mut(&mut self) -> &mut P {
        &mut self.publisher
    }
}

impl<P: ExecutionOutputPublisher> ExecutionFabricPort for Adapter<P> {
    fn profile(&self) -> &Profile {
        &self.profile
    }

    // r[impl molten.fabric_execution.lifecycle]
    // r[impl molten.fabric_execution.output]
    // r[impl molten.fabric_execution.uncertainty]
    fn execute(
        &mut self,
        request: &Request,
        resolved: &ResolvedContext,
        cancellation: Option<&std::sync::atomic::AtomicBool>,
    ) -> PortResult<Receipt> {
        validate_resolved_context(request, resolved).map_err(|detail| {
            failure(request, PortFailureKind::ResolutionDenied, RESOLUTION_DENIED_CODE, detail, FailureEvidence::NONE)
        })?;
        let run_request = bounded_request(request, resolved).map_err(|detail| {
            failure(request, PortFailureKind::RejectedBeforeStart, PRESTART_FAILURE_CODE, detail, FailureEvidence::NONE)
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
            failure(request, PortFailureKind::UnknownAfterStart, UNKNOWN_FAILURE_CODE, detail, FailureEvidence::NONE)
        })?;
        self.publish_and_record(request, process)
    }

    fn reconcile(&self, operation_ref: &str, generation: u64) -> ReconciliationStatus {
        let Some((recorded_generation, status)) = self.operations.get(operation_ref) else {
            return ReconciliationStatus::NotFound;
        };
        if *recorded_generation != generation {
            return ReconciliationStatus::NotFound;
        }
        match status {
            OperationStatus::DefinitePreStartFailure => ReconciliationStatus::DefinitePreStartFailure,
            OperationStatus::Terminal(receipt_ref) => ReconciliationStatus::Terminal {
                receipt_ref: receipt_ref.clone(),
            },
            OperationStatus::Unknown => ReconciliationStatus::UnknownRequiresReconciliation,
        }
    }
}

impl<P: ExecutionOutputPublisher> Adapter<P> {
    fn record_run_failure(&mut self, request: &Request, error: bounded_exec::RunError) -> Box<PortFailure> {
        let is_before_start = matches!(
            error,
            bounded_exec::RunError::InvalidLimits(_)
                | bounded_exec::RunError::EmptyProgram
                | bounded_exec::RunError::ProgramNotAbsolute
                | bounded_exec::RunError::EmptyCurrentDirectory
                | bounded_exec::RunError::CurrentDirectoryNotAbsolute
                | bounded_exec::RunError::DuplicateEnvironmentName
                | bounded_exec::RunError::InputLimitExceeded
                | bounded_exec::RunError::Io {
                    operation: bounded_exec::Operation::Spawn,
                    ..
                }
        );
        let (kind, code, status) = if is_before_start {
            (
                PortFailureKind::RejectedBeforeStart,
                PRESTART_FAILURE_CODE,
                OperationStatus::DefinitePreStartFailure,
            )
        } else {
            (PortFailureKind::UnknownAfterStart, UNKNOWN_FAILURE_CODE, OperationStatus::Unknown)
        };
        self.operations
            .insert(request.plan.request.operation_ref.clone(), (request.plan.request.generation, status));
        failure(request, kind, code, error.to_string(), FailureEvidence::NONE)
    }

    fn publish_and_record(&mut self, request: &Request, process: ProcessObservation) -> PortResult<Receipt> {
        let stdout_publication =
            publish_stream(&mut self.publisher, &request.plan.request.operation_ref, &process.stdout);
        let stderr_publication =
            publish_stream(&mut self.publisher, &request.plan.request.operation_ref, &process.stderr);
        let is_publication_failed = matches!(stdout_publication, StreamPublication::Failed { .. })
            || matches!(stderr_publication, StreamPublication::Failed { .. });
        let receipt = receipt(request, &self.profile, process.clone(), stdout_publication, stderr_publication)
            .map_err(|error| {
                self.operations.insert(
                    request.plan.request.operation_ref.clone(),
                    (request.plan.request.generation, OperationStatus::Unknown),
                );
                failure(
                    request,
                    PortFailureKind::ReceiptConstruction,
                    RECEIPT_FAILURE_CODE,
                    error.to_string(),
                    FailureEvidence {
                        process_observation: Some(process.clone()),
                        receipt: None,
                    },
                )
            })?;
        self.operations.insert(
            request.plan.request.operation_ref.clone(),
            (request.plan.request.generation, OperationStatus::Terminal(receipt.receipt_ref.clone())),
        );
        if is_publication_failed {
            return Err(failure(
                request,
                PortFailureKind::OutputPublication,
                PUBLICATION_FAILURE_CODE,
                "one or more retained execution streams were not published".to_string(),
                FailureEvidence {
                    process_observation: Some(process),
                    receipt: Some(receipt),
                },
            ));
        }
        Ok(receipt)
    }
}

fn failure(
    request: &Request,
    kind: PortFailureKind,
    diagnostic_code: &'static str,
    detail: String,
    evidence: FailureEvidence,
) -> Box<PortFailure> {
    let FailureEvidence {
        process_observation,
        receipt,
    } = evidence;
    Box::new(PortFailure {
        kind,
        operation_ref: request.plan.request.operation_ref.clone(),
        diagnostic_code,
        detail,
        process_observation,
        receipt,
        non_claims: REQUIRED_EXECUTION_NON_CLAIMS.to_vec(),
    })
}

pub fn unavailable_port_failure(operation_ref: &str) -> PortFailure {
    PortFailure {
        kind: PortFailureKind::ProfileUnavailable,
        operation_ref: operation_ref.to_string(),
        diagnostic_code: PROFILE_UNAVAILABLE_CODE,
        detail: "the exact admitted execution profile is unavailable; no fallback was selected".to_string(),
        process_observation: None,
        receipt: None,
        non_claims: REQUIRED_EXECUTION_NON_CLAIMS.to_vec(),
    }
}
