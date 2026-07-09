
fn stage_memo_hit(input: &StageMemo<'_>) -> Result<Option<JobStageRun>> {
    let current_policy_refs = combined_policy_refs(input.dag, input.request, Some(input.node));
    if let Ok(hit) = crate::eval_cache::get(input.cache_root, input.key_ref, &crate::eval_cache::GetInput {
        current_policy_refs,
        current_handler_profile_ref: input.request.handler_profile_ref.clone(),
        current_capability_refs: Vec::new(),
        current_revocation_refs: Vec::new(),
        semantic: true,
        ..crate::eval_cache::GetInput::default()
    }) && let Some(output) = hit.output
    {
        let output_values = parse_cached_stage_output(&output)?;
        let output_refs = refs_for_values(&output_values)?;
        let cache_ref = crate::preserves_rail::canonical_hash(&hit.receipt_value)?;
        let receipt_value = job_receipt_value(JobReceiptInput {
            operation: "memo-hit",
            decision: "pass",
            job_ref: Some(&input.dag.job_ref),
            request_ref: Some(&input.request.request_ref),
            stage_id: Some(&input.node.id),
            input_refs: &refs_for_values(input.inputs)?,
            output_refs: &output_refs,
            cache_ref: Some(&cache_ref),
            effect_refs: &[],
            policy_refs: &combined_policy_refs(input.dag, input.request, Some(input.node)),
            evidence_refs: std::slice::from_ref(&cache_ref),
            diagnostics: &[],
            checks: &[
                ("eval-cache-hit", "pass"),
                ("memo-key-bound", "pass"),
                ("policy-current-revalidation", "pass"),
            ],
        })?;
        return Ok(Some(JobStageRun {
            node_id: input.node.id.clone(),
            output_values,
            output_refs,
            receipt_value,
        }));
    }
    Ok(None)
}

fn stage_memo_store(input: &StageMemo<'_>, stage: JobStageRun) -> Result<JobStageRun> {
    let stage_output = crate::preserves_rail::sequence(stage.output_values.clone());
    let policy_refs = combined_policy_refs(input.dag, input.request, Some(input.node));
    let tier = if policy_refs.is_empty() {
        crate::eval_cache::TIER_PURE
    } else {
        crate::eval_cache::TIER_POLICY_CURRENT
    };
    let cache_put = crate::eval_cache::put(input.cache_root, input.key_input, &crate::eval_cache::ValueInput {
        tier: tier.to_string(),
        status: crate::eval_cache::STATUS_PASS.to_string(),
        output: Some(stage_output),
        dependency_refs: input.key_input.dependency_refs.clone(),
        policy_refs: policy_refs.clone(),
        evidence_refs: vec![crate::preserves_rail::canonical_hash(&stage.receipt_value)?],
        diagnostics: Vec::new(),
    })?;
    let cache_ref = crate::preserves_rail::canonical_hash(&cache_put.receipt_value)?;
    let stage_receipt_ref = crate::preserves_rail::canonical_hash(&stage.receipt_value)?;
    let evidence_refs = vec![stage_receipt_ref, cache_ref.clone()];
    let receipt_value = job_receipt_value(JobReceiptInput {
        operation: "stage",
        decision: "pass",
        job_ref: Some(&input.dag.job_ref),
        request_ref: Some(&input.request.request_ref),
        stage_id: Some(&input.node.id),
        input_refs: &refs_for_values(input.inputs)?,
        output_refs: &stage.output_refs,
        cache_ref: Some(&cache_ref),
        effect_refs: &[],
        policy_refs: &policy_refs,
        evidence_refs: &evidence_refs,
        diagnostics: &[],
        checks: &[
            ("eval-cache-miss", "pass"),
            ("stage-executed", "pass"),
            ("memo-key-bound", "pass"),
        ],
    })?;
    Ok(JobStageRun { receipt_value, ..stage })
}

