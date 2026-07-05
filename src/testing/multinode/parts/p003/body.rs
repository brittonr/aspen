pub fn evaluate_three_node_quorum_evidence(input: &ThreeNodeQuorumEvidenceInput) -> Result<ThreeNodeQuorumGate> {
    let mut diagnostics = Vec::new();
    collect_invalid_ref_diagnostics(
        "three-node quorum root",
        &[
            input.topology_ref.clone(),
            input.scenario_fixture_ref.clone(),
            input.membership_gate_ref.clone(),
            input.reconciliation_gate_ref.clone(),
        ],
        &mut diagnostics,
    )?;
    collect_invalid_ref_diagnostics("three-node node summary", &input.node_summary_refs, &mut diagnostics)?;
    collect_invalid_ref_diagnostics("three-node quorum", &input.quorum_refs, &mut diagnostics)?;
    collect_required_text_diagnostic("three-node-restarting-member", &input.restarting_member, &mut diagnostics)?;
    push_if(&mut diagnostics, input.node_summary_refs.is_empty(), "three-node-missing-node-summaries")?;
    push_if(&mut diagnostics, input.quorum_refs.is_empty(), "three-node-missing-quorum-refs")?;
    push_if(&mut diagnostics, input.duplicate_semantic_commit, "three-node-duplicate-semantic-commit")?;
    push_if(&mut diagnostics, input.log_only_quorum, "three-node-log-only-quorum")?;
    push_if(&mut diagnostics, input.caveats.is_empty(), "three-node-missing-evidence-caveat")?;
    let decision = decision_from_diagnostics(&diagnostics).to_string();
    let value = three_node_quorum_gate_value(input, &decision, &diagnostics)?;
    let gate_ref = canonical_hash(&value)?;
    Ok(ThreeNodeQuorumGate {
        decision,
        diagnostics,
        gate_ref,
        value,
    })
}

pub fn evaluate_vm_scenario_gate(input: &VmScenarioGateInput) -> Result<VmScenarioGate> {
    let mut diagnostics = Vec::new();
    collect_invalid_ref_diagnostics(
        "VM scenario gate",
        &[
            input.scenario_metadata_ref.clone(),
            input.topology_membership_gate_ref.clone(),
            input.reconciliation_gate_ref.clone(),
        ],
        &mut diagnostics,
    )?;
    collect_invalid_optional_ref_diagnostics(
        "VM scenario live transport gate",
        input.live_transport_gate_ref.as_deref(),
        &mut diagnostics,
    )?;
    push_if(&mut diagnostics, input.expected_artifact_kinds.is_empty(), "vm-scenario-missing-expected-artifacts")?;
    push_if(&mut diagnostics, input.observed_artifact_kinds.is_empty(), "vm-scenario-missing-observed-artifacts")?;
    if input.expected_artifact_kinds != input.observed_artifact_kinds {
        push_diagnostic(&mut diagnostics, "vm-scenario-artifact-kind-mismatch".to_string())?;
    }
    push_if(&mut diagnostics, input.unsupported_pass_claim, "vm-scenario-unsupported-pass-claim")?;
    push_if(&mut diagnostics, input.log_only_reconciliation, "vm-scenario-log-only-reconciliation")?;
    push_if(&mut diagnostics, input.caveats.is_empty(), "vm-scenario-missing-evidence-caveat")?;
    let decision = decision_from_diagnostics(&diagnostics).to_string();
    let value = vm_scenario_gate_value(input, &decision, &diagnostics)?;
    let gate_ref = canonical_hash(&value)?;
    Ok(VmScenarioGate {
        decision,
        diagnostics,
        gate_ref,
        value,
    })
}

pub fn export_vm_failure_repro(input: &VmFailureReproExportInput) -> Result<VmFailureReproExport> {
    let bundle_input = vm_failure_repro_bundle_input(input);
    let bundle = build_failure_repro_bundle(&bundle_input)?;
    let verification = verify_failure_repro_bundle(&bundle_input)?;
    let mut diagnostics = verification.diagnostics.clone();
    if !input.unavailable_host_support && !input.denied_or_failed_validation {
        push_diagnostic(&mut diagnostics, "vm-failure-repro-missing-failure-condition".to_string())?;
    }
    if input.caveats.is_empty() {
        push_diagnostic(&mut diagnostics, "vm-failure-repro-missing-caveat".to_string())?;
    }
    diagnostics.sort();
    diagnostics.dedup();
    let decision = decision_from_diagnostics(&diagnostics).to_string();
    let value = vm_failure_repro_export_value(
        input,
        &bundle.bundle_ref,
        &verification.verification_ref,
        &decision,
        &diagnostics,
    )?;
    let export_ref = canonical_hash(&value)?;
    Ok(VmFailureReproExport {
        decision,
        diagnostics,
        bundle_ref: bundle.bundle_ref,
        verification_ref: verification.verification_ref,
        export_ref,
        value,
    })
}

