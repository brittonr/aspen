type Command = super::NixosVmCommand;
type FilePath = std::path::PathBuf;
type Outcome<T> = molten::error::Result<T>;

struct TopologyInput {
    nodes: Vec<String>,
    package_ref: String,
    package_path: String,
    network: String,
    nix_inputs: Vec<String>,
    caveats: Vec<String>,
    out: Option<FilePath>,
}

struct NodeInput {
    node: String,
    state_root: FilePath,
    identity: Option<FilePath>,
    startup: FilePath,
    health: FilePath,
    control_loop: FilePath,
    heartbeat: FilePath,
    shutdown: Option<FilePath>,
    logs: Vec<FilePath>,
    out: Option<FilePath>,
}

struct ReceiptInput {
    topology: FilePath,
    node_evidence: Vec<FilePath>,
    scenario: String,
    fault_profile: String,
    child_refs: Vec<String>,
    logs: Vec<FilePath>,
    decision: String,
    replay_status: String,
    diagnostics: Vec<String>,
    caveats: Vec<String>,
    out: Option<FilePath>,
}

pub(super) fn run(command: Command) -> Outcome<()> {
    match command {
        Command::Topology {
            nodes,
            package_ref,
            package_path,
            network,
            nix_inputs,
            caveats,
            out,
        } => run_topology(TopologyInput {
            nodes,
            package_ref,
            package_path,
            network,
            nix_inputs,
            caveats,
            out,
        }),
        Command::NodeEvidence {
            node,
            state_root,
            identity,
            startup,
            health,
            control_loop,
            heartbeat,
            shutdown,
            logs,
            out,
        } => run_node(NodeInput {
            node,
            state_root,
            identity,
            startup,
            health,
            control_loop,
            heartbeat,
            shutdown,
            logs,
            out,
        }),
        Command::RunReceipt {
            topology,
            node_evidence,
            scenario,
            fault_profile,
            child_refs,
            logs,
            decision,
            replay_status,
            diagnostics,
            caveats,
            out,
        } => run_receipt(ReceiptInput {
            topology,
            node_evidence,
            scenario,
            fault_profile,
            child_refs,
            logs,
            decision,
            replay_status,
            diagnostics,
            caveats,
            out,
        }),
        Command::Show { artifact } => run_show(artifact),
    }
}

fn run_topology(input: TopologyInput) -> Outcome<()> {
    let value = molten::nixos_vm::topology_value(&molten::nixos_vm::NixosVmTopologyInput {
        nodes: &input.nodes,
        package_ref: &input.package_ref,
        package_path: &input.package_path,
        network: &input.network,
        nix_inputs: &input.nix_inputs,
        caveats: &input.caveats,
    })?;
    let reference = molten::preserves_rail::canonical_hash(&value)?;
    let is_written_to_file = super::io::write_optional_preserves(input.out.as_ref(), &value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!("nixos-vm topology ref={reference} nodes={} network={}", input.nodes.len(), input.network),
    );
    Ok(())
}

fn run_node(input: NodeInput) -> Outcome<()> {
    let identity_ref = super::io::optional_preserves_ref(input.identity.as_ref())?;
    let startup_ref = super::io::preserves_file_ref(&input.startup)?;
    let health_ref = super::io::preserves_file_ref(&input.health)?;
    let control_loop_ref = super::io::preserves_file_ref(&input.control_loop)?;
    let heartbeat_ref = super::io::preserves_file_ref(&input.heartbeat)?;
    let shutdown_ref = super::io::optional_preserves_ref(input.shutdown.as_ref())?;
    let log_refs = super::io::raw_file_refs(&input.logs)?;
    let state_root_text = input.state_root.display().to_string();
    let value = molten::nixos_vm::node_evidence_value(&molten::nixos_vm::NixosVmNodeEvidenceInput {
        node: &input.node,
        state_root: &state_root_text,
        identity_receipt_ref: identity_ref.as_deref(),
        startup_receipt_ref: &startup_ref,
        health_receipt_ref: &health_ref,
        control_loop_receipt_ref: &control_loop_ref,
        heartbeat_receipt_ref: &heartbeat_ref,
        shutdown_receipt_ref: shutdown_ref.as_deref(),
        log_refs: &log_refs,
    })?;
    let reference = molten::preserves_rail::canonical_hash(&value)?;
    let is_written_to_file = super::io::write_optional_preserves(input.out.as_ref(), &value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!("nixos-vm node-evidence ref={reference} node={}", input.node),
    );
    Ok(())
}

fn run_receipt(input: ReceiptInput) -> Outcome<()> {
    let topology_ref = super::io::preserves_file_ref(&input.topology)?;
    let node_evidence_refs = super::io::preserves_file_refs(&input.node_evidence)?;
    let log_refs = super::io::raw_file_refs(&input.logs)?;
    let value = molten::nixos_vm::test_run_value(&molten::nixos_vm::NixosVmTestRunInput {
        decision: &input.decision,
        topology_ref: &topology_ref,
        scenario: &input.scenario,
        fault_profile: &input.fault_profile,
        node_evidence_refs: &node_evidence_refs,
        child_workflow_refs: &input.child_refs,
        replay_status: &input.replay_status,
        diagnostics: &input.diagnostics,
        log_refs: &log_refs,
        caveats: &input.caveats,
    })?;
    let reference = molten::preserves_rail::canonical_hash(&value)?;
    let is_written_to_file = super::io::write_optional_preserves(input.out.as_ref(), &value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!("nixos-vm test-run ref={reference} decision={} scenario={}", input.decision, input.scenario),
    );
    Ok(())
}

fn run_show(artifact: FilePath) -> Outcome<()> {
    let value = super::io::read_preserves_file(&artifact)?;
    let reference = molten::preserves_rail::canonical_hash(&value)?;
    let rendered = molten::preserves_rail::to_text(&value)?;
    let kind = super::io::kind(&rendered);
    println!("nixos-vm {kind} ref={reference} path={}", artifact.display());
    Ok(())
}
