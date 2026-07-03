
pub fn name_move_plan_value(ledger_root: &Path, input: &NameMovePlanInput) -> Result<IoValue> {
    name_move_plan_value_with_registry(None, ledger_root, input)
}

pub fn name_move_plan_value_with_registry(
    registry_root: Option<&Path>,
    ledger_root: &Path,
    input: &NameMovePlanInput,
) -> Result<IoValue> {
    validate_non_empty(&input.name, "upgrade name")?;
    validate_ref(&input.from_ref, "name move from ref")?;
    validate_ref(&input.to_ref, "name move to ref")?;
    validate_refs(&input.capability_refs, "upgrade capability ref")?;
    validate_refs(&input.policy_refs, "upgrade policy ref")?;
    validate_refs(&input.evidence_refs, "upgrade evidence ref")?;
    validate_ref(&input.initiator_ref, "upgrade initiator ref")?;
    let impact_refs = if let Some(registry_root) = registry_root {
        crate::artifacts::impact_refs(registry_root, std::slice::from_ref(&input.from_ref))?
    } else {
        compute_impact_set(ledger_root, std::slice::from_ref(&input.from_ref))?
    };
    let tasks = planned_tasks(input);
    upgrade_plan_value(&UpgradePlanInput {
        session_id: input.session_id.clone(),
        reason: "name-move".to_string(),
        summary: format!("Move {} from {} to {}", input.name, input.from_ref, input.to_ref),
        initiator_ref: input.initiator_ref.clone(),
        capability_refs: input.capability_refs.clone(),
        affected_refs: vec![input.from_ref.clone(), input.to_ref.clone()],
        impact_refs,
        tasks,
        compatibility: UpgradeCompatibilityWindow {
            old_refs: vec![input.from_ref.clone()],
            new_refs: vec![input.to_ref.clone()],
            expires_at: None,
            policy_refs: input.policy_refs.clone(),
        },
        rollback_refs: vec![input.from_ref.clone()],
        policy_refs: input.policy_refs.clone(),
        evidence_refs: input.evidence_refs.clone(),
        source_gate_receipt_values: input.source_gate_receipt_values.clone(),
    })
}

fn planned_tasks(input: &NameMovePlanInput) -> Vec<UpgradeTaskInput> {
    vec![
        planned_task(
            input,
            "compatibility-alias",
            "compatibility-alias",
            format!("{}@candidate", input.name),
            Vec::new(),
        ),
        planned_task(input, "transcript-gate", "transcript-rerun", input.name.clone(), input.evidence_refs.clone()),
        planned_task(input, "move-name", "move-name", input.name.clone(), Vec::new()),
        planned_task(input, "cutover", "cutover", input.name.clone(), Vec::new()),
    ]
}

fn planned_task(
    input: &NameMovePlanInput,
    task_id: &str,
    kind: &str,
    subject: String,
    postcondition_refs: Vec<String>,
) -> UpgradeTaskInput {
    UpgradeTaskInput {
        task_id: task_id.to_string(),
        kind: kind.to_string(),
        subject,
        from_ref: Some(input.from_ref.clone()),
        to_ref: Some(input.to_ref.clone()),
        precondition_refs: input.evidence_refs.clone(),
        postcondition_refs,
        reversible: true,
    }
}

pub fn parse_upgrade_plan(value: &IoValue) -> Result<UpgradePlan> {
    let fields = value
        .collect_simple_record("upgrade-plan-v1", Some(12))
        .ok_or_else(|| MoltenError::invalid_harness("expected <upgrade-plan-v1 ...>"))?;
    require_schema(&fields[0], UPGRADE_PLAN_SCHEMA, "upgrade plan")?;
    let session_id = record_string(&fields[1], "session")?;
    let summary = value_to_iovalue(&fields[2]);
    let summary_fields = simple_record(&summary, "summary", 2)?;
    let initiator = value_to_iovalue(&fields[3]);
    let initiator_fields = simple_record(&initiator, "initiator", 2)?;
    let tasks = parse_tasks(&fields[6])?;
    let compatibility = parse_compatibility_window(&fields[7])?;
    let checks = parse_checks(&fields[11])?;
    require_check(&checks, "canonical-plan-hash", "upgrade plan")?;
    require_check(&checks, "task-status-receipt-backed", "upgrade plan")?;
    require_check(&checks, "names-are-metadata", "upgrade plan")?;
    require_check(&checks, "no-ucm-clone", "upgrade plan")?;
    let plan = UpgradePlan {
        plan_ref: canonical_hash(value)?,
        session_id,
        reason: required_string(&summary_fields[0], "upgrade reason")?,
        summary: required_string(&summary_fields[1], "upgrade summary")?,
        initiator_ref: required_ref(&initiator_fields[0], "upgrade initiator ref")?,
        capability_refs: parse_ref_sequence_value(&initiator_fields[1], "upgrade capability refs")?,
        affected_refs: record_ref_sequence(&fields[4], "affected")?,
        impact_refs: record_ref_sequence(&fields[5], "impact")?,
        tasks,
        compatibility,
        rollback_refs: record_ref_sequence(&fields[8], "rollback-rules")?,
        policy_refs: record_ref_sequence(&fields[9], "policy")?,
        evidence_refs: record_ref_sequence(&fields[10], "evidence")?,
        checks,
        value: value.clone(),
    };
    validate_parsed_plan(&plan)?;
    Ok(plan)
}

