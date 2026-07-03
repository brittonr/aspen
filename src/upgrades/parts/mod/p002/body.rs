
fn move_result(root: &Path, plan: &UpgradePlan, task: &UpgradeTask) -> Result<UpgradeTaskOutcome> {
    let from_ref =
        task.from_ref.as_deref().ok_or_else(|| MoltenError::invalid_harness("move-name missing from ref"))?;
    let to_ref = task.to_ref.as_deref().ok_or_else(|| MoltenError::invalid_harness("move-name missing to ref"))?;
    let current = read_name_pointer(root, &task.subject)?;
    if let Some(current) = current.as_ref()
        && current.artifact_ref != from_ref
    {
        return Ok((
            "deny",
            vec![format!(
                "name {} currently points to {}, expected {}",
                task.subject, current.artifact_ref, from_ref
            )],
            vec![("current-pointer", "fail")],
        ));
    }

    let pending_receipt_ref = local_ref("upgrade-pending-receipt", &plan.plan_ref, &task.task_id)?;
    let pointer = name_pointer_value(&task.subject, "name", to_ref, Some(from_ref), &pending_receipt_ref)?;
    write_preserves(&name_pointer_path(root, &task.subject)?, &pointer)?;
    Ok(("pass", Vec::new(), vec![
        ("metadata-pointer-move", "pass"),
        ("artifact-content-immutable", "pass"),
    ]))
}

fn cleanup_result(root: &Path, ledger_root: &Path, task: &UpgradeTask) -> Result<UpgradeTaskOutcome> {
    let cleanup_ref = task.to_ref.as_deref().or(task.from_ref.as_deref()).unwrap_or(&task.subject);
    let cleanup = cleanup_admission(root, ledger_root, cleanup_ref)?;
    if cleanup.decision == "pass" {
        Ok(("pass", Vec::new(), vec![("cleanup-safety", "pass")]))
    } else {
        Ok(("deny", vec![format!("cleanup denied by receipt {}", cleanup.receipt_ref)], vec![(
            "cleanup-safety",
            "fail",
        )]))
    }
}

pub fn rollback_task(root: &Path, plan_ref: &str, task_id: &str) -> Result<UpgradeReceipt> {
    ensure_dirs(root)?;
    let plan = read_plan(root, plan_ref)?;
    let task = plan
        .tasks
        .iter()
        .find(|task| task.task_id == task_id)
        .ok_or_else(|| MoltenError::invalid_harness(format!("upgrade plan missing task {task_id}")))?;
    let is_irreversible_task = matches!(task.kind.as_str(), "migrate-storage" | "cleanup" | "install-protocol-bridge");
    let (decision, diagnostics, checks) = if is_irreversible_task || !task.reversible {
        ("deny", vec![format!("task {} kind {} is not reversible", task.task_id, task.kind)], vec![
            ("reversible-metadata-only", "fail"),
            ("irreversible-effects-preserved", "pass"),
        ])
    } else if let Some(from_ref) = task.from_ref.as_deref() {
        let rollback_receipt_ref = local_ref("upgrade-rollback-pending", &plan.plan_ref, &task.task_id)?;
        let pointer =
            name_pointer_value(&task.subject, "name", from_ref, task.to_ref.as_deref(), &rollback_receipt_ref)?;
        if matches!(task.kind.as_str(), "move-name" | "compatibility-alias" | "cutover" | "rollback-pointer") {
            write_preserves(&name_pointer_path(root, &task.subject)?, &pointer)?;
        }
        ("pass", Vec::new(), vec![("reversible-metadata-only", "pass"), ("rollback-pointer", "pass")])
    } else {
        ("deny", vec![format!("task {} has no rollback ref", task.task_id)], vec![("rollback-ref", "fail")])
    };
    let receipt_value = upgrade_receipt_value(&UpgradeReceiptValueInput {
        operation: "rollback",
        decision,
        session_id: &plan.session_id,
        plan_ref: &plan.plan_ref,
        task_id: Some(&task.task_id),
        refs: &task_refs(task),
        diagnostics: &diagnostics,
        checks: &checks,
    })?;
    let receipt = parse_upgrade_receipt(&receipt_value)?;
    store_receipt(root, &receipt_value)?;
    Ok(receipt)
}

pub fn cleanup_admission(root: &Path, ledger_root: &Path, artifact_ref: &str) -> Result<UpgradeReceipt> {
    cleanup_admission_with_registry(root, ledger_root, None, artifact_ref)
}

