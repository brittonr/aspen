
fn complete_run(input: CompleteInput<'_>) -> Result<RunFinish> {
    let roots = requested_output_roots(input.dag, input.request)?;
    let final_outputs = collect_final_run_outputs(&input, roots)?;
    let evidence_refs = collect_run_evidence_refs(input.dag, input.stage_receipt_refs)?;
    let receipt_value = run_finish_receipt_value(&input, &final_outputs.refs, &evidence_refs)?;
    Ok(RunFinish {
        output_refs: final_outputs.refs,
        output_value: final_outputs.value,
        receipt_value,
    })
}

fn requested_output_roots(dag: &JobDag, request: &JobOutputRequest) -> Result<Vec<String>> {
    let roots = if request.roots.is_empty() {
        sink_nodes(dag)?
    } else {
        request.roots.clone()
    };
    ensure_count_at_most(roots.len(), MAX_JOB_ROOTS, "job output roots")?;
    Ok(roots)
}

fn collect_final_run_outputs(input: &CompleteInput<'_>, roots: Vec<String>) -> Result<FinalRunOutputs> {
    let mut final_values = Vec::with_capacity(roots.len());
    let mut final_refs = Vec::with_capacity(roots.len());
    for root in roots {
        let root_index = output_root_index(input.plan, &root)?;
        let values = output_values_for_root(input.outputs_by_index, root_index, &root)?;
        extend_cloned_bounded(&mut final_values, values, MAX_JOB_STAGE_VALUES, "job final values")?;
        if let Some(refs) = input.output_refs_by_index.get(root_index).and_then(Option::as_ref) {
            extend_cloned_bounded(&mut final_refs, refs, MAX_JOB_REFS, "job final refs")?;
        }
    }
    let value = crate::preserves_rail::sequence(final_values.clone());
    let refs = ensure_final_output_ref(final_refs, &value)?;
    Ok(FinalRunOutputs { refs, value })
}

fn output_root_index(plan: &TrellisExecutionPlan, root: &str) -> Result<usize> {
    plan.node_index
        .get(root)
        .copied()
        .ok_or_else(|| MoltenError::invalid_harness(format!("job output root {root} missing from node index")))
}

fn output_values_for_root<'a>(
    outputs_by_index: &'a [Option<Vec<IoValue>>],
    root_index: usize,
    root: &str,
) -> Result<&'a [IoValue]> {
    outputs_by_index
        .get(root_index)
        .and_then(Option::as_deref)
        .ok_or_else(|| MoltenError::invalid_harness(format!("job output root {root} was not executed")))
}

fn ensure_final_output_ref(mut final_refs: Vec<String>, output_value: &IoValue) -> Result<Vec<String>> {
    if final_refs.is_empty() {
        push_bounded(
            &mut final_refs,
            crate::preserves_rail::canonical_hash(output_value)?,
            MAX_JOB_REFS,
            "job final refs",
        )?;
    }
    Ok(final_refs)
}

fn collect_run_evidence_refs(dag: &JobDag, stage_receipt_refs: &[String]) -> Result<Vec<String>> {
    let evidence_count = checked_count_sum(
        dag.evidence_refs.len(),
        stage_receipt_refs.len(),
        MAX_JOB_REFS,
        "job receipt evidence refs",
    )?;
    let mut evidence_refs = Vec::with_capacity(evidence_count);
    extend_cloned_bounded(&mut evidence_refs, &dag.evidence_refs, MAX_JOB_REFS, "job receipt evidence refs")?;
    extend_cloned_bounded(&mut evidence_refs, stage_receipt_refs, MAX_JOB_REFS, "job receipt evidence refs")?;
    Ok(evidence_refs)
}

fn run_finish_receipt_value(
    input: &CompleteInput<'_>,
    final_refs: &[String],
    evidence_refs: &[String],
) -> Result<IoValue> {
    job_receipt_value(JobReceiptInput {
        operation: "run",
        decision: "pass",
        job_ref: Some(&input.dag.job_ref),
        request_ref: Some(&input.request.request_ref),
        stage_id: None,
        input_refs: input.stage_receipt_refs,
        output_refs: final_refs,
        cache_ref: None,
        effect_refs: &[],
        policy_refs: &combined_policy_refs(input.dag, input.request, None),
        evidence_refs,
        diagnostics: &[],
        checks: &[
            ("deterministic-topological-order", "pass"),
            ("trellis-topo-order", "pass"),
            ("trellis-deps-ready", "pass"),
            ("stage-receipts-bound", "pass"),
            ("output-refs-bound", "pass"),
        ],
    })
}

