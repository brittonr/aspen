/// Independent bounded evaluator: plain integer maps, no executor/routing/cache helpers.
fn interpret(export: &serde_json::Value) -> std::result::Result<OrderedMap<(String, String), Vec<u64>>, &'static str> {
    let dto: JobGraphExport = serde_json::from_value(export.clone()).map_err(|_| "dto")?;
    if dto.profile != SELECTED_SEQUENCE_PROFILE || dto.instances.is_empty() || dto.instances.len() > 256 || dto.connections.len() > 4096 {
        return Err("profile/budget");
    }
    let mut nodes = OrderedMap::new();
    for node in &dto.instances {
        if nodes.insert(node.id.as_str(), node).is_some() { return Err("collision"); }
        if !matches!(node.operation.as_str(), "source-u64" | "identity-u64" | "concat-u64" | "partition-even-u64" | "drop-u64" | "sum-u64") {
            return Err("operation");
        }
        if node.values.len() > 4096 || (node.operation != "source-u64" && !node.values.is_empty()) { return Err("values"); }
    }
    let mut seen = OrderedSet::new();
    for connection in &dto.connections {
        let from = nodes.get(connection.from.instance.as_str()).ok_or("dangling")?;
        let to = nodes.get(connection.to.instance.as_str()).ok_or("dangling")?;
        let ports: &[&str] = if from.operation == "partition-even-u64" { &["left", "right", "done"] } else { &["out", "done"] };
        if !ports.contains(&connection.from.port.as_str()) { return Err("output"); }
        if connection.to.port != "after" && !(connection.to.port == "in" && to.operation != "source-u64") { return Err("input"); }
        if (connection.from.port == "done") != (connection.to.port == "after") { return Err("direction"); }
        if !seen.insert((&connection.from.instance, &connection.from.port, &connection.to.instance, &connection.to.port)) { return Err("duplicate"); }
    }
    for node in &dto.instances {
        let count = dto.connections.iter().filter(|edge| edge.to.instance == node.id && edge.to.port == "in").count();
        if node.operation != "source-u64" && count == 0 { return Err("required"); }
        if matches!(node.operation.as_str(), "identity-u64" | "partition-even-u64" | "drop-u64") && count > 1 { return Err("multiplicity"); }
    }
    let mut outputs: OrderedMap<(String, String), Vec<u64>> = OrderedMap::new();
    let mut completed = OrderedSet::new();
    let max_outputs = nodes.len() * 3;
    while completed.len() < nodes.len() {
        if outputs.len() > max_outputs - 3 { return Err("output budget"); }
        let ready = nodes.values().find(|node| !completed.contains(node.id.as_str()) && dto.connections.iter()
            .filter(|edge| edge.to.instance == node.id).all(|edge| completed.contains(edge.from.instance.as_str()))).ok_or("cycle")?;
        let mut edges: Vec<_> = dto.connections.iter().filter(|edge| edge.to.instance == ready.id).collect();
        edges.sort_by_key(|edge| (&edge.to.port, &edge.from.instance, &edge.from.port));
        let mut data = Vec::new();
        for edge in edges {
            if edge.to.port == "after" { continue; }
            let values = outputs.get(&(edge.from.instance.clone(), edge.from.port.clone())).ok_or("missing output")?;
            if data.len() + values.len() > 4096 { return Err("fan-in"); }
            data.extend_from_slice(values);
        }
        match ready.operation.as_str() {
            "source-u64" => { outputs.insert((ready.id.clone(), "out".into()), ready.values.clone()); }
            "identity-u64" | "concat-u64" => { outputs.insert((ready.id.clone(), "out".into()), data); }
            "drop-u64" => { outputs.insert((ready.id.clone(), "out".into()), Vec::new()); }
            "partition-even-u64" => {
                let (left, right): (Vec<u64>, Vec<u64>) = data.into_iter().partition(|value| value % 2 == 0);
                outputs.insert((ready.id.clone(), "left".into()), left);
                outputs.insert((ready.id.clone(), "right".into()), right);
            }
            "sum-u64" => {
                let sum = data.iter().try_fold(0u64, |sum, value| sum.checked_add(*value)).ok_or("overflow")?;
                outputs.insert((ready.id.clone(), "out".into()), vec![sum]);
            }
            _ => return Err("operation"),
        }
        outputs.insert((ready.id.clone(), "done".into()), Vec::new());
        completed.insert(ready.id.as_str());
    }
    Ok(outputs)
}