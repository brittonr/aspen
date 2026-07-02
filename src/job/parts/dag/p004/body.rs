
pub fn job_worker_request_value(input: JobWorkerRequestValueInput<'_>) -> Result<IoValue> {
    validate_ref(input.job_ref, "job worker request job ref")?;
    validate_non_empty(input.target_peer, "job worker target peer")?;
    for stage_id in input.stage_ids {
        validate_node_id(stage_id)?;
    }
    validate_ref(input.sync_ref, "job worker sync receipt ref")?;
    validate_ref(input.admission_ref, "job worker admission receipt ref")?;
    validate_ref(input.execution_request_ref, "job worker execution request ref")?;
    validate_refs(input.authority_refs, "job worker authority ref")?;
    validate_refs(input.resource_refs, "job worker resource ref")?;
    validate_refs(input.peer_bootstrap_refs, "job worker peer bootstrap ref")?;
    validate_refs(input.node_identity_refs, "job worker node identity ref")?;
    validate_refs(input.evidence_refs, "job worker evidence ref")?;
    Ok(crate::preserves_rail::record("job-worker-request-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::JOB_WORKER_REQUEST_SCHEMA),
        crate::preserves_rail::record("job", vec![crate::preserves_rail::string(input.job_ref)]),
        crate::preserves_rail::record("target-peer", vec![crate::preserves_rail::string(input.target_peer)]),
        crate::preserves_rail::record("stages", vec![crate::preserves_rail::sequence(
            input.stage_ids.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("sync", vec![crate::preserves_rail::string(input.sync_ref)]),
        crate::preserves_rail::record("admission", vec![crate::preserves_rail::string(input.admission_ref)]),
        crate::preserves_rail::record("execution-request", vec![crate::preserves_rail::string(
            input.execution_request_ref,
        )]),
        crate::preserves_rail::record("authority", vec![refs_sequence(&sorted_unique(input.authority_refs))]),
        crate::preserves_rail::record("resource", vec![refs_sequence(&sorted_unique(input.resource_refs))]),
        crate::preserves_rail::record("peer-bootstrap", vec![refs_sequence(&sorted_unique(input.peer_bootstrap_refs))]),
        crate::preserves_rail::record("node-identity", vec![refs_sequence(&sorted_unique(input.node_identity_refs))]),
        crate::preserves_rail::record("evidence", vec![refs_sequence(&sorted_unique(input.evidence_refs))]),
        checks_value(&[
            "target-admission-required",
            "loopback-execution-required",
            "remote-dataspace-carrier",
            "transport-is-not-authority",
            "target-state-only",
            "no-mobile-closures",
            "full-ref-identity",
        ]),
    ]))
}

pub fn parse_job_worker_request_value(value: &IoValue) -> Result<JobWorkerRequest> {
    reject_worker_ambient_tokens(value)?;
    let fields = value
        .collect_simple_record("job-worker-request-v1", Some(13))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-worker-request-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::JOB_WORKER_REQUEST_SCHEMA, "job worker request")?;
    let checks = parse_checks(&fields[12])?;
    require_check(&checks, "target-admission-required", "job worker request")?;
    require_check(&checks, "loopback-execution-required", "job worker request")?;
    require_check(&checks, "remote-dataspace-carrier", "job worker request")?;
    require_check(&checks, "transport-is-not-authority", "job worker request")?;
    require_check(&checks, "target-state-only", "job worker request")?;
    require_check(&checks, "no-mobile-closures", "job worker request")?;
    Ok(JobWorkerRequest {
        request_ref: crate::preserves_rail::canonical_hash(value)?,
        job_ref: record_ref(&fields[1], "job")?,
        target_peer: record_string(&fields[2], "target-peer")?,
        stage_ids: record_node_id_sequence(&fields[3], "stages")?,
        sync_ref: record_ref(&fields[4], "sync")?,
        admission_ref: record_ref(&fields[5], "admission")?,
        execution_request_ref: record_ref(&fields[6], "execution-request")?,
        authority_refs: record_ref_sequence(&fields[7], "authority")?,
        resource_refs: record_ref_sequence(&fields[8], "resource")?,
        peer_bootstrap_refs: record_ref_sequence(&fields[9], "peer-bootstrap")?,
        node_identity_refs: record_ref_sequence(&fields[10], "node-identity")?,
        evidence_refs: record_ref_sequence(&fields[11], "evidence")?,
        value: value.clone(),
    })
}

pub fn job_worker_envelope(input: JobWorkerEnvelopeInput<'_>) -> Result<crate::remote_dataspace::Envelope> {
    let request = parse_job_worker_request_value(input.request_value)?;
    if input.to_peer != request.target_peer {
        return Err(MoltenError::invalid_harness(format!(
            "job worker envelope target {} does not match request target {}",
            input.to_peer, request.target_peer
        )));
    }
    crate::remote_dataspace::build_envelope(crate::remote_dataspace::EnvelopeInput {
        from_peer: input.from_peer.to_string(),
        from_actor: input.from_actor.to_string(),
        to_peer: input.to_peer.to_string(),
        topic: input.topic.to_string(),
        operation: crate::remote_dataspace::Operation::Message,
        payload: input.request_value.clone(),
        content_refs: Vec::new(),
        capability_refs: request.authority_refs.clone(),
        evidence_refs: sorted_unique(&request.evidence_refs),
    })
}

pub fn execute_worker_delivery(input: JobWorkerExecuteInput<'_>) -> Result<JobWorkerExecution> {
    let request = parse_job_worker_request_value(&input.delivery.envelope.payload)?;
    let assignment_value = job_worker_assignment_value(&request, input.delivery)?;
    let assignment_ref = crate::preserves_rail::canonical_hash(&assignment_value)?;
    let delivery = collect_delivery_checks(&input, &request)?;
    let run = run_worker_delivery(&input, &request, &delivery)?;
    finish_worker_delivery(FinishDeliveryInput {
        input,
        request,
        assignment_value,
        assignment_ref,
        delivery,
        run,
    })
}

pub fn live_unrecorded_worker_result(input: JobWorkerExecuteInput<'_>) -> Result<JobWorkerExecution> {
    let without_log = JobWorkerExecuteInput {
        delivery_log: None,
        ..input
    };
    execute_worker_delivery(without_log)
}

pub fn parse_job_worker_result_value(value: &IoValue) -> Result<JobWorkerResult> {
    let fields = value
        .collect_simple_record("job-worker-result-v1", Some(12))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-worker-result-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::JOB_WORKER_RESULT_SCHEMA, "job worker result")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_worker_decision(&decision)?;
    let checks = parse_checks(&fields[11])?;
    require_check(&checks, "canonical-result", "job worker result")?;
    Ok(JobWorkerResult {
        result_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        job_ref: record_ref(&fields[2], "job")?,
        target_peer: record_string(&fields[3], "target-peer")?,
        execution_receipt_ref: record_optional_ref(&fields[4], "execution-receipt")?,
        output_refs: record_ref_sequence(&fields[5], "outputs")?,
        stage_receipt_refs: record_stage_receipt_sequence(&fields[6], "stage-receipts")?,
        resource_receipt_refs: record_ref_sequence(&fields[7], "resource")?,
        delivery_log_ref: record_optional_ref(&fields[8], "delivery-log")?,
        diagnostics: record_string_sequence(&fields[9], "diagnostics")?,
        checks,
        value: value.clone(),
    })
}

