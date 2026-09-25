#[derive(Debug, clap::Subcommand)]
pub(crate) enum ClusterCommand {
    HarnessRun(ClusterHarnessRun),
    HarnessVerify(ClusterHarnessVerify),
    FabricTransportRun(FabricTransportRun),
    FabricTransportVerify(FabricTransportVerify),
    #[command(name = "fabric-transport-listener-child", hide = true)]
    FabricTransportListenerChild(FabricTransportChild),
    #[command(name = "fabric-transport-client-child", hide = true)]
    FabricTransportClientChild(FabricTransportChild),
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

pub(crate) fn run(command: ClusterCommand) -> molten::error::Result<()> {
    match command {
        ClusterCommand::HarnessRun(input) => harness_run(input),
        ClusterCommand::HarnessVerify(input) => harness_verify(input),
        ClusterCommand::FabricTransportRun(input) => fabric_transport_run(input),
        ClusterCommand::FabricTransportVerify(input) => fabric_transport_verify(input),
        ClusterCommand::FabricTransportListenerChild(input) => {
            molten::cluster_harness::run_distinct_process_listener_child(&input.run_dir)
        }
        ClusterCommand::FabricTransportClientChild(input) => {
            molten::cluster_harness::run_distinct_process_client_child(&input.run_dir)
        }
    }
}

// r[impl molten.testing.receipt_first_cluster_harness.cli_receipt_surface]
fn harness_run(input: ClusterHarnessRun) -> molten::error::Result<()> {
    let node_binary = input
        .node_binary
        .map_or_else(|| std::env::current_exe().map(|binary| binary.with_file_name("molten-node")), Ok)?;
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
        return Err(molten::error::Failure::invalid_harness(format!(
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
    let execution = molten::cluster_harness::execute_distinct_process_transport_run(
        &molten::cluster_harness::DistinctProcessTransportRunInput {
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
    let verification = molten::cluster_harness::verify_distinct_process_transport_run(&input.run_dir)?;
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
