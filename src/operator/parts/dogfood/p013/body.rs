
struct ServiceStepInput<'a> {
    state_root_ref: &'a str,
    checkpoints: &'a mut StepCheckpointBuffers,
}

fn record_service_step(input: ServiceStepInput<'_>) -> Result<()> {
    let service_suite = crate::service_runtime::two_service_suite_value()?;
    let service_run = crate::service_runtime::run_suite_value(&service_suite)?;
    let service_decision = if service_run.lifecycle_receipts.iter().all(service_lifecycle_pass) {
        "pass"
    } else {
        "deny"
    };
    push_step_checkpoint(input.checkpoints, StepCheckpointInput {
        name: "start-service",
        request_ref: Some(&crate::preserves_rail::canonical_hash(&service_suite)?),
        receipt_ref: Some(&service_run.report_ref),
        result_ref: Some(&service_run.report_ref),
        decision: service_decision,
        replay_status: "deterministic",
        mandatory: true,
        artifact_refs: &service_run
            .readiness_assertions
            .iter()
            .map(crate::preserves_rail::canonical_hash)
            .collect::<Result<Vec<_>>>()?,
        diagnostics: &[],
        state_root_ref: input.state_root_ref,
    })
}

struct RemoteStepInput<'a> {
    state_root: &'a Path,
    state_root_ref: &'a str,
    policy_refs: &'a [String],
    resource_refs: &'a [String],
    checkpoints: &'a mut StepCheckpointBuffers,
}

struct RemoteStep {
    run: crate::remote_dataspace::TwoPeerHarness,
    gate_ref: String,
}

fn record_remote_step(input: RemoteStepInput<'_>) -> Result<RemoteStep> {
    let remote = crate::remote_dataspace::two_peer_service_ready_harness(
        &input.state_root.join("remote-dataspace"),
        crate::remote_dataspace::DeliveryEvidence {
            peer_bootstrap_refs: vec![dogfood_ref("remote-peer-bootstrap")?],
            capability_refs: vec![dogfood_ref("remote-capability")?],
            policy_refs: input.policy_refs.to_vec(),
            resource_refs: input.resource_refs.to_vec(),
            authority_refs: vec![dogfood_ref("remote-authority")?],
        },
    )?;
    let gate_ref = crate::preserves_rail::canonical_hash(&remote.receipt_value)?;
    push_step_checkpoint(input.checkpoints, StepCheckpointInput {
        name: "publish-remote-assertion",
        request_ref: Some(&remote.delivery_log.log_ref),
        receipt_ref: Some(&gate_ref),
        result_ref: Some(&gate_ref),
        decision: "pass",
        replay_status: "recorded",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&remote.delivery_log.log_ref),
        diagnostics: &[],
        state_root_ref: input.state_root_ref,
    })?;
    Ok(RemoteStep { run: remote, gate_ref })
}

struct JobStepInput<'a> {
    state_root: &'a Path,
    source: &'a Path,
    target: &'a Path,
    state_root_ref: &'a str,
    policy_refs: &'a [String],
    capability_refs: &'a [String],
    resource_refs: &'a [String],
    checkpoints: &'a mut StepCheckpointBuffers,
}

fn record_job_step(input: JobStepInput<'_>) -> Result<JobRun> {
    let job = run_job_stack(JobStackInput {
        state_root: input.state_root,
        source: input.source,
        target: input.target,
        policy_refs: input.policy_refs,
        capability_refs: input.capability_refs,
        resource_refs: input.resource_refs,
    })?;
    push_step_checkpoint(input.checkpoints, StepCheckpointInput {
        name: "run-job-dag",
        request_ref: Some(&job.execution_request_ref),
        receipt_ref: Some(&job.execution_receipt_ref),
        result_ref: Some(&job.execution_receipt_ref),
        decision: &job.decision,
        replay_status: "recorded",
        mandatory: true,
        artifact_refs: &job.artifact_refs,
        diagnostics: &job.diagnostics,
        state_root_ref: input.state_root_ref,
    })?;
    Ok(job)
}

struct CatalogStepInput<'a> {
    ledger_root: &'a Path,
    registry_root: &'a Path,
    state_root_ref: &'a str,
    installed: &'a crate::artifacts::ArtifactInstall,
    remote_gate_value: &'a IoValue,
    checkpoints: &'a mut StepCheckpointBuffers,
}