fn execute_stage(
    dag: &JobDag,
    request: &JobOutputRequest,
    node: &JobNode,
    inputs: &[IoValue],
    options: &JobRunOptions<'_>,
) -> Result<JobStageRun> {
    let mut effects = Vec::new();
    let output_values = match node.kind.as_str() {
        "source" => execute_source(node, options, &mut effects)?,
        "map" => execute_map(node, inputs)?,
        "filter" => execute_filter(node, inputs)?,
        "reduce" => execute_reduce(node, inputs)?,
        "materialize" => execute_materialize(node, inputs, options, &mut effects)?,
        _ => return Err(MoltenError::invalid_harness(format!("unsupported job stage kind {}", node.kind))),
    };
    let output_refs = refs_for_values(&output_values)?;
    let receipt_value = job_receipt_value(JobReceiptInput {
        operation: if node.kind == "materialize" {
            "materialize"
        } else {
            "stage"
        },
        decision: "pass",
        job_ref: Some(&dag.job_ref),
        request_ref: Some(&request.request_ref),
        stage_id: Some(&node.id),
        input_refs: &refs_for_values(inputs)?,
        output_refs: &output_refs,
        cache_ref: None,
        effect_refs: &effects,
        policy_refs: &combined_policy_refs(dag, request, Some(node)),
        evidence_refs: &node.evidence_refs,
        diagnostics: &[],
        checks: &[
            ("deterministic-stage", "pass"),
            ("explicit-effect-boundary", "pass"),
            ("no-mobile-closures", "pass"),
        ],
    })?;
    Ok(JobStageRun {
        node_id: node.id.clone(),
        output_values,
        output_refs,
        receipt_value,
    })
}

fn execute_source(
    node: &JobNode,
    options: &JobRunOptions<'_>,
    effects: &mut impl crate::bounded::VecSink<String>,
) -> Result<Vec<IoValue>> {
    let source = simple_record(&node.config, "source", 1)?;
    let payload = crate::preserves_rail::value_to_iovalue(&source[0]);
    if let Some(values) = payload.collect_simple_record("values", Some(1)) {
        return sequence_items(&values[0], "source values");
    }
    if let Some(value) = payload.collect_simple_record("value", Some(1)) {
        return Ok(vec![crate::preserves_rail::value_to_iovalue(&value[0])]);
    }
    if let Some(typed) = payload.collect_simple_record("typed-storage", Some(3)) {
        let namespace = required_string(&typed[0], "source typed storage namespace")?;
        let key = required_string(&typed[1], "source typed storage key")?;
        let schema_ref = parse_optional_ref_value(&typed[2])?;
        let admission = crate::typed_storage::Admission::local_fixture(&format!("job:{}:{}", namespace, key));
        let get =
            crate::typed_storage::get_value(options.storage_root, &namespace, &key, schema_ref.as_deref(), &admission)?;
        effects.push_item(crate::preserves_rail::canonical_hash(&get.receipt_value)?);
        return Ok(vec![get.value]);
    }
    if let Some(chunk) = payload.collect_simple_record("chunk-manifest", Some(1)) {
        let manifest_ref = required_ref(&chunk[0], "source chunk manifest ref")?;
        let read = crate::chunk_store::read_object(options.chunk_root, &manifest_ref)?;
        effects.push_item(crate::preserves_rail::canonical_hash(&read.receipt_value)?);
        let value = crate::preserves_rail::parse_canonical_bytes(&read.bytes)?;
        if let Some(items) = value.collect_sequence() {
            ensure_count_at_most(items.len(), MAX_JOB_STAGE_VALUES, "source chunk values")?;
            let mut output = Vec::with_capacity(items.len());
            for item in items.iter() {
                push_bounded(
                    &mut output,
                    crate::preserves_rail::value_to_iovalue(item),
                    MAX_JOB_STAGE_VALUES,
                    "source chunk values",
                )?;
            }
            return Ok(output);
        }
        return Ok(vec![value]);
    }
    Err(MoltenError::invalid_harness(
        "unsupported source config; expected <source <values [...]>>, <source <value ...>>, <source <typed-storage ...>>, or <source <chunk-manifest ...>>",
    ))
}

fn execute_map(node: &JobNode, inputs: &[IoValue]) -> Result<Vec<IoValue>> {
    let op = stage_operation(&node.config)?;
    ensure_count_at_most(inputs.len(), MAX_JOB_STAGE_VALUES, "map input values")?;
    let mut output = Vec::with_capacity(inputs.len());
    for value in inputs {
        push_bounded(&mut output, apply_map_op(&op, value)?, MAX_JOB_STAGE_VALUES, "map output values")?;
    }
    Ok(output)
}

