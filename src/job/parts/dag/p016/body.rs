
fn push_authority_checks(
    request: &JobWorkerRequest,
    admission: &JobAdmissionReceipt,
    buffers: &mut DeliveryCheckBuffers,
) {
    let has_authority = !request.authority_refs.is_empty()
        && !admission.authority_receipt_refs.is_empty()
        && refs_are_bound_in_admission(&request.authority_refs, &admission.refs);
    buffers.push("authority-binding", has_authority);
    if !has_authority {
        buffers.note("job worker missing explicit authority refs admitted for job:execute");
    }

    let has_resource = !request.resource_refs.is_empty()
        && admission.resource_verdict == "pass"
        && refs_are_bound_in_admission(&request.resource_refs, &admission.refs);
    buffers.push("resource-binding", has_resource);
    if !has_resource {
        buffers.note("job worker missing admitted resource refs");
    }
}

fn push_target_state_check(
    request: &JobWorkerRequest,
    execution_request: &JobExecutionRequest,
    buffers: &mut DeliveryCheckBuffers,
) -> Result<()> {
    let has_target_state_only = execution_request.target_peer == request.target_peer
        && !crate::preserves_rail::to_text(&request.value)?.contains("<source-registry")
        && !crate::preserves_rail::to_text(&execution_request.value)?.contains("<source-registry");
    buffers.push("target-state-only", has_target_state_only);
    if !has_target_state_only {
        buffers.note("job worker execution request must run from target roots only");
    }
    Ok(())
}

fn refs_are_bound_in_admission(refs: &[String], admission_refs: &[String]) -> bool {
    refs.iter().all(|reference| admission_refs.iter().any(|admission_ref| admission_ref == reference))
}

fn recompute_execution_closure(
    target_registry: &FilePath,
    dag: &JobDag,
    stage_order: &[String],
) -> Result<Vec<String>> {
    let selected = stage_order.iter().cloned().collect::<OrderedSet<_>>();
    let roots = admission_roots(target_registry, dag, &selected)?;
    let closure = crate::artifacts::dependency_closure(target_registry, &roots)?;
    if !closure.missing_refs.is_empty() {
        return Err(MoltenError::invalid_harness(format!(
            "job execution target closure missing refs: {}",
            closure.missing_refs.join(",")
        )));
    }
    Ok(closure.closure_refs)
}

fn status(ok: bool) -> &'static str {
    if ok { "pass" } else { "fail" }
}

fn sync_roots(source_registry: &FilePath, dag: &JobDag, request: &JobSyncRequest) -> Result<Vec<String>> {
    let mut roots = vec![job_artifact_ref(source_registry, &dag.job_ref)?];
    let selected = request.stage_ids.iter().cloned().collect::<OrderedSet<_>>();
    for node in &dag.nodes {
        if (selected.is_empty() || selected.contains(&node.id))
            && let Some(stage_artifact_ref) = node.stage_artifact_ref.as_ref()
        {
            push_bounded(&mut roots, stage_artifact_ref.clone(), MAX_JOB_REFS, "job sync roots")?;
        }
    }
    roots.sort();
    roots.dedup();
    Ok(roots)
}

fn sync_install_order(source_registry: &FilePath, roots: &[String]) -> Result<Vec<String>> {
    let mut visited = OrderedSet::new();
    let mut order = Vec::new();
    for root in roots {
        sync_install_order_visit(source_registry, root, &mut visited, &mut order)?;
    }
    Ok(order)
}

fn sync_install_order_visit(
    source_registry: &FilePath,
    artifact_ref: &str,
    visited: &mut OrderedSet<String>,
    order: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    validate_ref(artifact_ref, "job sync artifact ref")?;
    let mut pending = Vec::with_capacity(1);
    push_bounded(&mut pending, (artifact_ref.to_string(), false), MAX_JOB_REFS, "job sync install order frames")?;
    while let Some((current_ref, is_exit_frame)) = pending.pop() {
        validate_ref(&current_ref, "job sync artifact ref")?;
        if is_exit_frame {
            push_bounded(order, current_ref, MAX_JOB_REFS, "job sync install order")?;
            continue;
        }
        let is_first_visit = visited.insert(current_ref.clone());
        if is_first_visit {
            let artifact = crate::artifacts::read_artifact(source_registry, &current_ref)?;
            push_bounded(&mut pending, (current_ref, true), MAX_JOB_REFS, "job sync install order frames")?;
            for dependency_ref in artifact.dependency_refs.iter().rev() {
                push_bounded(
                    &mut pending,
                    (dependency_ref.clone(), false),
                    MAX_JOB_REFS,
                    "job sync install order frames",
                )?;
            }
        }
    }
    Ok(())
}

