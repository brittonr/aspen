
fn install_job_parts(source: &Path, policy_refs: &[String], capability_refs: &[String]) -> Result<JobParts> {
    let stages = install_stage_artifacts(source, policy_refs, capability_refs)?;
    let dag = job_graph_value(&stages, policy_refs)?;
    let installed = crate::job_dag::install_job_dag(source, &dag)?;
    let provenance_refs = vec![
        stages.base_ref,
        stages.source_ref,
        stages.map_ref,
        installed.artifact_ref,
    ];
    Ok(JobParts {
        job_ref: installed.job_ref,
        provenance_values: provenance_values(&provenance_refs)?,
    })
}

fn install_stage_artifacts(
    source: &Path,
    policy_refs: &[String],
    capability_refs: &[String],
) -> Result<StageArtifacts> {
    let base = crate::artifacts::install_artifact(source, &crate::artifacts::ArtifactInstallInput {
        kind: "schema".to_string(),
        payload: crate::preserves_rail::record("schema", vec![crate::preserves_rail::string("dogfood-job-base")]),
        schema_refs: vec![dogfood_ref("job-schema")?],
        dependency_refs: Vec::new(),
        effect_manifest_ref: None,
        policy_refs: policy_refs.to_vec(),
        evidence_refs: vec![dogfood_ref("job-evidence")?],
        installer_ref: dogfood_ref("job-installer")?,
        capability_refs: capability_refs.to_vec(),
    })?;
    let source_stage = crate::artifacts::install_artifact(source, &crate::artifacts::ArtifactInstallInput {
        kind: "stage".to_string(),
        payload: crate::job_dag::builtin_stage_operation_value("source")?,
        schema_refs: vec![dogfood_ref("job-stage-schema")?],
        dependency_refs: Vec::new(),
        effect_manifest_ref: None,
        policy_refs: policy_refs.to_vec(),
        evidence_refs: vec![dogfood_ref("job-stage-evidence")?],
        installer_ref: dogfood_ref("job-stage-installer")?,
        capability_refs: capability_refs.to_vec(),
    })?;
    let map_stage = crate::artifacts::install_artifact(source, &crate::artifacts::ArtifactInstallInput {
        kind: "stage".to_string(),
        payload: crate::job_dag::builtin_stage_operation_value("identity")?,
        schema_refs: vec![dogfood_ref("job-stage-schema")?],
        dependency_refs: vec![base.artifact_ref.clone()],
        effect_manifest_ref: None,
        policy_refs: policy_refs.to_vec(),
        evidence_refs: vec![dogfood_ref("job-stage-evidence")?],
        installer_ref: dogfood_ref("job-stage-installer")?,
        capability_refs: capability_refs.to_vec(),
    })?;
    Ok(StageArtifacts {
        base_ref: base.artifact_ref,
        source_ref: source_stage.artifact_ref,
        map_ref: map_stage.artifact_ref,
    })
}

fn job_graph_value(stages: &StageArtifacts, policy_refs: &[String]) -> Result<IoValue> {
    let source_node = crate::job_dag::job_node_value(crate::job_dag::NodeValueInput {
        id: "source",
        kind: "source",
        stage_artifact_ref: Some(&stages.source_ref),
        input_ports: &[],
        output_ports: &["out".to_string()],
        config: crate::preserves_rail::record("source", vec![crate::preserves_rail::record("values", vec![
            crate::preserves_rail::sequence(vec![crate::preserves_rail::string("dogfood-job")]),
        ])]),
        effect_manifest_refs: &[],
        policy_refs: &[],
        evidence_refs: &[],
    })?;
    let map_node = crate::job_dag::job_node_value(crate::job_dag::NodeValueInput {
        id: "map",
        kind: "map",
        stage_artifact_ref: Some(&stages.map_ref),
        input_ports: &["in".to_string()],
        output_ports: &["out".to_string()],
        config: crate::preserves_rail::record("op", vec![crate::preserves_rail::string("identity")]),
        effect_manifest_refs: &[],
        policy_refs: &[],
        evidence_refs: &[],
    })?;
    let edge = crate::job_dag::job_edge_value(crate::job_dag::EdgeValueInput {
        from_node: "source",
        from_port: "out",
        to_node: "map",
        to_port: "in",
        schema_ref: None,
        partitioning: "single",
        materialization: "stream",
    })?;
    crate::job_dag::job_dag_value(crate::job_dag::DagValueInput {
        nodes: vec![source_node, map_node],
        edges: vec![edge],
        output_roots: &["map".to_string()],
        schema_refs: &[],
        effect_manifest_refs: &[],
        policy_refs,
        evidence_refs: std::slice::from_ref(&stages.base_ref),
    })
}

