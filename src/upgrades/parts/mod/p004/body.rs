
fn parse_task(value: &IoValue) -> Result<UpgradeTask> {
    let fields = value
        .collect_simple_record("upgrade-task-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <upgrade-task-v1 ...>"))?;
    let reversible_value = value_to_iovalue(&fields[7]);
    let reversible = simple_record(&reversible_value, "reversible", 1)?;
    Ok(UpgradeTask {
        task_id: required_string(&fields[0], "upgrade task id")?,
        kind: record_string(&fields[1], "kind")?,
        subject: record_string(&fields[2], "subject")?,
        from_ref: record_optional_ref(&fields[3], "from")?,
        to_ref: record_optional_ref(&fields[4], "to")?,
        precondition_refs: record_ref_sequence(&fields[5], "preconditions")?,
        postcondition_refs: record_ref_sequence(&fields[6], "postconditions")?,
        reversible: required_bool(&reversible[0], "reversible")?,
    })
}

pub fn parse_upgrade_receipt(value: &IoValue) -> Result<UpgradeReceipt> {
    let fields = value
        .collect_simple_record("upgrade-receipt-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <upgrade-receipt-v1 ...>"))?;
    require_schema(&fields[0], UPGRADE_RECEIPT_SCHEMA, "upgrade receipt")?;
    let session = value_to_iovalue(&fields[3]);
    let session_fields = simple_record(&session, "session", 2)?;
    let task = value_to_iovalue(&fields[4]);
    let task_fields = simple_record(&task, "task", 1)?;
    let checks = parse_checks(&fields[7])?;
    if checks.is_empty() {
        return Err(MoltenError::invalid_harness("upgrade receipt missing checks"));
    }
    Ok(UpgradeReceipt {
        receipt_ref: canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        decision: record_string(&fields[2], "decision")?,
        session_id: required_string(&session_fields[0], "upgrade receipt session id")?,
        plan_ref: required_ref(&session_fields[1], "upgrade receipt plan ref")?,
        task_id: parse_optional_string_value(&task_fields[0])?,
        value: value.clone(),
    })
}

