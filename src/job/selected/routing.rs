fn selected_port_output(node: &str, port: &str, values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record("job-port-output-v2", vec![
        crate::preserves_rail::string(node), crate::preserves_rail::string(port),
        crate::preserves_rail::sequence(values),
    ])
}

fn selected_output_values(value: &IoValue, node: &str, port: &str) -> Result<Vec<IoValue>> {
    let fields = simple_record(value, "job-port-output-v2", 3)?;
    if required_string(&fields[0], "selected output instance")? != node || required_string(&fields[1], "selected output port")? != port {
        return Err(MoltenError::invalid_harness("selected output handle mismatch"));
    }
    let values = sequence_items(&fields[2], "selected output values")?;
    for value in &values { required_u64_value(value, "selected output u64")?; }
    Ok(values)
}

fn validate_selected_outputs(node: &JobNode, values: &[IoValue]) -> Result<()> {
    if values.len() != node.output_ports.len() {
        return Err(MoltenError::invalid_harness("selected cached output signature mismatch"));
    }
    let mut count = 0;
    for (port, value) in node.output_ports.iter().zip(values) {
        let items = selected_output_values(value, &node.id, port)?;
        count = checked_count_sum(count, items.len(), MAX_JOB_STAGE_VALUES, "selected total output values")?;
        if port == "done" && !items.is_empty() {
            return Err(MoltenError::invalid_harness("selected control output must be Unit"));
        }
    }
    Ok(())
}

fn execute_selected_stage(node: &JobNode, inputs: &[IoValue]) -> Result<Vec<IoValue>> {
    let (operation, payload) = selected_operation(node)?;
    ensure_count_at_most(inputs.len(), MAX_JOB_STAGE_VALUES, "selected stage inputs")?;
    let mut integers = Vec::with_capacity(inputs.len());
    for input in inputs { integers.push(required_u64_value(input, "selected input u64")?); }
    let mut outputs = Vec::with_capacity(node.output_ports.len());
    match operation.as_str() {
        "source-u64" => {
            let fields = payload.collect_sequence().ok_or_else(|| MoltenError::invalid_harness("selected source requires sequence"))?;
            let mut values = Vec::with_capacity(fields.len());
            for value in fields.iter() { values.push(crate::preserves_rail::value_to_iovalue(value)); }
            outputs.push(selected_port_output(&node.id, "out", values));
        }
        "identity-u64" | "concat-u64" => outputs.push(selected_port_output(&node.id, "out", inputs.to_vec())),
        "drop-u64" => outputs.push(selected_port_output(&node.id, "out", Vec::new())),
        "partition-even-u64" => {
            let left_count = integers.iter().filter(|value| *value % 2 == 0).count();
            let mut left = Vec::with_capacity(left_count);
            let mut right = Vec::with_capacity(inputs.len() - left_count);
            for (input, value) in inputs.iter().zip(&integers) {
                if value % 2 == 0 { left.push(input.clone()); } else { right.push(input.clone()); }
            }
            outputs.push(selected_port_output(&node.id, "left", left));
            outputs.push(selected_port_output(&node.id, "right", right));
        }
        "sum-u64" => {
            let mut sum = 0u64;
            for value in integers { sum = sum.checked_add(value).ok_or_else(|| MoltenError::invalid_harness("selected sum overflow"))?; }
            outputs.push(selected_port_output(&node.id, "out", vec![crate::preserves_rail::u64_value(sum)]));
        }
        _ => return Err(MoltenError::invalid_harness("unsupported selected implementation")),
    }
    outputs.push(selected_port_output(&node.id, "done", Vec::new()));
    validate_selected_outputs(node, &outputs)?;
    Ok(outputs)
}

fn gather_selected_inputs(
    node: &JobNode, edges: &[JobEdge], outputs: &[Option<Vec<IoValue>>], index: &OrderedMap<String, usize>,
) -> Result<Vec<IoValue>> {
    let mut incoming: Vec<_> = edges.iter().filter(|edge| edge.to_node == node.id).collect();
    incoming.sort_by(|left, right| {
        (&left.to_port, &left.from_node, &left.from_port).cmp(&(&right.to_port, &right.from_node, &right.from_port))
    });
    let mut values = Vec::new();
    for edge in incoming {
        let from = indexed_stage_outputs(outputs, index, &edge.from_node)?;
        let mut selected = None;
        for output in from {
            let fields = simple_record(output, "job-port-output-v2", 3)?;
            if required_string(&fields[1], "selected output port")? == edge.from_port {
                if selected.is_some() { return Err(MoltenError::invalid_harness("duplicate routed output")); }
                selected = Some(selected_output_values(output, &edge.from_node, &edge.from_port)?);
            }
        }
        let selected = selected.ok_or_else(|| MoltenError::invalid_harness("selected output is unavailable"))?;
        if edge.to_port == "after" {
            if !selected.is_empty() { return Err(MoltenError::invalid_harness("control dependency carried data")); }
        } else { extend_cloned_bounded(&mut values, &selected, MAX_JOB_STAGE_VALUES, "selected routed fan-in")?; }
    }
    Ok(values)
}

fn validate_selected_current(dag: &JobDag) -> Result<()> {
    if dag.version != "v1" && dag.version != SELECTED_SEQUENCE_PROFILE {
        return Err(MoltenError::invalid_harness("unsupported job execution/planning profile"));
    }
    if (dag.version == SELECTED_SEQUENCE_PROFILE || dag.value.collect_simple_record("job-dag-v2", Some(10)).is_some())
        && parse_job_dag_value(&dag.value)? != *dag {
        return Err(MoltenError::invalid_harness("selected graph differs from admitted canonical graph"));
    }
    Ok(())
}

fn count_selected_final_values(dag: &JobDag, values: &[IoValue], count: &mut usize) -> Result<()> {
    if dag.version == SELECTED_SEQUENCE_PROFILE {
        for value in values {
            let fields = simple_record(value, "job-port-output-v2", 3)?;
            let items = required_sequence(&fields[2], "selected final output")?;
            *count = checked_count_sum(*count, items.len(), MAX_JOB_STAGE_VALUES, "selected final values")?;
        }
    }
    Ok(())
}

fn validate_selected_output_roots(dag: &JobDag, roots: &[String]) -> Result<()> {
    if dag.version == SELECTED_SEQUENCE_PROFILE && roots.iter().collect::<OrderedSet<_>>().len() != roots.len() {
        return Err(MoltenError::invalid_harness("duplicate selected output-request roots"));
    }
    Ok(())
}
