
pub fn sync_loopback(input: SyncLoopbackInput<'_>) -> Result<JobSyncLoopback> {
    let plan = sync_plan_value(input.source_registry, input.target_registry, input.request_value)?;
    let ordered_refs = sync_install_order(input.source_registry, &plan.root_refs)?;
    let CandidateSelection {
        install_candidates,
        already_present_refs,
        provenance_receipt_refs,
        diagnostics,
    } = collect_candidates(&input, &plan, ordered_refs)?;
    let installed_refs = if diagnostics.is_empty() {
        apply_candidates(input.target_registry, &plan.request, install_candidates)?
    } else {
        Vec::new()
    };
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = loopback_receipt(ReceiptInput {
        plan: &plan,
        decision,
        installed_refs: &installed_refs,
        already_present_refs: &already_present_refs,
        provenance_receipt_refs: &provenance_receipt_refs,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    Ok(JobSyncLoopback {
        receipt_ref,
        plan,
        decision: decision.to_string(),
        installed_refs,
        already_present_refs,
        provenance_receipt_refs,
        diagnostics,
        receipt_value,
    })
}

pub fn admission_plan_value(target_registry: &FilePath, request_value: &IoValue) -> Result<JobAdmissionPlan> {
    let request = parse_job_admission_request_value(request_value)?;
    let mut diagnostics = Vec::new();
    let has_explicit_authority = explicit_admission_authority(&request, &mut diagnostics);
    let (has_capability_authority, authority_receipt_refs) =
        capability_contexts_admit(target_registry, &request, &mut diagnostics)?;
    let has_sync_evidence = sync_evidence_bound(&request, &mut diagnostics);
    let (has_source_gate_evidence, source_gate_validation_refs) =
        source_gate_evidence_bound(target_registry, &request, &mut diagnostics)?;
    let readiness = scan_target(target_registry, &request, &mut diagnostics)?;
    let has_resource_profile = resource_profile_admits(&request, &readiness.stage_order, &mut diagnostics)?;
    finish_plan(PlanOutcomeInput {
        request,
        readiness,
        authority_receipt_refs,
        source_gate_validation_refs,
        has_explicit_authority,
        has_capability_authority,
        has_sync_evidence,
        has_source_gate_evidence,
        has_resource_profile,
        diagnostics,
    })
}

struct Readiness {
    has_target_closure: bool,
    has_valid_topology: bool,
    has_executable_artifacts: bool,
    closure_refs: Vec<String>,
    stage_order: Vec<String>,
    stage_verdicts: Vec<JobAdmissionStageVerdict>,
}

struct PlanOutcomeInput {
    request: JobAdmissionRequest,
    readiness: Readiness,
    authority_receipt_refs: Vec<String>,
    source_gate_validation_refs: Vec<String>,
    has_explicit_authority: bool,
    has_capability_authority: bool,
    has_sync_evidence: bool,
    has_source_gate_evidence: bool,
    has_resource_profile: bool,
    diagnostics: Vec<String>,
}

struct RecordInput<'a> {
    request: &'a JobAdmissionRequest,
    readiness: &'a Readiness,
    authority_receipt_refs: &'a [String],
    resource_verdict: &'a str,
    decision: &'a str,
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

struct StageScanInput<'a> {
    node_id: &'a str,
    plan: &'a TrellisExecutionPlan,
    node_map: &'a OrderedMap<String, &'a JobNode>,
    completed_indices: &'a [u64],
}

fn scan_target(
    target_registry: &FilePath,
    request: &JobAdmissionRequest,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<Readiness> {
    let mut readiness = Readiness {
        has_target_closure: true,
        has_valid_topology: true,
        has_executable_artifacts: true,
        closure_refs: Vec::new(),
        stage_order: Vec::new(),
        stage_verdicts: Vec::new(),
    };
    match read_job_dag(target_registry, &request.job_ref) {
        Ok(dag) => {
            let selected = selected_stage_set(&dag, &request.stage_ids, diagnostics, &mut readiness.stage_verdicts)?;
            if request.stage_ids.iter().any(|stage_id| !dag.nodes.iter().any(|node| node.id == *stage_id)) {
                readiness.has_valid_topology = false;
            }
            scan_topology(&dag, &selected, diagnostics, &mut readiness)?;
            let (is_complete, refs, notes) = target_closure_state(target_registry, &dag, &selected)?;
            readiness.has_target_closure = is_complete;
            readiness.closure_refs = refs;
            diagnostics.extend_cloned_items(&notes);
        }
        Err(error) => {
            readiness.has_target_closure = false;
            readiness.has_valid_topology = false;
            readiness.has_executable_artifacts = false;
            diagnostics.push_item(format!("target job not available: {error}"));
        }
    }
    Ok(readiness)
}

fn scan_topology(
    dag: &JobDag,
    selected: &OrderedSet<String>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    readiness: &mut Readiness,
) -> Result<()> {
    let plan = match trellis_execution_plan(&dag.nodes, &dag.edges) {
        Ok(plan) => plan,
        Err(error) => {
            readiness.has_valid_topology = false;
            diagnostics.push_item(format!("trellis topology denied: {error}"));
            return Ok(());
        }
    };
    let node_map = dag.nodes.iter().map(|node| (node.id.clone(), node)).collect::<OrderedMap<_, _>>();
    let mut completed_indices = Vec::new();
    for node_id in &plan.order_ids {
        if !selected.contains(node_id) {
            continue;
        }
        let node_index = scan_stage(
            StageScanInput {
                node_id,
                plan: &plan,
                node_map: &node_map,
                completed_indices: &completed_indices,
            },
            diagnostics,
            readiness,
        )?;
        push_bounded(&mut completed_indices, node_index, MAX_JOB_NODES, "job admission completed node indices")?;
    }
    Ok(())
}

fn scan_stage(
    input: StageScanInput<'_>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    readiness: &mut Readiness,
) -> Result<u64> {
    let mut stage_diagnostics = Vec::new();
    let deps = input.plan.dependency_indices.get(input.node_id).cloned().unwrap_or_default();
    if !trellis::job_dag::all_deps_satisfied(&deps, input.completed_indices)
        || trellis::job_dag::unsatisfied_count(&deps, input.completed_indices) != 0
    {
        readiness.has_valid_topology = false;
        stage_diagnostics.push(format!("unsatisfied selected-stage dependencies for {}", input.node_id));
    }
    let node = input
        .node_map
        .get(input.node_id)
        .ok_or_else(|| MoltenError::invalid_harness(format!("job admission missing node {}", input.node_id)))?;
    if node.stage_artifact_ref.is_none() {
        readiness.has_executable_artifacts = false;
        stage_diagnostics.push(format!("stage {} lacks artifact-backed executable operation", input.node_id));
    }
    diagnostics.extend_cloned_items(&stage_diagnostics);
    push_bounded(
        &mut readiness.stage_verdicts,
        JobAdmissionStageVerdict {
            stage_id: input.node_id.to_string(),
            decision: if stage_diagnostics.is_empty() { "pass" } else { "deny" }.to_string(),
            diagnostics: stage_diagnostics,
        },
        MAX_JOB_NODES,
        "job admission stage verdicts",
    )?;
    let node_index = *input.plan.node_index.get(input.node_id).ok_or_else(|| {
        MoltenError::invalid_harness(format!("job admission missing trellis index for {}", input.node_id))
    })?;
    push_bounded(&mut readiness.stage_order, input.node_id.to_string(), MAX_JOB_NODES, "job admission stage order")?;
    usize_to_u64(node_index, "job admission completed node index")
}

fn finish_plan(input: PlanOutcomeInput) -> Result<JobAdmissionPlan> {
    let decision = plan_decision(&input);
    let resource_verdict = if input.has_resource_profile { "pass" } else { "deny" }.to_string();
    let checks = plan_checks(&input);
    let value = plan_record(RecordInput {
        request: &input.request,
        readiness: &input.readiness,
        authority_receipt_refs: &input.authority_receipt_refs,
        resource_verdict: &resource_verdict,
        decision,
        diagnostics: &input.diagnostics,
        checks: &checks,
    });
    let plan_ref = crate::preserves_rail::canonical_hash(&value)?;
    let receipt_value = job_admission_receipt_value(JobAdmissionReceiptValueInput {
        operation: "admit-plan",
        decision,
        request: &input.request,
        plan_ref: &plan_ref,
        closure_refs: &input.readiness.closure_refs,
        stage_order: &input.readiness.stage_order,
        authority_receipt_refs: &input.authority_receipt_refs,
        source_gate_validation_refs: &input.source_gate_validation_refs,
        resource_verdict: &resource_verdict,
        diagnostics: &input.diagnostics,
        checks: &checks,
    })?;
    let PlanOutcomeInput {
        request,
        readiness,
        authority_receipt_refs,
        source_gate_validation_refs,
        diagnostics,
        ..
    } = input;
    Ok(JobAdmissionPlan {
        plan_ref,
        request,
        closure_refs: readiness.closure_refs,
        stage_order: readiness.stage_order,
        stage_verdicts: readiness.stage_verdicts,
        authority_receipt_refs,
        source_gate_validation_refs,
        resource_verdict,
        decision: decision.to_string(),
        diagnostics,
        value,
        receipt_value,
    })
}

fn plan_decision(input: &PlanOutcomeInput) -> &'static str {
    if input.readiness.has_target_closure
        && input.readiness.has_valid_topology
        && input.readiness.has_executable_artifacts
        && input.has_explicit_authority
        && input.has_capability_authority
        && input.has_sync_evidence
        && input.has_source_gate_evidence
        && input.has_resource_profile
    {
        "pass"
    } else {
        "deny"
    }
}

fn plan_checks(input: &PlanOutcomeInput) -> [(&'static str, &'static str); 9] {
    [
        ("target-closure-present", status(input.readiness.has_target_closure)),
        ("trellis-topology", status(input.readiness.has_valid_topology)),
        ("executable-artifact-gate", status(input.readiness.has_executable_artifacts)),
        ("explicit-authority", status(input.has_explicit_authority)),
        ("capability-authority-context", status(input.has_capability_authority)),
        ("resource-profile", status(input.has_resource_profile)),
        ("sync-evidence-bound", status(input.has_sync_evidence)),
        ("strict-octet-source-gate-bound", status(input.has_source_gate_evidence)),
        ("no-execution", "pass"),
    ]
}
