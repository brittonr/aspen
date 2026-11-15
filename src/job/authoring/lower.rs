fn lower_job_graph_export(
    export: &str, source: &str, evaluator_ref: &str, limiter_ref: &str,
    modules: &OrderedMap<String, String>,
) -> Result<LoweredNickelJob> {
    let dto = parse_graph_export(export)?;
    let nodes = lower_graph_instances(&dto)?;
    let edges = lower_graph_connections(&dto)?;
    validate_graph_origins(&dto, modules)?;
    let value = selected_dag_value(DagValueInput {
        nodes, edges, output_roots: &dto.outputs, schema_refs: &[],
        effect_manifest_refs: &[], policy_refs: &[], evidence_refs: &[],
    })?;
    let dag = parse_job_dag_value(&value)?;
    let provenance = graph_lowering_provenance(LoweringSources { export, source, evaluator_ref, limiter_ref }, &dag, &dto.origins)?;
    Ok(LoweredNickelJob { dag, provenance, origins: dto.origins })
}

fn lower_graph_instances(dto: &JobGraphExport) -> Result<Vec<IoValue>> {
    let mut nodes = Vec::with_capacity(dto.instances.len());
    let mut instance_ids = OrderedSet::new();
    for instance in &dto.instances {
        ensure_count_at_most(instance.id.len(), 128, "captured instance identifier")?;
        if !instance_ids.insert(instance.id.as_str()) {
            return Err(MoltenError::invalid_harness(format!("captured instance collision: {}", instance.id)));
        }
        ensure_count_at_most(instance.values.len(), MAX_JOB_STAGE_VALUES, "expanded instance values")?;
        if instance.operation != "source-u64" && !instance.values.is_empty() {
            return Err(MoltenError::invalid_harness("non-source captured instance has source values"));
        }
        nodes.push(selected_node_value(&instance.id, &instance.operation,
            crate::preserves_rail::sequence(instance.values.iter().copied().map(crate::preserves_rail::u64_value).collect()))?);
    }
    Ok(nodes)
}

fn lower_graph_connections(dto: &JobGraphExport) -> Result<Vec<IoValue>> {
    let mut edges = Vec::with_capacity(dto.connections.len());
    for connection in &dto.connections {
        for item in [&connection.from.instance, &connection.from.port, &connection.to.instance, &connection.to.port] {
            ensure_count_at_most(item.len(), 128, "captured handle identifier")?;
        }
        let schema = if connection.from.port == "done" { selected_control_schema()? } else { selected_sequence_schema()? };
        edges.push(job_edge_value(EdgeValueInput {
            from_node: &connection.from.instance, from_port: &connection.from.port,
            to_node: &connection.to.instance, to_port: &connection.to.port,
            schema_ref: Some(&schema), partitioning: "single", materialization: "stream",
        })?);
    }
    Ok(edges)
}

fn validate_graph_origins(dto: &JobGraphExport, modules: &OrderedMap<String, String>) -> Result<()> {
    let instances: OrderedMap<_, _> = dto.instances.iter().map(|instance| (instance.id.as_str(), instance)).collect();
    let mut origin_ids = OrderedSet::new();
    for origin in &dto.origins {
        let instance = instances.get(origin.instance.as_str())
            .ok_or_else(|| MoltenError::invalid_harness("captured origin instance unavailable"))?;
        if !origin_ids.insert((origin.instance.as_str(), origin.port.as_deref())) {
            return Err(MoltenError::invalid_harness("captured origin has repeated instance/port"));
        }
        for item in [&origin.instance, &origin.template, &origin.module] {
            ensure_count_at_most(item.len(), 128, "captured origin identifier")?;
            validate_node_id(item)?;
        }
        if origin.module != "main" && !modules.contains_key(&origin.module) {
            return Err(MoltenError::invalid_harness("captured origin module was not captured"));
        }
        if let Some(port) = &origin.port {
            let (inputs, outputs) = selected_ports(&instance.operation);
            if !inputs.contains(port) && !outputs.contains(port) {
                return Err(MoltenError::invalid_harness("captured origin port is not declared"));
            }
        }
    }
    Ok(())
}

struct LoweringSources<'a> {
    export: &'a str,
    source: &'a str,
    evaluator_ref: &'a str,
    limiter_ref: &'a str,
}

fn graph_lowering_provenance(
    sources: LoweringSources<'_>, dag: &JobDag, origins: &[JobGraphOrigin],
) -> Result<IoValue> {
    let LoweringSources { export, source, evaluator_ref, limiter_ref } = sources;
    let lowerer_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("job-nickel-lowerer-v1", vec![
        crate::preserves_rail::string(concat!(include_str!("lower.rs"), include_str!("model.rs"), include_str!("inputs.rs"), include_str!("evaluate.rs"),
            include_str!("inputs/parameters.rs"), include_str!("inputs/literal.rs"),
            include_str!("../selected/registry.rs"), include_str!("../selected/registry/admission.rs"), include_str!("../selected/routing.rs"))),
    ]))?;
    let origins = serde_json::to_string(origins).map_err(|error| MoltenError::invalid_harness(error.to_string()))?;
    Ok(crate::preserves_rail::record("job-nickel-lowering-v1", vec![
        crate::preserves_rail::string("molten.job-nickel-lowering.v1"),
        crate::preserves_rail::string(SELECTED_SEQUENCE_PROFILE),
        crate::preserves_rail::string(crate::preserves_rail::canonical_hash(&crate::preserves_rail::string(source))?),
        crate::preserves_rail::string(crate::preserves_rail::canonical_hash(&crate::preserves_rail::string(export))?),
        crate::preserves_rail::string(evaluator_ref), crate::preserves_rail::string(limiter_ref),
        crate::preserves_rail::string(lowerer_ref), crate::preserves_rail::string(&dag.job_ref),
        crate::preserves_rail::string(origins),
        crate::preserves_rail::string("bounded captured authoring; origins are author declarations; source locations unavailable; not execution authority"),
    ]))
}