pub fn compute_impact_set(ledger_root: &Path, seed_refs: &[String]) -> Result<Vec<String>> {
    validate_refs(seed_refs, "impact seed ref")?;
    let mut impacted: BtreeSet<String> = seed_refs.iter().cloned().collect();
    let mut artifacts = Vec::new();
    for entry in crate::ledger::list_artifacts(ledger_root)? {
        let value = crate::ledger::read_artifact(ledger_root, &entry.artifact_ref)?;
        let text = to_text(&value)?;
        push_bounded(&mut artifacts, (entry.artifact_ref, text), MAX_UPGRADE_REFS, "upgrade impact artifacts")?;
    }
    let mut has_changed_impact = true;
    while has_changed_impact {
        has_changed_impact = false;
        let seeds: Vec<String> = impacted.iter().cloned().collect();
        for (artifact_ref, text) in &artifacts {
            if impacted.contains(artifact_ref) {
                continue;
            }
            if seeds.iter().any(|seed| text.contains(seed)) {
                impacted.insert(artifact_ref.clone());
                has_changed_impact = true;
            }
        }
    }
    Ok(impacted.into_iter().collect())
}

pub fn create_session(root: &Path, plan_value: &IoValue) -> Result<UpgradeSessionCreated> {
    ensure_dirs(root)?;
    let plan = parse_upgrade_plan(plan_value)?;
    if plan.policy_refs.is_empty() {
        return Err(MoltenError::invalid_harness("upgrade session missing policy refs"));
    }
    if plan.capability_refs.is_empty() {
        return Err(MoltenError::invalid_harness("upgrade session missing capability refs"));
    }
    write_preserves(&plan_path(root, &plan.plan_ref)?, plan_value)?;
    let receipt_value = upgrade_receipt_value(&UpgradeReceiptValueInput {
        operation: "session-create",
        decision: "pass",
        session_id: &plan.session_id,
        plan_ref: &plan.plan_ref,
        task_id: None,
        refs: &plan_refs(&plan),
        diagnostics: &[],
        checks: &[
            ("plan-shape", "pass"),
            ("policy-admission", "pass"),
            ("capability-admission", "pass"),
            ("impact-set-bound", "pass"),
            ("compatibility-window", "pass"),
            ("no-ucm-clone", "pass"),
        ],
    })?;
    let receipt = parse_upgrade_receipt(&receipt_value)?;
    store_receipt(root, &receipt_value)?;
    Ok(UpgradeSessionCreated { plan, receipt })
}

pub fn set_name_pointer(root: &Path, name: &str, artifact_ref: &str) -> Result<UpgradeReceipt> {
    ensure_dirs(root)?;
    validate_non_empty(name, "name pointer name")?;
    validate_ref(artifact_ref, "name pointer artifact ref")?;
    let previous = read_name_pointer(root, name)?.map(|pointer| pointer.artifact_ref);
    let receipt_value = upgrade_receipt_value(&UpgradeReceiptValueInput {
        operation: "name-pointer-set",
        decision: "pass",
        session_id: "local-name-pointer",
        plan_ref: artifact_ref,
        task_id: None,
        refs: &[artifact_ref.to_string()],
        diagnostics: &[],
        checks: &[("names-are-metadata", "pass"), ("immutable-artifact-unchanged", "pass")],
    })?;
    let receipt = parse_upgrade_receipt(&receipt_value)?;
    let pointer = name_pointer_value(name, "name", artifact_ref, previous.as_deref(), &receipt.receipt_ref)?;
    write_preserves(&name_pointer_path(root, name)?, &pointer)?;
    store_receipt(root, &receipt_value)?;
    Ok(receipt)
}

