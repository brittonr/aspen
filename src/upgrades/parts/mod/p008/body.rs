
fn protocol_drain_task_outcome(
    ledger_root: &Path,
    plan: &UpgradePlan,
    task: &UpgradeTask,
) -> Result<UpgradeTaskOutcome> {
    let evidence_refs = protocol_drain_evidence_refs(task)?;
    let mut shell_diagnostics = Vec::new();
    let mut gate_evidence = Vec::new();
    for evidence_ref in &evidence_refs {
        match protocol_drain_gate_from_ledger(ledger_root, evidence_ref) {
            Ok(gate) => push_bounded(
                &mut gate_evidence,
                protocol_drain_gate_evidence(&gate),
                MAX_UPGRADE_REFS,
                "upgrade protocol drain gate evidence",
            )?,
            Err(diagnostic) => push_bounded(
                &mut shell_diagnostics,
                diagnostic,
                MAX_UPGRADE_DIAGNOSTICS,
                "upgrade protocol drain diagnostics",
            )?,
        }
    }
    let readiness = evaluate_upgrade_drain_readiness(&UpgradeDrainReadinessInput {
        task_id: &task.task_id,
        subject: &task.subject,
        from_ref: task.from_ref.as_deref(),
        to_ref: task.to_ref.as_deref(),
        affected_refs: &plan.affected_refs,
        compatibility_old_refs: &plan.compatibility.old_refs,
        compatibility_new_refs: &plan.compatibility.new_refs,
        evidence_refs: &evidence_refs,
        gate_evidence: &gate_evidence,
    })?;
    let mut diagnostics = shell_diagnostics;
    for diagnostic in readiness.diagnostics {
        push_bounded(
            &mut diagnostics,
            diagnostic,
            MAX_UPGRADE_DIAGNOSTICS,
            "upgrade protocol drain diagnostics",
        )?;
    }
    let has_bound_terminal_refs = !readiness.terminal_state_refs.is_empty();
    let decision = if diagnostics.is_empty() && has_bound_terminal_refs {
        readiness.decision
    } else {
        "deny"
    };
    Ok((decision, diagnostics, readiness.checks))
}

fn protocol_drain_gate_from_ledger(
    ledger_root: &Path,
    evidence_ref: &str,
) -> std::result::Result<crate::protocol_session::ProtocolSessionGateReceipt, String> {
    let value = crate::ledger::read_artifact(ledger_root, evidence_ref)
        .map_err(|error| format!("protocol drain evidence {evidence_ref} is not readable from ledger: {error}"))?;
    crate::protocol_session::parse_protocol_session_gate_receipt(&value).map_err(|error| {
        format!("protocol drain evidence {evidence_ref} is not a protocol session gate receipt: {error}")
    })
}

fn evaluate_upgrade_drain_readiness(input: &UpgradeDrainReadinessInput<'_>) -> Result<UpgradeDrainReadinessDecision> {
    validate_upgrade_drain_readiness_input(input)?;
    let mut diagnostics = Vec::new();
    if input.evidence_refs.is_empty() {
        push_upgrade_drain_diagnostic(
            &mut diagnostics,
            "drain-sessions task requires a protocol-session-gate-receipt-v1 precondition or postcondition ref"
                .to_string(),
        )?;
    }
    let expected_refs = protocol_drain_expected_protocol_refs_from_bindings(
        input.subject,
        input.from_ref,
        input.affected_refs,
        input.compatibility_old_refs,
    )?;
    let has_affected_binding = note_affected_ref_binding(input, &mut diagnostics)?;
    let has_compatibility_binding = note_compatibility_ref_binding(input, &mut diagnostics)?;
    let mut has_gate = false;
    let mut has_gate_decision_pass = false;
    let mut has_terminal_state = false;
    let mut has_protocol_match = false;
    let mut has_drained_gate = false;
    let mut terminal_state_refs = Vec::new();
    for gate in input.gate_evidence {
        has_gate = true;
        let is_decision_pass = gate.decision == "pass";
        let is_terminal = !gate.session_ids.is_empty() && !gate.terminal_state_refs.is_empty();
        let is_protocol_match = expected_refs.iter().any(|expected| expected == &gate.protocol_ref);
        has_gate_decision_pass |= is_decision_pass;
        has_terminal_state |= is_terminal;
        has_protocol_match |= is_protocol_match;
        note_protocol_drain_gate(gate, &expected_refs, DrainGateOutcome { is_decision_pass, is_terminal, is_protocol_match, diagnostics: &mut diagnostics })?;
        if is_decision_pass && is_terminal && is_protocol_match {
            has_drained_gate = true;
            push_terminal_state_refs(&mut terminal_state_refs, gate)?;
        }
    }
    if !input.evidence_refs.is_empty() && !has_gate {
        push_upgrade_drain_diagnostic(
            &mut diagnostics,
            "drain-sessions task did not bind any readable protocol session gate receipts".to_string(),
        )?;
    }
    let is_ready = diagnostics.is_empty()
        && has_drained_gate
        && has_affected_binding
        && has_compatibility_binding
        && !terminal_state_refs.is_empty();
    Ok(UpgradeDrainReadinessDecision {
        decision: if is_ready { "pass" } else { "deny" },
        diagnostics,
        checks: vec![
            ("protocol-session-gate-bound", pass_fail(has_gate)),
            ("protocol-session-gate-pass", pass_fail(has_gate_decision_pass)),
            ("protocol-terminal-state", pass_fail(has_terminal_state)),
            ("protocol-ref-bound", pass_fail(has_protocol_match)),
            ("protocol-affected-ref-bound", pass_fail(has_affected_binding)),
            ("protocol-compatibility-ref-bound", pass_fail(has_compatibility_binding)),
            ("protocol-session-drain", pass_fail(is_ready)),
            ("protocol-drain-is-not-authority", "pass"),
        ],
        terminal_state_refs,
    })
}