fn record_catalog_step(input: CatalogStepInput<'_>) -> Result<String> {
    crate::ledger::import_artifact(input.ledger_root, &input.installed.artifact.value)?;
    crate::ledger::import_artifact(input.ledger_root, input.remote_gate_value)?;
    let mcp_request =
        crate::catalog_mcp::mcp_request_value("catalog.list", vec![crate::preserves_rail::record("kind", vec![
            crate::preserves_rail::string("operator-artifact"),
        ])])?;
    let mcp_call = crate::catalog_mcp::call(input.registry_root, Some(input.ledger_root), &mcp_request)?;
    let mcp_receipt_ref = crate::preserves_rail::canonical_hash(&mcp_call.receipt_value)?;
    push_step_checkpoint(input.checkpoints, StepCheckpointInput {
        name: "query-catalog-mcp",
        request_ref: Some(&mcp_call.request.request_ref),
        receipt_ref: Some(&mcp_receipt_ref),
        result_ref: Some(&mcp_call.response_ref),
        decision: &mcp_call.decision,
        replay_status: "deterministic",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&mcp_call.response_ref),
        diagnostics: &[],
        state_root_ref: input.state_root_ref,
    })?;
    Ok(mcp_receipt_ref)
}

struct ReproStepInput<'a> {
    state_root_ref: &'a str,
    remote_gate_ref: &'a str,
    checkpoints: &'a mut StepCheckpointBuffers,
}

fn record_repro_steps(input: ReproStepInput<'_>) -> Result<DogfoodRepro> {
    let repro = build_dogfood_repro()?;
    push_step_checkpoint(input.checkpoints, StepCheckpointInput {
        name: "export-redacted-repro",
        request_ref: Some(&repro.report_ref),
        receipt_ref: Some(&repro.verify_ref),
        result_ref: Some(&repro.bundle_ref),
        decision: "pass",
        replay_status: "recorded",
        mandatory: true,
        artifact_refs: &[repro.gate_ref.clone(), repro.bundle_ref.clone()],
        diagnostics: &[],
        state_root_ref: input.state_root_ref,
    })?;
    push_step_checkpoint(input.checkpoints, StepCheckpointInput {
        name: "gate-evidence",
        request_ref: Some(&repro.report_ref),
        receipt_ref: Some(&repro.gate_ref),
        result_ref: Some(&repro.gate_ref),
        decision: "pass",
        replay_status: "deterministic",
        mandatory: true,
        artifact_refs: &[input.remote_gate_ref.to_string()],
        diagnostics: &[],
        state_root_ref: input.state_root_ref,
    })?;
    Ok(repro)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RetentionAdmissionFixtureInput<'a> {
    root: &'a Path,
    kind: &'a str,
    label: &'a str,
    requester_ref: &'a str,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
    action: &'a str,
    remote_refs: &'a [String],
}

struct JobParts {
    job_ref: String,
    provenance_values: Vec<IoValue>,
}

struct StageArtifacts {
    base_ref: String,
    source_ref: String,
    map_ref: String,
}

struct JobSyncInput<'a> {
    source: &'a Path,
    target: &'a Path,
    parts: &'a JobParts,
    policy_refs: &'a [String],
    capability_refs: &'a [String],
}

struct JobAdmissionParts {
    authority_ref: String,
    receipt_ref: String,
    receipt_value: IoValue,
    stage_order: Vec<String>,
}

struct JobAdmissionInput<'a> {
    target: &'a Path,
    job_ref: &'a str,
    sync_ref: &'a str,
    policy_refs: &'a [String],
    capability_refs: &'a [String],
    resource_refs: &'a [String],
}

struct JobExecutionParts {
    request_ref: String,
    receipt_ref: String,
    decision: String,
    diagnostics: Vec<String>,
    output_refs: Vec<String>,
}

struct JobExecutionInput<'a> {
    state_root: &'a Path,
    target: &'a Path,
    job_ref: &'a str,
    admission: &'a JobAdmissionParts,
    policy_refs: &'a [String],
    resource_refs: &'a [String],
}

fn run_job_stack(input: JobStackInput<'_>) -> Result<JobRun> {
    let JobStackInput {
        state_root,
        source,
        target,
        policy_refs,
        capability_refs,
        resource_refs,
    } = input;
    let parts = install_job_parts(source, policy_refs, capability_refs)?;
    let sync_ref = sync_job_stack(JobSyncInput {
        source,
        target,
        parts: &parts,
        policy_refs,
        capability_refs,
    })?;
    let admission = admit_job_stack(JobAdmissionInput {
        target,
        job_ref: &parts.job_ref,
        sync_ref: &sync_ref,
        policy_refs,
        capability_refs,
        resource_refs,
    })?;
    let execution = execute_job_stack(JobExecutionInput {
        state_root,
        target,
        job_ref: &parts.job_ref,
        admission: &admission,
        policy_refs,
        resource_refs,
    })?;
    let mut artifact_refs = vec![
        parts.job_ref,
        sync_ref,
        admission.receipt_ref,
        admission.authority_ref,
        execution.request_ref.clone(),
    ];
    artifact_refs.extend(execution.output_refs);
    Ok(JobRun {
        execution_request_ref: execution.request_ref,
        execution_receipt_ref: execution.receipt_ref,
        decision: execution.decision,
        diagnostics: execution.diagnostics,
        artifact_refs,
    })
}
