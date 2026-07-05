struct SimulationEventValueInput<'a> {
    tick: u64,
    operation_id: &'a str,
    kind: &'a str,
    decision: &'a str,
    diagnostic: &'a str,
    payload_ref: &'a str,
    commit_ref: &'a str,
}

struct TestRunValueInput<'a> {
    decision: &'a str,
    source_ref: &'a str,
    test_binary_ref: &'a str,
    topology_ref: &'a str,
    seed_ref: &'a str,
    scheduler_ref: &'a str,
    fault_plan_ref: &'a str,
    child_workflow_refs: &'a [String],
    event_refs: &'a [String],
    final_state_ref: &'a str,
    replay_status: &'a str,
    allowed_variance_refs: &'a [String],
    diagnostics: &'a [String],
}

fn evaluate_command(
    command: &SimulationCommand,
    faults: &[FaultEvent],
    committed: &OrderedSet<String>,
) -> Result<CommandEvaluation> {
    let active = active_faults(command, faults);
    if committed.contains(&command.operation_id) || has_fault_kind(&active, FAULT_DUPLICATE) {
        return Ok(CommandEvaluation {
            decision: PASS_DECISION.to_string(),
            kind: DUPLICATE_EVENT_KIND.to_string(),
            diagnostic: DUPLICATE_SUPPRESSED_DECISION.to_string(),
            counts_as_commit: false,
            diagnostics: vec![DUPLICATE_SUPPRESSED_DECISION.to_string()],
        });
    }
    if has_fault_kind(&active, FAULT_AMBIENT_STATE_DRIFT) {
        return deny_evaluation("undeclared-ambient-state");
    }
    if has_fault_kind(&active, FAULT_UNAUTHORIZED_TRANSPORT)
        || (command.transport_ref.is_some() && command.authority_ref.is_none())
    {
        return deny_evaluation("transport-evidence-does-not-grant-authority");
    }
    if command.requires_authority && command.authority_ref.is_none() {
        return deny_evaluation("missing-authority");
    }
    if command.requires_authority && command.policy_ref.is_none() {
        return deny_evaluation("missing-policy");
    }
    if command.requires_authority && command.resource_ref.is_none() {
        return deny_evaluation("missing-resource");
    }
    if command.requires_quorum && has_fault_kind(&active, FAULT_PARTITION) {
        return deny_evaluation("partitioned-quorum-denied-before-side-effects");
    }
    if has_fault_kind(&active, FAULT_STALE_EVIDENCE) {
        return deny_evaluation("stale-evidence-denied-before-side-effects");
    }
    if has_fault_kind(&active, FAULT_CORRUPTED_RECEIPT) {
        return deny_evaluation("corrupted-receipt-denied-before-side-effects");
    }
    if has_fault_kind(&active, FAULT_RESOURCE_PRESSURE) {
        return deny_evaluation("resource-pressure-denied-before-side-effects");
    }
    let diagnostics = benign_fault_diagnostics(&active);
    let kind = if has_fault_kind(&active, FAULT_CRASH) || has_fault_kind(&active, FAULT_RESTART) {
        REPLAY_EVENT_KIND
    } else {
        COMMIT_EVENT_KIND
    };
    Ok(CommandEvaluation {
        decision: PASS_DECISION.to_string(),
        kind: kind.to_string(),
        diagnostic: diagnostics.first().cloned().unwrap_or_else(|| "semantic-commit-accepted".to_string()),
        counts_as_commit: true,
        diagnostics,
    })
}

fn deny_evaluation(diagnostic: &str) -> Result<CommandEvaluation> {
    validate_text("deny diagnostic", diagnostic)?;
    Ok(CommandEvaluation {
        decision: DENY_DECISION.to_string(),
        kind: DENY_EVENT_KIND.to_string(),
        diagnostic: diagnostic.to_string(),
        counts_as_commit: false,
        diagnostics: vec![diagnostic.to_string()],
    })
}

