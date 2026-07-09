
fn stage_operation(config: &IoValue) -> Result<StageOperation> {
    if let Ok(fields) = simple_record(config, "op", 2) {
        let name = required_string(&fields[0], "stage operation")?;
        validate_stage_operation(&name)?;
        return Ok(StageOperation {
            name,
            argument: Some(crate::preserves_rail::value_to_iovalue(&fields[1])),
        });
    }
    let fields = simple_record(config, "op", 1)?;
    let name = required_string(&fields[0], "stage operation")?;
    validate_stage_operation(&name)?;
    Ok(StageOperation { name, argument: None })
}

fn apply_map_op(op: &StageOperation, value: &IoValue) -> Result<IoValue> {
    match op.name.as_str() {
        "identity" => Ok(value.clone()),
        "wrap" | "tag-record" => {
            let label = op
                .argument
                .as_ref()
                .ok_or_else(|| MoltenError::invalid_harness("wrap operation requires label argument"))?
                .as_string()
                .map(|value| value.into_owned())
                .ok_or_else(|| MoltenError::invalid_harness("wrap operation label must be a string"))?;
            Ok(crate::preserves_rail::record("wrapped", vec![crate::preserves_rail::string(&label), value.clone()]))
        }
        "project-field" => {
            let label = op
                .argument
                .as_ref()
                .ok_or_else(|| MoltenError::invalid_harness("project-field operation requires label argument"))?
                .as_string()
                .map(|value| value.into_owned())
                .ok_or_else(|| MoltenError::invalid_harness("project-field label must be a string"))?;
            if let Some(fields) = value.collect_simple_record(&label, Some(1)) {
                Ok(crate::preserves_rail::value_to_iovalue(&fields[0]))
            } else {
                Err(MoltenError::invalid_harness(format!("project-field did not match record label {label}")))
            }
        }
        other => Err(MoltenError::invalid_harness(format!("unsupported map operation {other}"))),
    }
}

fn apply_filter_op(op: &StageOperation, value: &IoValue) -> Result<bool> {
    match op.name.as_str() {
        "keep-all" => Ok(true),
        "drop-all" => Ok(false),
        "equals" => Ok(op.argument.as_ref().is_some_and(|expected| expected == value)),
        "match-record" => {
            let label = op
                .argument
                .as_ref()
                .ok_or_else(|| MoltenError::invalid_harness("match-record operation requires label argument"))?
                .as_string()
                .map(|value| value.into_owned())
                .ok_or_else(|| MoltenError::invalid_harness("match-record label must be a string"))?;
            Ok(value.collect_simple_record(&label, None).is_some())
        }
        other => Err(MoltenError::invalid_harness(format!("unsupported filter operation {other}"))),
    }
}

#[derive(Debug, Clone)]
struct MaterializeConfig {
    kind: String,
    namespace: Option<String>,
    key: Option<String>,
}

fn materialize_config(config: &IoValue) -> Result<MaterializeConfig> {
    if let Ok(fields) = simple_record(config, "materialize", 3) {
        let kind = required_string(&fields[0], "materialize kind")?;
        validate_request_materialization(&kind)?;
        return Ok(MaterializeConfig {
            kind,
            namespace: Some(required_string(&fields[1], "materialize namespace")?),
            key: Some(required_string(&fields[2], "materialize key")?),
        });
    }
    let fields = simple_record(config, "materialize", 1)?;
    let kind = required_string(&fields[0], "materialize kind")?;
    validate_request_materialization(&kind)?;
    Ok(MaterializeConfig {
        kind,
        namespace: None,
        key: None,
    })
}

fn default_output_request(dag: &JobDag) -> Result<JobOutputRequest> {
    let roots = if dag.output_roots.is_empty() {
        sink_nodes(dag)?
    } else {
        dag.output_roots.clone()
    };
    let value = job_output_request_value(OutputRequestValueInput {
        dag_ref: &dag.job_ref,
        roots: &roots,
        materialization: "inline",
        policy_refs: &dag.policy_refs,
        handler_profile_ref: None,
        seed_config_ref: None,
    })?;
    parse_job_output_request_value(&value, &dag.job_ref)
}

