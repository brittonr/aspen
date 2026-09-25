
pub fn parse_job_output_request_value(value: &IoValue, expected_dag_ref: &str) -> Result<JobOutputRequest> {
    let fields = value
        .collect_simple_record("job-output-request-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-output-request-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::JOB_DAG_OUTPUT_REQUEST_SCHEMA, "job output request")?;
    let dag_ref = record_ref(&fields[1], "dag")?;
    if dag_ref != expected_dag_ref {
        return Err(MoltenError::invalid_harness(format!(
            "job output request dag ref {dag_ref} does not match job {expected_dag_ref}"
        )));
    }
    let roots = record_node_id_sequence(&fields[2], "roots")?;
    let materialization = record_string(&fields[3], "materialization")?;
    validate_request_materialization(&materialization)?;
    let checks = parse_checks(&fields[7])?;
    require_check(&checks, "request-ref-bound", "job output request")?;
    Ok(JobOutputRequest {
        request_ref: crate::preserves_rail::canonical_hash(value)?,
        dag_ref,
        roots,
        materialization,
        policy_refs: record_ref_sequence(&fields[4], "policy")?,
        handler_profile_ref: record_optional_ref(&fields[5], "handler-profile")?,
        seed_config_ref: record_optional_ref(&fields[6], "seed-config")?,
        value: value.clone(),
    })
}

pub fn parse_job_receipt(value: &IoValue) -> Result<JobReceipt> {
    let fields = value
        .collect_simple_record("job-dag-receipt-v1", Some(14))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-dag-receipt-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::JOB_DAG_RECEIPT_SCHEMA, "job dag receipt")?;
    let checks = parse_checks(&fields[13])?;
    require_check(&checks, "canonical-receipt", "job dag receipt")?;
    Ok(JobReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        decision: record_string(&fields[2], "decision")?,
        job_ref: record_optional_ref(&fields[3], "job")?,
        request_ref: record_optional_ref(&fields[4], "request")?,
        stage_id: record_optional_string(&fields[5], "stage")?,
        input_refs: record_ref_sequence(&fields[6], "inputs")?,
        output_refs: record_ref_sequence(&fields[7], "outputs")?,
        cache_ref: record_optional_ref(&fields[8], "cache")?,
        checks,
        value: value.clone(),
    })
}

pub fn parse_job_admission_receipt_value(value: &IoValue) -> Result<JobAdmissionReceipt> {
    let fields = value
        .collect_simple_record("job-admission-receipt-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-admission-receipt-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::JOB_ADMISSION_RECEIPT_SCHEMA, "job admission receipt")?;
    let checks = parse_checks(&fields[14])?;
    require_check(&checks, "canonical-receipt", "job admission receipt")?;
    Ok(JobAdmissionReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        decision: record_string(&fields[2], "decision")?,
        job_ref: record_ref(&fields[3], "job")?,
        request_ref: record_ref(&fields[4], "request")?,
        plan_ref: record_ref(&fields[5], "artifact")?,
        sync_ref: record_ref(&fields[6], "sync")?,
        target_peer: record_string(&fields[7], "target-peer")?,
        closure_refs: record_ref_sequence(&fields[8], "closure")?,
        stage_order: record_node_id_sequence(&fields[9], "stages")?,
        authority_receipt_refs: record_ref_sequence(&fields[10], "authority")?,
        source_gate_validation_refs: Vec::new(),
        resource_verdict: record_string(&fields[11], "resource-verdict")?,
        diagnostics: record_string_sequence(&fields[12], "diagnostics")?,
        refs: record_ref_sequence(&fields[13], "refs")?,
        checks,
        value: value.clone(),
    })
}

