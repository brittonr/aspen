
fn replay_value(
    input: &JobWorkerScheduleReplayInput<'_>,
    decision: &str,
    completed_indices: &[u64],
    diagnostics: &[String],
) -> IoValue {
    crate::preserves_rail::record("job-worker-schedule-replay-v1", vec![
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(decision)]),
        crate::preserves_rail::record("schedule", vec![crate::preserves_rail::string(&input.schedule.receipt_ref)]),
        crate::preserves_rail::record("request", vec![crate::preserves_rail::string(&input.request.request_ref)]),
        crate::preserves_rail::record("stages", vec![crate::preserves_rail::sequence(
            input.expected_stage_order.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("completed", vec![crate::preserves_rail::sequence(
            completed_indices.iter().map(|index| crate::preserves_rail::u64_value(*index)).collect(),
        )]),
        crate::preserves_rail::record("outputs", vec![refs_sequence(input.expected_output_refs)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        checks_value_from_pairs(&[("schedule-replay", decision), ("stage-order-bound", decision)]),
    ])
}

pub fn receipt_summary(value: &IoValue) -> Result<String> {
    if let Ok(receipt) = parse_job_worker_schedule_receipt_value(value) {
        return Ok(format!(
            "job worker schedule decision={} job={} request={} queue={} lease={} token={} worker={} result={} diagnostics={}",
            receipt.decision,
            receipt.job_ref,
            receipt.request_ref,
            receipt.queue_key,
            receipt.lease_key,
            receipt.token_ref.unwrap_or_else(|| "-".to_string()),
            receipt.worker_receipt_ref.unwrap_or_else(|| "-".to_string()),
            receipt.result_ref.unwrap_or_else(|| "-".to_string()),
            receipt.diagnostics.join(";")
        ));
    }
    if let Ok(receipt) = parse_job_worker_receipt_value(value) {
        return Ok(format!(
            "job worker receipt decision={} job={} request={} result={} status={} diagnostics={}",
            receipt.decision,
            receipt.job_ref.unwrap_or_else(|| "-".to_string()),
            receipt.request_ref.unwrap_or_else(|| "-".to_string()),
            receipt.result_ref,
            receipt.status_refs.len(),
            receipt.diagnostics.join(";")
        ));
    }
    if let Ok(result) = parse_job_worker_result_value(value) {
        return Ok(format!(
            "job worker result decision={} job={} target={} execution={} outputs={} diagnostics={}",
            result.decision,
            result.job_ref,
            result.target_peer,
            result.execution_receipt_ref.unwrap_or_else(|| "-".to_string()),
            result.output_refs.len(),
            result.diagnostics.join(";")
        ));
    }
    let receipt = parse_job_receipt(value).or_else(|_| parse_blob_ref_job_receipt_value(value))?;
    Ok(format!(
        "job receipt operation={} decision={} job={} request={} stage={} outputs={}",
        receipt.operation,
        receipt.decision,
        receipt.job_ref.unwrap_or_else(|| "-".to_string()),
        receipt.request_ref.unwrap_or_else(|| "-".to_string()),
        receipt.stage_id.unwrap_or_else(|| "-".to_string()),
        receipt.output_refs.len()
    ))
}

pub fn dag_summary(dag: &JobDag) -> String {
    format!(
        "job dag {} nodes={} edges={} outputs={}",
        dag.job_ref,
        dag.nodes.len(),
        dag.edges.len(),
        dag.output_roots.join(",")
    )
}

struct StageMemo<'a> {
    dag: &'a JobDag,
    request: &'a JobOutputRequest,
    node: &'a JobNode,
    inputs: &'a [IoValue],
    cache_root: &'a FilePath,
    key_input: &'a crate::eval_cache::KeyInput,
    key_ref: &'a str,
}

fn run_stage_with_cache(
    dag: &JobDag,
    request: &JobOutputRequest,
    node: &JobNode,
    inputs: &[IoValue],
    options: &JobRunOptions<'_>,
) -> Result<JobStageRun> {
    let is_cacheable = node.kind != "materialize";
    let key_input = stage_cache_key_input(dag, request, node, inputs)?;
    let key_value = crate::eval_cache::key_value(&key_input)?;
    let key = crate::eval_cache::parse_key(&key_value)?;
    let memo = StageMemo {
        dag,
        request,
        node,
        inputs,
        cache_root: options.cache_root,
        key_input: &key_input,
        key_ref: &key.key_ref,
    };
    if is_cacheable && let Some(hit) = stage_memo_hit(&memo)? {
        return Ok(hit);
    }
    let stage = execute_stage(dag, request, node, inputs, options)?;
    if is_cacheable {
        stage_memo_store(&memo, stage)
    } else {
        Ok(stage)
    }
}
