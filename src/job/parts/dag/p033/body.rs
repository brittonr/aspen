
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

fn execute_materialize_with_capabilities(
    node: &JobNode,
    inputs: &[IoValue],
    options: &CapabilityJobRunOptions<'_>,
    effects: &mut impl crate::bounded::VecSink<String>,
) -> Result<Vec<IoValue>> {
    let config = materialize_config(&node.config)?;
    let value = crate::preserves_rail::sequence(inputs.to_vec());
    match config.kind.as_str() {
        "inline" => Ok(vec![value]),
        "typed-storage" => Err(MoltenError::invalid_harness(
            "typed-storage job materialization requires a capability-aware typed storage adapter",
        )),
        "chunk-manifest" => {
            let bytes = crate::preserves_rail::canonical_bytes(&value)?;
            let put = crate::chunk_store::put_bytes_with_root(
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
