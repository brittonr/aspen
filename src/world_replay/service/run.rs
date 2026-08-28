use molten_core::world_replay::*;

use super::super::*;
use super::model::*;
use super::support::*;
use super::validation::*;
use crate::error::MoltenError;
use crate::error::Result;

struct ReplayInputRecords {
    trace: CanonicalWorldReplayRecord,
    capsule: CanonicalWorldReplayRecord,
    plan: CanonicalWorldReplayRecord,
}

struct ReplayExecution {
    executions: Vec<WorldReplayExecutionObservation>,
    captures: Vec<WorldReplayCaptureObservation>,
    divergence_record: Option<CanonicalWorldReplayRecord>,
    divergence_ref: Option<String>,
    matched_steps: usize,
}

struct FinishRunInput<'a> {
    request: &'a WorldReplayPlanRequest,
    plan: WorldReplayPlan,
    records: ReplayInputRecords,
    restore: WorldReplayRestoreObservation,
    admission: WorldReplayAdmissionObservation,
    execution: ReplayExecution,
    dependency_refs: &'a [String],
    receipts: &'a mut dyn WorldReplayReceiptPort,
}

struct DeniedRunInput<'a> {
    request: &'a WorldReplayPlanRequest,
    plan: WorldReplayPlan,
    records: ReplayInputRecords,
    restore: WorldReplayRestoreObservation,
    admission: WorldReplayAdmissionObservation,
    dependency_refs: &'a [String],
    receipts: &'a mut dyn WorldReplayReceiptPort,
}

struct ReplayReceiptFields<'a> {
    request: &'a WorldReplayPlanRequest,
    decision: WorldReplayReceiptDecision,
    horizon: usize,
    actual_transition_refs: Vec<String>,
    divergence_ref: Option<String>,
    admission: &'a WorldReplayAdmissionObservation,
    dependency_refs: &'a [String],
    diagnostics: Vec<String>,
}

// r[impl molten.world_replay.execution_boundary]
// r[impl molten.world_replay.transition_chain]
// r[impl molten.world_replay.receipts]
pub fn run_world_replay(
    request: &WorldReplayPlanRequest,
    initial_commit: &crate::world_commit::CanonicalWorldCommit,
    dependency_refs: &[String],
    ports: WorldReplayPorts<'_>,
) -> Result<WorldReplayRunOutcome> {
    if initial_commit.commit_ref != request.trace.initial_commit {
        return Err(MoltenError::invalid_harness("world replay initial commit does not match the transition trace"));
    }
    validate_initial_profile(&request.trace.profile, initial_commit)?;
    validate_dependency_refs(dependency_refs)?;
    let plan = plan_world_replay(request).map_err(core_issues)?;
    let records = publish_input_records(request, &plan, ports.receipts)?;
    materialize_members(request, ports.materialization)?;
    let restore = restore_profile(request, initial_commit, ports.restore)?;
    let admission = ports.admission.observe_current(&request.trace, &request.capsule)?;
    validate_admission_binding(&request.trace, &request.capsule, &admission)?;
    if !admission.admitted() {
        return denied_run_outcome(DeniedRunInput {
            request,
            plan,
            records,
            restore,
            admission,
            dependency_refs,
            receipts: ports.receipts,
        });
    }
    let execution = execute_replay_steps(request, ports.transitions, ports.capture, ports.receipts)?;
    finish_run_outcome(FinishRunInput {
        request,
        plan,
        records,
        restore,
        admission,
        execution,
        dependency_refs,
        receipts: ports.receipts,
    })
}

fn publish_input_records(
    request: &WorldReplayPlanRequest,
    plan: &WorldReplayPlan,
    receipts: &mut dyn WorldReplayReceiptPort,
) -> Result<ReplayInputRecords> {
    let records = ReplayInputRecords {
        trace: canonical_world_transition_trace(&request.trace)?,
        capsule: canonical_world_replay_capsule(&request.capsule)?,
        plan: canonical_world_replay_plan(plan)?,
    };
    publish_exact(receipts, &records.trace)?;
    publish_exact(receipts, &records.capsule)?;
    publish_exact(receipts, &records.plan)?;
    Ok(records)
}

fn materialize_members(request: &WorldReplayPlanRequest, port: &mut dyn WorldReplayMaterializationPort) -> Result<()> {
    for member in &request.capsule.members {
        let observation = port.materialize(member)?;
        validate_materialization(member, &observation)?;
    }
    Ok(())
}

fn restore_profile(
    request: &WorldReplayPlanRequest,
    initial_commit: &crate::world_commit::CanonicalWorldCommit,
    port: &mut dyn WorldReplayRestorePort,
) -> Result<WorldReplayRestoreObservation> {
    let restore = match request.trace.profile.kind {
        WorldReplayProfileKind::Logical => port.restore_logical(&request.trace.profile, initial_commit)?,
        WorldReplayProfileKind::Opaque => port.restore_opaque_exact(&request.trace.profile, initial_commit)?,
    };
    validate_restore(&request.trace.profile, &restore)?;
    Ok(restore)
}

