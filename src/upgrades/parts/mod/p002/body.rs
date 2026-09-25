
fn cutover_result(root: &Path, plan: &UpgradePlan, task: &UpgradeTask) -> Result<UpgradeTaskOutcome> {
    let readiness = evaluate_cutover_readiness(root, plan, task)?;
    Ok((readiness.decision, readiness.diagnostics, readiness.checks))
}

fn migration_result(task: &UpgradeTask) -> Result<UpgradeTaskOutcome> {
    let has_recipe = task.to_ref.is_some();
    let has_migration_evidence = !task.postcondition_refs.is_empty();
    let has_policy = !task.precondition_refs.is_empty();
    if has_recipe && has_migration_evidence && has_policy {
        Ok(("pass", Vec::new(), vec![
            ("schema-or-storage-source-ref-bound", "pass"),
            ("migration-recipe-bound", "pass"),
            ("migration-receipt-required", "pass"),
            ("policy-evidence-bound", "pass"),
        ]))
    } else {
        let mut diagnostics = Vec::new();
        if !has_recipe {
            diagnostics.push("migration task lacks executable recipe or target schema ref".to_string());
        }
        if !has_migration_evidence {
            diagnostics.push("migration task lacks migration receipt evidence".to_string());
        }
        if !has_policy {
            diagnostics.push("migration task lacks policy evidence".to_string());
        }
        Ok(("deny", diagnostics, vec![
            ("schema-or-storage-source-ref-bound", pass_fail(task.from_ref.is_some())),
            ("migration-recipe-bound", pass_fail(has_recipe)),
            ("migration-receipt-required", pass_fail(has_migration_evidence)),
            ("policy-evidence-bound", pass_fail(has_policy)),
        ]))
    }
}

fn artifact_update_result(task: &UpgradeTask) -> Result<UpgradeTaskOutcome> {
    let has_exact_refs = task.from_ref.is_some() || task.to_ref.is_some();
    let has_review_evidence = !task.precondition_refs.is_empty() || !task.postcondition_refs.is_empty();
    if has_exact_refs && has_review_evidence {
        Ok(("pass", Vec::new(), vec![
            ("artifact-exact-ref-bound", "pass"),
            ("artifact-review-evidence", "pass"),
            ("side-effect-boundary", "pass"),
        ]))
    } else {
        Ok(("deny", vec!["artifact update requires exact artifact refs and review evidence".to_string()], vec![
            ("artifact-exact-ref-bound", pass_fail(has_exact_refs)),
            ("artifact-review-evidence", pass_fail(has_review_evidence)),
            ("side-effect-boundary", "pass"),
        ]))
    }
}

fn protocol_bridge_result(task: &UpgradeTask) -> Result<UpgradeTaskOutcome> {
    let has_protocol_refs = task.from_ref.is_some() && task.to_ref.is_some();
    let has_protocol_evidence = !task.precondition_refs.is_empty() || !task.postcondition_refs.is_empty();
    if has_protocol_refs && has_protocol_evidence {
        Ok(("pass", Vec::new(), vec![
            ("protocol-ref-bound", "pass"),
            ("protocol-session-gate-bound", "pass"),
            ("side-effect-boundary", "pass"),
        ]))
    } else {
        Ok(("deny", vec!["protocol bridge requires protocol refs and session gate evidence".to_string()], vec![
            ("protocol-ref-bound", pass_fail(has_protocol_refs)),
            ("protocol-session-gate-bound", pass_fail(has_protocol_evidence)),
            ("side-effect-boundary", "pass"),
        ]))
    }
}

fn policy_or_handler_update_result(task: &UpgradeTask) -> Result<UpgradeTaskOutcome> {
    let has_profile_refs = task.from_ref.is_some() && task.to_ref.is_some();
    let has_policy_evidence = !task.precondition_refs.is_empty() || !task.postcondition_refs.is_empty();
    if has_profile_refs && has_policy_evidence {
        Ok(("pass", Vec::new(), vec![
            ("policy-or-handler-ref-bound", "pass"),
            ("policy-admission-required", "pass"),
            ("capability-admission-required", "pass"),
        ]))
    } else {
        Ok(("deny", vec!["policy or handler update requires refs plus policy/capability evidence".to_string()], vec![
            ("policy-or-handler-ref-bound", pass_fail(has_profile_refs)),
            ("policy-admission-required", pass_fail(has_policy_evidence)),
            ("capability-admission-required", pass_fail(has_policy_evidence)),
        ]))
    }
}

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
    let has_retention_evidence = !task.precondition_refs.is_empty();
    let has_dependency_impact_evidence = !task.postcondition_refs.is_empty();
    if !has_retention_evidence || !has_dependency_impact_evidence {
        return Ok(("deny", vec![
            "cleanup requires retention and dependency impact evidence before destructive side effects".to_string(),
        ], vec![
            ("retention-evidence-bound", pass_fail(has_retention_evidence)),
            ("dependency-impact-evidence-bound", pass_fail(has_dependency_impact_evidence)),
            ("cleanup-safety", "fail"),
        ]));
    }
    let cleanup = cleanup_admission(root, ledger_root, cleanup_ref)?;
    if cleanup.decision == "pass" {
        Ok(("pass", Vec::new(), vec![
            ("retention-evidence-bound", "pass"),
            ("dependency-impact-evidence-bound", "pass"),
            ("cleanup-safety", "pass"),
        ]))
    } else {
        Ok(("deny", vec![format!("cleanup denied by receipt {}", cleanup.receipt_ref)], vec![
            ("retention-evidence-bound", "pass"),
            ("dependency-impact-evidence-bound", "pass"),
            ("cleanup-safety", "fail"),
        ]))
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
        if crate::preserves_rail::contains_structural_content_ref(&value, artifact_ref)? {
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
