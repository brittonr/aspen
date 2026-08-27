use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use super::mechanics::publish_stream;
use super::mechanics::validate_resolved_context;
use super::*;

const SIMULATION_PROFILE_CODE: &str = "execution-simulation-profile-mismatch";
const SIMULATION_SCRIPT_CODE: &str = "execution-simulation-script-missing";
const SIMULATION_OBSERVATION_CODE: &str = "execution-simulation-observation-invalid";
const SIMULATION_UNKNOWN_CODE: &str = "execution-simulation-outcome-unknown";
const SIMULATION_PRESTART_CODE: &str = "execution-simulation-prestart-failure";
const SIMULATION_PUBLICATION_CODE: &str = "execution-simulation-publication-failed";
const SIMULATION_RECEIPT_CODE: &str = "execution-simulation-receipt-failed";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptedExecutionObservation {
    pub process: ExecutionProcessObservation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulatedExecutionAdapterBuildError {
    WrongProfileKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SimulationStatus {
    DefinitePreStartFailure,
    Terminal(String),
    Unknown,
}

// r[impl molten.fabric_execution.simulation]
pub struct SimulatedExecutionAdapter<P: ExecutionOutputPublisher> {
    profile: CanonicalExecutionProfile,
    publisher: P,
    scripts: BTreeMap<String, ScriptedExecutionObservation>,
    operations: BTreeMap<String, (u64, SimulationStatus)>,
}

impl<P: ExecutionOutputPublisher> SimulatedExecutionAdapter<P> {
    pub fn new(
        profile: CanonicalExecutionProfile,
        publisher: P,
        scripts: BTreeMap<String, ScriptedExecutionObservation>,
    ) -> Result<Self, SimulatedExecutionAdapterBuildError> {
        if profile.profile.descriptor.kind != ExecutionProfileKind::DeterministicSimulation {
            return Err(SimulatedExecutionAdapterBuildError::WrongProfileKind);
        }
        Ok(Self {
            profile,
            publisher,
            scripts,
            operations: BTreeMap::new(),
        })
    }

    pub fn publisher(&self) -> &P {
        &self.publisher
    }
}

impl<P: ExecutionOutputPublisher> ExecutionFabricPort for SimulatedExecutionAdapter<P> {
    fn profile(&self) -> &CanonicalExecutionProfile {
        &self.profile
    }

    fn execute(
        &mut self,
        request: &CanonicalExecutionRequest,
        resolved: &ResolvedExecutionContext,
        cancellation: Option<&AtomicBool>,
    ) -> ExecutionPortResult<CanonicalExecutionReceipt> {
        if request.plan.profile.descriptor.kind != ExecutionProfileKind::DeterministicSimulation {
            return Err(simulation_failure(
                request,
                ExecutionPortFailureKind::ProfileUnavailable,
                SIMULATION_PROFILE_CODE,
                "simulation adapter received a non-simulation request".to_string(),
                None,
                None,
            ));
        }
        validate_resolved_context(request, resolved).map_err(|detail| {
            simulation_failure(
                request,
                ExecutionPortFailureKind::ResolutionDenied,
                SIMULATION_OBSERVATION_CODE,
                detail,
                None,
                None,
            )
        })?;
        let Some(script) = self.scripts.get(&request.plan.request.operation_ref) else {
            return Err(simulation_failure(
                request,
                ExecutionPortFailureKind::RejectedBeforeStart,
                SIMULATION_SCRIPT_CODE,
                "no deterministic execution script matches the exact operation".to_string(),
                None,
                None,
            ));
        };
        let mut process = script.process.clone();
        if cancellation.is_some_and(|flag| flag.load(Ordering::Acquire)) {
            process.lifecycle = ExecutionLifecycleState::Cancelled;
            process.disposition = ExecutionObservedDisposition::Cancelled;
            process.start_observed = true;
            process.terminal_observed = true;
            process.teardown_observed = true;
        }
        validate_scripted_process(request, &process).map_err(|detail| {
            simulation_failure(
                request,
                ExecutionPortFailureKind::ResolutionDenied,
                SIMULATION_OBSERVATION_CODE,
                detail,
                Some(process.clone()),
                None,
            )
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
            SimulationStatus::DefinitePreStartFailure => ExecutionReconciliationStatus::DefinitePreStartFailure,
            SimulationStatus::Terminal(receipt_ref) => ExecutionReconciliationStatus::Terminal {
                receipt_ref: receipt_ref.clone(),
            },
            SimulationStatus::Unknown => ExecutionReconciliationStatus::UnknownRequiresReconciliation,
        }
    }
}

impl<P: ExecutionOutputPublisher> SimulatedExecutionAdapter<P> {
    fn publish_and_record(
        &mut self,
        request: &CanonicalExecutionRequest,
        process: ExecutionProcessObservation,
    ) -> ExecutionPortResult<CanonicalExecutionReceipt> {
        if process.lifecycle == ExecutionLifecycleState::FailedBeforeStart {
            self.operations.insert(
                request.plan.request.operation_ref.clone(),
                (request.plan.request.generation, SimulationStatus::DefinitePreStartFailure),
            );
            return Err(simulation_failure(
                request,
                ExecutionPortFailureKind::RejectedBeforeStart,
                SIMULATION_PRESTART_CODE,
                "scripted execution refused before process start".to_string(),
                Some(process),
                None,
            ));
        }
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
            simulation_failure(
                request,
                ExecutionPortFailureKind::ReceiptConstruction,
                SIMULATION_RECEIPT_CODE,
                error.to_string(),
                Some(process.clone()),
                None,
            )
        })?;
        if matches!(
            process.lifecycle,
            ExecutionLifecycleState::Unknown
                | ExecutionLifecycleState::FailedAfterStart
                | ExecutionLifecycleState::TeardownIncomplete
        ) {
            self.operations.insert(
                request.plan.request.operation_ref.clone(),
                (request.plan.request.generation, SimulationStatus::Unknown),
            );
            return Err(simulation_failure(
                request,
                ExecutionPortFailureKind::UnknownAfterStart,
                SIMULATION_UNKNOWN_CODE,
                "scripted execution lacks definitive completion and teardown evidence".to_string(),
                Some(process),
                Some(receipt),
            ));
        }
        self.operations.insert(
            request.plan.request.operation_ref.clone(),
            (request.plan.request.generation, SimulationStatus::Terminal(receipt.receipt_ref.clone())),
        );
        if publication_failed {
            return Err(simulation_failure(
                request,
                ExecutionPortFailureKind::OutputPublication,
                SIMULATION_PUBLICATION_CODE,
                "one or more simulated output streams were not published".to_string(),
                Some(process),
                Some(receipt),
            ));
        }
        Ok(receipt)
    }
}

fn validate_scripted_process(
    request: &CanonicalExecutionRequest,
    process: &ExecutionProcessObservation,
) -> Result<(), String> {
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

fn simulation_failure(
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
