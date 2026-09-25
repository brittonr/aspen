
fn fault_validation_diagnostics(
    topology: &ParsedTopology,
    topology_ref: &str,
    descriptors: &[ParsedFaultDescriptor],
    descriptor_refs: &[String],
    receipts: &[ParsedFaultReceipt],
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    push_if(&mut diagnostics, descriptors.is_empty(), "vm-fault-missing-descriptors")?;
    push_if(&mut diagnostics, receipts.is_empty(), "vm-fault-missing-receipts")?;
    let topology_nodes = topology.nodes.iter().map(String::as_str).collect::<OrderedSet<_>>();
    let descriptor_ref_set = descriptor_refs.iter().map(String::as_str).collect::<OrderedSet<_>>();
    for descriptor in descriptors {
        push_if(&mut diagnostics, descriptor.id.trim().is_empty(), "vm-fault-descriptor-missing-id")?;
        push_if(&mut diagnostics, descriptor.topology_ref != topology_ref, "vm-fault-descriptor-topology-mismatch")?;
        push_if(
            &mut diagnostics,
            !topology_nodes.contains(descriptor.target_node.as_str()),
            "vm-fault-descriptor-target-outside-topology",
        )?;
        push_if(
            &mut diagnostics,
            descriptor.duration_millis < VM_FAULT_MIN_DURATION_MILLIS,
            "vm-fault-descriptor-unbounded-duration",
        )?;
        push_if(&mut diagnostics, descriptor.caveats.is_empty(), "vm-fault-descriptor-missing-caveats")?;
    }
    for receipt in receipts {
        push_if(
            &mut diagnostics,
            !descriptor_ref_set.contains(receipt.descriptor_ref.as_str()),
            "vm-fault-receipt-descriptor-missing",
        )?;
        push_if(
            &mut diagnostics,
            receipt.decision == "pass" && receipt.host_support != "supported",
            "vm-fault-unavailable-cannot-pass",
        )?;
        push_if(
            &mut diagnostics,
            receipt.decision == "pass" && receipt.pre_fault_refs.is_empty(),
            "vm-fault-pass-missing-pre-ref",
        )?;
        push_if(
            &mut diagnostics,
            receipt.decision == "pass" && receipt.injection_refs.is_empty(),
            "vm-fault-pass-missing-injection-ref",
        )?;
        push_if(
            &mut diagnostics,
            receipt.decision == "pass" && receipt.child_refs.is_empty(),
            "vm-fault-log-only-pass",
        )?;
        push_if(
            &mut diagnostics,
            receipt.decision == "pass" && receipt.post_fault_refs.is_empty(),
            "vm-fault-pass-missing-post-ref",
        )?;
        push_if(
            &mut diagnostics,
            receipt.decision != "pass" && receipt.diagnostics.is_empty(),
            "vm-fault-deny-missing-diagnostic",
        )?;
        push_if(&mut diagnostics, receipt.log_refs.is_empty(), "vm-fault-missing-log-ref")?;
        push_if(&mut diagnostics, receipt.replay_status.trim().is_empty(), "vm-fault-missing-replay-status")?;
        push_if(&mut diagnostics, receipt.caveats.is_empty(), "vm-fault-missing-caveats")?;
        validate_fault_expected_outcome(receipt, descriptors, descriptor_refs, &mut diagnostics)?;
    }
    Ok(diagnostics)
}

fn validate_fault_expected_outcome(
    receipt: &ParsedFaultReceipt,
    descriptors: &[ParsedFaultDescriptor],
    descriptor_refs: &[String],
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    let Some((descriptor, _)) = descriptors
        .iter()
        .zip(descriptor_refs.iter())
        .find(|(_, descriptor_ref)| descriptor_ref.as_str() == receipt.descriptor_ref)
    else {
        return Ok(());
    };
    push_if(
        diagnostics,
        descriptor.expected_outcome == "unavailable" && receipt.decision == "pass",
        "vm-fault-unavailable-expected-cannot-pass",
    )?;
    push_if(
        diagnostics,
        descriptor.fault_kind == "log-only-pass" && receipt.decision == "pass",
        "vm-fault-log-only-pass",
    )?;
    Ok(())
}

