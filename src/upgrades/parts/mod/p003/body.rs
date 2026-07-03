
fn protocol_drain_evidence_refs(task: &UpgradeTask) -> Result<Vec<String>> {
    let mut refs = BtreeSet::new();
    refs.extend(task.precondition_refs.iter().cloned());
    refs.extend(task.postcondition_refs.iter().cloned());
    let refs: Vec<String> = refs.into_iter().collect();
    validate_refs(&refs, "upgrade protocol drain evidence ref")?;
    Ok(refs)
}

fn protocol_drain_expected_protocol_refs_from_bindings(
    subject: &str,
    from_ref: Option<&str>,
    affected_refs: &[String],
    compatibility_old_refs: &[String],
) -> Result<Vec<String>> {
    let mut refs = BtreeSet::new();
    if let Some(from_ref) = from_ref {
        refs.insert(from_ref.to_string());
    } else if is_canonical_ref(subject) {
        refs.insert(subject.to_string());
    } else {
        refs.extend(compatibility_old_refs.iter().cloned());
        if refs.is_empty() {
            refs.extend(affected_refs.iter().cloned());
        }
    }
    if refs.is_empty() {
        return Err(MoltenError::invalid_harness("drain-sessions task has no protocol ref binding"));
    }
    let refs: Vec<String> = refs.into_iter().collect();
    validate_refs(&refs, "upgrade protocol drain expected protocol ref")?;
    Ok(refs)
}

fn is_canonical_ref(value: &str) -> bool {
    validate_content_ref(value).is_ok()
}

fn pass_fail(is_pass: bool) -> &'static str {
    if is_pass { "pass" } else { "fail" }
}

pub fn status(root: &Path, plan_ref: &str) -> Result<UpgradeStatus> {
    let plan = read_plan(root, plan_ref)?;
    ensure_count_at_most(plan.tasks.len(), MAX_UPGRADE_TASKS, "upgrade plan tasks")?;
    let mut tasks = Vec::with_capacity(plan.tasks.len());
    let mut remaining_task_ids = Vec::new();
    for task in &plan.tasks {
        let receipt_ref = read_status_receipt_ref(root, &plan, &task.task_id)?;
        let is_task_done = receipt_ref.is_some();
        if !is_task_done {
            push_bounded(&mut remaining_task_ids, task.task_id.clone(), MAX_UPGRADE_TASKS, "upgrade remaining tasks")?;
        }
        push_bounded(
            &mut tasks,
            UpgradeTaskStatus {
                task_id: task.task_id.clone(),
                kind: task.kind.clone(),
                done: is_task_done,
                receipt_ref,
            },
            MAX_UPGRADE_TASKS,
            "upgrade task status entries",
        )?;
    }
    Ok(UpgradeStatus {
        plan_ref: plan.plan_ref,
        session_id: plan.session_id,
        tasks,
        remaining_task_ids,
    })
}

fn read_plan(root: &Path, plan_ref: &str) -> Result<UpgradePlan> {
    validate_ref(plan_ref, "upgrade plan ref")?;
    parse_upgrade_plan(&read_preserves(&plan_path(root, plan_ref)?)?)
}

fn validate_upgrade_source_gates(input: &UpgradePlanInput) -> Result<Vec<String>> {
    if input.source_gate_receipt_values.is_empty() {
        return Err(MoltenError::invalid_harness("upgrade plan requires strict Octet source gate receipt values"));
    }
    ensure_count_at_most(
        input.source_gate_receipt_values.len(),
        MAX_UPGRADE_SOURCE_GATES,
        "upgrade source gate receipt values",
    )?;
    let subject_ref = source_gate_subject_ref(&input.session_id, &input.affected_refs)?;
    let mut validation_refs = Vec::new();
    let mut diagnostics = Vec::new();
    for value in &input.source_gate_receipt_values {
        let validation =
            crate::octet_gate::validate_octet_source_gate(&crate::octet_gate::OctetSourceGateValidationInput {
                consumer: "upgrade-plan".to_string(),
                subject_ref: subject_ref.clone(),
                receipt_value: Some(value.clone()),
                source_scope: Vec::new(),
            })?;
        push_bounded(
            &mut validation_refs,
            validation.validation_ref.clone(),
            MAX_UPGRADE_SOURCE_GATES,
            "upgrade source gate validation refs",
        )?;
        if validation.decision != "pass" {
            push_bounded(
                &mut diagnostics,
                format!("strict Octet source gate validation {} denied", validation.validation_ref),
                MAX_UPGRADE_DIAGNOSTICS,
                "upgrade source gate diagnostics",
            )?;
        }
    }
    if validation_refs.is_empty() || !diagnostics.is_empty() {
        return Err(MoltenError::invalid_harness(format!(
            "upgrade plan source gate validation failed: {}",
            diagnostics.join("; ")
        )));
    }
    Ok(validation_refs)
}