fn execute_filter(node: &JobNode, inputs: &[IoValue]) -> Result<Vec<IoValue>> {
    let op = stage_operation(&node.config)?;
    let mut output = Vec::new();
    for value in inputs {
        if apply_filter_op(&op, value)? {
            push_bounded(&mut output, value.clone(), MAX_JOB_STAGE_VALUES, "filter output values")?;
        }
    }
    Ok(output)
}

fn execute_reduce(node: &JobNode, inputs: &[IoValue]) -> Result<Vec<IoValue>> {
    let op = stage_operation(&node.config)?;
    match op.name.as_str() {
        "count" => Ok(vec![crate::preserves_rail::u64_value(inputs.len() as u64)]),
        "sum-u64" | "sum-integers" => {
            let mut sum = 0_u64;
            for value in inputs {
                sum = sum
                    .checked_add(required_u64_value(value, "sum-u64 input")?)
                    .ok_or_else(|| MoltenError::invalid_harness("sum-u64 reducer overflowed u64"))?;
            }
            Ok(vec![crate::preserves_rail::u64_value(sum)])
        }
        "concat-lists" => {
            let mut values = Vec::new();
            for value in inputs {
                if let Some(items) = value.collect_sequence() {
                    for item in items.iter() {
                        push_bounded(
                            &mut values,
                            crate::preserves_rail::value_to_iovalue(item),
                            MAX_JOB_STAGE_VALUES,
                            "concat-list output values",
                        )?;
                    }
                } else {
                    return Err(MoltenError::invalid_harness("concat-lists reducer requires sequence inputs"));
                }
            }
            Ok(vec![crate::preserves_rail::sequence(values)])
        }
        other => Err(MoltenError::invalid_harness(format!("unsupported reduce operation {other}"))),
    }
}

fn execute_materialize(
    node: &JobNode,
    inputs: &[IoValue],
    options: &JobRunOptions<'_>,
    effects: &mut impl crate::bounded::VecSink<String>,
) -> Result<Vec<IoValue>> {
    let config = materialize_config(&node.config)?;
    let value = crate::preserves_rail::sequence(inputs.to_vec());
    match config.kind.as_str() {
        "inline" => Ok(vec![value]),
        "typed-storage" => {
            let namespace = config
                .namespace
                .ok_or_else(|| MoltenError::invalid_harness("typed-storage materialization requires namespace"))?;
            let key = config
                .key
                .ok_or_else(|| MoltenError::invalid_harness("typed-storage materialization requires key"))?;
            let admission = crate::typed_storage::Admission::local_fixture(&format!("job:{namespace}:{key}"));
            let put = crate::typed_storage::put_value(options.storage_root, &crate::typed_storage::PutInput {
                namespace,
                key,
                schema_ref: None,
                value,
                producer_ref: local_ref("job-materialize-producer", &node.id)?,
                policy_refs: node.policy_refs.clone(),
                evidence_refs: node.evidence_refs.clone(),
                admission,
            })?;
            effects.push_item(crate::preserves_rail::canonical_hash(&put.receipt_value)?);
            Ok(vec![put.typed_ref_value])
        }
        "chunk-manifest" => {
            let bytes = crate::preserves_rail::canonical_bytes(&value)?;
            let put = crate::chunk_store::put_bytes(
                options.chunk_root,
                "job-materialization",
                &bytes,
                DEFAULT_FIXED_V1_CHUNK_SIZE,
            )?;
            effects.push_item(crate::preserves_rail::canonical_hash(&put.receipt_value)?);
            Ok(vec![crate::preserves_rail::record("chunk-manifest-ref", vec![
                crate::preserves_rail::string(&put.manifest_ref),
            ])])
        }
        other => Err(MoltenError::invalid_harness(format!("unsupported materialization kind {other}"))),
    }
}

#[derive(Debug, Clone)]
struct StageOperation {
    name: String,
    argument: Option<IoValue>,
}
