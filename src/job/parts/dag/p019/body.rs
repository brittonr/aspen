
fn parse_edge_sequence(value: &Value<IoValue>) -> Result<Vec<JobEdge>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = simple_record(&value, "edges", 1)?;
    let items = required_sequence(&record[0], "job edges")?;
    ensure_count_at_most(items.len(), MAX_JOB_EDGES, "job edges")?;
    let mut edges = Vec::with_capacity(items.len());
    for item in items.iter() {
        push_bounded(
            &mut edges,
            parse_job_edge_value(&crate::preserves_rail::value_to_iovalue(item))?,
            MAX_JOB_EDGES,
            "job edges",
        )?;
    }
    Ok(edges)
}

fn parse_job_edge_value(value: &IoValue) -> Result<JobEdge> {
    let fields = value
        .collect_simple_record("job-edge-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-edge-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::JOB_DAG_EDGE_SCHEMA, "job edge")?;
    let from = crate::preserves_rail::value_to_iovalue(&fields[1]);
    let from_fields = simple_record(&from, "from", 2)?;
    let to = crate::preserves_rail::value_to_iovalue(&fields[2]);
    let to_fields = simple_record(&to, "to", 2)?;
    let partitioning = record_string(&fields[4], "partitioning")?;
    let materialization = record_string(&fields[5], "materialization")?;
    validate_partitioning(&partitioning)?;
    validate_materialization(&materialization)?;
    Ok(JobEdge {
        from_node: required_string(&from_fields[0], "edge from node")?,
        from_port: required_string(&from_fields[1], "edge from port")?,
        to_node: required_string(&to_fields[0], "edge to node")?,
        to_port: required_string(&to_fields[1], "edge to port")?,
        schema_ref: record_optional_ref(&fields[3], "schema")?,
        partitioning,
        materialization,
    })
}

fn validate_topology(nodes: &[JobNode], edges: &[JobEdge]) -> Result<()> {
    execution_order(nodes, edges).map(|_| ())
}

fn execution_order(nodes: &[JobNode], edges: &[JobEdge]) -> Result<Vec<String>> {
    Ok(trellis_execution_plan(nodes, edges)?.order_ids)
}

fn trellis_execution_plan(nodes: &[JobNode], edges: &[JobEdge]) -> Result<TrellisExecutionPlan> {
    let mapping = plan_mapping(nodes, edges)?;
    if mapping.node_ids.len().checked_add(mapping.edges.len()).is_none() {
        return Err(MoltenError::invalid_harness("job dag trellis mapping exceeds topo-sort size precondition"));
    }
    let order_ids = plan_order_ids(&mapping.edges, &mapping.node_ids)?;
    let dependency_indices = plan_dependency_indices(&mapping.edges, &mapping.node_ids)?;
    Ok(TrellisExecutionPlan {
        order_ids,
        node_index: mapping.node_index,
        dependency_indices,
    })
}

fn plan_mapping(nodes: &[JobNode], edges: &[JobEdge]) -> Result<PlanMapping> {
    ensure_count_at_most(nodes.len(), MAX_JOB_NODES, "trellis nodes")?;
    ensure_count_at_most(edges.len(), MAX_JOB_EDGES, "trellis edges")?;
    let mut node_ids = Vec::with_capacity(nodes.len());
    for node in nodes {
        push_bounded(&mut node_ids, node.id.clone(), MAX_JOB_NODES, "trellis node ids")?;
    }
    node_ids.sort();
    node_ids.dedup();
    if node_ids.len() != nodes.len() {
        return Err(MoltenError::invalid_harness("job dag has duplicate node ids before trellis mapping"));
    }
    let mut node_index = OrderedMap::new();
    for (index, node) in node_ids.iter().enumerate() {
        insert_bounded(&mut node_index, node.clone(), index, MAX_JOB_NODES, "trellis node index")?;
    }
    let mut mapped_edges = Vec::with_capacity(edges.len());
    for edge in edges {
        let from = *node_index.get(&edge.from_node).ok_or_else(|| {
            MoltenError::invalid_harness(format!("trellis edge from unknown node {}", edge.from_node))
        })?;
        let to = *node_index
            .get(&edge.to_node)
            .ok_or_else(|| MoltenError::invalid_harness(format!("trellis edge to unknown node {}", edge.to_node)))?;
        push_bounded(&mut mapped_edges, (from, to), MAX_JOB_EDGES, "trellis edges")?;
    }
    mapped_edges.sort();
    Ok(PlanMapping {
        node_ids,
        node_index,
        edges: mapped_edges,
    })
}

fn plan_order_ids(edges: &[(usize, usize)], node_ids: &[String]) -> Result<Vec<String>> {
    let Some(order_indices) = trellis::topo_sort::topo_sort(edges, node_ids.len()) else {
        return Err(MoltenError::invalid_harness("trellis topo_sort rejected cyclic job dag"));
    };
    if !trellis::topo_sort::is_topo_order(edges, node_ids.len(), &order_indices) {
        return Err(MoltenError::invalid_harness("trellis topo_sort produced invalid job order"));
    }
    let mut order_ids = Vec::with_capacity(order_indices.len());
    for index in &order_indices {
        let node_id = node_ids
            .get(*index)
            .ok_or_else(|| MoltenError::invalid_harness(format!("trellis topo index {index} outside node set")))?;
        push_bounded(&mut order_ids, node_id.clone(), MAX_JOB_NODES, "trellis order ids")?;
    }
    Ok(order_ids)
}

fn plan_dependency_indices(edges: &[(usize, usize)], node_ids: &[String]) -> Result<OrderedMap<String, Vec<u64>>> {
    let incoming_counts = trellis_incoming_counts(edges, node_ids.len())?;
    let mut dependency_indices = OrderedMap::new();
    for (index, node_id) in node_ids.iter().enumerate() {
        insert_bounded(
            &mut dependency_indices,
            node_id.clone(),
            Vec::with_capacity(incoming_counts[index]),
            MAX_JOB_NODES,
            "trellis dependency index",
        )?;
    }
    for (from, to) in edges {
        let to_node = node_ids
            .get(*to)
            .ok_or_else(|| MoltenError::invalid_harness(format!("trellis dependency index {to} outside node set")))?;
        let dependency_values = dependency_indices
            .get_mut(to_node)
            .ok_or_else(|| MoltenError::invalid_harness(format!("dependency vector missing for {to_node}")))?;
        push_bounded(
            dependency_values,
            usize_to_u64(*from, "trellis dependency index")?,
            MAX_JOB_EDGES,
            "trellis dependency refs",
        )?;
    }
    for deps in dependency_indices.values_mut() {
        deps.sort();
        deps.dedup();
    }
    Ok(dependency_indices)
}

fn trellis_incoming_counts(trellis_edges: &[(usize, usize)], node_count: usize) -> Result<Vec<usize>> {
    ensure_count_at_most(node_count, MAX_JOB_NODES, "trellis incoming nodes")?;
    let mut counts = vec![0usize; node_count];
    for (_, to) in trellis_edges {
        let count = counts
            .get_mut(*to)
            .ok_or_else(|| MoltenError::invalid_harness(format!("trellis edge target {to} outside node set")))?;
        *count = count
            .checked_add(1)
            .ok_or_else(|| MoltenError::invalid_harness("trellis incoming edge count overflow"))?;
        ensure_count_at_most(*count, MAX_JOB_EDGES, "trellis incoming edges")?;
    }
    Ok(counts)
}

fn find_job_node<'a>(nodes: &'a [JobNode], node_id: &str) -> Result<&'a JobNode> {
    for node in nodes {
        if node.id == node_id {
            return Ok(node);
        }
    }
    Err(MoltenError::invalid_harness(format!("job node {node_id} missing from node set")))
}

