
struct GcStepInput<'a> {
    root: &'a Path,
    bundle_dir: &'a Path,
    ledger_root: &'a Path,
    registry_root: &'a Path,
    state_root_ref: &'a str,
    checkpoints: &'a mut StepCheckpointBuffers,
}

fn record_gc_steps(input: GcStepInput<'_>) -> Result<GcRun> {
    let GcStepInput {
        root,
        bundle_dir,
        ledger_root,
        registry_root,
        state_root_ref,
        checkpoints,
    } = input;
    let retention_gc = run_gc_workflow(GcWorkflowInput {
        root,
        bundle_dir,
        ledger_root,
        registry_root,
    })?;
    record_gc_plan_steps(checkpoints, state_root_ref, &retention_gc)?;
    record_gc_review_steps(checkpoints, state_root_ref, &retention_gc)?;
    Ok(retention_gc)
}

fn record_gc_plan_steps(
    checkpoints: &mut StepCheckpointBuffers,
    state_root_ref: &str,
    retention_gc: &GcRun,
) -> Result<()> {
    push_step_checkpoint(checkpoints, StepCheckpointInput {
        name: "plan-retention-gc",
        request_ref: Some(&retention_gc.object_ref),
        receipt_ref: Some(&retention_gc.plan_ref),
        result_ref: Some(&retention_gc.plan_ref),
        decision: &retention_gc.plan_decision,
        replay_status: "deterministic",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&retention_gc.plan_ref),
        diagnostics: &retention_gc.plan_diagnostics,
        state_root_ref,
    })?;
    push_step_checkpoint(checkpoints, StepCheckpointInput {
        name: "apply-retention-gc-plan",
        request_ref: Some(&retention_gc.plan_ref),
        receipt_ref: Some(&retention_gc.apply_ref),
        result_ref: Some(&retention_gc.apply_ref),
        decision: &retention_gc.apply_decision,
        replay_status: "recorded",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&retention_gc.apply_ref),
        diagnostics: &retention_gc.apply_diagnostics,
        state_root_ref,
    })?;
    push_step_checkpoint(checkpoints, StepCheckpointInput {
        name: "execute-retention-gc",
        request_ref: Some(&retention_gc.apply_ref),
        receipt_ref: Some(&retention_gc.execution_ref),
        result_ref: Some(&retention_gc.execution_ref),
        decision: &retention_gc.execution_decision,
        replay_status: "deterministic",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&retention_gc.execution_ref),
        diagnostics: &retention_gc.execution_diagnostics,
        state_root_ref,
    })
}

fn record_gc_review_steps(
    checkpoints: &mut StepCheckpointBuffers,
    state_root_ref: &str,
    retention_gc: &GcRun,
) -> Result<()> {
    push_step_checkpoint(checkpoints, StepCheckpointInput {
        name: "audit-retention-gc",
        request_ref: Some(&retention_gc.execution_ref),
        receipt_ref: Some(&retention_gc.audit_ref),
        result_ref: Some(&retention_gc.audit_ref),
        decision: &retention_gc.audit_decision,
        replay_status: "deterministic",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&retention_gc.audit_ref),
        diagnostics: &retention_gc.audit_diagnostics,
        state_root_ref,
    })?;
    push_step_checkpoint(checkpoints, StepCheckpointInput {
        name: "export-retention-gc-bundle",
        request_ref: Some(&retention_gc.explain_ref),
        receipt_ref: Some(&retention_gc.bundle_verify_ref),
        result_ref: Some(&retention_gc.bundle_ref),
        decision: &retention_gc.bundle_verify_decision,
        replay_status: "recorded",
        mandatory: true,
        artifact_refs: &[
            retention_gc.bundle_ref.clone(),
            retention_gc.bundle_profile_ref.clone(),
            retention_gc.bundle_verify_ref.clone(),
        ],
        diagnostics: &retention_gc.bundle_diagnostics,
        state_root_ref,
    })?;
    push_step_checkpoint(checkpoints, StepCheckpointInput {
        name: "search-retention-gc-catalog",
        request_ref: Some(&retention_gc.catalog_request_ref),
        receipt_ref: Some(&retention_gc.catalog_receipt_ref),
        result_ref: Some(&retention_gc.catalog_response_ref),
        decision: &retention_gc.catalog_decision,
        replay_status: "deterministic",
        mandatory: true,
        artifact_refs: &retention_gc.artifact_refs,
        diagnostics: &[],
        state_root_ref,
    })
}

struct FinishInput<'a> {
    ledger_root: &'a Path,
    state_root_ref: &'a str,
    startup_ref: &'a str,
    node_started: &'a crate::node_runtime::NodeRuntimeStart,
    installed: &'a crate::artifacts::ArtifactInstall,
    job: &'a JobRun,
    retention_gc: &'a GcRun,
    step_checkpoints: StepCheckpointBuffers,
    policy_refs: &'a [String],
    capability_refs: &'a [String],
    resource_refs: &'a [String],
    gate_receipt_refs: Vec<String>,
    repro_bundle_refs: Vec<String>,
    harness_gate_refs: Vec<String>,
    catalog_query_refs: Vec<String>,
    repro_verify_refs: Vec<String>,
    replay_index_refs: Vec<String>,
}

struct ReplayShutdownInput<'a> {
    state_root_ref: &'a str,
    startup_ref: &'a str,
    node_started: &'a crate::node_runtime::NodeRuntimeStart,
    installed: &'a crate::artifacts::ArtifactInstall,
    job: &'a JobRun,
    step_checkpoints: StepCheckpointBuffers,
    replay_index_refs: Vec<String>,
}