fn stage_cache_key_input(
    dag: &JobDag,
    request: &JobOutputRequest,
    node: &JobNode,
    inputs: &[IoValue],
) -> Result<crate::eval_cache::KeyInput> {
    let input_refs = refs_for_values(inputs)?;
    let stage_artifact_ref = stage_artifact_or_builtin_ref(node)?;
    let dependency_capacity = 3usize
        .saturating_add(input_refs.len())
        .saturating_add(node.stage_artifact_ref.iter().count())
        .saturating_add(node.effect_manifest_refs.len())
        .saturating_add(node.evidence_refs.len());
    ensure_count_at_most(dependency_capacity, MAX_JOB_REFS, "job stage dependency refs")?;
    let mut dependency_refs = Vec::with_capacity(dependency_capacity);
    dependency_refs.push(dag.job_ref.clone());
    dependency_refs.push(request.request_ref.clone());
    dependency_refs.push(stage_artifact_ref.clone());
    dependency_refs.extend(input_refs);
    if let Some(stage_artifact_ref) = node.stage_artifact_ref.as_ref() {
        dependency_refs.push(stage_artifact_ref.clone());
    }
    dependency_refs.extend(node.effect_manifest_refs.iter().cloned());
    dependency_refs.extend(node.evidence_refs.iter().cloned());
    let dependency_refs = sorted_unique(&dependency_refs);
    let dependency_closure_hash =
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("job-stage-dependency-closure", vec![
            refs_sequence(&dependency_refs),
        ]))?;
    let input_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("job-stage-input-v1", vec![
        crate::preserves_rail::record("job", vec![crate::preserves_rail::string(&dag.job_ref)]),
        crate::preserves_rail::record("request", vec![crate::preserves_rail::string(&request.request_ref)]),
        crate::preserves_rail::record("stage", vec![crate::preserves_rail::string(&node.id)]),
        crate::preserves_rail::record("stage-artifact", vec![crate::preserves_rail::string(&stage_artifact_ref)]),
        crate::preserves_rail::record("inputs", vec![crate::preserves_rail::sequence(inputs.to_vec())]),
        crate::preserves_rail::record("config", vec![node.config.clone()]),
    ]))?;
    let assumption_capacity = dag
        .schema_refs
        .len()
        .saturating_add(node.effect_manifest_refs.len())
        .saturating_add(request.seed_config_ref.iter().count())
        .saturating_add(1);
    ensure_count_at_most(assumption_capacity, MAX_JOB_REFS, "job stage assumption refs")?;
    let mut assumptions = Vec::with_capacity(assumption_capacity);
    assumptions.extend(dag.schema_refs.iter().cloned());
    assumptions.extend(node.effect_manifest_refs.iter().cloned());
    assumptions.extend(request.seed_config_ref.iter().cloned());
    assumptions.push(crate::preserves_rail::canonical_hash(&node.config)?);
    Ok(crate::eval_cache::KeyInput {
        operation: JOB_CACHE_OPERATION.to_string(),
        version: "v1".to_string(),
        input_ref,
        dependency_closure_hash,
        dependency_refs,
        artifact_refs: node.stage_artifact_ref.iter().cloned().collect(),
        schema_refs: dag.schema_refs.clone(),
        handler_profile_ref: request.handler_profile_ref.clone(),
        policy_refs: combined_policy_refs(dag, request, Some(node)),
        capability_refs: Vec::new(),
        revocation_refs: Vec::new(),
        resource_refs: Vec::new(),
        effect_manifest_refs: node.effect_manifest_refs.clone(),
        tool_ref: job_tool_ref()?,
        tool_version: JOB_TOOL_VERSION.to_string(),
        assumption_refs: sorted_unique(&assumptions),
        ..crate::eval_cache::KeyInput::default()
    })
}

fn stage_artifact_or_builtin_ref(node: &JobNode) -> Result<String> {
    if let Some(stage_artifact_ref) = node.stage_artifact_ref.as_ref() {
        return Ok(stage_artifact_ref.clone());
    }
    match node.kind.as_str() {
        "source" => builtin_stage_operation_ref("source"),
        "materialize" => builtin_stage_operation_ref("materialize"),
        "map" | "filter" | "reduce" => builtin_stage_operation_ref(&stage_operation(&node.config)?.name),
        other => Err(MoltenError::invalid_harness(format!("unsupported stage kind {other}"))),
    }
}

fn job_tool_ref() -> Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("job-dag-tool-v1", vec![
        crate::preserves_rail::string("molten-job-dag"),
        crate::preserves_rail::string(JOB_TOOL_VERSION),
    ]))
}