fn parse_topology(value: &IoValue) -> Result<ParsedTopology> {
    let topology = simple_record(value, "nixos-vm-topology-v1", TOPOLOGY_ARITY)?;
    require_schema(&topology[TOPOLOGY_SCHEMA_INDEX], crate::preserves_rail::NIXOS_VM_TOPOLOGY_SCHEMA, "topology")?;
    let nodes = node_names(&topology[TOPOLOGY_NODES_INDEX])?;
    let package = simple_field_record(&topology[TOPOLOGY_PACKAGE_INDEX], "package", "topology package")?;
    let package_record = value_to_iovalue(&package[0]);
    let molten_package = simple_record(&package_record, "molten-package", 2)?;
    let package_ref = required_record_string(&molten_package[0], "ref", "topology package ref")?;
    let network = required_record_string(&topology[TOPOLOGY_NETWORK_INDEX], "network", "topology network")?;
    let caveats = required_string_sequence_record(&topology[TOPOLOGY_CAVEATS_INDEX], "caveats", "topology caveats")?;
    Ok(ParsedTopology {
        nodes,
        package_ref,
        network,
        caveats,
    })
}

fn parse_node_evidence_values(values: &[IoValue]) -> Result<Vec<ParsedNodeEvidence>> {
    if values.len() > MAX_VM_VALIDATION_ITEMS {
        return Err(MoltenError::invalid_harness(format!(
            "VM node evidence count {} exceeds bound {MAX_VM_VALIDATION_ITEMS}",
            values.len()
        )));
    }
    let mut output = Vec::with_capacity(values.len());
    for value in values {
        output.push(parse_node_evidence(value)?);
    }
    Ok(output)
}

fn parse_node_evidence(value: &IoValue) -> Result<ParsedNodeEvidence> {
    let node = simple_record(value, "nixos-vm-node-evidence-v1", NODE_EVIDENCE_ARITY)?;
    require_schema(&node[NODE_SCHEMA_INDEX], crate::preserves_rail::NIXOS_VM_NODE_EVIDENCE_SCHEMA, "node evidence")?;
    Ok(ParsedNodeEvidence {
        node: required_record_string(&node[NODE_NAME_INDEX], "node", "node evidence node")?,
        state_root: required_record_string(&node[NODE_STATE_ROOT_INDEX], "state-root", "node evidence state root")?,
        startup_ref: required_record_ref(&node[NODE_STARTUP_INDEX], "startup-receipt", "node startup")?,
        health_ref: required_record_ref(&node[NODE_HEALTH_INDEX], "health-receipt", "node health")?,
        control_loop_ref: required_record_ref(
            &node[NODE_CONTROL_LOOP_INDEX],
            "control-loop-receipt",
            "node control loop",
        )?,
        heartbeat_ref: required_record_ref(&node[NODE_HEARTBEAT_INDEX], "heartbeat-receipt", "node heartbeat")?,
        log_refs: required_ref_sequence_record(&node[NODE_LOGS_INDEX], "logs", "node logs")?,
    })
}

fn parse_test_run(value: &IoValue) -> Result<ParsedTestRun> {
    let run = simple_record(value, "nixos-vm-test-run-v1", TEST_RUN_ARITY)?;
    require_schema(&run[TEST_RUN_SCHEMA_INDEX], crate::preserves_rail::NIXOS_VM_TEST_RUN_SCHEMA, "test run")?;
    Ok(ParsedTestRun {
        decision: required_record_string(&run[TEST_RUN_DECISION_INDEX], "decision", "test run decision")?,
        topology_ref: required_record_ref(&run[TEST_RUN_TOPOLOGY_INDEX], "topology", "test run topology")?,
        node_evidence_refs: required_ref_sequence_record(
            &run[TEST_RUN_NODE_EVIDENCE_INDEX],
            "node-evidence",
            "test run nodes",
        )?,
        child_refs: required_ref_sequence_record(
            &run[TEST_RUN_CHILDREN_INDEX],
            "child-workflows",
            "test run children",
        )?,
        replay_status: required_record_string(&run[TEST_RUN_REPLAY_INDEX], "replay-status", "test run replay status")?,
        log_refs: required_ref_sequence_record(&run[TEST_RUN_LOGS_INDEX], "logs", "test run logs")?,
        caveats: required_string_sequence_record(&run[TEST_RUN_CAVEATS_INDEX], "caveats", "test run caveats")?,
    })
}

fn parse_child_receipts(values: &[IoValue]) -> Result<Vec<ParsedChildReceipt>> {
    if values.len() > MAX_VM_VALIDATION_ITEMS {
        return Err(MoltenError::invalid_harness(format!(
            "VM child artifact count {} exceeds bound {MAX_VM_VALIDATION_ITEMS}",
            values.len()
        )));
    }
    let mut receipts = Vec::with_capacity(values.len());
    let mut refs = OrderedSet::new();
    for value in values {
        let receipt = parse_child_receipt(value)?;
        if !refs.insert(receipt.child_ref.clone()) {
            return Err(MoltenError::invalid_harness(format!("duplicate VM child artifact ref {}", receipt.child_ref)));
        }
        receipts.push(receipt);
    }
    Ok(receipts)
}