pub fn cleanup_admission_with_registry(
    root: &Path,
    ledger_root: &Path,
    registry_root: Option<&Path>,
    artifact_ref: &str,
) -> Result<UpgradeReceipt> {
    ensure_dirs(root)?;
    validate_ref(artifact_ref, "cleanup artifact ref")?;
    let mut diagnostics = Vec::new();
    for pointer in read_name_pointers(root)? {
        if pointer.artifact_ref == artifact_ref || pointer.previous_ref.as_deref() == Some(artifact_ref) {
            push_bounded(
                &mut diagnostics,
                format!("name pointer {} retains {}", pointer.name, artifact_ref),
                MAX_UPGRADE_DIAGNOSTICS,
                "upgrade cleanup diagnostics",
            )?;
        }
    }
    if store_text_contains_ref(&root.join("plans"), artifact_ref)? {
        push_bounded(
            &mut diagnostics,
            format!("upgrade plan retains {artifact_ref}"),
            MAX_UPGRADE_DIAGNOSTICS,
            "upgrade cleanup diagnostics",
        )?;
    }
    if store_text_contains_ref(&root.join("receipts"), artifact_ref)? {
        push_bounded(
            &mut diagnostics,
            format!("upgrade receipt retains {artifact_ref}"),
            MAX_UPGRADE_DIAGNOSTICS,
            "upgrade cleanup diagnostics",
        )?;
    }
    if let Some(registry_root) = registry_root {
        for diagnostic in crate::artifacts::reference_diagnostics(registry_root, artifact_ref)? {
            push_bounded(&mut diagnostics, diagnostic, MAX_UPGRADE_DIAGNOSTICS, "upgrade cleanup diagnostics")?;
        }
    }
    for entry in crate::ledger::list_artifacts(ledger_root)? {
        if entry.artifact_ref == artifact_ref {
            continue;
        }
        let value = crate::ledger::read_artifact(ledger_root, &entry.artifact_ref)?;
        if to_text(&value)?.contains(artifact_ref) {
            push_bounded(
                &mut diagnostics,
                format!("ledger artifact {} retains {}", entry.artifact_ref, artifact_ref),
                MAX_UPGRADE_DIAGNOSTICS,
                "upgrade cleanup diagnostics",
            )?;
        }
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let checks = if diagnostics.is_empty() {
        vec![("reference-index-empty", "pass"), ("cleanup-safety", "pass")]
    } else {
        vec![("reference-index-empty", "fail"), ("cleanup-safety", "fail")]
    };
    let receipt_value = upgrade_receipt_value(&UpgradeReceiptValueInput {
        operation: "cleanup",
        decision,
        session_id: "cleanup",
        plan_ref: artifact_ref,
        task_id: None,
        refs: &[artifact_ref.to_string()],
        diagnostics: &diagnostics,
        checks: &checks,
    })?;
    let receipt = parse_upgrade_receipt(&receipt_value)?;
    store_receipt(root, &receipt_value)?;
    Ok(receipt)
}

fn protocol_drain_gate_evidence(
    gate: &crate::protocol_session::ProtocolSessionGateReceipt,
) -> ProtocolDrainGateEvidence {
    ProtocolDrainGateEvidence {
        gate_ref: gate.receipt_ref.clone(),
        decision: gate.decision.clone(),
        protocol_ref: gate.protocol_ref.clone(),
        session_ids: gate.session_ids.clone(),
        terminal_state_refs: gate.final_state_refs.clone(),
    }
}

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
        note_protocol_drain_gate(gate, &expected_refs, is_decision_pass, is_terminal, is_protocol_match, &mut diagnostics)?;
        if is_decision_pass && is_terminal && is_protocol_match {
            has_drained_gate = true;
            for terminal_state_ref in &gate.terminal_state_refs {
                push_bounded(
                    &mut terminal_state_refs,
                    terminal_state_ref.clone(),
                    MAX_UPGRADE_REFS,
                    "upgrade protocol drain terminal state refs",
                )?;
            }
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

fn note_protocol_drain_gate(
    gate: &ProtocolDrainGateEvidence,
    expected_refs: &[String],
    is_decision_pass: bool,
    is_terminal: bool,
    is_protocol_match: bool,
    diagnostics: &mut Vec<String>,
) -> Result<()> {
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

fn note_affected_ref_binding(input: &UpgradeDrainReadinessInput<'_>, diagnostics: &mut Vec<String>) -> Result<bool> {
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
    diagnostics: &mut Vec<String>,
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

fn push_upgrade_drain_diagnostic(diagnostics: &mut Vec<String>, diagnostic: String) -> Result<()> {
    push_bounded(
        diagnostics,
        diagnostic,
        MAX_UPGRADE_DIAGNOSTICS,
        "upgrade protocol drain diagnostics",
    )
}
