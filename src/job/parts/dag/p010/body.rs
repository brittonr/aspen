
fn check_target_selection(
    input: SelectionInput<'_>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    checks: &mut impl crate::bounded::VecSink<(&'static str, &'static str)>,
) -> Result<()> {
    let stage_order = if input.request.stage_ids.is_empty() {
        input.admission.stage_order.clone()
    } else {
        input.request.stage_ids.clone()
    };
    let has_selected_stage_binding = stage_order == input.admission.stage_order;
    push_check(checks, "selected-stage-binding", has_selected_stage_binding);
    if !has_selected_stage_binding {
        diagnostics.push_item("job execution selected stages do not match admission stage order".to_string());
    }

    let full_stage_order = trellis_execution_plan(&input.dag.nodes, &input.dag.edges)?.order_ids;
    let has_full_stage_selection = stage_order == full_stage_order;
    push_check(checks, "selected-stages-full-target", has_full_stage_selection);
    if !has_full_stage_selection {
        diagnostics.push_item(
            "job execution loopback currently requires admitted stages to cover the full target DAG".to_string(),
        );
    }

    match recompute_execution_closure(input.target_registry, input.dag, &stage_order) {
        Ok(closure_refs) => {
            let has_recomputed_closure = sorted_unique(&closure_refs) == sorted_unique(&input.admission.closure_refs);
            push_check(checks, "target-closure-recomputed", has_recomputed_closure);
            if !has_recomputed_closure {
                diagnostics
                    .push_item("job execution recomputed target closure diverges from admission closure".to_string());
            }
        }
        Err(error) => {
            push_check(checks, "target-closure-recomputed", false);
            diagnostics.push_item(format!("job execution target closure recompute failed: {error}"));
        }
    }

    let has_request_ref_bindings = refs_are_bound_in_admission(&input.request.policy_refs, &input.admission.refs)
        && refs_are_bound_in_admission(&input.request.capability_refs, &input.admission.refs)
        && refs_are_bound_in_admission(&input.request.resource_refs, &input.admission.refs);
    push_check(checks, "request-ref-binding", has_request_ref_bindings);
    if !has_request_ref_bindings {
        diagnostics.push_item(
            "job execution request policy/capability/resource refs are not all bound by admission".to_string(),
        );
    }
    Ok(())
}

/// The registry, request, admission, and DAG a target selection is checked against.
#[derive(Clone, Copy)]
struct TargetSelectionInput<'a> {
    target_registry: &'a crate::artifacts::CapabilityArtifactRoot,
    request: &'a JobExecutionRequest,
    admission: &'a JobAdmissionReceipt,
    dag: &'a JobDag,
}

fn check_target_selection_with_root(
    selection: TargetSelectionInput<'_>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    checks: &mut impl crate::bounded::VecSink<(&'static str, &'static str)>,
) -> Result<()> {
    let TargetSelectionInput { target_registry, request, admission, dag } = selection;
    let stage_order = if request.stage_ids.is_empty() {
        admission.stage_order.clone()
    } else {
        request.stage_ids.clone()
    };
    let has_selected_stage_binding = stage_order == admission.stage_order;
    push_check(checks, "selected-stage-binding", has_selected_stage_binding);
    if !has_selected_stage_binding {
        diagnostics.push_item("job execution selected stages do not match admission stage order".to_string());
    }

    let full_stage_order = trellis_execution_plan(&dag.nodes, &dag.edges)?.order_ids;
    let has_full_stage_selection = stage_order == full_stage_order;
    push_check(checks, "selected-stages-full-target", has_full_stage_selection);
    if !has_full_stage_selection {
        diagnostics.push_item(
            "job execution loopback currently requires admitted stages to cover the full target DAG".to_string(),
        );
    }

    match recompute_execution_closure_with_root(target_registry, dag, &stage_order) {
        Ok(closure_refs) => {
            let has_recomputed_closure = sorted_unique(&closure_refs) == sorted_unique(&admission.closure_refs);
            push_check(checks, "target-closure-recomputed", has_recomputed_closure);
            if !has_recomputed_closure {
                diagnostics
                    .push_item("job execution recomputed target closure diverges from admission closure".to_string());
            }
        }
        Err(error) => {
            push_check(checks, "target-closure-recomputed", false);
            diagnostics.push_item(format!("job execution target closure recompute failed: {error}"));
        }
    }

    let has_request_ref_bindings = refs_are_bound_in_admission(&request.policy_refs, &admission.refs)
        && refs_are_bound_in_admission(&request.capability_refs, &admission.refs)
        && refs_are_bound_in_admission(&request.resource_refs, &admission.refs);
    push_check(checks, "request-ref-binding", has_request_ref_bindings);
    if !has_request_ref_bindings {
        diagnostics.push_item(
            "job execution request policy/capability/resource refs are not all bound by admission".to_string(),
        );
    }
    Ok(())
}

struct DenyInput<'a> {
    request: JobExecutionRequest,
    admission: JobAdmissionReceipt,
    diagnostics: Vec<String>,
    checks: Vec<(&'static str, &'static str)>,
    extra_checks: &'a [(&'static str, &'static str)],
}