pub fn install_job_dag(registry_root: &FilePath, value: &IoValue) -> Result<JobInstall> {
    let dag = parse_job_dag_value(value)?;
    let stage_deps = dag.nodes.iter().filter_map(|node| node.stage_artifact_ref.clone()).collect::<Vec<_>>();
    let install = crate::artifacts::install_artifact(registry_root, &crate::artifacts::ArtifactInstallInput {
        kind: JOB_ARTIFACT_KIND.to_string(),
        payload: dag.value.clone(),
        schema_refs: dag.schema_refs.clone(),
        dependency_refs: sorted_unique(&stage_deps),
        effect_manifest_ref: None,
        policy_refs: if dag.policy_refs.is_empty() {
            vec![local_ref("job-install-policy", &dag.job_ref)?]
        } else {
            dag.policy_refs.clone()
        },
        evidence_refs: if dag.evidence_refs.is_empty() {
            vec![local_ref("job-install-evidence", &dag.job_ref)?]
        } else {
            dag.evidence_refs.clone()
        },
        installer_ref: local_ref("job-installer", &dag.job_ref)?,
        capability_refs: vec![local_ref("job-install-capability", &dag.job_ref)?],
    })?;
    let artifact_receipt_ref = crate::preserves_rail::canonical_hash(&install.receipt_value)?;
    let evidence_refs = vec![artifact_receipt_ref];
    let diagnostics = install
        .missing_dependencies
        .iter()
        .map(|reference| format!("missing stage dependency {reference}"))
        .collect::<Vec<_>>();
    let receipt_value = job_receipt_value(JobReceiptInput {
        operation: "install",
        decision: &install.decision,
        job_ref: Some(&dag.job_ref),
        request_ref: None,
        stage_id: None,
        input_refs: &stage_deps,
        output_refs: std::slice::from_ref(&install.artifact_ref),
        cache_ref: None,
        effect_refs: &[],
        policy_refs: &install.artifact.policy_refs,
        evidence_refs: &evidence_refs,
        diagnostics: &diagnostics,
        checks: &[
            ("canonical-dag", "pass"),
            ("no-mobile-closures", "pass"),
            ("artifact-registry-install", if install.decision == "pass" { "pass" } else { "fail" }),
        ],
    })?;
    Ok(JobInstall {
        job_ref: dag.job_ref,
        artifact_ref: install.artifact_ref,
        decision: install.decision,
        receipt_value,
        artifact_receipt_value: install.receipt_value,
    })
}

pub fn job_artifact_ref(registry_root: &FilePath, job_ref: &str) -> Result<String> {
    validate_ref(job_ref, "job artifact lookup ref")?;
    for artifact in crate::artifacts::list_artifacts(registry_root, Some(JOB_ARTIFACT_KIND))? {
        let payload = crate::artifacts::read_payload(registry_root, &artifact.artifact_ref)?;
        let dag = parse_job_dag_value(&payload)?;
        if dag.job_ref == job_ref || artifact.artifact_ref == job_ref {
            return Ok(artifact.artifact_ref);
        }
    }
    Err(MoltenError::invalid_harness(format!("job artifact {job_ref} not found in registry")))
}

pub fn job_artifact_ref_with_root(
    registry_root: &crate::artifacts::CapabilityArtifactRoot,
    job_ref: &str,
) -> Result<String> {
    validate_ref(job_ref, "job artifact lookup ref")?;
    for artifact in crate::artifacts::list_artifacts_with_root(registry_root, Some(JOB_ARTIFACT_KIND))? {
        let payload = crate::artifacts::read_payload_with_root(registry_root, &artifact.artifact_ref)?;
        let dag = parse_job_dag_value(&payload)?;
        if dag.job_ref == job_ref || artifact.artifact_ref == job_ref {
            return Ok(artifact.artifact_ref);
        }
    }
    Err(MoltenError::invalid_harness(format!("job artifact {job_ref} not found in registry")))
}

pub fn read_job_dag(registry_root: &FilePath, reference: &str) -> Result<JobDag> {
    if validate_ref(reference, "job ref").is_ok() {
        if let Ok(payload) = crate::artifacts::read_payload(registry_root, reference)
            && let Ok(dag) = parse_job_dag_value(&payload)
        {
            return Ok(dag);
        }
        for artifact in crate::artifacts::list_artifacts(registry_root, Some(JOB_ARTIFACT_KIND))? {
            let payload = crate::artifacts::read_payload(registry_root, &artifact.artifact_ref)?;
            let dag = parse_job_dag_value(&payload)?;
            if dag.job_ref == reference || artifact.artifact_ref == reference {
                return Ok(dag);
            }
        }
    }
    Err(MoltenError::invalid_harness(format!("job dag {reference} not found in registry")))
}