fn upgrade_receipt_value(input: &UpgradeReceiptValueInput<'_>) -> Result<IoValue> {
    validate_non_empty(input.operation, "upgrade receipt operation")?;
    if input.decision != "pass" && input.decision != "deny" {
        return Err(MoltenError::invalid_harness(format!("unsupported upgrade receipt decision {}", input.decision)));
    }
    validate_non_empty(input.session_id, "upgrade receipt session id")?;
    validate_ref(input.plan_ref, "upgrade receipt plan ref")?;
    validate_refs(input.refs, "upgrade receipt ref")?;
    Ok(record("upgrade-receipt-v1", vec![
        string(UPGRADE_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("session", vec![string(input.session_id), string(input.plan_ref)]),
        record("task", vec![optional_string_value(input.task_id)]),
        record("refs", vec![refs_sequence(input.refs)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value_from_pairs(input.checks),
    ]))
}

fn name_pointer_value(
    name: &str,
    pointer_kind: &str,
    artifact_ref: &str,
    previous_ref: Option<&str>,
    receipt_ref: &str,
) -> Result<IoValue> {
    validate_non_empty(name, "name pointer name")?;
    validate_non_empty(pointer_kind, "name pointer kind")?;
    validate_ref(artifact_ref, "name pointer artifact ref")?;
    if let Some(previous_ref) = previous_ref {
        validate_ref(previous_ref, "name pointer previous ref")?;
    }
    validate_ref(receipt_ref, "name pointer receipt ref")?;
    Ok(record("upgrade-name-pointer-v1", vec![
        string(UPGRADE_NAME_POINTER_SCHEMA),
        record("name", vec![string(name)]),
        record("kind", vec![string(pointer_kind)]),
        record("artifact", vec![string(artifact_ref)]),
        record("previous", vec![optional_ref_value(previous_ref)]),
        record("receipt", vec![string(receipt_ref)]),
        checks_value(&["names-are-metadata", "artifact-content-immutable"]),
    ]))
}

fn parse_name_pointer(value: &IoValue) -> Result<NamePointer> {
    let fields = value
        .collect_simple_record("upgrade-name-pointer-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <upgrade-name-pointer-v1 ...>"))?;
    require_schema(&fields[0], UPGRADE_NAME_POINTER_SCHEMA, "upgrade name pointer")?;
    let checks = parse_checks(&fields[6])?;
    require_check(&checks, "names-are-metadata", "upgrade name pointer")?;
    Ok(NamePointer {
        name: record_string(&fields[1], "name")?,
        pointer_kind: record_string(&fields[2], "kind")?,
        artifact_ref: record_ref(&fields[3], "artifact")?,
        previous_ref: record_optional_ref(&fields[4], "previous")?,
        receipt_ref: record_ref(&fields[5], "receipt")?,
        value: value.clone(),
    })
}

fn plan_refs(plan: &UpgradePlan) -> Vec<String> {
    let mut refs = BtreeSet::new();
    refs.insert(plan.plan_ref.clone());
    refs.insert(plan.initiator_ref.clone());
    refs.extend(plan.capability_refs.iter().cloned());
    refs.extend(plan.affected_refs.iter().cloned());
    refs.extend(plan.impact_refs.iter().cloned());
    refs.extend(plan.rollback_refs.iter().cloned());
    refs.extend(plan.policy_refs.iter().cloned());
    refs.extend(plan.evidence_refs.iter().cloned());
    for task in &plan.tasks {
        refs.extend(task_refs(task));
    }
    refs.into_iter().collect()
}

fn task_refs(task: &UpgradeTask) -> Vec<String> {
    let mut refs = BtreeSet::new();
    if let Some(value) = task.from_ref.as_ref() {
        refs.insert(value.clone());
    }
    if let Some(value) = task.to_ref.as_ref() {
        refs.insert(value.clone());
    }
    refs.extend(task.precondition_refs.iter().cloned());
    refs.extend(task.postcondition_refs.iter().cloned());
    refs.into_iter().collect()
}

fn first_incomplete_prior_task(root: &Path, plan: &UpgradePlan, task_index: usize) -> Result<Option<String>> {
    for task in &plan.tasks[..task_index] {
        if read_status_receipt_ref(root, plan, &task.task_id)?.is_none() {
            return Ok(Some(task.task_id.clone()));
        }
    }
    Ok(None)
}

fn evaluate_cutover_readiness(
    root: &Path,
    plan: &UpgradePlan,
    task: &UpgradeTask,
) -> Result<UpgradeCutoverReadinessDecision> {
    let has_exact_cutover_refs = task.from_ref.is_some() && task.to_ref.is_some();
    let has_impact_evidence = !plan.impact_refs.is_empty();
    let has_compatibility =
        !plan.compatibility.old_refs.is_empty() && !plan.compatibility.new_refs.is_empty() && !plan.compatibility.policy_refs.is_empty();
    let has_policy = !plan.policy_refs.is_empty();
    let has_capability = !plan.capability_refs.is_empty();
    let has_source_gate = !plan.evidence_refs.is_empty();
    let has_rollback = !plan.rollback_refs.is_empty();
    let has_transcript_task = plan.tasks.iter().any(|candidate| candidate.kind == "transcript-rerun");
    let has_replay = task_kind_completed(root, plan, "transcript-rerun")?;
    let is_migration_complete = task_kind_completed_or_absent(root, plan, &["migrate-schema", "migrate-storage"])?;
    let is_protocol_complete = task_kind_completed_or_absent(root, plan, &["drain-sessions"])?;
    let is_ready = has_exact_cutover_refs
        && has_impact_evidence
        && has_compatibility
        && has_policy
        && has_capability
        && has_source_gate
        && has_rollback
        && has_transcript_task
        && has_replay
        && is_migration_complete
        && is_protocol_complete;
    let mut diagnostics = Vec::new();
    push_cutover_diagnostic(&mut diagnostics, has_exact_cutover_refs, "cutover requires exact from/to refs")?;
    push_cutover_diagnostic(&mut diagnostics, has_impact_evidence, "cutover requires dependency impact evidence")?;
    push_cutover_diagnostic(&mut diagnostics, has_compatibility, "cutover requires compatibility evidence")?;
    push_cutover_diagnostic(&mut diagnostics, has_policy, "cutover requires policy evidence")?;
    push_cutover_diagnostic(&mut diagnostics, has_capability, "cutover requires capability evidence")?;
    push_cutover_diagnostic(&mut diagnostics, has_source_gate, "cutover requires source-gate, provenance, build, or review evidence")?;
    push_cutover_diagnostic(&mut diagnostics, has_rollback, "cutover requires rollback strategy refs")?;
    push_cutover_diagnostic(&mut diagnostics, has_transcript_task, "cutover requires transcript replay task")?;
    push_cutover_diagnostic(&mut diagnostics, has_replay, "cutover requires passing replay receipt")?;
    push_cutover_diagnostic(&mut diagnostics, is_migration_complete, "cutover requires completed migration receipts")?;
    push_cutover_diagnostic(&mut diagnostics, is_protocol_complete, "cutover requires completed protocol session drain")?;
    Ok(UpgradeCutoverReadinessDecision {
        decision: if is_ready { "pass" } else { "deny" },
        diagnostics,
        checks: vec![
            ("exact-cutover-refs-bound", pass_fail(has_exact_cutover_refs)),
            ("impact-query-bound", pass_fail(has_impact_evidence)),
            ("compatibility-bound", pass_fail(has_compatibility)),
            ("policy-evidence-bound", pass_fail(has_policy)),
            ("capability-evidence-bound", pass_fail(has_capability)),
            ("source-gate-bound", pass_fail(has_source_gate)),
            ("rollback-strategy-bound", pass_fail(has_rollback)),
            ("replay-receipt-bound", pass_fail(has_replay)),
            ("migration-receipt-bound", pass_fail(is_migration_complete)),
            ("protocol-session-drain-bound", pass_fail(is_protocol_complete)),
            ("metadata-cutover", pass_fail(is_ready)),
        ],
    })
}

fn task_kind_completed_or_absent(root: &Path, plan: &UpgradePlan, kinds: &[&str]) -> Result<bool> {
    let tasks: Vec<_> = plan.tasks.iter().filter(|task| kinds.contains(&task.kind.as_str())).collect();
    if tasks.is_empty() {
        return Ok(true);
    }
    for task in tasks {
        if read_status_receipt_ref(root, plan, &task.task_id)?.is_none() {
            return Ok(false);
        }
    }
    Ok(true)
}

fn task_kind_completed(root: &Path, plan: &UpgradePlan, kind: &str) -> Result<bool> {
    let tasks: Vec<_> = plan.tasks.iter().filter(|task| task.kind == kind).collect();
    if tasks.is_empty() {
        return Ok(false);
    }
    for task in tasks {
        if read_status_receipt_ref(root, plan, &task.task_id)?.is_none() {
            return Ok(false);
        }
    }
    Ok(true)
}

fn push_cutover_diagnostic(diagnostics: &mut impl crate::bounded::VecSink<String>, condition: bool, diagnostic: &str) -> Result<()> {
    if !condition {
        push_bounded(
            diagnostics,
            diagnostic.to_string(),
            MAX_UPGRADE_DIAGNOSTICS,
            "upgrade cutover diagnostics",
        )?;
    }
    Ok(())
}

fn evaluate_upgrade_no_mutation_boundary(
    input: &UpgradeMutationBoundaryInput<'_>,
) -> Result<UpgradeMutationBoundaryDecision> {
    validate_non_empty(input.operation, "upgrade no-mutation operation")?;
    validate_ref(input.before_state_ref, "upgrade no-mutation before state ref")?;
    validate_ref(input.after_state_ref, "upgrade no-mutation after state ref")?;
    if input.decision != "pass" && input.decision != "deny" {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported upgrade no-mutation decision {}",
            input.decision
        )));
    }
    let is_preserved = input.before_state_ref == input.after_state_ref;
    let mut diagnostics = Vec::new();
    if input.decision == "deny" && !is_preserved {
        push_bounded(
            &mut diagnostics,
            format!(
                "upgrade {} denial mutated pre-cutover state: before {} after {}",
                input.operation, input.before_state_ref, input.after_state_ref
            ),
            MAX_UPGRADE_DIAGNOSTICS,
            "upgrade no-mutation diagnostics",
        )?;
    }
    let check_status = if input.decision == "deny" {
        pass_fail(is_preserved)
    } else {
        "pass"
    };
    Ok(UpgradeMutationBoundaryDecision {
        diagnostics,
        checks: vec![("no-mutation-on-deny", check_status)],
    })
}

fn upgrade_state_snapshot_ref(root: &Path) -> Result<String> {
    let mut entries = Vec::new();
    for dir_name in UPGRADE_STATE_SNAPSHOT_DIRS {
        collect_upgrade_state_snapshot_entries(root, dir_name, &mut entries)?;
    }
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    let values = entries
        .iter()
        .map(|(path, content_ref)| record("file", vec![string(path), string(content_ref)]))
        .collect();
    canonical_hash(&record("upgrade-state-snapshot-v1", vec![sequence(values)]))
}
