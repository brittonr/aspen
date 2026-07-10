const RUNNING_STATUS: &str = "running";

#[derive(Debug, clap::Subcommand)]
pub(crate) enum ClusterCommand {
    Init(ClusterInit),
    Start(ClusterRoot),
    Status(ClusterRoot),
    Stop(ClusterRoot),
}

#[derive(Debug, clap::Args)]
pub(crate) struct ClusterInit {
    #[arg(long)]
    state_root: std::path::PathBuf,
    #[arg(long = "node")]
    nodes: Vec<String>,
    #[arg(long)]
    force: bool,
}

#[derive(Debug, clap::Args)]
pub(crate) struct ClusterRoot {
    #[arg(long)]
    state_root: std::path::PathBuf,
}

pub(crate) fn run(command: ClusterCommand) -> molten::error::Result<()> {
    match command {
        ClusterCommand::Init(input) => init(input),
        ClusterCommand::Start(input) => start(input),
        ClusterCommand::Status(input) => status(input),
        ClusterCommand::Stop(input) => stop(input),
    }
}

fn init(input: ClusterInit) -> molten::error::Result<()> {
    let plan = molten::cluster::plan_cluster(&input.state_root, &input.nodes)?;
    prepare_cluster_init(&plan, input.force)?;
    let mut initialized_count = 0usize;
    for node in &plan.nodes {
        let init = molten::node_daemon::init_local(&molten::node_daemon::InitInput {
            state_root: &node.state_root,
            node_id: &node.node_id,
        })?;
        initialized_count += 1;
        println!(
            "cluster node init node={} state_root={} config={} identity_receipt={}",
            node.node_id,
            node.state_root.display(),
            init.config_ref,
            init.identity_receipt_ref
        );
    }
    write_cluster_manifest(&plan)?;
    println!("cluster init nodes={} state_root={}", initialized_count, plan.state_root.display());
    Ok(())
}

fn start(input: ClusterRoot) -> molten::error::Result<()> {
    let plan = read_cluster_plan(&input.state_root)?;
    for node in &plan.nodes {
        if let Some(status) = current_running_status(&node.state_root)? {
            println!(
                "cluster node start node={} state_root={} already_running=yes health={} control_receipt={}",
                node.node_id,
                node.state_root.display(),
                status.health_ref,
                status.control_receipt_ref
            );
            continue;
        }
        let run = molten::node_daemon::run_local(&molten::node_daemon::RunInput {
            state_root: &node.state_root,
        })?;
        println!(
            "cluster node start node={} state_root={} startup={} adapters={}",
            node.node_id,
            node.state_root.display(),
            run.startup_ref,
            run.adapter_receipt_refs.len()
        );
    }
    println!("cluster start nodes={} state_root={}", plan.nodes.len(), plan.state_root.display());
    Ok(())
}

fn status(input: ClusterRoot) -> molten::error::Result<()> {
    let plan = read_cluster_plan(&input.state_root)?;
    for node in &plan.nodes {
        let status = molten::node_daemon::status_local(&molten::node_daemon::StatusInput {
            state_root: &node.state_root,
        })?;
        println!(
            "cluster node status node={} state_root={} status={} health={} control_receipt={}",
            node.node_id,
            node.state_root.display(),
            status.status,
            status.health_ref,
            status.control_receipt_ref
        );
    }
    println!("cluster status nodes={} state_root={}", plan.nodes.len(), plan.state_root.display());
    Ok(())
}

fn stop(input: ClusterRoot) -> molten::error::Result<()> {
    let plan = read_cluster_plan(&input.state_root)?;
    for node in plan.nodes.iter().rev() {
        let stop = molten::node_daemon::stop_local(&molten::node_daemon::StopInput {
            state_root: &node.state_root,
        })?;
        println!(
            "cluster node stop node={} state_root={} shutdown={} control_receipt={}",
            node.node_id,
            node.state_root.display(),
            stop.shutdown_ref,
            stop.control_receipt_ref
        );
    }
    println!("cluster stop nodes={} state_root={}", plan.nodes.len(), plan.state_root.display());
    Ok(())
}

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
