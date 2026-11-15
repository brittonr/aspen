#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct JobGraphExport {
    profile: String,
    instances: Vec<JobGraphInstance>,
    connections: Vec<JobGraphConnection>,
    outputs: Vec<String>,
    origins: Vec<JobGraphOrigin>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct JobGraphInstance {
    id: String,
    operation: String,
    values: Vec<u64>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct JobGraphConnection {
    from: JobGraphHandle,
    to: JobGraphHandle,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct JobGraphHandle {
    instance: String,
    port: String,
}

fn parse_graph_export(export: &str) -> Result<JobGraphExport> {
    ensure_count_at_most(export.len(), MAX_JOB_AUTHORING_BYTES, "job graph export bytes")?;
    let mut dto: JobGraphExport = serde_json::from_str(export)
        .map_err(|_| MoltenError::invalid_harness("captured graph export violates closed DTO; origin unavailable before decoding"))?;
    if dto.profile != SELECTED_SEQUENCE_PROFILE {
        return Err(MoltenError::invalid_harness("unsupported captured JobDag profile"));
    }
    ensure_count_at_most(dto.instances.len(), MAX_JOB_NODES, "expanded graph instances")?;
    ensure_count_at_most(dto.connections.len(), MAX_JOB_EDGES, "expanded graph connections")?;
    ensure_count_at_most(dto.outputs.len(), MAX_JOB_ROOTS, "expanded graph outputs")?;
    ensure_count_at_most(dto.origins.len(), MAX_JOB_NODES, "expanded graph origins")?;
    dto.instances.sort_by(|left, right| left.id.cmp(&right.id));
    dto.connections.sort_by(|left, right| {
        (&left.to.instance, &left.to.port, &left.from.instance, &left.from.port)
            .cmp(&(&right.to.instance, &right.to.port, &right.from.instance, &right.from.port))
    });
    for root in &dto.outputs { ensure_count_at_most(root.len(), 128, "captured output identifier")?; }
    Ok(dto)
}
