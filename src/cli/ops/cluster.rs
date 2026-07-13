const RUNNING_STATUS: &str = "running";

#[derive(Debug, clap::Subcommand)]
pub(crate) enum ClusterCommand {
    Init(ClusterInit),
    Start(ClusterRoot),
    Status(ClusterRoot),
    Stop(ClusterRoot),
    HarnessRun(ClusterHarnessRun),
    HarnessVerify(ClusterHarnessVerify),
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

#[derive(Debug, clap::Args)]
pub(crate) struct ClusterHarnessRun {
    #[arg(long)]
    fixture: std::path::PathBuf,
    #[arg(long)]
    state_root: std::path::PathBuf,
    #[arg(long)]
    run_dir: std::path::PathBuf,
    #[arg(long)]
    node_binary: Option<std::path::PathBuf>,
    #[arg(long, default_value_t = molten::cluster_harness::DEFAULT_CLUSTER_CHILD_TIMEOUT_MS)]
    child_timeout_ms: u64,
    #[arg(long)]
    force: bool,
}

#[derive(Debug, clap::Args)]
pub(crate) struct ClusterHarnessVerify {
    #[arg(long)]
    run_dir: std::path::PathBuf,
}

pub(crate) fn run(command: ClusterCommand) -> molten::error::Result<()> {
    match command {
        ClusterCommand::Init(input) => init(input),
        ClusterCommand::Start(input) => start(input),
        ClusterCommand::Status(input) => status(input),
        ClusterCommand::Stop(input) => stop(input),
        ClusterCommand::HarnessRun(input) => harness_run(input),
        ClusterCommand::HarnessVerify(input) => harness_verify(input),
    }
}

// r[impl molten.testing.receipt_first_cluster_harness.cli_receipt_surface]
fn harness_run(input: ClusterHarnessRun) -> molten::error::Result<()> {
    let node_binary = input.node_binary.map_or_else(std::env::current_exe, Ok)?;
    let execution =
        molten::cluster_harness::execute_cluster_harness(&molten::cluster_harness::ClusterHarnessExecutionInput {
            fixture_path: input.fixture,
            state_root: input.state_root,
            output_directory: input.run_dir,
            node_binary,
            child_timeout_ms: input.child_timeout_ms,
            force: input.force,
        })?;
    println!(
        "cluster harness run decision={} parent={} verification={} run_dir={}",
        execution.decision,
        execution.parent_ref,
        execution.verification_ref,
        execution.output_directory.display()
    );
    if let Some(bundle_ref) = &execution.failure_bundle_ref {
        println!("cluster harness failure_bundle={bundle_ref} evidence_scope=diagnostic-only");
    }
    if execution.decision != "pass" {
        return Err(molten::error::MoltenError::invalid_harness(format!(
            "cluster harness run denied: {}",
            execution.diagnostics.join(",")
        )));
    }
    Ok(())
}

// r[impl molten.testing.receipt_first_cluster_harness.run_artifact_directory]
fn harness_verify(input: ClusterHarnessVerify) -> molten::error::Result<()> {
    let verification = molten::cluster_harness::verify_cluster_run_directory(&input.run_dir)?;
    println!(
        "cluster harness verify decision={} index={} verification={} run_dir={}",
        verification.decision,
        verification.index_ref,
        verification.receipt.verification_ref,
        input.run_dir.display()
    );
    if verification.decision != "pass" {
        return Err(molten::error::MoltenError::invalid_harness(format!(
            "cluster harness verification denied: {}",
            verification.receipt.diagnostics.join(",")
        )));
    }
    Ok(())
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