struct ReplayShutdown {
    replay_verify: crate::deterministic_replay::ReplayVerifyReceipt,
    replay_index: crate::deterministic_replay::ReplayIndexReceipt,
    shutdown_ref: String,
    health_ref: String,
    step_checkpoints: StepCheckpointBuffers,
    replay_index_refs: Vec<String>,
}

struct ReplayStep {
    replay_verify: crate::deterministic_replay::ReplayVerifyReceipt,
    replay_index: crate::deterministic_replay::ReplayIndexReceipt,
    replay_index_refs: Vec<String>,
}

struct ShutdownStepInput<'a> {
    state_root_ref: &'a str,
    startup_ref: &'a str,
    node_started: &'a crate::node_runtime::NodeRuntimeStart,
    installed: &'a crate::artifacts::ArtifactInstall,
    job: &'a JobRun,
    checkpoints: &'a mut StepCheckpointBuffers,
}

struct ShutdownStep {
    shutdown_ref: String,
    health_ref: String,
}

fn record_replay_shutdown(input: ReplayShutdownInput<'_>) -> Result<ReplayShutdown> {
    let ReplayShutdownInput {
        state_root_ref,
        startup_ref,
        node_started,
        installed,
        job,
        mut step_checkpoints,
        replay_index_refs,
    } = input;
    let replay = record_replay_step(state_root_ref, &mut step_checkpoints, replay_index_refs)?;
    let shutdown = record_shutdown_step(ShutdownStepInput {
        state_root_ref,
        startup_ref,
        node_started,
        installed,
        job,
        checkpoints: &mut step_checkpoints,
    })?;
    Ok(ReplayShutdown {
        replay_verify: replay.replay_verify,
        replay_index: replay.replay_index,
        shutdown_ref: shutdown.shutdown_ref,
        health_ref: shutdown.health_ref,
        step_checkpoints,
        replay_index_refs: replay.replay_index_refs,
    })
}

fn record_replay_step(
    state_root_ref: &str,
    checkpoints: &mut StepCheckpointBuffers,
    mut replay_index_refs: Vec<String>,
) -> Result<ReplayStep> {
    let replay_verify =
        crate::deterministic_replay::verify_fixture_value(crate::deterministic_replay::ReplayFixtureVariant::Baseline)?;
    let replay_index =
        crate::deterministic_replay::index_replay_evidence(&[crate::deterministic_replay::ReplayIndexInput {
            expected_ref: Some(replay_verify.receipt_ref.clone()),
            value: replay_verify.value.clone(),
        }])?;
    replay_index_refs.push_limited_value(
        replay_index.index_ref.clone(),
        MAX_OPERATOR_REFS,
        "dogfood replay index refs",
    )?;
    push_step_checkpoint(checkpoints, StepCheckpointInput {
        name: "index-replay-evidence",
        request_ref: Some(&replay_verify.receipt_ref),
        receipt_ref: Some(&replay_index.index_ref),
        result_ref: Some(&replay_index.index_ref),
        decision: &replay_index.decision,
        replay_status: "deterministic",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&replay_verify.receipt_ref),
        diagnostics: &[],
        state_root_ref,
    })?;
    Ok(ReplayStep {
        replay_verify,
        replay_index,
        replay_index_refs,
    })
}

fn record_shutdown_step(input: ShutdownStepInput<'_>) -> Result<ShutdownStep> {
    let shutdown = crate::node_runtime::node_shutdown_receipt_value(&crate::node_runtime::ShutdownReceiptValueInput {
        decision: "pass",
        startup_receipt_ref: input.startup_ref,
        adapter_receipts: &input.node_started.adapter_receipts,
        drained_job_refs: std::slice::from_ref(&input.job.execution_receipt_ref),
        index_receipt_refs: &[dogfood_ref("shutdown-index")?],
        diagnostics: &[],
    })?;
    let shutdown_ref = crate::preserves_rail::canonical_hash(&shutdown)?;
    let health =
        crate::node_runtime::node_restart_health_receipt_value(&crate::node_runtime::RestartHealthReceiptValueInput {
            startup_receipt: &input.node_started.startup_receipt,
            shutdown_receipt_ref: Some(&shutdown_ref),
            index_receipt_refs: &[dogfood_ref("restart-health-index")?],
            head_refs: &[
                input.installed.artifact_ref.clone(),
                input.job.execution_receipt_ref.clone(),
            ],
            open_job_refs: &[],
            diagnostics: &[],
        })?;
    let health_ref = crate::preserves_rail::canonical_hash(&health)?;
    push_step_checkpoint(input.checkpoints, StepCheckpointInput {
        name: "shutdown-node",
        request_ref: Some(input.startup_ref),
        receipt_ref: Some(&shutdown_ref),
        result_ref: Some(&health_ref),
        decision: "pass",
        replay_status: "recorded",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&health_ref),
        diagnostics: &[],
        state_root_ref: input.state_root_ref,
    })?;
    Ok(ShutdownStep {
        shutdown_ref,
        health_ref,
    })
}

struct FinishReportInput<'a> {
    state_root_ref: &'a str,
    shutdown_ref: &'a str,
    health_ref: &'a str,
    checkpoints: &'a StepCheckpointBuffers,
    policy_refs: &'a [String],
    capability_refs: &'a [String],
    resource_refs: &'a [String],
    gate_receipt_refs: &'a [String],
    repro_bundle_refs: &'a [String],
}