fn provenance_values(artifact_refs: &[String]) -> Result<Vec<IoValue>> {
    let mut values = Vec::with_capacity(artifact_refs.len());
    for artifact_ref in artifact_refs {
        values.push_limited_value(
            crate::provenance::synthetic_reviewed_record(artifact_ref)?,
            MAX_OPERATOR_REFS,
            "dogfood sync provenance",
        )?;
    }
    Ok(values)
}

fn sync_job_stack(input: JobSyncInput<'_>) -> Result<String> {
    let sync_request = crate::job_dag::job_sync_request_value(crate::job_dag::SyncRequestValueInput {
        job_ref: &input.parts.job_ref,
        stage_ids: &[],
        target_peer: "peer:dogfood",
        policy_refs: input.policy_refs,
        capability_refs: input.capability_refs,
        evidence_refs: &[dogfood_ref("job-sync-evidence")?],
    })?;
    let sync = crate::job_dag::sync_loopback(crate::job_dag::SyncLoopbackInput {
        source_registry: input.source,
        target_registry: input.target,
        request_value: &sync_request,
        provenance_values: &input.parts.provenance_values,
        build_verification_values: &[],
    })?;
    crate::preserves_rail::canonical_hash(&sync.receipt_value)
}

fn admit_job_stack(input: JobAdmissionInput<'_>) -> Result<JobAdmissionParts> {
    let authority_ref =
        install_job_execute_authority_context(input.target, input.job_ref, input.policy_refs, input.capability_refs)?;
    let source_gate_ref = install_clean_octet_gate(input.target, input.policy_refs, input.capability_refs)?;
    let admission_request = crate::job_dag::job_admission_request_value(crate::job_dag::AdmissionRequestValueInput {
        job_ref: input.job_ref,
        sync_ref: input.sync_ref,
        stage_ids: &[],
        target_peer: "peer:dogfood",
        policy_refs: input.policy_refs,
        capability_refs: std::slice::from_ref(&authority_ref),
        evidence_refs: &[input.sync_ref.to_string(), source_gate_ref],
        resource_refs: input.resource_refs,
    })?;
    let admission = crate::job_dag::admission_loopback(input.target, &admission_request)?;
    Ok(JobAdmissionParts {
        authority_ref,
        receipt_ref: crate::preserves_rail::canonical_hash(&admission.receipt_value)?,
        receipt_value: admission.receipt_value,
        stage_order: admission.plan.stage_order,
    })
}

fn execute_job_stack(input: JobExecutionInput<'_>) -> Result<JobExecutionParts> {
    let execution_request = crate::job_dag::job_execution_request_value(crate::job_dag::ExecutionRequestValueInput {
        job_ref: input.job_ref,
        admission_ref: &input.admission.receipt_ref,
        stage_ids: &input.admission.stage_order,
        target_peer: "peer:dogfood",
        storage_profile_ref: &dogfood_ref("job-storage-profile")?,
        cache_profile_ref: &dogfood_ref("job-cache-profile")?,
        chunk_profile_ref: &dogfood_ref("job-chunk-profile")?,
        policy_refs: input.policy_refs,
        capability_refs: std::slice::from_ref(&input.admission.authority_ref),
        resource_refs: input.resource_refs,
    })?;
    let request_ref = crate::preserves_rail::canonical_hash(&execution_request)?;
    let execution = crate::job_dag::execution_loopback(crate::job_dag::ExecutionLoopbackInput {
        target_registry: input.target,
        storage_root: &input.state_root.join("job-storage"),
        cache_root: &input.state_root.join("job-cache"),
        chunk_root: &input.state_root.join("job-chunks"),
        admission_receipt_value: &input.admission.receipt_value,
        request_value: &execution_request,
    })?;
    let mut output_refs = Vec::new();
    if let Some(run) = execution.run.as_ref() {
        output_refs.extend(run.output_refs.iter().cloned());
    }
    Ok(JobExecutionParts {
        request_ref,
        receipt_ref: execution.receipt_ref,
        decision: execution.decision,
        diagnostics: execution.diagnostics,
        output_refs,
    })
}

