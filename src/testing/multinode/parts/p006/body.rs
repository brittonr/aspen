fn vm_failure_repro_export_value(
    input: &VmFailureReproExportInput,
    bundle_ref: &str,
    verification_ref: &str,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("vm-failure-repro-export-v1", vec![
        string(VM_FAILURE_REPRO_EXPORT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("bundle", vec![string(bundle_ref)]),
        record("verification", vec![string(verification_ref)]),
        record("topology", vec![string(&input.topology_ref)]),
        record("node-summaries", vec![refs_sequence(&input.node_summary_refs)]),
        record("child-receipts", vec![refs_sequence(&input.child_receipt_refs)]),
        record("validation", vec![refs_sequence(&input.validation_refs)]),
        record("diagnostic-logs", vec![refs_sequence(&input.diagnostic_log_refs)]),
        record("unavailable-host-support", vec![bool_value(input.unavailable_host_support)]),
        record("denied-or-failed-validation", vec![bool_value(input.denied_or_failed_validation)]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        record("caveats", vec![strings_sequence(&input.caveats)]),
        checks_value(&[
            ("diagnostic-only", PASS_DECISION),
            ("non-replayable-vm-observation", PASS_DECISION),
            (
                "failure-condition-bound",
                status(input.unavailable_host_support || input.denied_or_failed_validation),
            ),
        ]),
    ]))
}

fn generated_case_value(case: &GeneratedDistributedCase) -> Result<IoValue> {
    let topology_ref =
        canonical_hash(&crate::distributed_core::topology_value(&case.simulation.topology)?)?;
    let scheduler_ref = canonical_hash(&crate::distributed_core::scheduler_profile_value(&case.simulation.scheduler)?)?;
    let seed_ref = canonical_hash(&crate::distributed_core::seed_value(&case.simulation.seed)?)?;
    let fault_plan_ref = canonical_hash(&crate::distributed_core::fault_plan_value(&case.simulation.fault_plan)?)?;
    Ok(record("generated-distributed-case-v1", vec![
        string(GENERATED_DISTRIBUTED_CASE_SCHEMA),
        record("id", vec![string(&case.case_id)]),
        record("invariant", vec![string(&case.invariant_name)]),
        record("topology", vec![string(topology_ref)]),
        record("scheduler", vec![string(scheduler_ref)]),
        record("seed", vec![string(seed_ref)]),
        record("fault-plan", vec![string(fault_plan_ref)]),
        record("commands", vec![strings_sequence(
            &case.simulation.commands.iter().map(|command| command.operation_id.clone()).collect::<Vec<_>>(),
        )]),
        checks_value(&[
            ("seed-bound", PASS_DECISION),
            ("ambient-randomness-excluded", PASS_DECISION),
        ]),
    ]))
}

struct GeneratedReproValueInput<'a> {
    case: &'a GeneratedDistributedCase,
    case_ref: &'a str,
    first: &'a crate::distributed_core::SimulationRun,
    replay: &'a crate::distributed_core::SimulationRun,
    decision: &'a str,
    diagnostics: &'a [String],
}

