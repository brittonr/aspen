
fn run_stages(
    dag: &JobDag,
    request: &JobOutputRequest,
    plan: &TrellisExecutionPlan,
    options: &JobRunOptions<'_>,
) -> Result<RunStages> {
    let mut completed_indices = Vec::with_capacity(plan.order_ids.len());
    let mut outputs_by_index: Vec<Option<Vec<IoValue>>> = vec![None; dag.nodes.len()];
    let mut output_refs_by_index: Vec<Option<Vec<String>>> = vec![None; dag.nodes.len()];
    let mut receipt_refs = Vec::with_capacity(plan.order_ids.len());
    for node_id in &plan.order_ids {
        let deps = plan.dependency_indices.get(node_id).cloned().unwrap_or_default();
        if !trellis::job_dag::all_deps_satisfied(&deps, &completed_indices)
            || trellis::job_dag::unsatisfied_count(&deps, &completed_indices) != 0
        {
            return Err(MoltenError::invalid_harness(format!(
                "trellis dependency readiness failed for job node {node_id}"
            )));
        }
        let node = find_job_node(&dag.nodes, node_id)?;
        let inputs = gather_inputs(node, &dag.edges, &outputs_by_index, &plan.node_index)?;
        let stage = run_stage_with_cache(dag, request, node, &inputs, options)?;
        let receipt_ref = crate::preserves_rail::canonical_hash(&stage.receipt_value)?;
        if let Some(ledger_root) = options.ledger_root {
            crate::ledger::import_artifact(ledger_root, &stage.receipt_value)?;
        }
        ensure_count_at_most(stage.output_refs.len(), MAX_JOB_REFS, "job stage output refs")?;
        ensure_count_at_most(stage.output_values.len(), MAX_JOB_STAGE_VALUES, "job stage output values")?;
        push_bounded(&mut receipt_refs, receipt_ref, MAX_JOB_NODES, "job stage receipt refs")?;
        let node_index = *plan
            .node_index
            .get(node_id)
            .ok_or_else(|| MoltenError::invalid_harness(format!("trellis node index missing for {node_id}")))?;
        let output_refs_slot = output_refs_by_index.get_mut(node_index).ok_or_else(|| {
            MoltenError::invalid_harness(format!("job output refs index {node_index} outside node set"))
        })?;
        *output_refs_slot = Some(stage.output_refs.clone());
        let output_slot = outputs_by_index
            .get_mut(node_index)
            .ok_or_else(|| MoltenError::invalid_harness(format!("job output index {node_index} outside node set")))?;
        *output_slot = Some(stage.output_values);
        push_bounded(
            &mut completed_indices,
            usize_to_u64(node_index, "trellis completed node index")?,
            MAX_JOB_NODES,
            "trellis completed node indices",
        )?;
    }
    Ok(RunStages {
        receipt_refs,
        outputs_by_index,
        output_refs_by_index,
    })
}

fn run_stages_with_capabilities(
    dag: &JobDag,
    request: &JobOutputRequest,
    plan: &TrellisExecutionPlan,
    options: &CapabilityJobRunOptions<'_>,
) -> Result<RunStages> {
    let mut completed_indices = Vec::with_capacity(plan.order_ids.len());
    let mut outputs_by_index: Vec<Option<Vec<IoValue>>> = vec![None; dag.nodes.len()];
    let mut output_refs_by_index: Vec<Option<Vec<String>>> = vec![None; dag.nodes.len()];
    let mut receipt_refs = Vec::with_capacity(plan.order_ids.len());
    for node_id in &plan.order_ids {
        let deps = plan.dependency_indices.get(node_id).cloned().unwrap_or_default();
        if !trellis::job_dag::all_deps_satisfied(&deps, &completed_indices)
            || trellis::job_dag::unsatisfied_count(&deps, &completed_indices) != 0
        {
            return Err(MoltenError::invalid_harness(format!(
                "trellis dependency readiness failed for job node {node_id}"
            )));
        }
        let node = find_job_node(&dag.nodes, node_id)?;
        let inputs = gather_inputs(node, &dag.edges, &outputs_by_index, &plan.node_index)?;
        let stage = execute_stage_with_capabilities(dag, request, node, &inputs, options)?;
        let receipt_ref = crate::preserves_rail::canonical_hash(&stage.receipt_value)?;
        ensure_count_at_most(stage.output_refs.len(), MAX_JOB_REFS, "job stage output refs")?;
        ensure_count_at_most(stage.output_values.len(), MAX_JOB_STAGE_VALUES, "job stage output values")?;
        push_bounded(&mut receipt_refs, receipt_ref, MAX_JOB_NODES, "job stage receipt refs")?;
        let node_index = *plan
            .node_index
            .get(node_id)
            .ok_or_else(|| MoltenError::invalid_harness(format!("trellis node index missing for {node_id}")))?;
        let output_refs_slot = output_refs_by_index.get_mut(node_index).ok_or_else(|| {
            MoltenError::invalid_harness(format!("job output refs index {node_index} outside node set"))
        })?;
        *output_refs_slot = Some(stage.output_refs.clone());
        let output_slot = outputs_by_index
            .get_mut(node_index)
            .ok_or_else(|| MoltenError::invalid_harness(format!("job output index {node_index} outside node set")))?;
        *output_slot = Some(stage.output_values);
        push_bounded(
            &mut completed_indices,
            usize_to_u64(node_index, "trellis completed node index")?,
            MAX_JOB_NODES,
            "trellis completed node indices",
        )?;
    }
    Ok(RunStages {
        receipt_refs,
        outputs_by_index,
        output_refs_by_index,
    })
}

struct RunFinish {
    output_refs: Vec<String>,
    output_value: IoValue,
    receipt_value: IoValue,
}

struct CompleteInput<'a> {
    dag: &'a JobDag,
    request: &'a JobOutputRequest,
    plan: &'a TrellisExecutionPlan,
    outputs_by_index: &'a [Option<Vec<IoValue>>],
    output_refs_by_index: &'a [Option<Vec<String>>],
    stage_receipt_refs: &'a [String],
}

struct FinalRunOutputs {
    refs: Vec<String>,
    value: IoValue,
}