fn source_gate_subject_ref(session_id: &str, affected_refs: &[String]) -> Result<String> {
    canonical_hash(&record("upgrade-source-gate-subject-v1", vec![
        string(session_id),
        refs_sequence(&sorted_refs(affected_refs.to_vec())),
    ]))
}

fn sorted_refs(mut refs: Vec<String>) -> Vec<String> {
    refs.sort();
    refs.dedup();
    refs
}

fn validate_plan_input(input: &UpgradePlanInput) -> Result<()> {
    validate_non_empty(&input.session_id, "upgrade session id")?;
    validate_non_empty(&input.reason, "upgrade reason")?;
    validate_non_empty(&input.summary, "upgrade summary")?;
    validate_ref(&input.initiator_ref, "upgrade initiator ref")?;
    validate_refs(&input.capability_refs, "upgrade capability ref")?;
    validate_refs(&input.affected_refs, "upgrade affected ref")?;
    validate_refs(&input.impact_refs, "upgrade impact ref")?;
    validate_refs(&input.rollback_refs, "upgrade rollback ref")?;
    validate_refs(&input.policy_refs, "upgrade policy ref")?;
    validate_refs(&input.evidence_refs, "upgrade evidence ref")?;
    validate_compatibility(&input.compatibility)?;
    if input.tasks.is_empty() {
        return Err(MoltenError::invalid_harness("upgrade plan must contain at least one task"));
    }
    let mut seen = BtreeSet::new();
    for task in &input.tasks {
        validate_task_input(task)?;
        if !seen.insert(task.task_id.clone()) {
            return Err(MoltenError::invalid_harness(format!("duplicate upgrade task id {}", task.task_id)));
        }
    }
    Ok(())
}

fn validate_parsed_plan(plan: &UpgradePlan) -> Result<()> {
    validate_non_empty(&plan.session_id, "upgrade session id")?;
    validate_ref(&plan.initiator_ref, "upgrade initiator ref")?;
    validate_refs(&plan.capability_refs, "upgrade capability ref")?;
    validate_refs(&plan.affected_refs, "upgrade affected ref")?;
    validate_refs(&plan.impact_refs, "upgrade impact ref")?;
    validate_refs(&plan.rollback_refs, "upgrade rollback ref")?;
    validate_refs(&plan.policy_refs, "upgrade policy ref")?;
    validate_refs(&plan.evidence_refs, "upgrade evidence ref")?;
    validate_compatibility(&plan.compatibility)?;
    if plan.tasks.is_empty() {
        return Err(MoltenError::invalid_harness("upgrade plan must contain at least one task"));
    }
    let mut seen = BtreeSet::new();
    for task in &plan.tasks {
        validate_task(task)?;
        if !seen.insert(task.task_id.clone()) {
            return Err(MoltenError::invalid_harness(format!("duplicate upgrade task id {}", task.task_id)));
        }
    }
    if plan.tasks.iter().any(|task| task.kind == "cutover")
        && !plan.tasks.iter().any(|task| task.kind == "transcript-rerun")
    {
        return Err(MoltenError::invalid_harness("upgrade cutover requires a transcript-rerun task before cutover"));
    }
    Ok(())
}

fn validate_task_input(task: &UpgradeTaskInput) -> Result<()> {
    validate_non_empty(&task.task_id, "upgrade task id")?;
    validate_non_empty(&task.subject, "upgrade task subject")?;
    validate_task_kind(&task.kind)?;
    if let Some(value) = task.from_ref.as_deref() {
        validate_ref(value, "upgrade task from ref")?;
    }
    if let Some(value) = task.to_ref.as_deref() {
        validate_ref(value, "upgrade task to ref")?;
    }
    validate_refs(&task.precondition_refs, "upgrade task precondition ref")?;
    validate_refs(&task.postcondition_refs, "upgrade task postcondition ref")?;
    validate_task_shape(&task.kind, task.from_ref.as_deref(), task.to_ref.as_deref(), task.reversible)
}

