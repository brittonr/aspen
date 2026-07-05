fn collect_topology_profile_diagnostics(
    profile: &TopologyProfile,
    diagnostics: &mut impl DiagnosticSink,
) -> Result<()> {
    collect_required_text_diagnostic("topology-profile-id", &profile.id, diagnostics)?;
    collect_required_text_diagnostic("topology-profile-scope", &profile.evidence_scope, diagnostics)?;
    push_if(diagnostics, profile.roles.is_empty(), "topology-profile-missing-roles")?;
    push_if(diagnostics, profile.allowed_links.is_empty(), "topology-profile-missing-links")?;
    push_if(diagnostics, profile.required_receipt_kinds.is_empty(), "topology-profile-missing-required-receipts")?;
    push_if(diagnostics, profile.caveats.is_empty(), "topology-profile-missing-caveats")?;
    let mut role_nodes = OrderedSet::new();
    for profile_role in &profile.roles {
        collect_required_text_diagnostic("topology-role-node", &profile_role.node_id, diagnostics)?;
        collect_required_text_diagnostic("topology-role", &profile_role.role, diagnostics)?;
        validate_membership(&profile_role.membership, diagnostics)?;
        if !role_nodes.insert(profile_role.node_id.as_str()) {
            push_diagnostic(diagnostics, format!("duplicate-topology-role-node:{}", profile_role.node_id))?;
        }
    }
    for profile_link in &profile.allowed_links {
        collect_required_text_diagnostic("topology-link-from", &profile_link.from, diagnostics)?;
        collect_required_text_diagnostic("topology-link-to", &profile_link.to, diagnostics)?;
        collect_required_text_diagnostic("topology-link-topic", &profile_link.topic, diagnostics)?;
    }
    Ok(())
}

fn validate_membership(membership: &str, diagnostics: &mut impl DiagnosticSink) -> Result<()> {
    match membership {
        MEMBERSHIP_VOTER | MEMBERSHIP_SUBSCRIBER | MEMBERSHIP_TRANSPORT_ONLY => Ok(()),
        _ => push_diagnostic(diagnostics, format!("unsupported-topology-membership:{membership}")),
    }
}

fn required_topology_profiles() -> [&'static str; REQUIRED_DEFAULT_TOPOLOGY_PROFILE_COUNT] {
    [
        PROFILE_PAIRWISE_TRANSPORT,
        PROFILE_CONTROL_QUORUM,
        PROFILE_RESTART_REJOIN,
        PROFILE_SUBSCRIBER_PEER,
        PROFILE_THREE_NODE_QUORUM,
        PROFILE_WRONG_TOPOLOGY,
    ]
}

fn reconciliation_diagnostics(input: &ReconciliationInput) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    collect_invalid_ref_diagnostics(
        "reconciliation root",
        &[input.topology_ref.clone(), input.scenario_fixture_ref.clone()],
        &mut diagnostics,
    )?;
    collect_invalid_ref_diagnostics("reconciliation required receipt", &input.required_receipt_refs, &mut diagnostics)?;
    collect_invalid_ref_diagnostics("reconciliation variance", &input.allowed_variance_refs, &mut diagnostics)?;
    push_if(&mut diagnostics, input.node_summaries.is_empty(), "reconciliation-missing-node-summaries")?;
    push_if(&mut diagnostics, input.required_receipt_refs.is_empty(), "reconciliation-log-only-claim")?;
    push_if(&mut diagnostics, input.caveats.is_empty(), "reconciliation-missing-evidence-caveat")?;
    let mut available_receipts = OrderedSet::new();
    let mut commits = OrderedMap::<String, OrderedSet<String>>::new();
    for summary in &input.node_summaries {
        collect_node_summary_diagnostics(input, summary, &mut diagnostics)?;
        for receipt_ref in &summary.receipt_refs {
            available_receipts.insert(receipt_ref.as_str());
        }
        for commit in &summary.semantic_commits {
            commits.entry(commit.operation_id.clone()).or_default().insert(commit.commit_ref.clone());
        }
    }
    for required_ref in &input.required_receipt_refs {
        if !available_receipts.contains(required_ref.as_str()) {
            push_diagnostic(&mut diagnostics, format!("required-receipt-missing:{required_ref}"))?;
        }
    }
    for equality in &input.equality_classes {
        collect_equality_class_diagnostics(input, equality, &mut diagnostics)?;
    }
    for (operation_id, commit_refs) in commits {
        if commit_refs.len() > 1 {
            push_diagnostic(&mut diagnostics, format!("duplicate-semantic-commit:{operation_id}"))?;
        }
    }
    Ok(diagnostics)
}

