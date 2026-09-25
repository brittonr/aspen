type Command = super::NixosVmCommand;
type FilePath = std::path::PathBuf;
type Outcome<T> = molten::error::Result<T>;

pub(super) fn run(command: Command) -> Outcome<()> {
    match command {
        Command::Topology(input) => run_topology(input),
        Command::NodeEvidence(input) => run_node(input),
        Command::RunReceipt(input) => run_receipt(input),
        Command::Validate(input) => run_validate(input),
        Command::Manifest(input) => run_manifest(input),
        Command::FaultDescriptor(input) => run_fault_descriptor(input),
        Command::FaultReceipt(input) => run_fault_receipt(input),
        Command::FaultValidate(input) => run_fault_validate(input),
        Command::ShardRun(input) => run_shard_run(input),
        Command::Aggregate(input) => run_aggregate(input),
        Command::Show { artifact } => run_show(artifact),
    }
}

fn run_topology(input: super::command::TopologyInput) -> Outcome<()> {
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

fn run_node(input: super::command::NodeInput) -> Outcome<()> {
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

fn run_receipt(input: super::command::ReceiptInput) -> Outcome<()> {
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

fn run_validate(input: super::command::ValidateInput) -> Outcome<()> {
    let topology = super::io::read_preserves_file(&input.topology)?;
    let node_evidence = read_preserves_files(&input.node_evidence)?;
    let test_run = super::io::read_preserves_file(&input.test_run)?;
    let prod_soak = read_preserves_files(&input.prod_soak)?;
    let child_artifacts = read_preserves_files(&input.child_artifacts)?;
    let expected_child_receipts = parse_expected_child_receipts(&input.expected_child_receipts)?;
    let validation = molten::nixos_vm::validate_nixos_vm_evidence(&molten::nixos_vm::NixosVmEvidenceValidationInput {
        topology_value: &topology,
        node_evidence_values: &node_evidence,
        test_run_value: &test_run,
        prod_soak_values: &prod_soak,
        child_artifact_values: &child_artifacts,
        expected_nodes: &input.expected_nodes,
        expected_package_ref: input.expected_package_ref.as_deref(),
        expected_child_refs: &input.expected_child_refs,
        expected_child_receipts: &expected_child_receipts,
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

fn run_manifest(input: super::command::ManifestInput) -> Outcome<()> {
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
    let required_artifacts = parse_required_artifacts(&input.required_artifacts)?;
    let manifest = molten::nixos_vm::build_vm_evidence_manifest(&molten::nixos_vm::VmEvidenceManifestInput {
        entries: &entries,
        required_artifacts: &required_artifacts,
        caveats: &input.caveats,
    })?;
    let is_written_to_file = super::io::write_optional_preserves(input.out.as_ref(), &manifest.value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!(
            "nixos-vm manifest ref={} decision={} entries={} diagnostics={}",
            manifest.manifest_ref,
            manifest.decision,
            entries.len(),
            manifest.diagnostics.len()
        ),
    );
    if manifest.decision == "pass" {
        Ok(())
    } else {
        Err(molten::error::MoltenError::invalid_harness(format!(
            "nixos VM evidence manifest denied: {}",
            manifest.diagnostics.join(",")
        )))
    }
}

fn run_fault_descriptor(input: super::command::FaultDescriptorInput) -> Outcome<()> {
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

fn run_fault_receipt(input: super::command::FaultReceiptInput) -> Outcome<()> {
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

fn run_fault_validate(input: super::command::FaultValidateInput) -> Outcome<()> {
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