fn stage_plan_values(dag: &JobDag, plan: &TrellisExecutionPlan) -> Result<Vec<IoValue>> {
    let mut node_map = OrderedMap::new();
    for node in &dag.nodes {
        insert_bounded(&mut node_map, node.id.clone(), node, MAX_JOB_NODES, "job plan node map")?;
    }
    let mut stage_values = Vec::with_capacity(plan.order_ids.len());
    for node_id in &plan.order_ids {
        let node = node_map
            .get(node_id)
            .ok_or_else(|| MoltenError::invalid_harness(format!("job plan missing node {node_id}")))?;
        let index = *plan
            .node_index
            .get(node_id)
            .ok_or_else(|| MoltenError::invalid_harness(format!("job plan missing index for {node_id}")))?;
        let deps = dependency_ids(plan, node_id)?;
        push_bounded(
            &mut stage_values,
            crate::preserves_rail::record("job-stage-plan-v1", vec![
                crate::preserves_rail::record("id", vec![crate::preserves_rail::string(node_id)]),
                crate::preserves_rail::record("trellis-index", vec![crate::preserves_rail::u64_value(usize_to_u64(
                    index,
                    "job plan trellis index",
                )?)]),
                crate::preserves_rail::record("dependencies", vec![crate::preserves_rail::sequence(
                    deps.iter().map(crate::preserves_rail::string).collect(),
                )]),
                crate::preserves_rail::record("placement", vec![crate::preserves_rail::string("local")]),
                crate::preserves_rail::record("cache-projection", vec![crate::preserves_rail::string(
                    if node.kind == "materialize" {
                        "not-cacheable"
                    } else {
                        "eligible"
                    },
                )]),
                crate::preserves_rail::record("policy", vec![refs_sequence(&node.policy_refs)]),
                crate::preserves_rail::record("resources", vec![crate::preserves_rail::sequence(Vec::new())]),
                checks_value(&["trellis-dependencies-bound", "placement-is-proposal", "local-only-plan"]),
            ]),
            MAX_JOB_STAGE_VALUES,
            "job plan stage values",
        )?;
    }
    Ok(stage_values)
}

