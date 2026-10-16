use super::*;

const SIMULATION_PROFILE_CODE: &str = "execution-simulation-profile-mismatch";
const SIMULATION_SCRIPT_CODE: &str = "execution-simulation-script-missing";
const SIMULATION_OBSERVATION_CODE: &str = "execution-simulation-observation-invalid";
const SIMULATION_UNKNOWN_CODE: &str = "execution-simulation-outcome-unknown";
const SIMULATION_PRESTART_CODE: &str = "execution-simulation-prestart-failure";
const SIMULATION_PUBLICATION_CODE: &str = "execution-simulation-publication-failed";
const SIMULATION_RECEIPT_CODE: &str = "execution-simulation-receipt-failed";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptedObservation {
    pub process: ProcessObservation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulatedAdapterBuildError {
    WrongProfileKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Status {
    DefinitePreStartFailure,
    Terminal(String),
    Unknown,
}

// r[impl molten.fabric_execution.simulation]
pub struct SimulatedAdapter<P: ExecutionOutputPublisher> {
    profile: Profile,
    publisher: P,
    scripts: std::collections::BTreeMap<String, ScriptedObservation>,
    operations: std::collections::BTreeMap<String, (u64, Status)>,
}

impl<P: ExecutionOutputPublisher> SimulatedAdapter<P> {
    pub fn new(
        profile: Profile,
        publisher: P,
        scripts: std::collections::BTreeMap<String, ScriptedObservation>,
    ) -> Result<Self, SimulatedAdapterBuildError> {
        if profile.profile.descriptor.kind != ExecutionProfileKind::DeterministicSimulation {
            return Err(SimulatedAdapterBuildError::WrongProfileKind);
        }
        Ok(Self {
            profile,
            publisher,
            scripts,
            operations: std::collections::BTreeMap::new(),
        })
    }

    pub fn publisher(&self) -> &P {
        &self.publisher
    }
}

impl<P: ExecutionOutputPublisher> ExecutionFabricPort for SimulatedAdapter<P> {
    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn execute(
        &mut self,
        request: &Request,
        resolved: &ResolvedContext,
        cancellation: Option<&std::sync::atomic::AtomicBool>,
    ) -> PortResult<Receipt> {
        if request.plan.profile.descriptor.kind != ExecutionProfileKind::DeterministicSimulation {
            return Err(failure(
                request,
                PortFailureKind::ProfileUnavailable,
                SIMULATION_PROFILE_CODE,
                "simulation adapter received a non-simulation request".to_string(),
                super::mechanics::FailureEvidence::NONE,
            ));
        }
        super::mechanics::validate_resolved_context(request, resolved).map_err(|detail| {
            failure(
                request,
                PortFailureKind::ResolutionDenied,
                SIMULATION_OBSERVATION_CODE,
                detail,
                super::mechanics::FailureEvidence::NONE,
            )
        })?;
        let Some(script) = self.scripts.get(&request.plan.request.operation_ref) else {
            return Err(failure(
                request,
                PortFailureKind::RejectedBeforeStart,
                SIMULATION_SCRIPT_CODE,
                "no deterministic execution script matches the exact operation".to_string(),
                super::mechanics::FailureEvidence::NONE,
            ));
        };
        let mut process = script.process.clone();
        if cancellation.is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Acquire)) {
            process.lifecycle = ExecutionLifecycleState::Cancelled;
            process.disposition = ObservedDisposition::Cancelled;
            process.start_observed = true;
            process.terminal_observed = true;
            process.teardown_observed = true;
        }
        validate_scripted_process(request, &process).map_err(|detail| {
            failure(
                request,
                PortFailureKind::ResolutionDenied,
                SIMULATION_OBSERVATION_CODE,
                detail,
                super::mechanics::FailureEvidence {
                    process_observation: Some(process.clone()),
                    receipt: None,
                },
            )
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
            Status::DefinitePreStartFailure => ReconciliationStatus::DefinitePreStartFailure,
            Status::Terminal(receipt_ref) => ReconciliationStatus::Terminal {
                receipt_ref: receipt_ref.clone(),
            },
            Status::Unknown => ReconciliationStatus::UnknownRequiresReconciliation,
        }
    }
}

impl<P: ExecutionOutputPublisher> SimulatedAdapter<P> {
    fn publish_and_record(&mut self, request: &Request, process: ProcessObservation) -> PortResult<Receipt> {
        if process.lifecycle == ExecutionLifecycleState::FailedBeforeStart {
            self.operations.insert(
                request.plan.request.operation_ref.clone(),
                (request.plan.request.generation, Status::DefinitePreStartFailure),
            );
            return Err(failure(
                request,
                PortFailureKind::RejectedBeforeStart,
                SIMULATION_PRESTART_CODE,
                "scripted execution refused before process start".to_string(),
                super::mechanics::FailureEvidence {
                    process_observation: Some(process),
                    receipt: None,
                },
            ));
        }
        let stdout_publication =
            super::mechanics::publish_stream(&mut self.publisher, &request.plan.request.operation_ref, &process.stdout);
        let stderr_publication =
            super::mechanics::publish_stream(&mut self.publisher, &request.plan.request.operation_ref, &process.stderr);
        let is_publication_failed = matches!(stdout_publication, StreamPublication::Failed { .. })
            || matches!(stderr_publication, StreamPublication::Failed { .. });
        let receipt = receipt(request, &self.profile, process.clone(), stdout_publication, stderr_publication)
            .map_err(|error| {
                failure(
                    request,
                    PortFailureKind::ReceiptConstruction,
                    SIMULATION_RECEIPT_CODE,
                    error.to_string(),
                    super::mechanics::FailureEvidence {
                        process_observation: Some(process.clone()),
                        receipt: None,
                    },
                )
            })?;
        if matches!(
            process.lifecycle,
            ExecutionLifecycleState::Unknown
                | ExecutionLifecycleState::FailedAfterStart
                | ExecutionLifecycleState::TeardownIncomplete
        ) {
            self.operations
                .insert(request.plan.request.operation_ref.clone(), (request.plan.request.generation, Status::Unknown));
            return Err(failure(
                request,
                PortFailureKind::UnknownAfterStart,
                SIMULATION_UNKNOWN_CODE,
                "scripted execution lacks definitive completion and teardown evidence".to_string(),
                super::mechanics::FailureEvidence {
                    process_observation: Some(process),
                    receipt: Some(receipt),
                },
            ));
        }
        self.operations.insert(
            request.plan.request.operation_ref.clone(),
            (request.plan.request.generation, Status::Terminal(receipt.receipt_ref.clone())),
        );
        if is_publication_failed {
            return Err(failure(
                request,
                PortFailureKind::OutputPublication,
                SIMULATION_PUBLICATION_CODE,
                "one or more simulated output streams were not published".to_string(),
                super::mechanics::FailureEvidence {
                    process_observation: Some(process),
                    receipt: Some(receipt),
                },
            ));
        }
        Ok(receipt)
    }
}

fn validate_scripted_process(request: &Request, process: &ProcessObservation) -> Result<(), String> {
    if !process.lifecycle.is_terminal() {
        return Err("scripted execution observation is not terminal".to_string());
    }
    for (stream, maximum) in [
        (&process.stdout, request.plan.request.limits.stdout_max_bytes),
        (&process.stderr, request.plan.request.limits.stderr_max_bytes),
    ] {
        if stream.retained_byte_count > maximum {
            return Err(format!("scripted {} retained bytes exceed the admitted bound", stream.role));
        }
        if usize::try_from(stream.retained_byte_count).ok() != Some(stream.retained_bytes.len()) {
            return Err(format!("scripted {} retained byte count differs from supplied bytes", stream.role));
        }
        if stream.observed_bytes < stream.retained_byte_count {
            return Err(format!("scripted {} observed bytes are less than retained bytes", stream.role));
        }
        if stream.truncated != (stream.observed_bytes > stream.retained_byte_count) {
            return Err(format!("scripted {} truncation fact is inconsistent", stream.role));
        }
    }
    Ok(())
}

fn failure(
    request: &Request,
    kind: PortFailureKind,
    diagnostic_code: &'static str,
    detail: String,
    evidence: super::mechanics::FailureEvidence,
) -> Box<PortFailure> {
    let super::mechanics::FailureEvidence {
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