pub fn parse_job_worker_receipt_value(value: &IoValue) -> Result<JobWorkerReceipt> {
    let fields = value
        .collect_simple_record("job-worker-receipt-v1", Some(13))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-worker-receipt-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::JOB_WORKER_RECEIPT_SCHEMA, "job worker receipt")?;
    let operation = record_string(&fields[1], "operation")?;
    if operation != "worker-execute" {
        return Err(MoltenError::invalid_harness(format!("unsupported job worker receipt operation {operation}")));
    }
    let decision = record_string(&fields[2], "decision")?;
    validate_worker_decision(&decision)?;
    require_check(&parse_checks(&fields[12])?, "canonical-receipt", "job worker receipt")?;
    Ok(JobWorkerReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        job_ref: record_optional_ref(&fields[3], "job")?,
        request_ref: record_optional_ref(&fields[4], "request")?,
        assignment_ref: record_ref(&fields[5], "assignment")?,
        status_refs: record_ref_sequence(&fields[6], "status")?,
        result_ref: record_ref(&fields[7], "result")?,
        execution_receipt_ref: record_optional_ref(&fields[8], "execution-receipt")?,
        delivery_log_ref: record_optional_ref(&fields[9], "delivery-log")?,
        diagnostics: record_string_sequence(&fields[10], "diagnostics")?,
        value: value.clone(),
    })
}

