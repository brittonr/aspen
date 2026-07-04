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

struct ValidateInput {
    topology: FilePath,
    node_evidence: Vec<FilePath>,
    test_run: FilePath,
    prod_soak: Vec<FilePath>,
    expected_nodes: Vec<String>,
    expected_package_ref: Option<String>,
    expected_child_refs: Vec<String>,
    out: Option<FilePath>,
}

struct ManifestInput {
    root: Option<FilePath>,
    artifacts: Vec<FilePath>,
    logs: Vec<FilePath>,
    caveats: Vec<String>,
    out: Option<FilePath>,
}

struct FaultDescriptorInput {
    fault_id: String,
    topology: FilePath,
    target_node: String,
    target_link: Option<String>,
    fault_kind: String,
    command_profile: String,
    expected_outcome: String,
    duration_millis: u64,
    trigger: String,
    preflight: Vec<FilePath>,
    caveats: Vec<String>,
    out: Option<FilePath>,
}

struct FaultReceiptInput {
    descriptor: FilePath,
    decision: String,
    host_support: String,
    pre_fault: Vec<FilePath>,
    injection: Vec<FilePath>,
    children: Vec<FilePath>,
    post_fault: Vec<FilePath>,
    replay_status: String,
    diagnostics: Vec<String>,
    logs: Vec<FilePath>,
    caveats: Vec<String>,
    out: Option<FilePath>,
}