fn collect_node_summary_diagnostics(
    input: &ReconciliationInput,
    summary: &NodeSummary,
    diagnostics: &mut impl DiagnosticSink,
) -> Result<()> {
    collect_required_text_diagnostic("node-summary-node", &summary.node_id, diagnostics)?;
    if summary.topology_ref != input.topology_ref {
        push_diagnostic(diagnostics, format!("node-summary-topology-mismatch:{}", summary.node_id))?;
    }
    if summary.scenario_fixture_ref != input.scenario_fixture_ref {
        push_diagnostic(diagnostics, format!("node-summary-scenario-mismatch:{}", summary.node_id))?;
    }
    collect_invalid_ref_diagnostics(
        "node summary",
        &[
            summary.topology_ref.clone(),
            summary.scenario_fixture_ref.clone(),
            summary.queue_ref.clone(),
            summary.ledger_ref.clone(),
            summary.dispatch_ref.clone(),
            summary.ack_ref.clone(),
            summary.protocol_ref.clone(),
        ],
        diagnostics,
    )?;
    collect_invalid_ref_diagnostics("node summary receipt", &summary.receipt_refs, diagnostics)?;
    collect_invalid_ref_diagnostics("node summary log", &summary.diagnostic_log_refs, diagnostics)?;
    push_if(diagnostics, summary.receipt_refs.is_empty(), "node-summary-missing-receipts")?;
    push_if(diagnostics, summary.diagnostic_log_refs.is_empty(), "node-summary-missing-diagnostic-logs")?;
    let mut node_commits = OrderedSet::new();
    for commit in &summary.semantic_commits {
        collect_required_text_diagnostic("semantic-commit-operation", &commit.operation_id, diagnostics)?;
        collect_invalid_ref_diagnostics("semantic commit", &[commit.commit_ref.clone()], diagnostics)?;
        if !node_commits.insert(commit.operation_id.as_str()) {
            push_diagnostic(diagnostics, format!("duplicate-node-semantic-commit:{}", commit.operation_id))?;
        }
    }
    Ok(())
}

fn collect_equality_class_diagnostics(
    input: &ReconciliationInput,
    equality: &ReconciliationEqualityClass,
    diagnostics: &mut impl DiagnosticSink,
) -> Result<()> {
    collect_required_text_diagnostic("equality-class", &equality.name, diagnostics)?;
    collect_invalid_ref_diagnostics("equality class", &equality.refs, diagnostics)?;
    collect_invalid_optional_ref_diagnostics("equality variance", equality.variance_ref.as_deref(), diagnostics)?;
    let distinct_refs = equality.refs.iter().map(String::as_str).collect::<OrderedSet<_>>();
    if distinct_refs.len() > 1 {
        match equality.variance_ref.as_deref() {
            Some(variance_ref) if input.allowed_variance_refs.iter().any(|allowed| allowed == variance_ref) => Ok(()),
            _ => push_diagnostic(diagnostics, format!("divergent-ref-class:{}", equality.name)),
        }?;
    }
    Ok(())
}