fn execute_replay_steps(
    request: &WorldReplayPlanRequest,
    transitions: &mut dyn WorldReplayTransitionPort,
    capture: &mut dyn WorldReplayCapturePort,
    receipts: &mut dyn WorldReplayReceiptPort,
) -> Result<ReplayExecution> {
    let mut execution = ReplayExecution {
        executions: Vec::with_capacity(request.trace.steps.len()),
        captures: Vec::with_capacity(request.trace.steps.len()),
        divergence_record: None,
        divergence_ref: None,
        matched_steps: 0,
    };
    for step in &request.trace.steps {
        let observed_execution = transitions.execute_transition(step)?;
        validate_execution(step, &observed_execution)?;
        let observed_capture = capture.capture_successor(step, &observed_execution)?;
        validate_ref(&observed_capture.observation_ref, "world replay successor capture")?;
        let comparison = compare_captured_step(request, step, &observed_capture)?;
        execution.executions.push(observed_execution);
        execution.captures.push(observed_capture);
        if let Some(divergence) = comparison.divergence {
            let record = canonical_world_replay_divergence(&divergence)?;
            execution.divergence_ref = Some(record.record_ref.clone());
            publish_exact(receipts, &record)?;
            execution.divergence_record = Some(record);
            break;
        }
        execution.matched_steps = execution
            .matched_steps
            .checked_add(comparison.matched_steps)
            .ok_or_else(|| MoltenError::invalid_harness("world replay matched-step count overflowed"))?;
    }
    Ok(execution)
}

fn compare_captured_step(
    request: &WorldReplayPlanRequest,
    step: &WorldTransitionStep,
    capture: &WorldReplayCaptureObservation,
) -> Result<WorldReplayComparison> {
    if capture.transition.position != step.position {
        return Err(MoltenError::invalid_harness(
            "world replay successor capture returned the wrong transition position",
        ));
    }
    let trace = WorldTransitionTrace {
        schema: request.trace.schema.clone(),
        trace_ref: request.trace.trace_ref.clone(),
        initial_commit: step.expected_parent.clone(),
        profile: request.trace.profile.clone(),
        steps: vec![step.clone()],
    };
    compare_world_replay(&trace, &request.commits, std::slice::from_ref(&capture.transition), &request.bounds)
        .map_err(core_issues)
}

fn finish_run_outcome(input: FinishRunInput<'_>) -> Result<WorldReplayRunOutcome> {
    let decision = if input.execution.divergence_record.is_some() {
        WorldReplayReceiptDecision::Diverged
    } else {
        WorldReplayReceiptDecision::Replayed
    };
    let diagnostics = if decision == WorldReplayReceiptDecision::Diverged {
        vec!["replay stopped at the earliest complete-world divergence".to_string()]
    } else {
        Vec::new()
    };
    let receipt_input = replay_receipt_input(ReplayReceiptFields {
        request: input.request,
        decision,
        horizon: input.execution.matched_steps,
        actual_transition_refs: input
            .execution
            .captures
            .iter()
            .map(|capture| capture.observation_ref.clone())
            .collect(),
        divergence_ref: input.execution.divergence_ref.clone(),
        admission: &input.admission,
        dependency_refs: input.dependency_refs,
        diagnostics,
    });
    let (receipt, receipt_record) = canonicalize_world_replay_receipt(receipt_input)?;
    publish_exact(input.receipts, &receipt_record)?;
    Ok(WorldReplayRunOutcome {
        plan: input.plan,
        trace_record: input.records.trace,
        capsule_record: input.records.capsule,
        plan_record: input.records.plan,
        restore: input.restore,
        admission: input.admission,
        executions: input.execution.executions,
        captures: input.execution.captures,
        divergence_record: input.execution.divergence_record,
        receipt,
        receipt_record,
    })
}

fn denied_run_outcome(input: DeniedRunInput<'_>) -> Result<WorldReplayRunOutcome> {
    let receipt_input = replay_receipt_input(ReplayReceiptFields {
        request: input.request,
        decision: WorldReplayReceiptDecision::Denied,
        horizon: 0,
        actual_transition_refs: Vec::new(),
        divergence_ref: None,
        admission: &input.admission,
        dependency_refs: input.dependency_refs,
        diagnostics: vec!["current replay admission denied".to_string()],
    });
    let (receipt, receipt_record) = canonicalize_world_replay_receipt(receipt_input)?;
    publish_exact(input.receipts, &receipt_record)?;
    Ok(WorldReplayRunOutcome {
        plan: input.plan,
        trace_record: input.records.trace,
        capsule_record: input.records.capsule,
        plan_record: input.records.plan,
        restore: input.restore,
        admission: input.admission,
        executions: Vec::new(),
        captures: Vec::new(),
        divergence_record: None,
        receipt,
        receipt_record,
    })
}

fn replay_receipt_input(fields: ReplayReceiptFields<'_>) -> WorldReplayReceipt {
    WorldReplayReceipt {
        schema: WORLD_REPLAY_RECEIPT_SCHEMA.to_string(),
        receipt_ref: placeholder_ref(),
        decision: fields.decision,
        trace_ref: fields.request.trace.trace_ref.clone(),
        capsule_ref: fields.request.capsule.capsule_ref.clone(),
        profile_ref: fields.request.trace.profile.profile_ref.as_str().to_string(),
        horizon: fields.horizon,
        actual_transition_refs: fields.actual_transition_refs,
        divergence_ref: fields.divergence_ref,
        current_admission_ref: Some(fields.admission.observation_ref.clone()),
        dependency_refs: fields.dependency_refs.to_vec(),
        diagnostics: fields.diagnostics,
        non_claims: world_replay_non_claims(),
    }
}
