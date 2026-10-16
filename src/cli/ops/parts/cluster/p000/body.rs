const RUNNING_STATUS: &str = "running";

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    Init(Init),
    Start(Root),
    Status(Root),
    Stop(Root),
    HarnessRun(HarnessRun),
    HarnessVerify(HarnessVerify),
    FabricTransportRun(FabricTransportRun),
    FabricTransportVerify(FabricTransportVerify),
    #[command(name = "fabric-transport-listener-child", hide = true)]
    FabricTransportListenerChild(FabricTransportChild),
    #[command(name = "fabric-transport-client-child", hide = true)]
    FabricTransportClientChild(FabricTransportChild),
}

#[derive(Debug, clap::Args)]
pub(crate) struct Init {
    #[arg(long)]
    state_root: std::path::PathBuf,
    #[arg(long = "node")]
    nodes: Vec<String>,
    #[arg(long)]
    force: bool,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Root {
    #[arg(long)]
    state_root: std::path::PathBuf,
}

#[derive(Debug, clap::Args)]
pub(crate) struct HarnessRun {
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
pub(crate) struct HarnessVerify {
    #[arg(long)]
    run_dir: std::path::PathBuf,
}

#[derive(Debug, clap::Args)]
pub(crate) struct FabricTransportRun {
    #[arg(long)]
    run_dir: std::path::PathBuf,
    #[arg(long)]
    process_binary: Option<std::path::PathBuf>,
    #[arg(long, default_value_t = molten::cluster_harness::DEFAULT_DISTINCT_PROCESS_TIMEOUT_MS)]
    child_timeout_ms: u64,
    #[arg(long)]
    force: bool,
}

#[derive(Debug, clap::Args)]
pub(crate) struct FabricTransportVerify {
    #[arg(long)]
    run_dir: std::path::PathBuf,
}

#[derive(Debug, clap::Args)]
pub(crate) struct FabricTransportChild {
    #[arg(long)]
    run_dir: std::path::PathBuf,
}

pub(crate) fn run(command: Command) -> molten::error::Result<()> {
    match command {
        Command::Init(input) => init(input),
        Command::Start(input) => start(input),
        Command::Status(input) => status(input),
        Command::Stop(input) => stop(input),
        Command::HarnessRun(input) => harness_run(input),
        Command::HarnessVerify(input) => harness_verify(input),
        Command::FabricTransportRun(input) => fabric_transport_run(input),
        Command::FabricTransportVerify(input) => fabric_transport_verify(input),
        Command::FabricTransportListenerChild(input) => {
            molten::cluster_harness::run_distinct_process_listener_child(&input.run_dir)
        }
        Command::FabricTransportClientChild(input) => {
            molten::cluster_harness::run_distinct_process_client_child(&input.run_dir)
        }
    }
}

// r[impl molten.testing.receipt_first_cluster_harness.cli_receipt_surface]
fn harness_run(input: HarnessRun) -> molten::error::Result<()> {
    let node_binary = input.node_binary.map_or_else(std::env::current_exe, Ok)?;
    let execution =
        molten::cluster_harness::execute(&molten::cluster_harness::ExecutionInput {
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
        return Err(molten::error::Failure::invalid_harness(format!(
            "cluster harness run denied: {}",
            execution.diagnostics.join(",")
        )));
    }
    Ok(())
}

// r[impl molten.testing.receipt_first_cluster_harness.run_artifact_directory]
fn harness_verify(input: HarnessVerify) -> molten::error::Result<()> {
    let verification = molten::cluster_harness::verify_run_directory(&input.run_dir)?;
    println!(
        "cluster harness verify decision={} index={} verification={} run_dir={}",
        verification.decision,
        verification.index_ref,
        verification.receipt.verification_ref,
        input.run_dir.display()
    );
    if verification.decision != "pass" {
        return Err(molten::error::Failure::invalid_harness(format!(
            "cluster harness verification denied: {}",
            verification.receipt.diagnostics.join(",")
        )));
    }
    Ok(())
}

// r[impl molten.fabric_transport.distinct_process_evidence]
fn fabric_transport_run(input: FabricTransportRun) -> molten::error::Result<()> {
    let process_binary = input.process_binary.map_or_else(std::env::current_exe, Ok)?;
    let execution = molten::cluster_harness::execute_distinct_process_run(
        &molten::cluster_harness::DistinctProcessRunInput {
            run_directory: input.run_dir,
            process_binary,
            child_timeout_ms: input.child_timeout_ms,
            force: input.force,
            request_ref: molten::cluster_harness::DEFAULT_DISTINCT_PROCESS_REQUEST_REF.to_string(),
            payload: molten::cluster_harness::DEFAULT_DISTINCT_PROCESS_PAYLOAD.to_vec(),
        },
    )?;
    println!(
        "fabric transport distinct-process run decision={} parent={} verification={} run_dir={}",
        execution.decision,
        execution.parent_ref,
        execution.verification_ref,
        execution.run_directory.display()
    );
    if execution.decision != "pass" {
        return Err(molten::error::Failure::invalid_harness(format!(
            "fabric transport distinct-process run denied: {}",
            execution.diagnostics.join(",")
        )));
    }
    Ok(())
}

// r[impl molten.fabric_transport.distinct_process_evidence]
fn fabric_transport_verify(input: FabricTransportVerify) -> molten::error::Result<()> {
    let verification = molten::cluster_harness::verify_distinct_process_run(&input.run_dir)?;
    println!(
        "fabric transport distinct-process verify decision={} parent={} verification={} run_dir={}",
        verification.decision,
        verification.parent_ref,
        verification.verification_ref,
        input.run_dir.display()
    );
    if verification.decision != "pass" {
        return Err(molten::error::Failure::invalid_harness(format!(
            "fabric transport distinct-process verification denied: {}",
            verification.diagnostics.join(",")
        )));
    }
    Ok(())
}

fn init(input: Init) -> molten::error::Result<()> {
    let plan = molten::cluster::plan(&input.state_root, &input.nodes)?;
    prepare_init(&plan, input.force)?;
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
    write_manifest(&plan)?;
    println!("cluster init nodes={} state_root={}", initialized_count, plan.state_root.display());
    Ok(())
}

fn start(input: Root) -> molten::error::Result<()> {
    let plan = read_plan(&input.state_root)?;
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

fn status(input: Root) -> molten::error::Result<()> {
    let plan = read_plan(&input.state_root)?;
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

fn stop(input: Root) -> molten::error::Result<()> {
    let plan = read_plan(&input.state_root)?;
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