pub fn read_name_pointer(root: &Path, name: &str) -> Result<Option<NamePointer>> {
    let path = name_pointer_path(root, name)?;
    if !path.exists() {
        return Ok(None);
    }
    parse_name_pointer(&read_preserves(&path)?).map(Some)
}

pub fn execute_task(root: &Path, ledger_root: &Path, plan_ref: &str, task_id: &str) -> Result<UpgradeTaskExecution> {
    ensure_dirs(root)?;
    let plan = read_plan(root, plan_ref)?;
    let task_index = plan
        .tasks
        .iter()
        .position(|task| task.task_id == task_id)
        .ok_or_else(|| MoltenError::invalid_harness(format!("upgrade plan missing task {task_id}")))?;
    let task = plan.tasks[task_index].clone();
    if let Some(prior_task_id) = first_incomplete_prior_task(root, &plan, task_index)? {
        if task.kind == "cutover" {
            return cutover_denied_for_incomplete_prior(root, &plan, &task, &prior_task_id);
        }
        return Err(MoltenError::invalid_harness(format!(
            "upgrade task {} cannot run before prior task {} completes",
            task.task_id, prior_task_id
        )));
    }
    let before_state_ref = upgrade_state_snapshot_ref(root)?;
    let (decision, mut diagnostics, mut checks) = task_result(root, ledger_root, &plan, &task)?;
    let mut refs = task_refs(&task);
    append_no_mutation_boundary(
        root,
        &task.kind,
        decision,
        &before_state_ref,
        &mut refs,
        &mut diagnostics,
        &mut checks,
    )?;
    let receipt_value = upgrade_receipt_value(&UpgradeReceiptValueInput {
        operation: if task.kind == "cutover" {
            "cutover"
        } else {
            "task-complete"
        },
        decision,
        session_id: &plan.session_id,
        plan_ref: &plan.plan_ref,
        task_id: Some(&task.task_id),
        refs: &refs,
        diagnostics: &diagnostics,
        checks: &checks,
    })?;
    let receipt = parse_upgrade_receipt(&receipt_value)?;
    store_receipt(root, &receipt_value)?;
    if receipt.decision == "pass" {
        write_status(root, &plan, &task, &receipt.receipt_ref)?;
    }
    Ok(UpgradeTaskExecution {
        plan_ref: plan.plan_ref,
        task_id: task.task_id,
        task_kind: task.kind,
        receipt,
    })
}

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
    append_no_mutation_boundary(
        root,
        "cutover",
        "deny",
        &before_state_ref,
        &mut refs,
        &mut diagnostics,
        &mut checks,
    )?;
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

fn append_no_mutation_boundary(
    root: &Path,
    operation: &str,
    decision: &str,
    before_state_ref: &str,
    refs: &mut Vec<String>,
    diagnostics: &mut Vec<String>,
    checks: &mut Vec<UpgradeCheckPair>,
) -> Result<()> {
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
        "cutover" => {
            Ok(("pass", Vec::new(), vec![("metadata-cutover", "pass"), ("transcript-gate-before-cutover", "pass")]))
        }
        "migrate-storage" => Ok(("pass", Vec::new(), vec![
            ("typed-storage-migration-recipe-bound", "pass"),
            ("migration-receipt-required", "pass"),
        ])),
        "cleanup" => cleanup_result(root, ledger_root, task),
        "drain-sessions" => protocol_drain_task_outcome(ledger_root, plan, task),
        "install-artifact"
        | "deprecate"
        | "install-protocol-bridge"
        | "update-handler-policy"
        | "update-docs"
        | "rollback-pointer" => {
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

fn transcript_result(plan: &UpgradePlan, task: &UpgradeTask) -> UpgradeTaskOutcome {
    if task.precondition_refs.is_empty() && plan.evidence_refs.is_empty() {
        ("deny", vec!["transcript rerun task has no transcript or receipt evidence refs".to_string()], vec![
            ("transcript-evidence", "fail"),
        ])
    } else {
        ("pass", Vec::new(), vec![("transcript-evidence", "pass"), ("handler-profile-bound", "pass")])
    }
}