fn active_faults<'a>(command: &SimulationCommand, faults: &'a [FaultEvent]) -> Vec<&'a FaultEvent> {
    faults
        .iter()
        .filter(|fault| {
            fault.operation_id.as_deref() == Some(command.operation_id.as_str())
                || fault.target == command.from_peer
                || fault.target == command.to_peer
                || fault.target == command.operation_id
        })
        .collect()
}

fn has_fault_kind(faults: &[&FaultEvent], kind: &str) -> bool {
    faults.iter().any(|fault| fault.kind == kind)
}

fn benign_fault_diagnostics(faults: &[&FaultEvent]) -> Vec<String> {
    faults
        .iter()
        .filter(|fault| {
            matches!(
                fault.kind.as_str(),
                FAULT_DELAY | FAULT_DROP | FAULT_REORDER | FAULT_REJOIN | FAULT_CRASH | FAULT_RESTART
            )
        })
        .map(|fault| format!("{}:{}", fault.kind, fault.diagnostic))
        .collect()
}

fn test_run_value(input: TestRunValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_ref(input.source_ref, "distributed source")?;
    validate_ref(input.test_binary_ref, "distributed test binary")?;
    validate_ref(input.topology_ref, "distributed topology")?;
    validate_ref(input.seed_ref, "distributed seed")?;
    validate_ref(input.scheduler_ref, "distributed scheduler")?;
    validate_ref(input.fault_plan_ref, "distributed fault plan")?;
    validate_ref_slice("distributed child workflow", input.child_workflow_refs)?;
    validate_ref_slice("distributed event", input.event_refs)?;
    validate_ref(input.final_state_ref, "distributed final state")?;
    validate_text("distributed replay status", input.replay_status)?;
    validate_ref_slice("distributed allowed variance", input.allowed_variance_refs)?;
    validate_strings("distributed diagnostic", input.diagnostics, MAX_DISTRIBUTED_TEXT)?;
    Ok(record("distributed-test-run-v1", vec![
        string(DISTRIBUTED_RUN_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("source", vec![string(input.source_ref)]),
        record("test-binary", vec![string(input.test_binary_ref)]),
        record("topology", vec![string(input.topology_ref)]),
        record("seed", vec![string(input.seed_ref)]),
        record("scheduler", vec![string(input.scheduler_ref)]),
        record("fault-plan", vec![string(input.fault_plan_ref)]),
        record("child-workflows", vec![refs_sequence(input.child_workflow_refs)]),
        record("events", vec![refs_sequence(input.event_refs)]),
        record("final-state", vec![string(input.final_state_ref)]),
        record("replay-status", vec![string(input.replay_status)]),
        record("allowed-variance", vec![refs_sequence(input.allowed_variance_refs)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value(&[
            ("topology-bound", PASS_DECISION),
            ("fault-plan-bound", PASS_DECISION),
            ("simulation-does-not-grant-authority", PASS_DECISION),
            ("diagnostic-logs-not-authority", PASS_DECISION),
        ]),
    ]))
}

fn simulation_event_value(input: SimulationEventValueInput<'_>) -> Result<IoValue> {
    validate_text("simulation operation", input.operation_id)?;
    validate_text("simulation event kind", input.kind)?;
    validate_event_decision(input.decision)?;
    validate_text("simulation diagnostic", input.diagnostic)?;
    validate_ref(input.payload_ref, "simulation payload")?;
    validate_ref(input.commit_ref, "simulation commit")?;
    Ok(record("distributed-simulation-event-v1", vec![
        string(DISTRIBUTED_EVENT_SCHEMA),
        record("tick", vec![u64_value(input.tick)]),
        record("operation", vec![string(input.operation_id)]),
        record("kind", vec![string(input.kind)]),
        record("decision", vec![string(input.decision)]),
        record("diagnostic", vec![string(input.diagnostic)]),
        record("payload", vec![string(input.payload_ref)]),
        record("commit", vec![string(input.commit_ref)]),
    ]))
}

fn final_state_value(committed: &[String], denied: &[String], event_refs: &[String]) -> Result<IoValue> {
    validate_strings("committed operation", committed, MAX_DISTRIBUTED_COMMANDS)?;
    validate_strings("denied operation", denied, MAX_DISTRIBUTED_COMMANDS)?;
    validate_ref_slice("final-state event", event_refs)?;
    Ok(record("distributed-simulation-final-state-v1", vec![
        string(DISTRIBUTED_FINAL_STATE_SCHEMA),
        record("committed-operations", vec![sequence(committed.iter().map(string).collect())]),
        record("denied-operations", vec![sequence(denied.iter().map(string).collect())]),
        record("events", vec![refs_sequence(event_refs)]),
    ]))
}

fn validate_simulation_input(input: &SimulationInput) -> Result<()> {
    validate_topology(&input.topology)?;
    scheduler_profile_value(&input.scheduler)?;
    seed_value(&input.seed)?;
    validate_fault_plan(&input.fault_plan)?;
    validate_ref(&input.source_ref, "distributed simulation source")?;
    validate_ref(&input.test_binary_ref, "distributed simulation test binary")?;
    validate_ref_slice("distributed child workflow", &input.child_workflow_refs)?;
    validate_ref_slice("distributed allowed variance", &input.allowed_variance_refs)?;
    validate_commands(&input.topology, &input.commands)
}

fn validate_topology(topology: &Topology) -> Result<()> {
    if topology.peers.is_empty() {
        return Err(MoltenError::invalid_harness("distributed topology requires peers"));
    }
    ensure_count_at_most(topology.peers.len(), MAX_DISTRIBUTED_PEERS, "distributed peers")?;
    ensure_count_at_most(topology.channels.len(), MAX_DISTRIBUTED_CHANNELS, "distributed channels")?;
    validate_strings("topology caveat", &topology.caveats, MAX_DISTRIBUTED_TEXT)?;
    let mut peers = OrderedSet::new();
    for peer in &topology.peers {
        validate_text("distributed peer", &peer.id)?;
        ensure_count_at_most(peer.roles.len(), MAX_DISTRIBUTED_ROLES, "distributed peer roles")?;
        validate_strings("distributed peer role", &peer.roles, MAX_DISTRIBUTED_TEXT)?;
        if !peers.insert(peer.id.as_str()) {
            return Err(MoltenError::invalid_harness(format!("duplicate distributed peer {}", peer.id)));
        }
    }
    let mut channels = OrderedSet::new();
    for channel in &topology.channels {
        validate_text("distributed channel", &channel.id)?;
        validate_text("distributed channel topic", &channel.topic)?;
        if !peers.contains(channel.from_peer.as_str()) || !peers.contains(channel.to_peer.as_str()) {
            return Err(MoltenError::invalid_harness(format!(
                "distributed channel {} references peer outside topology",
                channel.id
            )));
        }
        if !channels.insert(channel.id.as_str()) {
            return Err(MoltenError::invalid_harness(format!("duplicate distributed channel {}", channel.id)));
        }
    }
    Ok(())
}

fn validate_fault_plan(plan: &FaultPlan) -> Result<()> {
    ensure_count_at_most(plan.events.len(), MAX_DISTRIBUTED_FAULTS, "distributed faults")?;
    validate_strings("fault-plan caveat", &plan.caveats, MAX_DISTRIBUTED_TEXT)?;
    for event in &plan.events {
        validate_fault_kind(&event.kind)?;
        validate_text("fault target kind", &event.target_kind)?;
        validate_text("fault target", &event.target)?;
        validate_text("fault diagnostic", &event.diagnostic)?;
        if let Some(operation_id) = &event.operation_id {
            validate_text("fault operation", operation_id)?;
        }
    }
    Ok(())
}