pub fn plan_job_dag(dag: &JobDag, output_request: Option<&IoValue>) -> Result<JobPlan> {
    let request = request_for_analysis(dag, output_request)?;
    let plan = trellis_execution_plan(&dag.nodes, &dag.edges)?;
    let stage_values = stage_plan_values(dag, &plan)?;
    let value = crate::preserves_rail::record("job-plan-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::JOB_PLAN_SCHEMA),
        crate::preserves_rail::record("job", vec![crate::preserves_rail::string(&dag.job_ref)]),
        crate::preserves_rail::record("request", vec![crate::preserves_rail::string(&request.request_ref)]),
        crate::preserves_rail::record("stage-order", vec![crate::preserves_rail::sequence(
            plan.order_ids.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("stages", vec![crate::preserves_rail::sequence(stage_values)]),
        crate::preserves_rail::record("policy", vec![refs_sequence(&combined_policy_refs(dag, &request, None))]),
        checks_value(&[
            "trellis-topo-order",
            "trellis-deps-ready",
            "canonical-node-index-map",
            "placement-proposals-only",
        ]),
    ]);
    let plan_ref = crate::preserves_rail::canonical_hash(&value)?;
    let receipt_value = analysis_receipt_value(AnalysisReceiptValueInput {
        label: "job-plan-receipt-v1",
        schema: crate::preserves_rail::JOB_PLAN_RECEIPT_SCHEMA,
        operation: "plan",
        job_ref: &dag.job_ref,
        request_ref: &request.request_ref,
        artifact_ref: &plan_ref,
        diagnostics: &[],
        checks: &[
            ("trellis-topo-order", "pass"),
            ("trellis-deps-ready", "pass"),
            ("canonical-plan-ref", "pass"),
        ],
    })?;
    Ok(JobPlan {
        plan_ref,
        job_ref: dag.job_ref.clone(),
        request_ref: request.request_ref,
        stage_order: plan.order_ids,
        value,
        receipt_value,
    })
}

struct StageProfiles {
    config_bytes: u64,
    values: Vec<IoValue>,
}

fn stage_profile_values(dag: &JobDag, plan: &TrellisExecutionPlan, cache_entries: usize) -> Result<StageProfiles> {
    let mut config_bytes = 0_u64;
    let mut values = Vec::with_capacity(plan.order_ids.len());
    for node_id in &plan.order_ids {
        let node = dag
            .nodes
            .iter()
            .find(|candidate| candidate.id == *node_id)
            .ok_or_else(|| MoltenError::invalid_harness(format!("job profile missing node {node_id}")))?;
        let bytes =
            usize_to_u64(crate::preserves_rail::canonical_bytes(&node.config)?.len(), "job profile config bytes")?;
        config_bytes = config_bytes
            .checked_add(bytes)
            .ok_or_else(|| MoltenError::invalid_harness("job profile estimated config bytes overflowed"))?;
        push_bounded(
            &mut values,
            crate::preserves_rail::record("job-stage-profile-v1", vec![
                crate::preserves_rail::record("id", vec![crate::preserves_rail::string(node_id)]),
                crate::preserves_rail::record("kind", vec![crate::preserves_rail::string(&node.kind)]),
                crate::preserves_rail::record("estimated-config-bytes", vec![crate::preserves_rail::u64_value(bytes)]),
                crate::preserves_rail::record("cache-projection", vec![crate::preserves_rail::string(
                    if node.kind == "materialize" {
                        "not-cacheable"
                    } else if cache_entries == 0 {
                        "projected-miss"
                    } else {
                        "candidate-hit-or-miss"
                    },
                )]),
                checks_value(&["deterministic-estimate", "no-wall-clock-time"]),
            ]),
            MAX_JOB_STAGE_VALUES,
            "job profile stage values",
        )?;
    }
    Ok(StageProfiles { config_bytes, values })
}

struct ProfileCounts {
    cache_entries: usize,
    materialization_boundaries: u64,
    stage_count: u64,
    edge_count: u64,
}

pub fn profile_job_dag(
    dag: &JobDag,
    output_request: Option<&IoValue>,
    cache_root: Option<&FilePath>,
) -> Result<JobProfile> {
    let request = request_for_analysis(dag, output_request)?;
    let plan = trellis_execution_plan(&dag.nodes, &dag.edges)?;
    let counts = profile_counts(dag, cache_root)?;
    let profile_stages = stage_profile_values(dag, &plan, counts.cache_entries)?;
    let value = profile_value(dag, &request, profile_stages, &counts)?;
    let profile_ref = crate::preserves_rail::canonical_hash(&value)?;
    let receipt_value = profile_receipt_value(dag, &request, &profile_ref)?;
    Ok(JobProfile {
        profile_ref,
        job_ref: dag.job_ref.clone(),
        request_ref: request.request_ref,
        stage_count: counts.stage_count,
        edge_count: counts.edge_count,
        materialization_boundaries: counts.materialization_boundaries,
        value,
        receipt_value,
    })
}

fn profile_counts(dag: &JobDag, cache_root: Option<&FilePath>) -> Result<ProfileCounts> {
    Ok(ProfileCounts {
        cache_entries: cache_entry_count(cache_root)?,
        materialization_boundaries: materialization_boundary_count(dag)?,
        stage_count: usize_to_u64(dag.nodes.len(), "job profile stage count")?,
        edge_count: usize_to_u64(dag.edges.len(), "job profile edge count")?,
    })
}

fn cache_entry_count(cache_root: Option<&FilePath>) -> Result<usize> {
    if let Some(cache_root) = cache_root {
        Ok(crate::eval_cache::list(cache_root, &crate::eval_cache::ListFilter {
            operation: Some(JOB_CACHE_OPERATION.to_string()),
            ..crate::eval_cache::ListFilter::default()
        })?
        .len())
    } else {
        Ok(0)
    }
}