fn gather_inputs(
    node: &JobNode,
    edges: &[JobEdge],
    outputs_by_index: &[Option<Vec<IoValue>>],
    node_index: &OrderedMap<String, usize>,
) -> Result<Vec<IoValue>> {
    ensure_count_at_most(edges.len(), MAX_JOB_EDGES, "job input edges")?;
    let mut incoming = Vec::with_capacity(edges.len());
    for edge in edges {
        if edge.to_node == node.id {
            push_bounded(&mut incoming, edge, MAX_JOB_EDGES, "job incoming edges")?;
        }
    }
    incoming.sort_by(|left, right| {
        (&left.to_port, &left.from_node, &left.from_port).cmp(&(&right.to_port, &right.from_node, &right.from_port))
    });
    let mut value_count = 0usize;
    for edge in &incoming {
        let from_values = indexed_stage_outputs(outputs_by_index, node_index, &edge.from_node)?;
        value_count = checked_count_sum(value_count, from_values.len(), MAX_JOB_STAGE_VALUES, "job input values")?;
    }
    let mut values = Vec::with_capacity(value_count);
    for edge in incoming {
        let from_values = indexed_stage_outputs(outputs_by_index, node_index, &edge.from_node)?;
        extend_cloned_bounded(&mut values, from_values, MAX_JOB_STAGE_VALUES, "job input values")?;
    }
    Ok(values)
}