fn generated_repro_value(input: &GeneratedReproValueInput<'_>) -> Result<IoValue> {
    Ok(record("generated-distributed-repro-v1", vec![
        string(GENERATED_DISTRIBUTED_REPRO_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("case", vec![string(input.case_ref)]),
        record("invariant", vec![string(&input.case.invariant_name)]),
        record("run", vec![string(&input.first.receipt_ref)]),
        record("replay-run", vec![string(&input.replay.receipt_ref)]),
        record("topology", vec![string(&input.first.topology_ref)]),
        record("scheduler", vec![string(&input.first.scheduler_ref)]),
        record("seed", vec![string(&input.first.seed_ref)]),
        record("fault-plan", vec![string(&input.first.fault_plan_ref)]),
        record("events", vec![refs_sequence(&input.first.event_refs)]),
        record("final-state", vec![string(&input.first.final_state_ref)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        record("evidence-scope", vec![string(DIAGNOSTIC_ONLY)]),
        checks_value(&[
            ("replay-seed-bound", status(input.first.receipt_ref == input.replay.receipt_ref)),
            ("diagnostic-only-unless-gated", PASS_DECISION),
        ]),
    ]))
}

fn failure_repro_payload_value(input: &FailureReproBundleInput) -> Result<IoValue> {
    Ok(record("multinode-failure-repro-payload-v1", vec![
        string(MULTINODE_FAILURE_REPRO_PAYLOAD_SCHEMA),
        record("scenario-fixture", vec![string(&input.scenario_fixture_ref)]),
        record("topology", vec![string(&input.topology_ref)]),
        record("scheduler", vec![string(&input.scheduler_ref)]),
        record("seed", vec![string(&input.seed_ref)]),
        record("fault-plan", vec![string(&input.fault_plan_ref)]),
        record("commands", vec![refs_sequence(&input.command_refs)]),
        record("node-summaries", vec![refs_sequence(&input.node_summary_refs)]),
        record("receipts", vec![refs_sequence(&input.receipt_refs)]),
        record("diagnostics", vec![refs_sequence(&input.diagnostic_refs)]),
        record("logs", vec![refs_sequence(&input.log_refs)]),
        record("redaction-policy", vec![string(&input.redaction_policy_ref)]),
        record("replay-status", vec![string(&input.replay_status)]),
        record("diagnostic-only", vec![bool_value(input.diagnostic_only)]),
        record("private-attachments", vec![refs_sequence(&input.private_attachment_refs)]),
        record("reveal-receipts", vec![refs_sequence(&input.reveal_receipt_refs)]),
        record("caveats", vec![strings_sequence(&input.caveats)]),
    ]))
}

fn failure_repro_bundle_value(
    input: &FailureReproBundleInput,
    payload: &IoValue,
    claimed_payload_ref: &str,
) -> Result<IoValue> {
    Ok(record("multinode-failure-repro-bundle-v1", vec![
        string(MULTINODE_FAILURE_REPRO_BUNDLE_SCHEMA),
        record("sealed", vec![bool_value(input.sealed)]),
        record("payload-ref", vec![string(claimed_payload_ref)]),
        record("payload", vec![payload.clone()]),
        checks_value(&[
            ("sealed", status(input.sealed)),
            ("diagnostic-only-unless-gated", PASS_DECISION),
            (
                "private-content-requires-reveal",
                status(input.private_attachment_refs.is_empty() || !input.reveal_receipt_refs.is_empty()),
            ),
        ]),
    ]))
}

fn failure_repro_verify_value(
    input: &FailureReproBundleInput,
    payload_ref: &str,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("multinode-failure-repro-verify-v1", vec![
        string(MULTINODE_FAILURE_REPRO_VERIFY_SCHEMA),
        record("decision", vec![string(decision)]),
        record("payload", vec![string(payload_ref)]),
        record("claimed-payload", vec![optional_ref_value(input.claimed_payload_ref.as_deref())]),
        record("replay-status", vec![string(&input.replay_status)]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        checks_value(&[
            ("seal-metadata-valid", status(!diagnostics.iter().any(|item| item.contains("seal")))),
            ("redaction-policy-bound", status(!diagnostics.iter().any(|item| item.contains("redaction")))),
            ("diagnostic-only-not-pass", PASS_DECISION),
        ]),
    ]))
}

fn failure_repro_pass_gate_value(
    verification: &FailureReproVerification,
    diagnostic_only: bool,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("multinode-failure-repro-pass-gate-v1", vec![
        string(MULTINODE_FAILURE_REPRO_PASS_GATE_SCHEMA),
        record("decision", vec![string(decision)]),
        record("verification", vec![string(&verification.verification_ref)]),
        record("payload", vec![string(&verification.payload_ref)]),
        record("diagnostic-only", vec![bool_value(diagnostic_only)]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        checks_value(&[
            ("verified-before-use", status(verification.decision == PASS_DECISION)),
            ("diagnostic-bundle-not-pass", status(!diagnostic_only)),
        ]),
    ]))
}

fn live_transport_vm_gate_value(
    input: &LiveTransportVmEvidenceInput,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("nixos-vm-live-transport-gate-v1", vec![
        string(LIVE_TRANSPORT_VM_GATE_SCHEMA),
        record("decision", vec![string(decision)]),
        record("sender", vec![record("node", vec![string(&input.actual_sender_node)])]),
        record("receiver", vec![record("node", vec![string(&input.actual_receiver_node)])]),
        record("peer", vec![string(&input.actual_peer)]),
        record("topic", vec![string(&input.topic)]),
        record("operation", vec![string(&input.operation_id)]),
        record("ticket", vec![string(&input.ticket_ref)]),
        record("peer-admission", vec![string(&input.peer_admission_ref)]),
        record("authority", vec![string(&input.authority_ref)]),
        record("send", vec![string(&input.send_ref)]),
        record("receive", vec![string(&input.receive_ref)]),
        record("ingress", vec![string(&input.ingress_ref)]),
        record("queue", vec![string(&input.queue_ref)]),
        record("dispatch", vec![string(&input.dispatch_ref)]),
        record("reconcile", vec![string(&input.reconcile_ref)]),
        record("ack", vec![string(&input.ack_ref)]),
        record("protocol-gate", vec![string(&input.protocol_gate_ref)]),
        record("logs", vec![refs_sequence(&input.log_refs)]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        record("caveats", vec![strings_sequence(&input.caveats)]),
        checks_value(&[
            ("receive-receipt-bound", status(!input.receive_ref.trim().is_empty())),
            ("protocol-gate-bound", status(!input.protocol_gate_ref.trim().is_empty())),
            ("logs-diagnostic-only", PASS_DECISION),
            ("vm-topology-scoped", PASS_DECISION),
        ]),
    ]))
}

fn vm_fault_support_matrix_value(
    cases: &[VmFaultSupportCase],
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("nixos-vm-fault-support-matrix-v1", vec![
        string(VM_FAULT_SUPPORT_MATRIX_SCHEMA),
        record("decision", vec![string(decision)]),
        record("cases", vec![sequence(cases.iter().map(vm_fault_case_value).collect::<Vec<_>>())]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        checks_value(&[
            ("support-status-explicit", status(!cases.is_empty())),
            ("unsupported-is-not-pass", status(!diagnostics.iter().any(|item| item.contains("unsupported-pass")))),
            ("canonical-diagnostics-required", PASS_DECISION),
        ]),
    ]))
}

fn vm_fault_case_value(case: &VmFaultSupportCase) -> IoValue {
    record("fault", vec![
        record("kind", vec![string(&case.fault_kind)]),
        record("capability", vec![string(&case.required_capability)]),
        record("target", vec![string(&case.target)]),
        record("command-profile", vec![string(&case.command_profile)]),
        record("expected-outcome", vec![string(&case.expected_outcome)]),
        record("host-support", vec![string(&case.host_support)]),
        record("preflight", vec![refs_sequence(&case.preflight_refs)]),
        record("injection", vec![refs_sequence(&case.injection_refs)]),
        record("children", vec![refs_sequence(&case.child_refs)]),
        record("post-fault", vec![refs_sequence(&case.post_fault_refs)]),
        record("diagnostics", vec![refs_sequence(&case.diagnostic_refs)]),
        record("caveats", vec![strings_sequence(&case.caveats)]),
    ])
}

fn topology_role_values(roles: &[TopologyRole]) -> Vec<IoValue> {
    roles
        .iter()
        .map(|role_item| {
            record("role", vec![
                record("node", vec![string(&role_item.node_id)]),
                record("role", vec![string(&role_item.role)]),
                record("membership", vec![string(&role_item.membership)]),
            ])
        })
        .collect()
}