fn push_terminal_state_refs(
    terminal_state_refs: &mut impl crate::bounded::VecSink<String>,
    gate: &ProtocolDrainGateEvidence,
) -> Result<()> {
    for terminal_state_ref in &gate.terminal_state_refs {
        push_bounded(
            terminal_state_refs,
            terminal_state_ref.clone(),
            MAX_UPGRADE_REFS,
            "upgrade protocol drain terminal state refs",
        )?;
    }
    Ok(())
}

fn validate_upgrade_drain_readiness_input(input: &UpgradeDrainReadinessInput<'_>) -> Result<()> {
    validate_non_empty(input.task_id, "upgrade drain task id")?;
    validate_non_empty(input.subject, "upgrade drain task subject")?;
    if let Some(from_ref) = input.from_ref {
        validate_ref(from_ref, "upgrade drain task from ref")?;
    }
    if let Some(to_ref) = input.to_ref {
        validate_ref(to_ref, "upgrade drain task to ref")?;
    }
    validate_refs(input.affected_refs, "upgrade drain affected ref")?;
    validate_refs(input.compatibility_old_refs, "upgrade drain compatibility old ref")?;
    validate_refs(input.compatibility_new_refs, "upgrade drain compatibility new ref")?;
    validate_refs(input.evidence_refs, "upgrade drain evidence ref")?;
    for gate in input.gate_evidence {
        validate_ref(&gate.gate_ref, "upgrade drain gate ref")?;
        validate_ref(&gate.protocol_ref, "upgrade drain gate protocol ref")?;
        validate_refs(&gate.terminal_state_refs, "upgrade drain gate terminal state ref")?;
    }
    Ok(())
}

struct DrainGateOutcome<'a> {
    is_decision_pass: bool,
    is_terminal: bool,
    is_protocol_match: bool,
    diagnostics: &'a mut Vec<String>,
}

fn note_protocol_drain_gate(gate: &ProtocolDrainGateEvidence, expected_refs: &[String], input: DrainGateOutcome<'_>) -> Result<()> {
    let DrainGateOutcome { is_decision_pass, is_terminal, is_protocol_match, diagnostics } = input;
    if !is_decision_pass {
        push_upgrade_drain_diagnostic(
            diagnostics,
            format!("protocol drain gate {} denied with decision {}", gate.gate_ref, gate.decision),
        )?;
    }
    if !is_terminal {
        push_upgrade_drain_diagnostic(
            diagnostics,
            format!("protocol drain gate {} does not bind terminal session state", gate.gate_ref),
        )?;
    }
    if !is_protocol_match {
        push_upgrade_drain_diagnostic(
            diagnostics,
            format!(
                "protocol drain gate {} is for {}, expected one of {}",
                gate.gate_ref,
                gate.protocol_ref,
                expected_refs.join(",")
            ),
        )?;
    }
    Ok(())
}

fn note_affected_ref_binding(input: &UpgradeDrainReadinessInput<'_>, diagnostics: &mut impl crate::bounded::VecSink<String>) -> Result<bool> {
    let mut is_bound = true;
    if let Some(from_ref) = input.from_ref
        && !input.affected_refs.iter().any(|affected_ref| affected_ref == from_ref)
    {
        push_upgrade_drain_diagnostic(
            diagnostics,
            format!("upgrade drain task {} from ref {from_ref} is not in affected refs", input.task_id),
        )?;
        is_bound = false;
    }
    if let Some(to_ref) = input.to_ref
        && !input.affected_refs.iter().any(|affected_ref| affected_ref == to_ref)
    {
        push_upgrade_drain_diagnostic(
            diagnostics,
            format!("upgrade drain task {} to ref {to_ref} is not in affected refs", input.task_id),
        )?;
        is_bound = false;
    }
    Ok(is_bound)
}

fn note_compatibility_ref_binding(
    input: &UpgradeDrainReadinessInput<'_>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<bool> {
    let mut is_bound = true;
    if let Some(from_ref) = input.from_ref
        && !input.compatibility_old_refs.iter().any(|old_ref| old_ref == from_ref)
    {
        push_upgrade_drain_diagnostic(
            diagnostics,
            format!(
                "stale compatibility ref: upgrade drain task {} from ref {from_ref} is not in compatibility old refs",
                input.task_id
            ),
        )?;
        is_bound = false;
    }
    if let Some(to_ref) = input.to_ref
        && !input.compatibility_new_refs.iter().any(|new_ref| new_ref == to_ref)
    {
        push_upgrade_drain_diagnostic(
            diagnostics,
            format!(
                "stale compatibility ref: upgrade drain task {} to ref {to_ref} is not in compatibility new refs",
                input.task_id
            ),
        )?;
        is_bound = false;
    }
    Ok(is_bound)
}

fn push_upgrade_drain_diagnostic(diagnostics: &mut impl crate::bounded::VecSink<String>, diagnostic: String) -> Result<()> {
    push_bounded(
        diagnostics,
        diagnostic,
        MAX_UPGRADE_DIAGNOSTICS,
        "upgrade protocol drain diagnostics",
    )
}