pub fn run_generated_distributed_case(case: &GeneratedDistributedCase) -> Result<GeneratedDistributedRepro> {
    let case_value = generated_case_value(case)?;
    let case_ref = canonical_hash(&case_value)?;
    let first = crate::distributed_core::run_simulation(&case.simulation)?;
    let replay = crate::distributed_core::run_simulation(&case.simulation)?;
    let mut diagnostics = Vec::new();
    push_if(&mut diagnostics, first.receipt_ref != replay.receipt_ref, "generated-replay-run-ref-mismatch")?;
    push_if(
        &mut diagnostics,
        first.final_state_ref != replay.final_state_ref,
        "generated-replay-final-state-mismatch",
    )?;
    push_if(&mut diagnostics, first.event_refs != replay.event_refs, "generated-replay-event-ref-mismatch")?;
    let decision = decision_from_diagnostics(&diagnostics).to_string();
    let value = generated_repro_value(&GeneratedReproValueInput {
        case,
        case_ref: &case_ref,
        first: &first,
        replay: &replay,
        decision: &decision,
        diagnostics: &diagnostics,
    })?;
    let repro_ref = canonical_hash(&value)?;
    Ok(GeneratedDistributedRepro {
        decision,
        diagnostics,
        case_ref,
        run_ref: first.receipt_ref,
        replay_run_ref: replay.receipt_ref,
        repro_ref,
        value,
    })
}

pub fn build_failure_repro_bundle(input: &FailureReproBundleInput) -> Result<FailureReproBundle> {
    let payload = failure_repro_payload_value(input)?;
    let payload_ref = canonical_hash(&payload)?;
    let claimed_payload_ref = input.claimed_payload_ref.as_deref().unwrap_or(payload_ref.as_str());
    let value = failure_repro_bundle_value(input, &payload, claimed_payload_ref)?;
    let bundle_ref = canonical_hash(&value)?;
    Ok(FailureReproBundle {
        payload_ref,
        bundle_ref,
        value,
    })
}

pub fn verify_failure_repro_bundle(input: &FailureReproBundleInput) -> Result<FailureReproVerification> {
    let payload = failure_repro_payload_value(input)?;
    let payload_ref = canonical_hash(&payload)?;
    let mut diagnostics = Vec::new();
    collect_failure_repro_diagnostics(input, &payload_ref, &mut diagnostics)?;
    diagnostics.sort();
    diagnostics.dedup();
    let decision = decision_from_diagnostics(&diagnostics).to_string();
    let value = failure_repro_verify_value(input, &payload_ref, &decision, &diagnostics)?;
    let verification_ref = canonical_hash(&value)?;
    Ok(FailureReproVerification {
        decision,
        diagnostics,
        payload_ref,
        verification_ref,
        value,
    })
}

pub fn gate_failure_repro_as_pass(
    verification: &FailureReproVerification,
    diagnostic_only: bool,
) -> Result<FailureReproPassGate> {
    let mut diagnostics = Vec::new();
    if verification.decision != PASS_DECISION {
        push_diagnostic(&mut diagnostics, "repro-verification-not-pass".to_string())?;
    }
    if diagnostic_only {
        push_diagnostic(&mut diagnostics, "diagnostic-bundle-cannot-satisfy-pass".to_string())?;
    }
    let decision = decision_from_diagnostics(&diagnostics).to_string();
    let value = failure_repro_pass_gate_value(verification, diagnostic_only, &decision, &diagnostics)?;
    let gate_ref = canonical_hash(&value)?;
    Ok(FailureReproPassGate {
        decision,
        diagnostics,
        gate_ref,
        value,
    })
}

pub fn evaluate_live_transport_vm_gate(input: &LiveTransportVmEvidenceInput) -> Result<LiveTransportVmGate> {
    let mut diagnostics = Vec::new();
    push_if(
        &mut diagnostics,
        input.expected_sender_node != input.actual_sender_node,
        "live-transport-sender-node-mismatch",
    )?;
    push_if(
        &mut diagnostics,
        input.expected_receiver_node != input.actual_receiver_node,
        "live-transport-receiver-node-mismatch",
    )?;
    push_if(&mut diagnostics, input.expected_peer != input.actual_peer, "live-transport-peer-mismatch")?;
    push_if(&mut diagnostics, input.topic.trim().is_empty(), "live-transport-missing-topic")?;
    push_if(&mut diagnostics, input.operation_id.trim().is_empty(), "live-transport-missing-operation-id")?;
    collect_invalid_ref_diagnostics("live transport", &live_transport_refs(input), &mut diagnostics)?;
    push_if(&mut diagnostics, input.ticket_ref.trim().is_empty(), "live-transport-missing-ticket")?;
    push_if(&mut diagnostics, input.receive_ref.trim().is_empty(), "live-transport-missing-receive")?;
    push_if(&mut diagnostics, input.protocol_gate_ref.trim().is_empty(), "live-transport-missing-protocol-gate")?;
    push_if(&mut diagnostics, input.log_refs.is_empty(), "live-transport-missing-diagnostic-logs")?;
    push_if(&mut diagnostics, input.caveats.is_empty(), "live-transport-missing-scope-caveat")?;
    let decision = decision_from_diagnostics(&diagnostics).to_string();
    let value = live_transport_vm_gate_value(input, &decision, &diagnostics)?;
    let gate_ref = canonical_hash(&value)?;
    Ok(LiveTransportVmGate {
        decision,
        diagnostics,
        gate_ref,
        value,
    })
}