fn collect_process_plan_collisions(
    input: &LocalMultiprocessPlanInput,
    diagnostics: &mut impl DiagnosticSink,
) -> Result<()> {
    let mut nodes = OrderedSet::new();
    let mut state_roots = OrderedMap::<&str, &str>::new();
    let mut transports = OrderedMap::<&str, &str>::new();
    for node in &input.nodes {
        collect_required_text_diagnostic("local node", &node.node_id, diagnostics)?;
        collect_required_text_diagnostic("local state root", &node.state_root_handle, diagnostics)?;
        collect_required_text_diagnostic("local transport", &node.transport_handle, diagnostics)?;
        if !nodes.insert(node.node_id.as_str()) {
            push_diagnostic(diagnostics, format!("local-plan-duplicate-node:{}", node.node_id))?;
        }
        if let Some(existing) = state_roots.insert(node.state_root_handle.as_str(), node.node_id.as_str()) {
            push_diagnostic(diagnostics, format!("local-plan-state-root-collision:{existing}:{}", node.node_id))?;
        }
        if let Some(existing) = transports.insert(node.transport_handle.as_str(), node.node_id.as_str()) {
            push_diagnostic(diagnostics, format!("local-plan-transport-collision:{existing}:{}", node.node_id))?;
        }
    }
    Ok(())
}

fn collect_failure_repro_diagnostics(
    input: &FailureReproBundleInput,
    payload_ref: &str,
    diagnostics: &mut impl DiagnosticSink,
) -> Result<()> {
    let all_root_refs = [
        input.scenario_fixture_ref.clone(),
        input.topology_ref.clone(),
        input.scheduler_ref.clone(),
        input.seed_ref.clone(),
        input.fault_plan_ref.clone(),
        input.redaction_policy_ref.clone(),
    ];
    collect_invalid_ref_diagnostics("failure repro root", &all_root_refs, diagnostics)?;
    collect_invalid_ref_diagnostics("failure repro command", &input.command_refs, diagnostics)?;
    collect_invalid_ref_diagnostics("failure repro node summary", &input.node_summary_refs, diagnostics)?;
    collect_invalid_ref_diagnostics("failure repro receipt", &input.receipt_refs, diagnostics)?;
    collect_invalid_ref_diagnostics("failure repro diagnostic", &input.diagnostic_refs, diagnostics)?;
    collect_invalid_ref_diagnostics("failure repro log", &input.log_refs, diagnostics)?;
    collect_invalid_ref_diagnostics("failure repro private", &input.private_attachment_refs, diagnostics)?;
    collect_invalid_ref_diagnostics("failure repro reveal", &input.reveal_receipt_refs, diagnostics)?;
    collect_invalid_optional_ref_diagnostics(
        "failure repro claimed seal",
        input.claimed_payload_ref.as_deref(),
        diagnostics,
    )?;
    push_if(diagnostics, input.command_refs.is_empty(), "failure-repro-missing-commands")?;
    push_if(diagnostics, input.node_summary_refs.is_empty(), "failure-repro-missing-node-summaries")?;
    push_if(diagnostics, input.receipt_refs.is_empty(), "failure-repro-missing-receipts")?;
    push_if(diagnostics, input.log_refs.is_empty(), "failure-repro-missing-logs")?;
    push_if(diagnostics, input.caveats.is_empty(), "failure-repro-missing-caveats")?;
    push_if(diagnostics, !input.sealed, "failure-repro-unsealed")?;
    push_if(
        diagnostics,
        !input.private_attachment_refs.is_empty() && input.reveal_receipt_refs.is_empty(),
        "failure-repro-private-without-reveal",
    )?;
    push_if(diagnostics, input.redaction_policy_ref.trim().is_empty(), "failure-repro-missing-redaction-policy")?;
    if let Some(claimed) = &input.claimed_payload_ref {
        if claimed != payload_ref {
            push_diagnostic(diagnostics, "failure-repro-seal-mismatch".to_string())?;
        }
    }
    Ok(())
}