fn parse_child_receipt(value: &IoValue) -> Result<ParsedChildReceipt> {
    let child_ref = crate::preserves_rail::canonical_hash(value)?;
    let receipt_class = child_receipt_class(value).to_string();
    let decision = child_receipt_decision(value).unwrap_or_else(|| UNKNOWN_CHILD_DECISION.to_string());
    Ok(ParsedChildReceipt {
        child_ref,
        receipt_class,
        decision,
        node_id: None,
        peer_id: None,
        operation_id: child_receipt_operation_id(value),
    })
}

fn child_receipt_class(value: &IoValue) -> &'static str {
    for record_label in CHILD_RECEIPT_CLASSES {
        if value.collect_simple_record(record_label, None).is_some() {
            return record_label;
        }
    }
    crate::ledger::artifact_kind(value)
}

fn child_receipt_decision(value: &IoValue) -> Option<String> {
    if let Ok(run) = simple_record(value, "nixos-vm-test-run-v1", TEST_RUN_ARITY) {
        return required_record_string(&run[TEST_RUN_DECISION_INDEX], "decision", "test run decision").ok();
    }
    if let Ok(receipt) = simple_record(value, "nixos-vm-fault-receipt-v1", FAULT_RECEIPT_ARITY) {
        return required_record_string(&receipt[FAULT_RECEIPT_DECISION_INDEX], "decision", "fault receipt decision")
            .ok();
    }
    if let Ok(shard) = simple_record(value, "nixos-vm-shard-run-v1", SHARD_RUN_ARITY) {
        return required_record_string(&shard[SHARD_RUN_DECISION_INDEX], "decision", "shard run decision").ok();
    }
    if let Ok(aggregate) = simple_record(value, "nixos-vm-multinode-aggregate-v1", AGGREGATE_ARITY) {
        return required_record_string(&aggregate[AGGREGATE_DECISION_INDEX], "decision", "aggregate decision").ok();
    }
    None
}

fn child_receipt_operation_id(value: &IoValue) -> Option<String> {
    if let Ok(receipt) = simple_record(value, "nixos-vm-fault-receipt-v1", FAULT_RECEIPT_ARITY) {
        return required_record_string(&receipt[FAULT_RECEIPT_DESCRIPTOR_INDEX], "descriptor", "fault descriptor").ok();
    }
    None
}

fn parse_fault_descriptors(values: &[IoValue]) -> Result<Vec<ParsedFaultDescriptor>> {
    if values.len() > MAX_VM_VALIDATION_ITEMS {
        return Err(MoltenError::invalid_harness(format!(
            "VM fault descriptor count {} exceeds bound {MAX_VM_VALIDATION_ITEMS}",
            values.len()
        )));
    }
    let mut output = Vec::with_capacity(values.len());
    for value in values {
        output.push(parse_fault_descriptor(value)?);
    }
    Ok(output)
}

fn parse_fault_descriptor(value: &IoValue) -> Result<ParsedFaultDescriptor> {
    let descriptor = simple_record(value, "nixos-vm-fault-descriptor-v1", FAULT_DESCRIPTOR_ARITY)?;
    require_schema(&descriptor[FAULT_DESCRIPTOR_SCHEMA_INDEX], VM_FAULT_DESCRIPTOR_SCHEMA, "fault descriptor")?;
    Ok(ParsedFaultDescriptor {
        id: required_record_string(&descriptor[FAULT_DESCRIPTOR_ID_INDEX], "id", "fault descriptor id")?,
        topology_ref: required_record_ref(
            &descriptor[FAULT_DESCRIPTOR_TOPOLOGY_INDEX],
            "topology",
            "fault descriptor topology",
        )?,
        target_node: required_record_string(
            &descriptor[FAULT_DESCRIPTOR_TARGET_NODE_INDEX],
            "target-node",
            "fault descriptor target node",
        )?,
        fault_kind: required_record_string(
            &descriptor[FAULT_DESCRIPTOR_KIND_INDEX],
            "fault-kind",
            "fault descriptor kind",
        )?,
        expected_outcome: required_record_string(
            &descriptor[FAULT_DESCRIPTOR_EXPECTED_INDEX],
            "expected-outcome",
            "fault descriptor expected outcome",
        )?,
        duration_millis: required_record_u64(
            &descriptor[FAULT_DESCRIPTOR_DURATION_INDEX],
            "duration-millis",
            "fault descriptor duration",
        )?,
        caveats: required_string_sequence_record(
            &descriptor[FAULT_DESCRIPTOR_CAVEATS_INDEX],
            "caveats",
            "fault descriptor caveats",
        )?,
    })
}
