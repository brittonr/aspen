
fn prepare_cluster_init(plan: &molten::cluster::ClusterPlan, force: bool) -> molten::error::Result<()> {
    if force {
        for node in &plan.nodes {
            if node.state_root.exists() {
                std::fs::remove_dir_all(&node.state_root).map_err(molten::error::MoltenError::from)?;
            }
        }
        return Ok(());
    }
    let manifest_path = molten::cluster::cluster_manifest_path(&plan.state_root);
    if manifest_path.exists() {
        return Err(molten::error::MoltenError::invalid_harness(format!(
            "cluster init denied: manifest already exists at {}; pass --force to overwrite the cluster manifest",
            manifest_path.display()
        )));
    }
    for node in &plan.nodes {
        let state = molten::node_daemon::inspect_node_lifecycle_state(&node.state_root);
        if state != molten::node_daemon::NodeLifecycleState::Empty {
            return Err(molten::error::MoltenError::invalid_harness(format!(
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

fn read_cluster_plan(state_root: &std::path::Path) -> molten::error::Result<molten::cluster::ClusterPlan> {
    let path = molten::cluster::cluster_manifest_path(state_root);
    let source = std::fs::read_to_string(&path).map_err(molten::error::MoltenError::from)?;
    let nodes = molten::cluster::parse_cluster_manifest(&source)?;
    molten::cluster::plan_cluster(state_root, &nodes)
}

fn write_cluster_manifest(plan: &molten::cluster::ClusterPlan) -> molten::error::Result<()> {
    std::fs::create_dir_all(&plan.state_root).map_err(molten::error::MoltenError::from)?;
    let path = molten::cluster::cluster_manifest_path(&plan.state_root);
    std::fs::write(path, molten::cluster::render_cluster_manifest(plan)).map_err(molten::error::MoltenError::from)
}