fn validate_task(task: &UpgradeTask) -> Result<()> {
    validate_non_empty(&task.task_id, "upgrade task id")?;
    validate_non_empty(&task.subject, "upgrade task subject")?;
    validate_task_kind(&task.kind)?;
    if let Some(value) = task.from_ref.as_deref() {
        validate_ref(value, "upgrade task from ref")?;
    }
    if let Some(value) = task.to_ref.as_deref() {
        validate_ref(value, "upgrade task to ref")?;
    }
    validate_refs(&task.precondition_refs, "upgrade task precondition ref")?;
    validate_refs(&task.postcondition_refs, "upgrade task postcondition ref")?;
    validate_task_shape(&task.kind, task.from_ref.as_deref(), task.to_ref.as_deref(), task.reversible)
}

fn validate_task_shape(kind: &str, from_ref: Option<&str>, to_ref: Option<&str>, reversible: bool) -> Result<()> {
    match kind {
        "move-name" | "compatibility-alias" | "cutover" | "rollback-pointer" => {
            if from_ref.is_none() || to_ref.is_none() {
                return Err(MoltenError::invalid_harness(format!("upgrade task kind {kind} requires from/to refs")));
            }
        }
        "migrate-storage" => {
            if from_ref.is_none() || to_ref.is_none() {
                return Err(MoltenError::invalid_harness("storage migration upgrade task requires recipe/source refs"));
            }
            if reversible {
                return Err(MoltenError::invalid_harness(
                    "storage migration upgrade task cannot claim reversible rollback",
                ));
            }
        }
        "cleanup" if from_ref.is_none() && to_ref.is_none() => {
            return Err(MoltenError::invalid_harness("cleanup upgrade task requires an artifact ref"));
        }
        "cleanup" => {}
        _ => {}
    }
    Ok(())
}

fn validate_task_kind(kind: &str) -> Result<()> {
    if SUPPORTED_TASK_KINDS.contains(&kind) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "unsupported upgrade task kind {kind}; expected one of {:?}",
            SUPPORTED_TASK_KINDS
        )))
    }
}

fn validate_compatibility(compatibility: &UpgradeCompatibilityWindow) -> Result<()> {
    validate_refs(&compatibility.old_refs, "compatibility old ref")?;
    validate_refs(&compatibility.new_refs, "compatibility new ref")?;
    validate_refs(&compatibility.policy_refs, "compatibility policy ref")?;
    let old: BtreeSet<_> = compatibility.old_refs.iter().collect();
    if compatibility.new_refs.iter().any(|new_ref| old.contains(new_ref)) {
        return Err(MoltenError::invalid_harness("compatibility window old/new refs must be explicit and distinct"));
    }
    Ok(())
}

fn compatibility_window_value(compatibility: &UpgradeCompatibilityWindow) -> Result<IoValue> {
    validate_compatibility(compatibility)?;
    Ok(record("compatibility-window", vec![
        record("old", vec![refs_sequence(&compatibility.old_refs)]),
        record("new", vec![refs_sequence(&compatibility.new_refs)]),
        record("expires-at", vec![optional_u64_value(compatibility.expires_at)]),
        record("policy", vec![refs_sequence(&compatibility.policy_refs)]),
    ]))
}

fn parse_compatibility_window(value: &Value<IoValue>) -> Result<UpgradeCompatibilityWindow> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, "compatibility-window", 4)?;
    Ok(UpgradeCompatibilityWindow {
        old_refs: record_ref_sequence(&fields[0], "old")?,
        new_refs: record_ref_sequence(&fields[1], "new")?,
        expires_at: record_optional_u64(&fields[2], "expires-at")?,
        policy_refs: record_ref_sequence(&fields[3], "policy")?,
    })
}

fn parse_tasks(value: &Value<IoValue>) -> Result<Vec<UpgradeTask>> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, "tasks", 1)?;
    let items = required_sequence(&fields[0], "upgrade tasks")?;
    let mut tasks = Vec::with_capacity(items.len());
    for item in items.iter() {
        tasks.push(parse_task(&value_to_iovalue(item))?);
    }
    Ok(tasks)
}
