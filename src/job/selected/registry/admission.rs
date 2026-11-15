// r[impl molten.job_dag.selected_sequence]
fn validate_selected_graph(nodes: &[JobNode], edges: &[JobEdge], roots: &[String]) -> Result<()> {
    let mut operations = Vec::with_capacity(nodes.len());
    for node in nodes {
        ensure_count_at_most(node.id.len(), 128, "selected instance identifier bytes")?;
        let (operation, payload) = selected_operation(node)?;
        let signature = simple_record(&node.config, "selected-stage-v2", 3)?;
        if node.kind != selected_kind(&operation)? || node.stage_artifact_ref.is_some() || required_string(&signature[2], "selected signature")? != selected_stage_signature(&operation)? {
            return Err(MoltenError::invalid_harness("selected stage implementation/signature mismatch"));
        }
        let (inputs, outputs) = selected_ports(&operation);
        if node.input_ports != inputs || node.output_ports != outputs {
            return Err(MoltenError::invalid_harness("selected stage port direction/signature mismatch"));
        }
        if operation == "source-u64" {
            let items = payload.collect_sequence().ok_or_else(|| MoltenError::invalid_harness("selected source requires u64 sequence"))?;
            ensure_count_at_most(items.len(), MAX_JOB_STAGE_VALUES, "selected source values")?;
            for item in items.iter() { required_u64_value(&crate::preserves_rail::value_to_iovalue(item), "selected source u64")?; }
        } else if payload != crate::preserves_rail::sequence(Vec::new()) {
            return Err(MoltenError::invalid_harness("selected operation has unsupported parameters"));
        }
        operations.push(operation);
    }
    let data_schema = selected_sequence_schema()?;
    let control_schema = selected_control_schema()?;
    let mut connections = OrderedSet::new();
    let mut input_counts = OrderedMap::new();
    for edge in edges {
        let from = find_job_node(nodes, &edge.from_node)?;
        let to = find_job_node(nodes, &edge.to_node)?;
        if !from.output_ports.contains(&edge.from_port) || !to.input_ports.contains(&edge.to_port) {
            return Err(MoltenError::invalid_harness("unknown or reversed selected port"));
        }
        let is_control = edge.from_port == "done" && edge.to_port == "after";
        if (edge.from_port == "done" || edge.to_port == "after") && !is_control {
            return Err(MoltenError::invalid_harness("selected control/data direction mismatch"));
        }
        let schema = if is_control { &control_schema } else { &data_schema };
        if edge.schema_ref.as_ref() != Some(schema) || edge.partitioning != "single" || edge.materialization != "stream" {
            return Err(MoltenError::invalid_harness("incompatible selected schema or edge profile"));
        }
        if !connections.insert((&edge.from_node, &edge.from_port, &edge.to_node, &edge.to_port)) {
            return Err(MoltenError::invalid_harness("duplicate selected connection"));
        }
        *input_counts.entry((edge.to_node.as_str(), edge.to_port.as_str())).or_insert(0usize) += 1;
    }
    for (node, operation) in nodes.iter().zip(&operations) {
        let count = input_counts.get(&(node.id.as_str(), "in")).copied().unwrap_or(0);
        if operation != "source-u64" && count == 0 {
            return Err(MoltenError::invalid_harness("missing required selected input"));
        }
        if matches!(operation.as_str(), "identity-u64" | "partition-even-u64" | "drop-u64") && count > 1 {
            return Err(MoltenError::invalid_harness("excess selected single-input connections"));
        }
    }
    let mut unique_roots = OrderedSet::new();
    for root in roots {
        if !unique_roots.insert(root) { return Err(MoltenError::invalid_harness("duplicate selected output root")); }
    }
    Ok(())
}

fn validate_selected_port_metadata(value: &Value<IoValue>) -> Result<()> {
    for node in record_sequence_values(value, "nodes")? {
        let fields = simple_record(&node, "job-node-v1", 11)?;
        for (index, direction) in [(4, "inputs"), (5, "outputs")] {
            for port in record_sequence_values(&fields[index], direction)? {
                let declaration = simple_record(&port, "port", 2)?;
                if declaration[1].collect_simple_record("none", Some(0)).is_none() {
                    return Err(MoltenError::invalid_harness("selected port schemas are registry-owned; metadata must be none"));
                }
            }
        }
    }
    Ok(())
}