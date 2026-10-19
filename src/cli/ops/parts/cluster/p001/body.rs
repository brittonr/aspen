
fn prepare_init(plan: &molten::cluster::Plan, force: bool) -> molten::error::Result<()> {
    if force {
        for node in &plan.nodes {
            if node.state_root.exists() {
                std::fs::remove_dir_all(&node.state_root).map_err(molten::error::Failure::from)?;
            }
        }
        return Ok(());
    }
    let manifest_path = molten::cluster::manifest_path(&plan.state_root);
    if manifest_path.exists() {
        return Err(molten::error::Failure::invalid_harness(format!(
            "cluster init denied: manifest already exists at {}; pass --force to overwrite the cluster manifest",
            manifest_path.display()
        )));
    }
    for node in &plan.nodes {
        let state = molten::node_daemon::inspect_node_lifecycle_state(&node.state_root);
        if state != molten::node_daemon::NodeLifecycleState::Empty {
            return Err(molten::error::Failure::invalid_harness(format!(
                "cluster init denied: node {} already has {state:?} lifecycle state at {}; pass --force to reset that node root",
                node.node_id,
                node.state_root.display()
            )));
        }
    }
    Ok(())
}

fn current_running_status(state_root: &std::path::Path) -> molten::error::Result<Option<molten::node_daemon::Status>> {
    match molten::node_daemon::status_local(&molten::node_daemon::StatusInput { state_root }) {
        Ok(status) if status.status == RUNNING_STATUS => Ok(Some(status)),
        Ok(_) | Err(_) => Ok(None),
    }
}

fn read_plan(state_root: &std::path::Path) -> molten::error::Result<molten::cluster::Plan> {
    use std::io::Read as _;

    let path = molten::cluster::manifest_path(state_root);
    let mut source = String::new();
    std::fs::File::open(&path)
        .map_err(molten::error::Failure::from)?
        .take(molten::cluster::MAX_CLUSTER_MANIFEST_BYTES as u64 + 1)
        .read_to_string(&mut source)
        .map_err(molten::error::Failure::from)?;
    let nodes = molten::cluster::parse_manifest(&source)?;
    molten::cluster::plan(state_root, &nodes)
}

fn write_manifest(plan: &molten::cluster::Plan) -> molten::error::Result<()> {
    std::fs::create_dir_all(&plan.state_root).map_err(molten::error::Failure::from)?;
    let path = molten::cluster::manifest_path(&plan.state_root);
    std::fs::write(path, molten::cluster::render_manifest(plan)).map_err(molten::error::Failure::from)
}