fn request_for_analysis(dag: &JobDag, output_request: Option<&IoValue>) -> Result<JobOutputRequest> {
    request_for_dag(dag, output_request)
}

fn request_for_dag(dag: &JobDag, output_request: Option<&IoValue>) -> Result<JobOutputRequest> {
    let request = if let Some(output_request) = output_request {
        parse_job_output_request_value(output_request, &dag.job_ref)?
    } else {
        default_output_request(dag)?
    };
    validate_output_request_roots(dag, &request)?;
    Ok(request)
}

fn validate_output_request_roots(dag: &JobDag, request: &JobOutputRequest) -> Result<()> {
    let roots = requested_output_roots(dag, request)?;
    let node_ids = dag.nodes.iter().map(|node| node.id.clone()).collect::<OrderedSet<_>>();
    for root in roots {
        if !node_ids.contains(&root) {
            return Err(MoltenError::invalid_harness(format!("job output root {root} is not a node")));
        }
    }
    Ok(())
}

fn dependency_ids(plan: &TrellisExecutionPlan, node_id: &str) -> Result<Vec<String>> {
    let deps = plan.dependency_indices.get(node_id).cloned().unwrap_or_default();
    let mut ids = Vec::with_capacity(deps.len());
    for dep in deps {
        let dep_index = usize::try_from(dep).map_err(|error| {
            MoltenError::invalid_harness(format!("trellis dependency index cannot convert to usize: {error}"))
        })?;
        let dep_id = plan
            .node_index
            .iter()
            .find_map(|(id, index)| (*index == dep_index).then_some(id.clone()))
            .ok_or_else(|| MoltenError::invalid_harness(format!("trellis dependency index {dep_index} has no node")))?;
        ids.push(dep_id);
    }
    ids.sort();
    Ok(ids)
}

fn fusion_edge_sort_key<'a>(
    positions: &OrderedMap<String, usize>,
    edge: &'a JobEdge,
) -> (bool, usize, &'a String, &'a String) {
    match positions.get(&edge.from_node) {
        Some(position) => (false, *position, &edge.from_node, &edge.to_node),
        None => (true, 0, &edge.from_node, &edge.to_node),
    }
}

fn fusion_edge_safe(from: &JobNode, to: &JobNode, edge: &JobEdge) -> bool {
    matches!(from.kind.as_str(), "map" | "filter")
        && matches!(to.kind.as_str(), "map" | "filter")
        && edge.schema_ref.is_none()
        && edge.materialization == "stream"
        && from.effect_manifest_refs.is_empty()
        && to.effect_manifest_refs.is_empty()
        && from.policy_refs.is_empty()
        && to.policy_refs.is_empty()
}

fn analysis_receipt_value(input: AnalysisReceiptValueInput<'_>) -> Result<IoValue> {
    validate_ref(input.job_ref, "job analysis receipt job ref")?;
    validate_ref(input.request_ref, "job analysis receipt request ref")?;
    validate_ref(input.artifact_ref, "job analysis receipt artifact ref")?;
    let mut checks = input.checks.to_vec();
    checks.push(("canonical-receipt", "pass"));
    Ok(crate::preserves_rail::record(input.label, vec![
        crate::preserves_rail::string(input.schema),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(input.operation)]),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
        crate::preserves_rail::record("job", vec![crate::preserves_rail::string(input.job_ref)]),
        crate::preserves_rail::record("request", vec![crate::preserves_rail::string(input.request_ref)]),
        crate::preserves_rail::record("artifact", vec![crate::preserves_rail::string(input.artifact_ref)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        checks_value_from_pairs(&checks),
    ]))
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