pub fn build_vm_fault_support_matrix(cases: &[VmFaultSupportCase]) -> Result<VmFaultSupportMatrix> {
    let mut diagnostics = Vec::new();
    ensure_count_at_most(cases.len(), MAX_MULTINODE_ITEMS, "VM fault support cases")?;
    push_if(&mut diagnostics, cases.is_empty(), "vm-fault-support-matrix-empty")?;
    for case in cases {
        collect_vm_fault_support_diagnostics(case, &mut diagnostics)?;
    }
    diagnostics.sort();
    diagnostics.dedup();
    let decision = decision_from_diagnostics(&diagnostics).to_string();
    let value = vm_fault_support_matrix_value(cases, &decision, &diagnostics)?;
    let matrix_ref = canonical_hash(&value)?;
    Ok(VmFaultSupportMatrix {
        decision,
        diagnostics,
        matrix_ref,
        value,
    })
}

struct ScenarioMetadataValueInput<'a> {
    decision: &'a str,
    diagnostics: &'a [String],
    fixture_ref: &'a str,
    topology_profile_ref: &'a str,
    fixture: &'a ScenarioFixture,
}

fn scenario_fixture_diagnostics(
    fixture: &ScenarioFixture,
    execution_profiles: &[crate::distributed_core::CiProfile],
    topology_profiles: &[TopologyProfile],
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    collect_required_text_diagnostic("scenario-id", &fixture.scenario_id, &mut diagnostics)?;
    collect_required_text_diagnostic("purpose", &fixture.purpose, &mut diagnostics)?;
    collect_required_text_diagnostic("evidence-scope", &fixture.evidence_scope, &mut diagnostics)?;
    collect_required_text_diagnostic("topology-profile", &fixture.topology_profile_id, &mut diagnostics)?;
    collect_required_text_diagnostic("execution-profile", &fixture.execution_profile_id, &mut diagnostics)?;
    collect_required_text_diagnostic("command-surface", &fixture.command_surface, &mut diagnostics)?;
    collect_required_text_diagnostic("unavailable-policy", &fixture.unavailable_policy, &mut diagnostics)?;
    push_if(&mut diagnostics, fixture.expected_artifact_kinds.is_empty(), "fixture-missing-artifact-kind")?;
    push_if(&mut diagnostics, fixture.receipt_refs.is_empty(), "fixture-missing-receipt-ref")?;
    push_if(&mut diagnostics, fixture.variance_refs.is_empty(), "fixture-missing-variance-ref")?;
    push_if(&mut diagnostics, fixture.diagnostic_log_refs.is_empty(), "fixture-missing-diagnostic-log-ref")?;
    push_if(&mut diagnostics, fixture.caveats.is_empty(), "fixture-missing-evidence-caveat")?;
    if fixture.unsupported_claims_pass {
        push_diagnostic(&mut diagnostics, "fixture-unsupported-pass-claim".to_string())?;
    }
    collect_invalid_ref_diagnostics(
        "scenario fixture",
        &[
            fixture.topology_ref.clone(),
            fixture.seed_ref.clone(),
            fixture.fault_plan_ref.clone(),
        ],
        &mut diagnostics,
    )?;
    collect_invalid_ref_diagnostics("scenario receipt", &fixture.receipt_refs, &mut diagnostics)?;
    collect_invalid_ref_diagnostics("scenario variance", &fixture.variance_refs, &mut diagnostics)?;
    collect_invalid_ref_diagnostics("scenario diagnostic log", &fixture.diagnostic_log_refs, &mut diagnostics)?;
    match execution_profiles.iter().find(|profile| profile.id == fixture.execution_profile_id) {
        Some(profile) => {
            if fixture.command_surface != profile.command {
                push_diagnostic(&mut diagnostics, "fixture-command-profile-mismatch".to_string())?;
            }
            if fixture.expected_artifact_kinds != profile.expected_artifact_kinds {
                push_diagnostic(&mut diagnostics, "fixture-artifact-kind-mismatch".to_string())?;
            }
        }
        None => push_diagnostic(&mut diagnostics, "fixture-execution-profile-missing".to_string())?,
    }
    if !topology_profiles.iter().any(|profile| profile.id == fixture.topology_profile_id) {
        push_diagnostic(&mut diagnostics, "fixture-topology-profile-missing".to_string())?;
    }
    Ok(diagnostics)
}

