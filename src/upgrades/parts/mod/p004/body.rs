
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

fn ensure_prior_tasks_complete(root: &Path, plan: &UpgradePlan, task_index: usize) -> Result<()> {
    for task in &plan.tasks[..task_index] {
        if read_status_receipt_ref(root, plan, &task.task_id)?.is_none() {
            return Err(MoltenError::invalid_harness(format!(
                "upgrade task {} cannot run before prior task {} completes",
                plan.tasks[task_index].task_id, task.task_id
            )));
        }
    }
    Ok(())
}

fn write_status(root: &Path, plan: &UpgradePlan, task: &UpgradeTask, receipt_ref: &str) -> Result<()> {
    validate_ref(receipt_ref, "upgrade task status receipt ref")?;
    let path = status_path(root, &plan.session_id, &task.task_id)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, receipt_ref).map_err(MoltenError::from)
}

fn read_status_receipt_ref(root: &Path, plan: &UpgradePlan, task_id: &str) -> Result<Option<String>> {
    let path = status_path(root, &plan.session_id, task_id)?;
    if !path.exists() {
        return Ok(None);
    }
    let receipt_ref = fs::read_to_string(path).map_err(MoltenError::from)?;
    validate_ref(&receipt_ref, "upgrade task status receipt ref")?;
    Ok(Some(receipt_ref))
}

fn read_name_pointers(root: &Path) -> Result<Vec<NamePointer>> {
    let names = root.join("names");
    if !names.exists() {
        return Ok(Vec::new());
    }
    let mut pointers = Vec::new();
    for entry in fs::read_dir(names).map_err(MoltenError::from)? {
        let entry = entry.map_err(MoltenError::from)?;
        if entry.file_type().map_err(MoltenError::from)?.is_file() {
            push_bounded(
                &mut pointers,
                parse_name_pointer(&read_preserves(&entry.path())?)?,
                MAX_UPGRADE_POINTERS,
                "upgrade name pointers",
            )?;
        }
    }
    Ok(pointers)
}

fn store_text_contains_ref(dir: &Path, target_ref: &str) -> Result<bool> {
    if !dir.exists() {
        return Ok(false);
    }
    let mut pending_dirs = Vec::with_capacity(1);
    pending_dirs.push(dir.to_path_buf());
    let mut scanned_entries = 0usize;
    while let Some(current_dir) = pending_dirs.pop() {
        for entry in fs::read_dir(current_dir).map_err(MoltenError::from)? {
            scanned_entries = scanned_entries
                .checked_add(1)
                .ok_or_else(|| MoltenError::invalid_harness("upgrade store scan count overflow"))?;
            ensure_count_at_most(scanned_entries, MAX_UPGRADE_POINTERS, "upgrade store scan entries")?;
            let entry = entry.map_err(MoltenError::from)?;
            if entry.file_type().map_err(MoltenError::from)?.is_dir() {
                push_bounded(&mut pending_dirs, entry.path(), MAX_UPGRADE_POINTERS, "upgrade store scan dirs")?;
            } else if fs::read_to_string(entry.path()).map_err(MoltenError::from)?.contains(target_ref) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn ensure_dirs(root: &Path) -> Result<()> {
    fs::create_dir_all(root.join("plans")).map_err(MoltenError::from)?;
    fs::create_dir_all(root.join("receipts")).map_err(MoltenError::from)?;
    fs::create_dir_all(root.join("names")).map_err(MoltenError::from)?;
    fs::create_dir_all(root.join("status")).map_err(MoltenError::from)
}

fn write_preserves(path: &Path, value: &IoValue) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, to_text(value)?).map_err(MoltenError::from)
}

fn read_preserves(path: &Path) -> Result<IoValue> {
    parse_text(&fs::read_to_string(path).map_err(MoltenError::from)?)
}

fn store_receipt(root: &Path, receipt_value: &IoValue) -> Result<()> {
    let receipt_ref = canonical_hash(receipt_value)?;
    write_preserves(&receipt_path(root, &receipt_ref)?, receipt_value)
}

fn plan_path(root: &Path, plan_ref: &str) -> Result<PathBuf> {
    Ok(root.join("plans").join(filename_for_ref(plan_ref)?))
}

fn receipt_path(root: &Path, receipt_ref: &str) -> Result<PathBuf> {
    Ok(root.join("receipts").join(filename_for_ref(receipt_ref)?))
}

fn name_pointer_path(root: &Path, name: &str) -> Result<PathBuf> {
    let key = canonical_hash(&record("upgrade-name-pointer-key", vec![string(name)]))?;
    Ok(root.join("names").join(filename_for_ref(&key)?))
}

fn status_path(root: &Path, session_id: &str, task_id: &str) -> Result<PathBuf> {
    let session = canonical_hash(&record("upgrade-session-status-key", vec![string(session_id)]))?;
    let task = canonical_hash(&record("upgrade-task-status-key", vec![string(task_id)]))?;
    Ok(root.join("status").join(filename_for_ref(&session)?).join(filename_for_ref(&task)?))
}

fn filename_for_ref(value_ref: &str) -> Result<String> {
    let hex = content_ref_hex(value_ref)
        .map_err(|error| MoltenError::invalid_harness(format!("unsupported ref {value_ref}: {error}")))?;
    Ok(format!("blake3_{hex}.preserves"))
}

fn local_ref(kind: &str, a: &str, b: &str) -> Result<String> {
    canonical_hash(&record("upgrade-local-ref", vec![string(kind), string(a), string(b)]))
}

fn refs_sequence(refs: &[String]) -> IoValue {
    sequence(refs.iter().map(string).collect())
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn optional_string_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn optional_u64_value(value: Option<u64>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![u64_value(value)]))
}

fn parse_optional_ref_value(value: &Value<IoValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(fields) = value.collect_simple_record("some", Some(1)) {
        return required_ref(&fields[0], "optional ref").map(Some);
    }
    required_ref(value, "optional ref").map(Some)
}

fn parse_optional_string_value(value: &Value<IoValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(fields) = value.collect_simple_record("some", Some(1)) {
        return required_string(&fields[0], "optional string").map(Some);
    }
    required_string(value, "optional string").map(Some)
}
