fn subscriber_peer_profile() -> TopologyProfile {
    TopologyProfile {
        id: PROFILE_SUBSCRIBER_PEER.to_string(),
        roles: vec![
            role("voter", ROLE_VOTER, MEMBERSHIP_VOTER),
            role("subscriber", ROLE_SUBSCRIBER, MEMBERSHIP_SUBSCRIBER),
        ],
        allowed_links: vec![link("voter", "subscriber", "observation")],
        evidence_scope: "non-voting subscriber observation evidence".to_string(),
        required_receipt_kinds: vec!["observe".to_string(), "membership-denial".to_string()],
        caveats: vec!["subscriber evidence cannot satisfy voter membership".to_string()],
    }
}

fn three_node_quorum_profile() -> TopologyProfile {
    TopologyProfile {
        id: PROFILE_THREE_NODE_QUORUM.to_string(),
        roles: vec![
            role("node-a", ROLE_VOTER, MEMBERSHIP_VOTER),
            role("node-b", ROLE_RESTARTING_MEMBER, MEMBERSHIP_VOTER),
            role("node-c", ROLE_SUBSCRIBER, MEMBERSHIP_SUBSCRIBER),
        ],
        allowed_links: vec![
            link("node-a", "node-b", "raft-control"),
            link("node-b", "node-c", "observation"),
            link("node-c", "node-a", "diagnostic-observation"),
        ],
        evidence_scope: "three-node quorum, restart/rejoin, and subscriber-negative evidence".to_string(),
        required_receipt_kinds: vec![
            "membership".to_string(),
            "quorum".to_string(),
            "reconciliation".to_string(),
            "duplicate-suppression".to_string(),
        ],
        caveats: vec!["three-node evidence is topology-scoped and not fleet-scale evidence".to_string()],
    }
}

fn wrong_topology_profile() -> TopologyProfile {
    TopologyProfile {
        id: PROFILE_WRONG_TOPOLOGY.to_string(),
        roles: vec![role("wrong-node", ROLE_TRANSPORT_ONLY, MEMBERSHIP_TRANSPORT_ONLY)],
        allowed_links: vec![link("wrong-node", "missing-node", "negative")],
        evidence_scope: "negative wrong-topology fixture".to_string(),
        required_receipt_kinds: vec!["deny".to_string()],
        caveats: vec!["negative fixture cannot satisfy pass evidence".to_string()],
    }
}

pub fn build_topology_matrix(profiles: &[TopologyProfile]) -> Result<TopologyMatrix> {
    let mut diagnostics = Vec::new();
    ensure_count_at_most(profiles.len(), MAX_MULTINODE_ITEMS, "topology profiles")?;
    let mut ids = OrderedSet::new();
    let mut profile_refs = Vec::with_capacity(profiles.len());
    for profile in profiles {
        collect_topology_profile_diagnostics(profile, &mut diagnostics)?;
        if !ids.insert(profile.id.as_str()) {
            push_diagnostic(&mut diagnostics, format!("duplicate-topology-profile:{}", profile.id))?;
        }
        profile_refs.push(canonical_hash(&topology_profile_value(profile)?)?);
    }
    for required in required_topology_profiles() {
        if !ids.contains(required) {
            push_diagnostic(&mut diagnostics, format!("missing-topology-profile:{required}"))?;
        }
    }
    let decision = decision_from_diagnostics(&diagnostics).to_string();
    let value = topology_matrix_value(&decision, profiles, &profile_refs, &diagnostics)?;
    let matrix_ref = canonical_hash(&value)?;
    Ok(TopologyMatrix {
        decision,
        diagnostics,
        profile_refs,
        matrix_ref,
        value,
    })
}

pub fn derive_scenario_metadata(
    fixture: &ScenarioFixture,
    execution_profiles: &[crate::distributed_core::CiProfile],
    topology_profiles: &[TopologyProfile],
) -> Result<ScenarioMetadata> {
    let mut diagnostics = scenario_fixture_diagnostics(fixture, execution_profiles, topology_profiles)?;
    diagnostics.sort();
    diagnostics.dedup();
    let decision = decision_from_diagnostics(&diagnostics).to_string();
    let fixture_value = scenario_fixture_value(fixture)?;
    let fixture_ref = canonical_hash(&fixture_value)?;
    let topology_profile_ref = topology_profiles
        .iter()
        .find(|profile| profile.id == fixture.topology_profile_id)
        .map(topology_profile_value)
        .transpose()?
        .map(|value| canonical_hash(&value))
        .transpose()?
        .unwrap_or_else(|| content_ref_from_text("missing-topology-profile"));
    let value = scenario_metadata_value(&ScenarioMetadataValueInput {
        decision: &decision,
        diagnostics: &diagnostics,
        fixture_ref: &fixture_ref,
        topology_profile_ref: &topology_profile_ref,
        fixture,
    })?;
    let metadata_ref = canonical_hash(&value)?;
    Ok(ScenarioMetadata {
        decision,
        diagnostics,
        fixture_ref,
        topology_profile_ref,
        metadata_ref,
        value,
    })
}