struct FaultValidateInput {
    topology: FilePath,
    descriptors: Vec<FilePath>,
    receipts: Vec<FilePath>,
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
        Command::Validate {
            topology,
            node_evidence,
            test_run,
            prod_soak,
            expected_nodes,
            expected_package_ref,
            expected_child_refs,
            out,
        } => run_validate(ValidateInput {
            topology,
            node_evidence,
            test_run,
            prod_soak,
            expected_nodes,
            expected_package_ref,
            expected_child_refs,
            out,
        }),
        Command::Manifest {
            root,
            artifacts,
            logs,
            caveats,
            out,
        } => run_manifest(ManifestInput {
            root,
            artifacts,
            logs,
            caveats,
            out,
        }),
        Command::FaultDescriptor {
            fault_id,
            topology,
            target_node,
            target_link,
            fault_kind,
            command_profile,
            expected_outcome,
            duration_millis,
            trigger,
            preflight,
            caveats,
            out,
        } => run_fault_descriptor(FaultDescriptorInput {
            fault_id,
            topology,
            target_node,
            target_link,
            fault_kind,
            command_profile,
            expected_outcome,
            duration_millis,
            trigger,
            preflight,
            caveats,
            out,
        }),
        Command::FaultReceipt {
            descriptor,
            decision,
            host_support,
            pre_fault,
            injection,
            children,
            post_fault,
            replay_status,
            diagnostics,
            logs,
            caveats,
            out,
        } => run_fault_receipt(FaultReceiptInput {
            descriptor,
            decision,
            host_support,
            pre_fault,
            injection,
            children,
            post_fault,
            replay_status,
            diagnostics,
            logs,
            caveats,
            out,
        }),
        Command::FaultValidate {
            topology,
            descriptors,
            receipts,
            out,
        } => run_fault_validate(FaultValidateInput {
            topology,
            descriptors,
            receipts,
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

fn run_validate(input: ValidateInput) -> Outcome<()> {
    let topology = super::io::read_preserves_file(&input.topology)?;
    let node_evidence = read_preserves_files(&input.node_evidence)?;
    let test_run = super::io::read_preserves_file(&input.test_run)?;
    let prod_soak = read_preserves_files(&input.prod_soak)?;
    let validation = molten::nixos_vm::validate_nixos_vm_evidence(&molten::nixos_vm::NixosVmEvidenceValidationInput {
        topology_value: &topology,
        node_evidence_values: &node_evidence,
        test_run_value: &test_run,
        prod_soak_values: &prod_soak,
        expected_nodes: &input.expected_nodes,
        expected_package_ref: input.expected_package_ref.as_deref(),
        expected_child_refs: &input.expected_child_refs,
    })?;
    let is_written_to_file = super::io::write_optional_preserves(input.out.as_ref(), &validation.value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!(
            "nixos-vm validation ref={} decision={} diagnostics={}",
            validation.validation_ref,
            validation.decision,
            validation.diagnostics.len()
        ),
    );
    if validation.decision == "pass" {
        Ok(())
    } else {
        Err(molten::error::MoltenError::invalid_harness(format!(
            "nixos VM evidence validation denied: {}",
            validation.diagnostics.join(",")
        )))
    }
}

fn run_manifest(input: ManifestInput) -> Outcome<()> {
    let mut entries = Vec::with_capacity(input.artifacts.len() + input.logs.len());
    for artifact in &input.artifacts {
        let value = super::io::read_preserves_file(artifact)?;
        let rendered = molten::preserves_rail::to_text(&value)?;
        entries.push(molten::nixos_vm::VmEvidenceManifestEntry {
            path: manifest_path(input.root.as_ref(), artifact),
            kind: super::io::kind(&rendered).to_string(),
            content_ref: molten::preserves_rail::canonical_hash(&value)?,
            diagnostic_only: false,
        });
    }
    for log in &input.logs {
        entries.push(molten::nixos_vm::VmEvidenceManifestEntry {
            path: manifest_path(input.root.as_ref(), log),
            kind: "log".to_string(),
            content_ref: super::io::raw_file_ref(log)?,
            diagnostic_only: true,
        });
    }
    let value = molten::nixos_vm::vm_evidence_manifest_value(&entries, &input.caveats)?;
    let reference = molten::preserves_rail::canonical_hash(&value)?;
    let is_written_to_file = super::io::write_optional_preserves(input.out.as_ref(), &value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!("nixos-vm manifest ref={reference} entries={}", entries.len()),
    );
    Ok(())
}

fn run_fault_descriptor(input: FaultDescriptorInput) -> Outcome<()> {
    let topology_ref = super::io::preserves_file_ref(&input.topology)?;
    let preflight_refs = super::io::preserves_file_refs(&input.preflight)?;
    let value = molten::nixos_vm::vm_fault_descriptor_value(&molten::nixos_vm::NixosVmFaultDescriptorInput {
        fault_id: &input.fault_id,
        topology_ref: &topology_ref,
        target_node: &input.target_node,
        target_link: input.target_link.as_deref(),
        fault_kind: &input.fault_kind,
        command_profile: &input.command_profile,
        expected_outcome: &input.expected_outcome,
        duration_millis: input.duration_millis,
        trigger: &input.trigger,
        preflight_refs: &preflight_refs,
        caveats: &input.caveats,
    })?;
    let reference = molten::preserves_rail::canonical_hash(&value)?;
    let is_written_to_file = super::io::write_optional_preserves(input.out.as_ref(), &value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!("nixos-vm fault-descriptor ref={reference} fault={}", input.fault_id),
    );
    Ok(())
}

fn run_fault_receipt(input: FaultReceiptInput) -> Outcome<()> {
    let descriptor_ref = super::io::preserves_file_ref(&input.descriptor)?;
    let pre_fault_refs = super::io::preserves_file_refs(&input.pre_fault)?;
    let injection_refs = super::io::preserves_file_refs(&input.injection)?;
    let child_refs = super::io::preserves_file_refs(&input.children)?;
    let post_fault_refs = super::io::preserves_file_refs(&input.post_fault)?;
    let log_refs = super::io::raw_file_refs(&input.logs)?;
    let value = molten::nixos_vm::vm_fault_receipt_value(&molten::nixos_vm::NixosVmFaultReceiptInput {
        decision: &input.decision,
        descriptor_ref: &descriptor_ref,
        host_support: &input.host_support,
        pre_fault_refs: &pre_fault_refs,
        injection_refs: &injection_refs,
        child_refs: &child_refs,
        post_fault_refs: &post_fault_refs,
        replay_status: &input.replay_status,
        diagnostics: &input.diagnostics,
        log_refs: &log_refs,
        caveats: &input.caveats,
    })?;
    let reference = molten::preserves_rail::canonical_hash(&value)?;
    let is_written_to_file = super::io::write_optional_preserves(input.out.as_ref(), &value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!("nixos-vm fault-receipt ref={reference} decision={}", input.decision),
    );
    Ok(())
}

fn run_fault_validate(input: FaultValidateInput) -> Outcome<()> {
    let topology = super::io::read_preserves_file(&input.topology)?;
    let descriptors = read_preserves_files(&input.descriptors)?;
    let receipts = read_preserves_files(&input.receipts)?;
    let validation =
        molten::nixos_vm::validate_nixos_vm_fault_evidence(&molten::nixos_vm::NixosVmFaultEvidenceValidationInput {
            topology_value: &topology,
            descriptor_values: &descriptors,
            receipt_values: &receipts,
        })?;
    let is_written_to_file = super::io::write_optional_preserves(input.out.as_ref(), &validation.value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!(
            "nixos-vm fault-validation ref={} decision={} diagnostics={}",
            validation.validation_ref,
            validation.decision,
            validation.diagnostics.len()
        ),
    );
    if validation.decision == "pass" {
        Ok(())
    } else {
        Err(molten::error::MoltenError::invalid_harness(format!(
            "nixos VM fault validation denied: {}",
            validation.diagnostics.join(",")
        )))
    }
}

fn read_preserves_files(paths: &[FilePath]) -> Outcome<Vec<preserves::IOValue>> {
    let mut values = Vec::with_capacity(paths.len());
    for path in paths {
        values.push(super::io::read_preserves_file(path)?);
    }
    Ok(values)
}

fn manifest_path(root: Option<&FilePath>, path: &std::path::Path) -> String {
    if let Some(root) = root
        && let Ok(relative) = path.strip_prefix(root)
    {
        return relative.display().to_string();
    }
    path.display().to_string()
}

fn run_show(artifact: FilePath) -> Outcome<()> {
    let value = super::io::read_preserves_file(&artifact)?;
    let reference = molten::preserves_rail::canonical_hash(&value)?;
    let rendered = molten::preserves_rail::to_text(&value)?;
    let kind = super::io::kind(&rendered);
    println!("nixos-vm {kind} ref={reference} path={}", artifact.display());
    Ok(())
}
