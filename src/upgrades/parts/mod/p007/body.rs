
fn cutover_denied_for_incomplete_prior(
    root: &Path,
    plan: &UpgradePlan,
    task: &UpgradeTask,
    prior_task_id: &str,
) -> Result<UpgradeTaskExecution> {
    let before_state_ref = upgrade_state_snapshot_ref(root)?;
    let mut refs = task_refs(task);
    let mut diagnostics = vec![format!(
        "upgrade task {} cannot run before prior task {} completes",
        task.task_id, prior_task_id
    )];
    let mut checks = vec![
        ("task-order", "fail"),
        ("metadata-cutover", "fail"),
        ("transcript-gate-before-cutover", "fail"),
    ];
    append_no_mutation_boundary(root, "cutover", "deny", &before_state_ref, BoundarySinks { refs: &mut refs, diagnostics: &mut diagnostics, checks: &mut checks })?;
    let receipt_value = upgrade_receipt_value(&UpgradeReceiptValueInput {
        operation: "cutover",
        decision: "deny",
        session_id: &plan.session_id,
        plan_ref: &plan.plan_ref,
        task_id: Some(&task.task_id),
        refs: &refs,
        diagnostics: &diagnostics,
        checks: &checks,
    })?;
    let receipt = parse_upgrade_receipt(&receipt_value)?;
    store_receipt(root, &receipt_value)?;
    Ok(UpgradeTaskExecution {
        plan_ref: plan.plan_ref.clone(),
        task_id: task.task_id.clone(),
        task_kind: task.kind.clone(),
        receipt,
    })
}

struct BoundarySinks<'a> {
    refs: &'a mut Vec<String>,
    diagnostics: &'a mut Vec<String>,
    checks: &'a mut Vec<UpgradeCheckPair>,
}

fn append_no_mutation_boundary(root: &Path, operation: &str, decision: &str, before_state_ref: &str, input: BoundarySinks<'_>) -> Result<()> {
    let BoundarySinks { refs, diagnostics, checks } = input;
    if decision != "deny" {
        return Ok(());
    }
    let after_state_ref = upgrade_state_snapshot_ref(root)?;
    push_bounded(
        refs,
        before_state_ref.to_string(),
        MAX_UPGRADE_REFS,
        "upgrade denial state refs",
    )?;
    push_bounded(refs, after_state_ref.clone(), MAX_UPGRADE_REFS, "upgrade denial state refs")?;
    let boundary = evaluate_upgrade_no_mutation_boundary(&UpgradeMutationBoundaryInput {
        operation,
        decision,
        before_state_ref,
        after_state_ref: &after_state_ref,
    })?;
    for diagnostic in boundary.diagnostics {
        push_bounded(
            diagnostics,
            diagnostic,
            MAX_UPGRADE_DIAGNOSTICS,
            "upgrade no-mutation diagnostics",
        )?;
    }
    checks.extend(boundary.checks);
    Ok(())
}

fn task_result(root: &Path, ledger_root: &Path, plan: &UpgradePlan, task: &UpgradeTask) -> Result<UpgradeTaskOutcome> {
    match task.kind.as_str() {
        "compatibility-alias" => alias_result(root, plan, task),
        "transcript-rerun" => Ok(transcript_result(plan, task)),
        "move-name" => move_result(root, plan, task),
        "cutover" => cutover_result(root, plan, task),
        "migrate-storage" | "migrate-schema" => migration_result(task),
        "cleanup" => cleanup_result(root, ledger_root, task),
        "drain-sessions" => protocol_drain_task_outcome(ledger_root, plan, task),
        "replace-artifact" | "install-artifact" | "deprecate" => artifact_update_result(task),
        "install-protocol-bridge" => protocol_bridge_result(task),
        "update-policy" | "update-handler-profile" | "update-handler-policy" => policy_or_handler_update_result(task),
        "update-docs" | "rollback-pointer" => {
            Ok(("pass", Vec::new(), vec![("task-admission", "pass"), ("side-effect-boundary", "pass")]))
        }
        other => Err(MoltenError::invalid_harness(format!(
            "unsupported upgrade task kind {other}; expected one of {:?}",
            SUPPORTED_TASK_KINDS
        ))),
    }
}

fn alias_result(root: &Path, plan: &UpgradePlan, task: &UpgradeTask) -> Result<UpgradeTaskOutcome> {
    let to_ref = task
        .to_ref
        .as_deref()
        .ok_or_else(|| MoltenError::invalid_harness("compatibility alias missing target ref"))?;
    let previous = task.from_ref.as_deref();
    let pending_receipt_ref = local_ref("upgrade-pending-receipt", &plan.plan_ref, &task.task_id)?;
    let pointer = name_pointer_value(&task.subject, "alias", to_ref, previous, &pending_receipt_ref)?;
    write_preserves(&name_pointer_path(root, &task.subject)?, &pointer)?;
    Ok(("pass", Vec::new(), vec![("compatibility-alias", "pass"), ("old-and-new-coexist", "pass")]))
}

fn transcript_result(_plan: &UpgradePlan, task: &UpgradeTask) -> UpgradeTaskOutcome {
    if task.precondition_refs.is_empty() {
        ("deny", vec!["transcript rerun task has no transcript, replay, or receipt evidence refs".to_string()], vec![
            ("transcript-evidence", "fail"),
            ("replay-receipt-required", "fail"),
        ])
    } else {
        ("pass", Vec::new(), vec![
            ("transcript-evidence", "pass"),
            ("replay-receipt-required", "pass"),
            ("handler-profile-bound", "pass"),
        ])
    }
}