struct JobReceiptInput<'a> {
    operation: &'a str,
    decision: &'a str,
    job_ref: Option<&'a str>,
    request_ref: Option<&'a str>,
    stage_id: Option<&'a str>,
    input_refs: &'a [String],
    output_refs: &'a [String],
    cache_ref: Option<&'a str>,
    effect_refs: &'a [String],
    policy_refs: &'a [String],
    evidence_refs: &'a [String],
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

fn job_receipt_value(input: JobReceiptInput<'_>) -> Result<IoValue> {
    validate_receipt_operation(input.operation)?;
    validate_decision(input.decision)?;
    if let Some(job_ref) = input.job_ref {
        validate_ref(job_ref, "job receipt job ref")?;
    }
    if let Some(request_ref) = input.request_ref {
        validate_ref(request_ref, "job receipt request ref")?;
    }
    if let Some(stage_id) = input.stage_id {
        validate_node_id(stage_id)?;
    }
    validate_refs(input.input_refs, "job receipt input ref")?;
    validate_refs(input.output_refs, "job receipt output ref")?;
    if let Some(cache_ref) = input.cache_ref {
        validate_ref(cache_ref, "job receipt cache ref")?;
    }
    validate_refs(input.effect_refs, "job receipt effect ref")?;
    validate_refs(input.policy_refs, "job receipt policy ref")?;
    validate_refs(input.evidence_refs, "job receipt evidence ref")?;
    let mut checks = input.checks.to_vec();
    checks.push(("canonical-receipt", "pass"));
    Ok(crate::preserves_rail::record("job-dag-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::JOB_DAG_RECEIPT_SCHEMA),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(input.operation)]),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("job", vec![optional_ref_value(input.job_ref)]),
        crate::preserves_rail::record("request", vec![optional_ref_value(input.request_ref)]),
        crate::preserves_rail::record("stage", vec![optional_string_value(input.stage_id)]),
        crate::preserves_rail::record("inputs", vec![refs_sequence(&sorted_unique(input.input_refs))]),
        crate::preserves_rail::record("outputs", vec![refs_sequence(&sorted_unique(input.output_refs))]),
        crate::preserves_rail::record("cache", vec![optional_ref_value(input.cache_ref)]),
        crate::preserves_rail::record("effects", vec![refs_sequence(&sorted_unique(input.effect_refs))]),
        crate::preserves_rail::record("policy", vec![refs_sequence(&sorted_unique(input.policy_refs))]),
        crate::preserves_rail::record("evidence", vec![refs_sequence(&sorted_unique(input.evidence_refs))]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        checks_value_from_pairs(&checks),
    ]))
}

fn parse_node_sequence(value: &Value<IoValue>) -> Result<Vec<JobNode>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = simple_record(&value, "nodes", 1)?;
    let items = required_sequence(&record[0], "job nodes")?;
    ensure_count_at_most(items.len(), MAX_JOB_NODES, "job nodes")?;
    let mut nodes = Vec::with_capacity(items.len());
    for item in items.iter() {
        push_bounded(
            &mut nodes,
            parse_job_node_value(&crate::preserves_rail::value_to_iovalue(item))?,
            MAX_JOB_NODES,
            "job nodes",
        )?;
    }
    Ok(nodes)
}

fn parse_job_node_value(value: &IoValue) -> Result<JobNode> {
    let fields = value
        .collect_simple_record("job-node-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-node-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::JOB_DAG_NODE_SCHEMA, "job node")?;
    let id = record_string(&fields[1], "id")?;
    validate_node_id(&id)?;
    let kind = record_string(&fields[2], "kind")?;
    validate_stage_kind(&kind)?;
    let stage_artifact_ref = record_optional_ref(&fields[3], "stage-artifact")?;
    let input_ports = record_port_sequence(&fields[4], "inputs")?;
    let output_ports = record_port_sequence(&fields[5], "outputs")?;
    let config = record_iovalue(&fields[6], "config")?;
    reject_mobile_closure_config(&config)?;
    let checks = parse_checks(&fields[10])?;
    require_check(&checks, "stage-artifact-not-closure", "job node")?;
    Ok(JobNode {
        id,
        kind,
        stage_artifact_ref,
        input_ports,
        output_ports,
        config,
        effect_manifest_refs: record_ref_sequence(&fields[7], "effects")?,
        policy_refs: record_ref_sequence(&fields[8], "policy")?,
        evidence_refs: record_ref_sequence(&fields[9], "evidence")?,
        checks,
    })
}