fn collect_vm_fault_support_diagnostics(
    case: &VmFaultSupportCase,
    diagnostics: &mut impl DiagnosticSink,
) -> Result<()> {
    collect_required_text_diagnostic("fault-kind", &case.fault_kind, diagnostics)?;
    collect_required_text_diagnostic("fault-capability", &case.required_capability, diagnostics)?;
    collect_required_text_diagnostic("fault-target", &case.target, diagnostics)?;
    collect_required_text_diagnostic("fault-command-profile", &case.command_profile, diagnostics)?;
    collect_required_text_diagnostic("fault-expected-outcome", &case.expected_outcome, diagnostics)?;
    collect_required_text_diagnostic("fault-host-support", &case.host_support, diagnostics)?;
    collect_invalid_ref_diagnostics("fault preflight", &case.preflight_refs, diagnostics)?;
    collect_invalid_ref_diagnostics("fault injection", &case.injection_refs, diagnostics)?;
    collect_invalid_ref_diagnostics("fault child", &case.child_refs, diagnostics)?;
    collect_invalid_ref_diagnostics("fault post", &case.post_fault_refs, diagnostics)?;
    collect_invalid_ref_diagnostics("fault diagnostic", &case.diagnostic_refs, diagnostics)?;
    let is_pass_claim = case.expected_outcome == PASS_DECISION;
    if is_pass_claim && case.host_support != SUPPORTED {
        push_diagnostic(diagnostics, format!("vm-fault-unsupported-pass:{}", case.fault_kind))?;
    }
    if is_pass_claim && case.injection_refs.is_empty() {
        push_diagnostic(diagnostics, format!("vm-fault-missing-injection:{}", case.fault_kind))?;
    }
    if is_pass_claim && case.child_refs.is_empty() {
        push_diagnostic(diagnostics, format!("vm-fault-missing-child:{}", case.fault_kind))?;
    }
    if case.host_support == UNAVAILABLE && case.diagnostic_refs.is_empty() {
        push_diagnostic(diagnostics, format!("vm-fault-unavailable-missing-diagnostic:{}", case.fault_kind))?;
    }
    if case.caveats.is_empty() {
        push_diagnostic(diagnostics, format!("vm-fault-missing-caveat:{}", case.fault_kind))?;
    }
    Ok(())
}

fn scenario_fixture_value(fixture: &ScenarioFixture) -> Result<IoValue> {
    ensure_count_at_most(fixture.expected_artifact_kinds.len(), MAX_MULTINODE_ITEMS, "fixture artifacts")?;
    ensure_count_at_most(fixture.receipt_refs.len(), MAX_MULTINODE_ITEMS, "fixture receipts")?;
    ensure_count_at_most(fixture.variance_refs.len(), MAX_MULTINODE_ITEMS, "fixture variance")?;
    ensure_count_at_most(fixture.diagnostic_log_refs.len(), MAX_MULTINODE_ITEMS, "fixture logs")?;
    Ok(record("multinode-scenario-fixture-v1", vec![
        string(MULTINODE_SCENARIO_FIXTURE_SCHEMA),
        record("id", vec![string(&fixture.scenario_id)]),
        record("purpose", vec![string(&fixture.purpose)]),
        record("evidence-scope", vec![string(&fixture.evidence_scope)]),
        record("topology-profile", vec![string(&fixture.topology_profile_id)]),
        record("execution-profile", vec![string(&fixture.execution_profile_id)]),
        record("command", vec![string(&fixture.command_surface)]),
        record("artifact-kinds", vec![strings_sequence(&fixture.expected_artifact_kinds)]),
        record("topology", vec![string(&fixture.topology_ref)]),
        record("seed", vec![string(&fixture.seed_ref)]),
        record("fault-plan", vec![string(&fixture.fault_plan_ref)]),
        record("receipts", vec![refs_sequence(&fixture.receipt_refs)]),
        record("variance", vec![refs_sequence(&fixture.variance_refs)]),
        record("diagnostic-logs", vec![refs_sequence(&fixture.diagnostic_log_refs)]),
        record("unavailable-policy", vec![string(&fixture.unavailable_policy)]),
        record("unsupported-claims-pass", vec![bool_value(fixture.unsupported_claims_pass)]),
        record("caveats", vec![strings_sequence(&fixture.caveats)]),
        checks_value(&[
            ("fixture-declarative", PASS_DECISION),
            ("ambient-runtime-state-excluded", PASS_DECISION),
            ("logs-diagnostic-only", PASS_DECISION),
        ]),
    ]))
}