fn indexed_stage_outputs<'a>(
    outputs_by_index: &'a [Option<Vec<IoValue>>],
    node_index: &OrderedMap<String, usize>,
    node_id: &str,
) -> Result<&'a Vec<IoValue>> {
    let from_index = *node_index
        .get(node_id)
        .ok_or_else(|| MoltenError::invalid_harness(format!("job edge input from {node_id} lacks node index")))?;
    outputs_by_index
        .get(from_index)
        .and_then(Option::as_ref)
        .ok_or_else(|| MoltenError::invalid_harness(format!("job edge input from {node_id} not available")))
}

fn sink_nodes(dag: &JobDag) -> Result<Vec<String>> {
    ensure_count_at_most(dag.nodes.len(), MAX_JOB_NODES, "job sink nodes")?;
    ensure_count_at_most(dag.edges.len(), MAX_JOB_EDGES, "job sink edges")?;
    let mut from = OrderedSet::new();
    for edge in &dag.edges {
        from.insert(edge.from_node.clone());
    }
    let mut sinks = Vec::with_capacity(dag.nodes.len());
    for node in &dag.nodes {
        if !from.contains(&node.id) {
            push_bounded(&mut sinks, node.id.clone(), MAX_JOB_NODES, "job sink nodes")?;
        }
    }
    if sinks.is_empty() {
        for node in &dag.nodes {
            push_bounded(&mut sinks, node.id.clone(), MAX_JOB_NODES, "job sink nodes")?;
        }
    }
    sinks.sort();
    Ok(sinks)
}

fn refs_for_values(values: &[IoValue]) -> Result<Vec<String>> {
    ensure_count_at_most(values.len(), MAX_JOB_STAGE_VALUES, "job values to hash")?;
    let mut refs = Vec::with_capacity(values.len());
    for value in values {
        push_bounded(&mut refs, crate::preserves_rail::canonical_hash(value)?, MAX_JOB_REFS, "job value refs")?;
    }
    Ok(refs)
}

fn parse_cached_stage_output(value: &IoValue) -> Result<Vec<IoValue>> {
    if let Some(items) = value.collect_sequence() {
        ensure_count_at_most(items.len(), MAX_JOB_STAGE_VALUES, "cached job stage output")?;
        let mut values = Vec::with_capacity(items.len());
        for item in items.iter() {
            push_bounded(
                &mut values,
                crate::preserves_rail::value_to_iovalue(item),
                MAX_JOB_STAGE_VALUES,
                "cached job stage output",
            )?;
        }
        Ok(values)
    } else {
        Err(MoltenError::invalid_harness("cached job stage output must be a sequence"))
    }
}

fn combined_policy_refs(dag: &JobDag, request: &JobOutputRequest, node: Option<&JobNode>) -> Vec<String> {
    let node_policy_count = node.map_or(0, |node| node.policy_refs.len());
    let capacity = dag
        .policy_refs
        .len()
        .saturating_add(request.policy_refs.len())
        .saturating_add(node_policy_count)
        .min(MAX_JOB_REFS);
    let mut refs = Vec::with_capacity(capacity);
    refs.extend(dag.policy_refs.iter().cloned());
    refs.extend(request.policy_refs.iter().cloned());
    if let Some(node) = node {
        refs.extend(node.policy_refs.iter().cloned());
    }
    sorted_unique(&refs)
}

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_string(&record[0], label)
}

fn record_ref(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_ref(&record[0], label)
}

fn record_optional_ref(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_optional_ref_value(&record[0])
}
