// Lowering of an admitted Choregraph v3 projection onto a `selected-sequence-v2` JobDag through the
// normal parser; included by `construction.rs`. Lowering never executes a stage.

const JOB_CONSTRUCTION_NODE_PREFIX: &str = "c";
const JOB_CONSTRUCTION_NODE_LOCAL: &str = "node";

fn lower_construction_projection(
    projection: &choregraph_nickel::construction::model::projection::LatticeV3,
    outputs: &[&[&str]],
) -> Result<JobDag> {
    if !projection.effects.is_empty() {
        return Err(MoltenError::invalid_harness("graph construction declared effects; selected-sequence-v2 is pure"));
    }
    ensure_count_at_most(projection.nodes.len(), MAX_JOB_NODES, "constructed graph nodes")?;
    checked_count_sum(projection.edges.len(), projection.control_edges.len(), MAX_JOB_EDGES, "constructed graph edges")?;
    ensure_count_at_most(outputs.len(), MAX_JOB_ROOTS, "constructed graph outputs")?;
    let mut node_ids = OrderedMap::new();
    let mut ports = OrderedMap::new();
    let mut nodes = Vec::with_capacity(projection.nodes.len());
    for node in &projection.nodes {
        let id = construction_node_name(node.id);
        let is_join = node.operation == JOB_CONSTRUCTION_SUM_JOIN;
        let operation = if is_join { "sum-u64" } else { node.operation.as_str() };
        nodes.push(selected_node_value(&id, operation, construction_payload(node)?)?);
        for port in &node.ports {
            let is_input = port.direction == choregraph_nickel::construction::model::interface::Direction::Input;
            let name = if is_join && is_input { "in" } else { port.name.as_str() };
            if ports.len() >= JOB_CONSTRUCTION_MAX_PORTS {
                return Err(MoltenError::invalid_harness("constructed graph ports exceed the admission ceiling"));
            }
            ports.insert(port.id, (id.clone(), name.to_owned()));
        }
        if node_ids.len() >= MAX_JOB_NODES {
            return Err(MoltenError::invalid_harness("constructed graph nodes exceed the runtime ceiling"));
        }
        node_ids.insert(node.id, id);
    }
    let sequence_schema = selected_sequence_schema()?;
    let control_schema = selected_control_schema()?;
    let mut edges = Vec::with_capacity(projection.edges.len() + projection.control_edges.len());
    for edge in &projection.edges {
        let (from_node, from_port) = construction_lookup(&ports, edge.source, "constructed data source")?;
        let (to_node, to_port) = construction_lookup(&ports, edge.target, "constructed data target")?;
        edges.push(job_edge_value(EdgeValueInput {
            from_node, from_port, to_node, to_port,
            schema_ref: Some(&sequence_schema), partitioning: "single", materialization: "stream",
        })?);
    }
    for edge in &projection.control_edges {
        let from_node = construction_lookup(&node_ids, edge.source, "constructed control source")?;
        let to_node = construction_lookup(&node_ids, edge.target, "constructed control target")?;
        edges.push(job_edge_value(EdgeValueInput {
            from_node, from_port: "done", to_node, to_port: "after",
            schema_ref: Some(&control_schema), partitioning: "single", materialization: "stream",
        })?);
    }
    let mut roots = Vec::with_capacity(outputs.len());
    for path in outputs {
        let node = construction_node(projection, path)?;
        roots.push(construction_lookup(&node_ids, node.id, "constructed output")?.clone());
    }
    let value = selected_dag_value(DagValueInput {
        nodes, edges, output_roots: &roots, schema_refs: &[],
        effect_manifest_refs: &[], policy_refs: &[], evidence_refs: &[],
    })?;
    parse_job_dag_value(&value)
}