pub fn parse_job_worker_schedule_receipt_value(value: &IoValue) -> Result<JobWorkerScheduleReceipt> {
    let fields = value
        .collect_simple_record("job-worker-schedule-receipt-v1", Some(20))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-worker-schedule-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::JOB_WORKER_SCHEDULE_RECEIPT_SCHEMA,
        "job worker schedule receipt",
    )?;
    let operation = record_string(&fields[1], "operation")?;
    if operation != "worker-schedule-local" {
        return Err(MoltenError::invalid_harness(format!("unsupported job worker schedule operation {operation}")));
    }
    let decision = record_string(&fields[2], "decision")?;
    validate_decision(&decision)?;
    require_check(&parse_checks(&fields[19])?, "canonical-receipt", "job worker schedule receipt")?;
    Ok(JobWorkerScheduleReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        operation,
        decision,
        job_ref: record_ref(&fields[3], "job")?,
        request_ref: record_ref(&fields[4], "request")?,
        queue_key: record_string(&fields[5], "queue-key")?,
        lease_key: record_string(&fields[6], "lease-key")?,
        worker_session: record_string(&fields[7], "worker-session")?,
        coordination_report_ref: record_ref(&fields[8], "coordination-report")?,
        token_ref: record_optional_ref(&fields[14], "token")?,
        worker_receipt_ref: record_optional_ref(&fields[15], "worker-receipt")?,
        result_ref: record_optional_ref(&fields[16], "result")?,
        diagnostics: record_string_sequence(&fields[17], "diagnostics")?,
        refs: record_ref_sequence(&fields[18], "refs")?,
        value: value.clone(),
    })
}

pub fn parse_blob_ref_job_receipt_value(value: &IoValue) -> Result<JobReceipt> {
    let fields = value
        .collect_simple_record("job-ref-receipt-v1", Some(18))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-ref-receipt-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::JOB_REF_RECEIPT_SCHEMA, "job ref receipt")?;
    let checks = parse_checks(&fields[17])?;
    require_check(&checks, "content-refs-only", "job ref receipt")?;
    require_check(&checks, "no-inline-large-bytes", "job ref receipt")?;
    Ok(JobReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        decision: record_string(&fields[2], "decision")?,
        job_ref: Some(record_string(&fields[4], "job-id")?),
        request_ref: Some(record_ref(&fields[3], "submission")?),
        stage_id: None,
        input_refs: record_ref_sequence(&fields[7], "inputs")?,
        output_refs: record_optional_ref(&fields[13], "output")?.into_iter().collect(),
        cache_ref: record_optional_ref(&fields[14], "output-put")?,
        checks,
        value: value.clone(),
    })
}

pub fn parse_job_dag_value(value: &IoValue) -> Result<JobDag> {
    let fields = value
        .collect_simple_record("job-dag-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-dag-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::JOB_DAG_SCHEMA, "job dag")?;
    let version = record_string(&fields[1], "version")?;
    if version != "v1" {
        return Err(MoltenError::invalid_harness(format!("unsupported job dag version {version}")));
    }
    let nodes = parse_node_sequence(&fields[2])?;
    if nodes.is_empty() {
        return Err(MoltenError::invalid_harness("job dag requires at least one node"));
    }
    let mut node_ids = OrderedSet::new();
    for node in &nodes {
        if !node_ids.insert(node.id.clone()) {
            return Err(MoltenError::invalid_harness(format!("duplicate job node id {}", node.id)));
        }
    }
    let edges = parse_edge_sequence(&fields[3])?;
    for edge in &edges {
        if !node_ids.contains(&edge.from_node) {
            return Err(MoltenError::invalid_harness(format!("job edge from unknown node {}", edge.from_node)));
        }
        if !node_ids.contains(&edge.to_node) {
            return Err(MoltenError::invalid_harness(format!("job edge to unknown node {}", edge.to_node)));
        }
    }
    let output_roots = record_node_id_sequence(&fields[4], "outputs")?;
    for root in &output_roots {
        if !node_ids.contains(root) {
            return Err(MoltenError::invalid_harness(format!("job output root {root} is not a node")));
        }
    }
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "canonical-dag", "job dag")?;
    require_check(&checks, "no-name-identity", "job dag")?;
    validate_topology(&nodes, &edges)?;
    Ok(JobDag {
        job_ref: crate::preserves_rail::canonical_hash(value)?,
        version,
        nodes,
        edges,
        output_roots,
        schema_refs: record_ref_sequence(&fields[5], "schemas")?,
        effect_manifest_refs: record_ref_sequence(&fields[6], "effect-manifests")?,
        policy_refs: record_ref_sequence(&fields[7], "policies")?,
        evidence_refs: record_ref_sequence(&fields[8], "evidence")?,
        value: value.clone(),
    })
}