fn deny_result(input: DenyInput<'_>) -> Result<JobExecutionLoopback> {
    let receipt_value = job_execution_receipt_value(ExecutionReceiptValueInput {
        decision: "deny",
        request: &input.request,
        admission: &input.admission,
        stage_receipt_refs: &[],
        output_refs: &[],
        run_receipt_refs: &[],
        diagnostics: &input.diagnostics,
        checks: &checks_with_extra(&input.checks, input.extra_checks),
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    Ok(JobExecutionLoopback {
        receipt_ref,
        request: input.request,
        admission: input.admission,
        run: None,
        decision: "deny".to_string(),
        diagnostics: input.diagnostics,
        receipt_value,
    })
}

struct PassInput {
    request: JobExecutionRequest,
    admission: JobAdmissionReceipt,
    run: JobRun,
    diagnostics: Vec<String>,
    checks: Vec<(&'static str, &'static str)>,
}

fn pass_result(input: PassInput) -> Result<JobExecutionLoopback> {
    let receipt_value = job_execution_receipt_value(ExecutionReceiptValueInput {
        decision: "pass",
        request: &input.request,
        admission: &input.admission,
        stage_receipt_refs: &input.run.stage_receipt_refs,
        output_refs: &input.run.output_refs,
        run_receipt_refs: &[crate::preserves_rail::canonical_hash(&input.run.receipt_value)?],
        diagnostics: &input.diagnostics,
        checks: &checks_with_extra(&input.checks, &[
            ("executed-on-target-state", "pass"),
            ("stage-receipts-bound", "pass"),
            ("output-refs-bound", "pass"),
        ]),
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    Ok(JobExecutionLoopback {
        receipt_ref,
        request: input.request,
        admission: input.admission,
        run: Some(input.run),
        decision: "pass".to_string(),
        diagnostics: input.diagnostics,
        receipt_value,
    })
}

pub fn fusion_preview_job_dag(dag: &JobDag, output_request: Option<&IoValue>) -> Result<JobFusionPreview> {
    let request = request_for_analysis(dag, output_request)?;
    let plan = trellis_execution_plan(&dag.nodes, &dag.edges)?;
    let (chain_values, chains) = adjacent_chains(dag, &plan.order_ids)?;
    let value = crate::preserves_rail::record("job-fusion-plan-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::JOB_FUSION_PLAN_SCHEMA),
        crate::preserves_rail::record("job", vec![crate::preserves_rail::string(&dag.job_ref)]),
        crate::preserves_rail::record("request", vec![crate::preserves_rail::string(&request.request_ref)]),
        crate::preserves_rail::record("chains", vec![crate::preserves_rail::sequence(chain_values)]),
        checks_value(&[
            "trellis-order-bound",
            "no-reduce-materialize-fusion",
            "effect-policy-boundaries-preserved",
            "fusion-is-preview-only",
        ]),
    ]);
    let fusion_ref = crate::preserves_rail::canonical_hash(&value)?;
    let receipt_value = analysis_receipt_value(AnalysisReceiptValueInput {
        label: "job-fusion-receipt-v1",
        schema: crate::preserves_rail::JOB_FUSION_RECEIPT_SCHEMA,
        operation: "fusion-preview",
        job_ref: &dag.job_ref,
        request_ref: &request.request_ref,
        artifact_ref: &fusion_ref,
        diagnostics: &[],
        checks: &[
            ("trellis-order-bound", "pass"),
            ("effect-policy-boundaries-preserved", "pass"),
            ("fusion-preview-only", "pass"),
        ],
    })?;
    Ok(JobFusionPreview {
        fusion_ref,
        job_ref: dag.job_ref.clone(),
        request_ref: request.request_ref,
        chains,
        value,
        receipt_value,
    })
}

fn adjacent_chains(dag: &JobDag, order_ids: &[String]) -> Result<(Vec<IoValue>, Vec<Vec<String>>)> {
    let positions = order_ids
        .iter()
        .enumerate()
        .map(|(index, node_id)| (node_id.clone(), index))
        .collect::<OrderedMap<_, _>>();
    let node_map = dag.nodes.iter().map(|node| (node.id.clone(), node)).collect::<OrderedMap<_, _>>();
    let mut edges = dag.edges.iter().collect::<Vec<_>>();
    edges.sort_by(|left, right| fusion_edge_sort_key(&positions, left).cmp(&fusion_edge_sort_key(&positions, right)));

    let mut chain_values = Vec::new();
    let mut chains = Vec::new();
    for edge in edges {
        let from = node_map
            .get(&edge.from_node)
            .ok_or_else(|| MoltenError::invalid_harness(format!("fusion edge from missing node {}", edge.from_node)))?;
        let to = node_map
            .get(&edge.to_node)
            .ok_or_else(|| MoltenError::invalid_harness(format!("fusion edge to missing node {}", edge.to_node)))?;
        if fusion_edge_safe(from, to, edge) {
            let chain = vec![from.id.clone(), to.id.clone()];
            push_bounded(&mut chain_values, adjacent_chain_value(&chain), MAX_JOB_EDGES, "job fusion chain values")?;
            push_bounded(&mut chains, chain, MAX_JOB_EDGES, "job fusion chains")?;
        }
    }
    Ok((chain_values, chains))
}

fn adjacent_chain_value(chain: &[String]) -> IoValue {
    crate::preserves_rail::record("job-fusion-chain-v1", vec![
        crate::preserves_rail::record("stages", vec![crate::preserves_rail::sequence(
            chain.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("reason", vec![crate::preserves_rail::string("pure-adjacent-map-filter")]),
        checks_value(&[
            "trellis-adjacent-order",
            "no-materialization-boundary",
            "no-effect-policy-boundary",
            "schema-boundary-preserved",
        ]),
    ])
}
