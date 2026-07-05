fn scenario_metadata_value(input: &ScenarioMetadataValueInput<'_>) -> Result<IoValue> {
    Ok(record("multinode-scenario-metadata-v1", vec![
        string(MULTINODE_SCENARIO_METADATA_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("fixture", vec![string(input.fixture_ref)]),
        record("topology-profile-ref", vec![string(input.topology_profile_ref)]),
        record("scenario-id", vec![string(&input.fixture.scenario_id)]),
        record("execution-profile", vec![string(&input.fixture.execution_profile_id)]),
        record("command", vec![string(&input.fixture.command_surface)]),
        record("artifact-kinds", vec![strings_sequence(&input.fixture.expected_artifact_kinds)]),
        record("receipts", vec![refs_sequence(&input.fixture.receipt_refs)]),
        record("variance", vec![refs_sequence(&input.fixture.variance_refs)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            ("fixture-ref-bound", status(input.decision == PASS_DECISION)),
            ("metadata-derived-without-ambient-state", PASS_DECISION),
            ("unsupported-execution-not-pass", status(!input.fixture.unsupported_claims_pass)),
        ]),
    ]))
}

fn topology_profile_value(profile: &TopologyProfile) -> Result<IoValue> {
    Ok(record("multinode-topology-profile-v1", vec![
        string(MULTINODE_TOPOLOGY_PROFILE_SCHEMA),
        record("id", vec![string(&profile.id)]),
        record("roles", vec![sequence(topology_role_values(&profile.roles))]),
        record("links", vec![sequence(topology_link_values(&profile.allowed_links))]),
        record("evidence-scope", vec![string(&profile.evidence_scope)]),
        record("required-receipts", vec![strings_sequence(&profile.required_receipt_kinds)]),
        record("caveats", vec![strings_sequence(&profile.caveats)]),
        checks_value(&[
            ("roles-explicit", status(!profile.roles.is_empty())),
            ("links-explicit", status(!profile.allowed_links.is_empty())),
            ("transport-not-authority", PASS_DECISION),
        ]),
    ]))
}

fn topology_matrix_value(
    decision: &str,
    profiles: &[TopologyProfile],
    profile_refs: &[String],
    diagnostics: &[String],
) -> Result<IoValue> {
    let profile_ids = profiles.iter().map(|profile| profile.id.clone()).collect::<Vec<_>>();
    Ok(record("multinode-topology-profile-matrix-v1", vec![
        string(MULTINODE_TOPOLOGY_MATRIX_SCHEMA),
        record("decision", vec![string(decision)]),
        record("profile-ids", vec![strings_sequence(&profile_ids)]),
        record("profiles", vec![refs_sequence(profile_refs)]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        checks_value(&[
            ("required-profiles-present", status(decision == PASS_DECISION)),
            ("role-membership-explicit", PASS_DECISION),
        ]),
    ]))
}

fn topology_membership_gate_value(
    decision: &str,
    profile: &TopologyProfile,
    claim: &TopologyMembershipClaim,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("multinode-topology-membership-gate-v1", vec![
        string(MULTINODE_TOPOLOGY_MEMBERSHIP_GATE_SCHEMA),
        record("decision", vec![string(decision)]),
        record("profile", vec![string(&profile.id)]),
        record("topology", vec![string(&claim.topology_ref)]),
        record("scenario-topology", vec![string(&claim.scenario_topology_ref)]),
        record("claimed-roles", vec![sequence(topology_role_values(&claim.node_roles))]),
        record("quorum", vec![optional_ref_value(claim.quorum_ref.as_deref())]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        record("caveats", vec![strings_sequence(&claim.caveats)]),
        checks_value(&[
            ("subscriber-not-voter", status(!diagnostics.iter().any(|item| item.contains("subscriber")))),
            ("wrong-topology-denies", status(!diagnostics.iter().any(|item| item == "wrong-topology"))),
            ("transport-not-authority", status(!claim.transport_only_authority_claim)),
        ]),
    ]))
}

fn reconciliation_gate_value(input: &ReconciliationInput, decision: &str, diagnostics: &[String]) -> Result<IoValue> {
    Ok(record("multinode-reconciliation-gate-v1", vec![
        string(MULTINODE_RECONCILIATION_SCHEMA),
        record("decision", vec![string(decision)]),
        record("topology", vec![string(&input.topology_ref)]),
        record("scenario-fixture", vec![string(&input.scenario_fixture_ref)]),
        record("required-receipts", vec![refs_sequence(&input.required_receipt_refs)]),
        record("node-summaries", vec![sequence(node_summary_values(&input.node_summaries))]),
        record("equality", vec![sequence(equality_class_values(&input.equality_classes))]),
        record("allowed-variance", vec![refs_sequence(&input.allowed_variance_refs)]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        record("caveats", vec![strings_sequence(&input.caveats)]),
        checks_value(&[
            ("node-summaries-bound", status(!input.node_summaries.is_empty())),
            ("logs-diagnostic-only", PASS_DECISION),
            ("undeclared-drift-denies", status(decision == PASS_DECISION)),
        ]),
    ]))
}

fn local_multiprocess_plan_value(
    input: &LocalMultiprocessPlanInput,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("local-multiprocess-plan-v1", vec![
        string(LOCAL_MULTIPROCESS_PLAN_SCHEMA),
        record("decision", vec![string(decision)]),
        record("fixture", vec![string(&input.fixture_ref)]),
        record("nodes", vec![sequence(local_process_node_values(&input.nodes))]),
        record("command-plan", vec![string(&input.command_plan_ref)]),
        record("expected-receipts", vec![refs_sequence(&input.expected_receipt_refs)]),
        record("cleanup-policy", vec![string(&input.cleanup_policy)]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        record("caveats", vec![strings_sequence(&input.caveats)]),
        checks_value(&[
            (
                "state-roots-isolated",
                status(!diagnostics.iter().any(|item| item.contains("state-root-collision"))),
            ),
            ("transports-isolated", status(!diagnostics.iter().any(|item| item.contains("transport-collision")))),
            ("cleanup-policy-declared", status(input.cleanup_policy == CLEANUP_POLICY_REQUIRED)),
        ]),
    ]))
}

fn local_multiprocess_run_value(
    input: &LocalMultiprocessRunInput,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("local-multiprocess-run-v1", vec![
        string(LOCAL_MULTIPROCESS_RUN_SCHEMA),
        record("decision", vec![string(decision)]),
        record("plan", vec![string(&input.plan_ref)]),
        record("startup", vec![refs_sequence(&input.startup_refs)]),
        record("workflow", vec![refs_sequence(&input.workflow_refs)]),
        record("shutdown", vec![refs_sequence(&input.shutdown_refs)]),
        record("cleanup", vec![refs_sequence(&input.cleanup_refs)]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        record("caveats", vec![strings_sequence(&input.caveats)]),
        checks_value(&[
            ("process-receipts-bound", status(decision == PASS_DECISION)),
            ("local-evidence-not-vm-evidence", PASS_DECISION),
            ("cleanup-recorded", status(!input.cleanup_refs.is_empty())),
        ]),
    ]))
}

fn local_multiprocess_executable_run_value(
    input: &LocalMultiprocessExecutableRunInput,
    plan_ref: &str,
    run_ref: &str,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("local-multiprocess-executable-run-v1", vec![
        string(LOCAL_MULTIPROCESS_EXECUTABLE_RUN_SCHEMA),
        record("decision", vec![string(decision)]),
        record("plan", vec![string(plan_ref)]),
        record("run", vec![string(run_ref)]),
        record("startup", vec![refs_sequence(&input.startup_refs)]),
        record("workflow", vec![refs_sequence(&input.workflow_refs)]),
        record("shutdown", vec![refs_sequence(&input.shutdown_refs)]),
        record("cleanup", vec![refs_sequence(&input.cleanup_refs)]),
        record("ticket-status", vec![string(&input.ticket_status)]),
        record("child-timed-out", vec![bool_value(input.child_timed_out)]),
        record("orphaned-processes", vec![strings_sequence(&input.orphaned_processes)]),
        record("cleanup-succeeded", vec![bool_value(input.cleanup_succeeded)]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        record("caveats", vec![strings_sequence(&input.caveats)]),
        checks_value(&[
            ("thin-shell-observations-bound", status(decision == PASS_DECISION)),
            ("stale-ticket-denies", status(input.ticket_status == TICKET_STATUS_CURRENT)),
            ("orphaned-process-denies", status(input.orphaned_processes.is_empty())),
            ("local-evidence-not-vm-evidence", PASS_DECISION),
        ]),
    ]))
}

fn three_node_quorum_gate_value(
    input: &ThreeNodeQuorumEvidenceInput,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("three-node-quorum-gate-v1", vec![
        string(THREE_NODE_QUORUM_GATE_SCHEMA),
        record("decision", vec![string(decision)]),
        record("topology", vec![string(&input.topology_ref)]),
        record("scenario-fixture", vec![string(&input.scenario_fixture_ref)]),
        record("membership-gate", vec![string(&input.membership_gate_ref)]),
        record("reconciliation-gate", vec![string(&input.reconciliation_gate_ref)]),
        record("node-summaries", vec![refs_sequence(&input.node_summary_refs)]),
        record("quorum", vec![refs_sequence(&input.quorum_refs)]),
        record("restarting-member", vec![string(&input.restarting_member)]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        record("caveats", vec![strings_sequence(&input.caveats)]),
        checks_value(&[
            ("quorum-refs-bound", status(!input.quorum_refs.is_empty())),
            ("duplicate-commit-denies", status(!input.duplicate_semantic_commit)),
            ("log-only-quorum-denies", status(!input.log_only_quorum)),
            ("topology-scoped", PASS_DECISION),
        ]),
    ]))
}

fn vm_scenario_gate_value(input: &VmScenarioGateInput, decision: &str, diagnostics: &[String]) -> Result<IoValue> {
    Ok(record("vm-scenario-gate-v1", vec![
        string(VM_SCENARIO_GATE_SCHEMA),
        record("decision", vec![string(decision)]),
        record("scenario-metadata", vec![string(&input.scenario_metadata_ref)]),
        record("topology-membership-gate", vec![string(&input.topology_membership_gate_ref)]),
        record("reconciliation-gate", vec![string(&input.reconciliation_gate_ref)]),
        record("live-transport-gate", vec![optional_ref_value(input.live_transport_gate_ref.as_deref())]),
        record("expected-artifacts", vec![strings_sequence(&input.expected_artifact_kinds)]),
        record("observed-artifacts", vec![strings_sequence(&input.observed_artifact_kinds)]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        record("caveats", vec![strings_sequence(&input.caveats)]),
        checks_value(&[
            ("scenario-metadata-bound", status(decision == PASS_DECISION)),
            ("reconciliation-gate-required", status(!input.log_only_reconciliation)),
            ("unsupported-pass-denies", status(!input.unsupported_pass_claim)),
            ("logs-diagnostic-only", PASS_DECISION),
        ]),
    ]))
}

fn vm_failure_repro_bundle_input(input: &VmFailureReproExportInput) -> FailureReproBundleInput {
    let mut receipt_refs = input.child_receipt_refs.clone();
    receipt_refs.extend(input.validation_refs.clone());
    FailureReproBundleInput {
        scenario_fixture_ref: input.scenario_fixture_ref.clone(),
        topology_ref: input.topology_ref.clone(),
        scheduler_ref: input.scheduler_ref.clone(),
        seed_ref: input.seed_ref.clone(),
        fault_plan_ref: input.fault_plan_ref.clone(),
        command_refs: input.command_refs.clone(),
        node_summary_refs: input.node_summary_refs.clone(),
        receipt_refs,
        diagnostic_refs: input.validation_refs.clone(),
        log_refs: input.diagnostic_log_refs.clone(),
        redaction_policy_ref: input.redaction_policy_ref.clone(),
        replay_status: NON_REPLAYABLE_VM.to_string(),
        diagnostic_only: true,
        sealed: true,
        private_attachment_refs: input.private_attachment_refs.clone(),
        reveal_receipt_refs: input.reveal_receipt_refs.clone(),
        claimed_payload_ref: None,
        caveats: input.caveats.clone(),
    }
}