pub fn evaluate_topology_membership_claim(
    profile: &TopologyProfile,
    claim: &TopologyMembershipClaim,
) -> Result<TopologyMembershipGate> {
    let mut diagnostics = Vec::new();
    if claim.profile_id != profile.id {
        push_diagnostic(&mut diagnostics, "topology-profile-mismatch".to_string())?;
    }
    if claim.topology_ref != claim.scenario_topology_ref {
        push_diagnostic(&mut diagnostics, "wrong-topology".to_string())?;
    }
    collect_invalid_ref_diagnostics(
        "topology claim",
        &[claim.topology_ref.clone(), claim.scenario_topology_ref.clone()],
        &mut diagnostics,
    )?;
    collect_invalid_optional_ref_diagnostics("quorum", claim.quorum_ref.as_deref(), &mut diagnostics)?;
    let declared_nodes = profile.roles.iter().map(|role| role.node_id.as_str()).collect::<OrderedSet<_>>();
    let declared_subscribers = profile
        .roles
        .iter()
        .filter(|role| role.membership == MEMBERSHIP_SUBSCRIBER)
        .map(|role| role.node_id.as_str())
        .collect::<OrderedSet<_>>();
    let is_quorum_required = profile.required_receipt_kinds.iter().any(|kind| kind == "quorum");
    for role_claim in &claim.node_roles {
        if !declared_nodes.contains(role_claim.node_id.as_str()) {
            push_diagnostic(&mut diagnostics, format!("undeclared-node:{}", role_claim.node_id))?;
        }
        if declared_subscribers.contains(role_claim.node_id.as_str()) && role_claim.membership == MEMBERSHIP_VOTER {
            push_diagnostic(&mut diagnostics, format!("subscriber-promoted-to-voter:{}", role_claim.node_id))?;
        }
    }
    if claim.transport_only_authority_claim {
        push_diagnostic(&mut diagnostics, "transport-only-authority-claim".to_string())?;
    }
    if is_quorum_required && claim.quorum_ref.is_none() {
        push_diagnostic(&mut diagnostics, "missing-quorum-evidence".to_string())?;
    }
    if claim.caveats.is_empty() {
        push_diagnostic(&mut diagnostics, "missing-evidence-scope-caveat".to_string())?;
    }
    let decision = decision_from_diagnostics(&diagnostics).to_string();
    let value = topology_membership_gate_value(&decision, profile, claim, &diagnostics)?;
    let gate_ref = canonical_hash(&value)?;
    Ok(TopologyMembershipGate {
        decision,
        diagnostics,
        gate_ref,
        value,
    })
}

pub fn evaluate_reconciliation(input: &ReconciliationInput) -> Result<ReconciliationGate> {
    let mut diagnostics = reconciliation_diagnostics(input)?;
    diagnostics.sort();
    diagnostics.dedup();
    let decision = decision_from_diagnostics(&diagnostics).to_string();
    let value = reconciliation_gate_value(input, &decision, &diagnostics)?;
    let receipt_ref = canonical_hash(&value)?;
    Ok(ReconciliationGate {
        decision,
        diagnostics,
        receipt_ref,
        value,
    })
}

pub fn build_local_multiprocess_plan(input: &LocalMultiprocessPlanInput) -> Result<LocalMultiprocessPlan> {
    let mut diagnostics = Vec::new();
    collect_invalid_ref_diagnostics(
        "local plan",
        &[input.fixture_ref.clone(), input.command_plan_ref.clone()],
        &mut diagnostics,
    )?;
    collect_invalid_ref_diagnostics("local expected receipt", &input.expected_receipt_refs, &mut diagnostics)?;
    push_if(&mut diagnostics, input.nodes.is_empty(), "local-plan-missing-nodes")?;
    push_if(&mut diagnostics, input.expected_receipt_refs.is_empty(), "local-plan-missing-expected-receipts")?;
    push_if(&mut diagnostics, input.cleanup_policy.trim().is_empty(), "local-plan-missing-cleanup-policy")?;
    push_if(
        &mut diagnostics,
        input.cleanup_policy != CLEANUP_POLICY_REQUIRED,
        "local-plan-cleanup-policy-not-reviewed",
    )?;
    collect_process_plan_collisions(input, &mut diagnostics)?;
    let decision = decision_from_diagnostics(&diagnostics).to_string();
    let value = local_multiprocess_plan_value(input, &decision, &diagnostics)?;
    let plan_ref = canonical_hash(&value)?;
    Ok(LocalMultiprocessPlan {
        decision,
        diagnostics,
        plan_ref,
        value,
    })
}