fn run_gc_workflow(input: GcWorkflowInput<'_>) -> Result<GcRun> {
    let object_ref = dogfood_ref("retention-object")?;
    let requester_ref = dogfood_ref("retention-requester")?;
    let peer_ref = dogfood_ref("retention-peer")?;
    let remote_ref = dogfood_ref("retention-remote-cache")?;
    let remote_refs = vec![remote_ref.clone()];
    let seed = GcSeed {
        root: input.root,
        object_ref: &object_ref,
        requester_ref: &requester_ref,
        peer_ref: &peer_ref,
        remote_ref: &remote_ref,
        remote_refs: &remote_refs,
        object_kind: "chunk",
        class: crate::retention::CLASS_DURABLE_VALUE,
        action: crate::retention::ACTION_DELETE,
    };
    let admissions = gc_admissions(seed)?;
    let flow = gc_flow(input, seed, &admissions.evidence)?;
    let ledger_import_refs = import_gc_values(input.ledger_root, &admissions, &flow)?;
    let (mcp_call, catalog_receipt_ref) = gc_catalog(input.registry_root, input.ledger_root, seed.object_ref)?;
    let bundle_diagnostics = gc_bundle_diagnostics(&flow)?;
    let artifact_refs = gc_artifact_refs(&admissions, &flow, &mcp_call.response_ref, ledger_import_refs);
    Ok(finish_gc_run(GcFinishInput {
        object_ref,
        flow,
        mcp_call,
        catalog_receipt_ref,
        artifact_refs,
        bundle_diagnostics,
    }))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GcSeed<'a> {
    root: &'a Path,
    object_ref: &'a str,
    requester_ref: &'a str,
    peer_ref: &'a str,
    remote_ref: &'a str,
    remote_refs: &'a [String],
    object_kind: &'a str,
    class: &'a str,
    action: &'a str,
}

struct GcAdmissions {
    policy: crate::retention::EvidenceAdmission,
    authority: crate::retention::EvidenceAdmission,
    support: crate::retention::EvidenceAdmission,
    index: crate::retention::EvidenceAdmission,
    remote_gc: crate::retention::EvidenceAdmission,
    clearance: crate::retention::RemoteGcClearance,
    evidence: crate::retention::DestructiveEvidence,
}

struct GcFlow {
    plan: crate::retention::GcPlan,
    apply: crate::retention::GcApply,
    execution: crate::retention::GcExecutionGate,
    audit: crate::retention::GcAudit,
    explain: crate::retention::CandidateExplain,
    bundle: crate::retention::CandidateBundle,
    profile: crate::retention::CandidateBundleProfile,
    verify: crate::retention::CandidateBundleVerify,
}

struct GcFinishInput {
    object_ref: String,
    flow: GcFlow,
    mcp_call: crate::catalog_mcp::Call,
    catalog_receipt_ref: String,
    artifact_refs: Vec<String>,
    bundle_diagnostics: Vec<String>,
}

fn store_gc_fixture(
    seed: GcSeed<'_>,
    kind: &str,
    label: &str,
    remote_refs: &[String],
) -> Result<crate::retention::EvidenceAdmission> {
    store_retention_admission_fixture(RetentionAdmissionFixtureInput {
        root: seed.root,
        kind,
        label,
        requester_ref: seed.requester_ref,
        object_ref: seed.object_ref,
        object_kind: seed.object_kind,
        retention_class: seed.class,
        action: seed.action,
        remote_refs,
    })
}