pub fn read_job_dag_with_root(
    registry_root: &crate::artifacts::CapabilityArtifactRoot,
    reference: &str,
) -> Result<JobDag> {
    if validate_ref(reference, "job ref").is_ok() {
        if let Ok(payload) = crate::artifacts::read_payload_with_root(registry_root, reference)
            && let Ok(dag) = parse_job_dag_value(&payload)
        {
            return Ok(dag);
        }
        for artifact in crate::artifacts::list_artifacts_with_root(registry_root, Some(JOB_ARTIFACT_KIND))? {
            let payload = crate::artifacts::read_payload_with_root(registry_root, &artifact.artifact_ref)?;
            let dag = parse_job_dag_value(&payload)?;
            if dag.job_ref == reference || artifact.artifact_ref == reference {
                return Ok(dag);
            }
        }
    }
    Err(MoltenError::invalid_harness(format!("job dag {reference} not found in registry")))
}

pub fn read_job_dag_file_or_registry(registry_root: &FilePath, spec: &str) -> Result<JobDag> {
    let path = FilePath::new(spec);
    if path.exists() {
        let text = std::fs::read_to_string(path).map_err(MoltenError::from)?;
        let value = crate::preserves_rail::parse_text(&text)?;
        parse_job_dag_value(&value)
    } else {
        read_job_dag(registry_root, spec)
    }
}

pub fn run_job_dag_value(value: &IoValue, options: &JobRunOptions<'_>) -> Result<JobRun> {
    let dag = parse_job_dag_value(value)?;
    run_job_dag(&dag, options)
}

pub fn run_job_dag(dag: &JobDag, options: &JobRunOptions<'_>) -> Result<JobRun> {
    let request = request_for_dag(dag, options.output_request.as_ref())?;
    ensure_count_at_most(dag.nodes.len(), MAX_JOB_NODES, "job run nodes")?;
    ensure_count_at_most(dag.edges.len(), MAX_JOB_EDGES, "job run edges")?;
    let plan = trellis_execution_plan(&dag.nodes, &dag.edges)?;
    let stages = run_stages(dag, &request, &plan, options)?;
    let finish = complete_run(CompleteInput {
        dag,
        request: &request,
        plan: &plan,
        outputs_by_index: &stages.outputs_by_index,
        output_refs_by_index: &stages.output_refs_by_index,
        stage_receipt_refs: &stages.receipt_refs,
    })?;
    if let Some(ledger_root) = options.ledger_root {
        crate::ledger::import_artifact(ledger_root, &finish.receipt_value)?;
    }
    Ok(JobRun {
        job_ref: dag.job_ref.clone(),
        request_ref: request.request_ref,
        stage_receipt_refs: stages.receipt_refs,
        output_refs: finish.output_refs,
        output_value: finish.output_value,
        receipt_value: finish.receipt_value,
    })
}

fn run_job_dag_with_capabilities(
    dag: &JobDag,
    options: &CapabilityJobRunOptions<'_>,
) -> Result<JobRun> {
    let request = request_for_dag(dag, None)?;
    ensure_count_at_most(dag.nodes.len(), MAX_JOB_NODES, "job run nodes")?;
    ensure_count_at_most(dag.edges.len(), MAX_JOB_EDGES, "job run edges")?;
    let plan = trellis_execution_plan(&dag.nodes, &dag.edges)?;
    let stages = run_stages_with_capabilities(dag, &request, &plan, options)?;
    let finish = complete_run(CompleteInput {
        dag,
        request: &request,
        plan: &plan,
        outputs_by_index: &stages.outputs_by_index,
        output_refs_by_index: &stages.output_refs_by_index,
        stage_receipt_refs: &stages.receipt_refs,
    })?;
    Ok(JobRun {
        job_ref: dag.job_ref.clone(),
        request_ref: request.request_ref,
        stage_receipt_refs: stages.receipt_refs,
        output_refs: finish.output_refs,
        output_value: finish.output_value,
        receipt_value: finish.receipt_value,
    })
}

struct RunStages {
    receipt_refs: Vec<String>,
    outputs_by_index: Vec<Option<Vec<IoValue>>>,
    output_refs_by_index: Vec<Option<Vec<String>>>,
}