fn construction_payload(node: &choregraph_nickel::construction::model::projection::ExpandedNode) -> Result<IoValue> {
    if node.operation != "source-u64" {
        if !node.configuration.is_empty() {
            return Err(MoltenError::invalid_harness("constructed non-source stage has configuration"));
        }
        return Ok(crate::preserves_rail::sequence(Vec::new()));
    }
    let Some(choregraph_nickel::construction::model::projection::ExpandedExpression::Literal {
        value: serde_json::Value::Array(values),
    }) = node.configuration.get(JOB_CONSTRUCTION_SOURCE_FIELD)
    else {
        return Err(MoltenError::invalid_harness("constructed source requires literal u64 values"));
    };
    ensure_count_at_most(values.len(), MAX_JOB_STAGE_VALUES, "constructed source values")?;
    let mut encoded = Vec::with_capacity(values.len());
    for value in values {
        let number = value
            .as_u64()
            .ok_or_else(|| MoltenError::invalid_harness("constructed source value is not an exact u64"))?;
        encoded.push(crate::preserves_rail::u64_value(number));
    }
    Ok(crate::preserves_rail::sequence(encoded))
}

fn construction_lookup<'a, T>(map: &'a OrderedMap<u64, T>, id: u64, label: &str) -> Result<&'a T> {
    map.get(&id).ok_or_else(|| MoltenError::invalid_harness(format!("{label} is unavailable")))
}

/// The Aspen node identity of the operation at a fragment key path from the root key.
pub fn constructed_node_id(job: &ConstructedNickelJob, path: &[&str]) -> Result<String> {
    Ok(construction_node_name(construction_node(&job.projection, path)?.id))
}

fn construction_node_name(id: u64) -> String {
    format!("{JOB_CONSTRUCTION_NODE_PREFIX}{id}")
}

fn construction_node<'a>(
    projection: &'a choregraph_nickel::construction::model::projection::LatticeV3,
    path: &[&str],
) -> Result<&'a choregraph_nickel::construction::model::projection::ExpandedNode> {
    let scoped = construction_scoped_id(path)?;
    projection
        .nodes
        .iter()
        .find(|node| node.scoped_id == scoped)
        .ok_or_else(|| MoltenError::invalid_harness("construction path does not name an operation"))
}

/// Choregraph's framed scoped node identity for an operation key path.
fn construction_scoped_id(path: &[&str]) -> Result<String> {
    if path.is_empty() {
        return Err(MoltenError::invalid_harness("construction output path is empty"));
    }
    let mut scoped = String::new();
    for segment in path.iter().copied().chain(std::iter::once(JOB_CONSTRUCTION_NODE_LOCAL)) {
        scoped.push_str(&segment.len().to_string());
        scoped.push(':');
        scoped.push_str(segment);
    }
    Ok(scoped)
}

struct ConstructionProvenance<'a> {
    binding: &'a choregraph_nickel::construction::association::CompilationBinding,
    evaluator_binding: &'a [u8],
    worker_ref: &'a str,
    registry_ref: &'a str,
    lowerer_ref: &'a str,
    dag: &'a JobDag,
    origins: &'a [choregraph_nickel::construction::model::projection::Origin],
}

fn construction_provenance(sources: ConstructionProvenance<'_>) -> Result<IoValue> {
    let origins = serde_json::to_string(sources.origins)
        .map_err(|error| MoltenError::invalid_harness(format!("construction origins: {error}")))?;
    Ok(crate::preserves_rail::record("job-construction-lowering-v1", vec![
        crate::preserves_rail::string("molten.job-construction-lowering.v1"),
        crate::preserves_rail::string(SELECTED_SEQUENCE_PROFILE),
        crate::preserves_rail::string(&sources.binding.association),
        crate::preserves_rail::string(crate::preserves_rail::content_ref_from_bytes(sources.evaluator_binding)),
        crate::preserves_rail::string(sources.worker_ref),
        crate::preserves_rail::string(sources.registry_ref),
        crate::preserves_rail::string(sources.lowerer_ref),
        crate::preserves_rail::string(&sources.dag.job_ref),
        crate::preserves_rail::string(origins),
        crate::preserves_rail::string(
            "Choregraph construction over nickel-eval captured evaluation; bindings are linkage, not execution authority",
        ),
    ]))
}
