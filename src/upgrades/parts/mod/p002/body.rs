
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

#[derive(Clone, Copy)]
struct GateFacts {
    is_decision_pass: bool,
    is_terminal: bool,
    is_protocol_match: bool,
}

#[derive(Default)]
struct DrainState {
    diagnostics: Vec<String>,
    has_gate: bool,
    has_gate_decision_pass: bool,
    has_terminal_state: bool,
    has_protocol_match: bool,
    has_drained_gate: bool,
}

impl DrainState {
    fn push(&mut self, message: String) -> Result<()> {
        push_bounded(&mut self.diagnostics, message, MAX_UPGRADE_DIAGNOSTICS, "upgrade protocol drain diagnostics")
    }

    fn require_refs(&mut self, refs: &[String]) -> Result<()> {
        if refs.is_empty() {
            self.push(
                "drain-sessions task requires a protocol-session-gate-receipt-v1 precondition or postcondition ref"
                    .to_string(),
            )?;
        }
        Ok(())
    }

    fn inspect_ref(&mut self, ledger_root: &Path, evidence_ref: &str, expected_refs: &[String]) -> Result<()> {
        let value = match crate::ledger::read_artifact(ledger_root, evidence_ref) {
            Ok(value) => value,
            Err(error) => {
                self.push(format!("protocol drain evidence {evidence_ref} is not readable from ledger: {error}"))?;
                return Ok(());
            }
        };
        let gate = match crate::protocol_session::parse_protocol_session_gate_receipt(&value) {
            Ok(gate) => gate,
            Err(error) => {
                self.push(format!(
                    "protocol drain evidence {evidence_ref} is not a protocol session gate receipt: {error}"
                ))?;
                return Ok(());
            }
        };
        self.observe(&gate, expected_refs)
    }

    fn observe(
        &mut self,
        gate: &crate::protocol_session::ProtocolSessionGateReceipt,
        expected_refs: &[String],
    ) -> Result<()> {
        self.has_gate = true;
        let facts = GateFacts {
            is_decision_pass: gate.decision == "pass",
            is_terminal: !gate.session_ids.is_empty() && !gate.final_state_refs.is_empty(),
            is_protocol_match: expected_refs.iter().any(|expected| expected == &gate.protocol_ref),
        };
        self.has_gate_decision_pass |= facts.is_decision_pass;
        self.has_terminal_state |= facts.is_terminal;
        self.has_protocol_match |= facts.is_protocol_match;
        self.note_gate(gate, expected_refs, facts)?;
        self.has_drained_gate |= facts.is_decision_pass && facts.is_terminal && facts.is_protocol_match;
        Ok(())
    }

    fn note_gate(
        &mut self,
        gate: &crate::protocol_session::ProtocolSessionGateReceipt,
        expected_refs: &[String],
        facts: GateFacts,
    ) -> Result<()> {
        if !facts.is_decision_pass {
            self.push(format!("protocol drain gate {} denied with decision {}", gate.receipt_ref, gate.decision))?;
        }
        if !facts.is_terminal {
            self.push(format!("protocol drain gate {} does not bind terminal session state", gate.receipt_ref))?;
        }
        if !facts.is_protocol_match {
            self.push(format!(
                "protocol drain gate {} is for {}, expected one of {}",
                gate.receipt_ref,
                gate.protocol_ref,
                expected_refs.join(",")
            ))?;
        }
        Ok(())
    }

    fn require_gate(&mut self, refs: &[String]) -> Result<()> {
        if !refs.is_empty() && !self.has_gate {
            self.push("drain-sessions task did not bind any readable protocol session gate receipts".to_string())?;
        }
        Ok(())
    }

    fn outcome(self) -> UpgradeTaskOutcome {
        let decision = if self.diagnostics.is_empty() && self.has_drained_gate {
            "pass"
        } else {
            "deny"
        };
        (decision, self.diagnostics, vec![
            ("protocol-session-gate-bound", pass_fail(self.has_gate)),
            ("protocol-session-gate-pass", pass_fail(self.has_gate_decision_pass)),
            ("protocol-terminal-state", pass_fail(self.has_terminal_state)),
            ("protocol-ref-bound", pass_fail(self.has_protocol_match)),
            ("protocol-session-drain", pass_fail(self.has_drained_gate)),
            ("protocol-drain-is-not-authority", "pass"),
        ])
    }
}

fn protocol_drain_task_outcome(
    ledger_root: &Path,
    plan: &UpgradePlan,
    task: &UpgradeTask,
) -> Result<UpgradeTaskOutcome> {
    let evidence_refs = protocol_drain_evidence_refs(task)?;
    let expected_refs = protocol_drain_expected_protocol_refs(plan, task)?;
    let mut state = DrainState::default();
    state.require_refs(&evidence_refs)?;
    for evidence_ref in &evidence_refs {
        state.inspect_ref(ledger_root, evidence_ref, &expected_refs)?;
    }
    state.require_gate(&evidence_refs)?;
    Ok(state.outcome())
}