pub fn build_local_multiprocess_run_receipt(input: &LocalMultiprocessRunInput) -> Result<LocalMultiprocessRunReceipt> {
    let mut diagnostics = input.diagnostics.clone();
    collect_invalid_ref_diagnostics("local run plan", &[input.plan_ref.clone()], &mut diagnostics)?;
    collect_invalid_ref_diagnostics("local run startup", &input.startup_refs, &mut diagnostics)?;
    collect_invalid_ref_diagnostics("local run workflow", &input.workflow_refs, &mut diagnostics)?;
    collect_invalid_ref_diagnostics("local run shutdown", &input.shutdown_refs, &mut diagnostics)?;
    collect_invalid_ref_diagnostics("local run cleanup", &input.cleanup_refs, &mut diagnostics)?;
    push_if(&mut diagnostics, input.startup_refs.is_empty(), "local-run-missing-startup-receipts")?;
    push_if(&mut diagnostics, input.workflow_refs.is_empty(), "local-run-missing-workflow-receipts")?;
    push_if(&mut diagnostics, input.shutdown_refs.is_empty(), "local-run-missing-shutdown-receipts")?;
    push_if(&mut diagnostics, input.cleanup_refs.is_empty(), "local-run-missing-cleanup-receipts")?;
    push_if(&mut diagnostics, input.caveats.is_empty(), "local-run-missing-evidence-caveats")?;
    let decision = decision_from_diagnostics(&diagnostics).to_string();
    let value = local_multiprocess_run_value(input, &decision, &diagnostics)?;
    let receipt_ref = canonical_hash(&value)?;
    Ok(LocalMultiprocessRunReceipt {
        decision,
        diagnostics,
        receipt_ref,
        value,
    })
}

pub fn build_local_multiprocess_executable_run(
    input: &LocalMultiprocessExecutableRunInput,
) -> Result<LocalMultiprocessExecutableRunReceipt> {
    let plan = build_local_multiprocess_plan(&input.plan)?;
    let mut diagnostics = input.diagnostics.clone();
    if plan.decision != PASS_DECISION {
        push_diagnostic(&mut diagnostics, "local-executable-plan-denied".to_string())?;
        for diagnostic in &plan.diagnostics {
            push_diagnostic(&mut diagnostics, format!("plan:{diagnostic}"))?;
        }
    }
    collect_required_text_diagnostic("local-executable-ticket-status", &input.ticket_status, &mut diagnostics)?;
    if input.ticket_status != TICKET_STATUS_CURRENT {
        push_diagnostic(&mut diagnostics, "local-executable-stale-ticket".to_string())?;
    }
    if input.child_timed_out {
        push_diagnostic(&mut diagnostics, "local-executable-child-timeout".to_string())?;
    }
    for orphaned_process in &input.orphaned_processes {
        collect_required_text_diagnostic("local-executable-orphaned-process", orphaned_process, &mut diagnostics)?;
        push_diagnostic(&mut diagnostics, format!("local-executable-orphaned-process:{orphaned_process}"))?;
    }
    if !input.cleanup_succeeded {
        push_diagnostic(&mut diagnostics, "local-executable-cleanup-failed".to_string())?;
    }
    let run = build_local_multiprocess_run_receipt(&LocalMultiprocessRunInput {
        plan_ref: plan.plan_ref.clone(),
        startup_refs: input.startup_refs.clone(),
        workflow_refs: input.workflow_refs.clone(),
        shutdown_refs: input.shutdown_refs.clone(),
        cleanup_refs: input.cleanup_refs.clone(),
        diagnostics,
        caveats: input.caveats.clone(),
    })?;
    let value = local_multiprocess_executable_run_value(
        input,
        &plan.plan_ref,
        &run.receipt_ref,
        &run.decision,
        &run.diagnostics,
    )?;
    let executable_ref = canonical_hash(&value)?;
    Ok(LocalMultiprocessExecutableRunReceipt {
        decision: run.decision,
        diagnostics: run.diagnostics,
        plan_ref: plan.plan_ref,
        run_ref: run.receipt_ref,
        executable_ref,
        value,
    })
}

