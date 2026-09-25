
fn collect_upgrade_state_snapshot_entries(
    root: &Path,
    dir_name: &str,
    entries: &mut impl crate::bounded::VecSink<(String, String)>,
) -> Result<()> {
    let snapshot_root = root.join(dir_name);
    if !snapshot_root.exists() {
        return Ok(());
    }
    let mut pending_dirs = vec![snapshot_root];
    while let Some(current_dir) = pending_dirs.pop() {
        for entry in fs::read_dir(&current_dir).map_err(MoltenError::from)? {
            let entry = entry.map_err(MoltenError::from)?;
            let path = entry.path();
            if entry.file_type().map_err(MoltenError::from)?.is_dir() {
                push_bounded(
                    &mut pending_dirs,
                    path,
                    MAX_UPGRADE_POINTERS,
                    "upgrade state snapshot dirs",
                )?;
                continue;
            }
            let relative_path = path
                .strip_prefix(root)
                .map_err(|error| MoltenError::invalid_harness(format!("upgrade snapshot path escaped root: {error}")))?;
            let text = fs::read_to_string(&path).map_err(MoltenError::from)?;
            let content_ref = canonical_hash(&record("upgrade-state-file-v1", vec![string(&text)]))?;
            push_bounded(
                entries,
                (relative_path.to_string_lossy().into_owned(), content_ref),
                MAX_UPGRADE_POINTERS,
                "upgrade state snapshot entries",
            )?;
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
    let Ok(receipt) = read_stored_receipt(root, &receipt_ref) else {
        return Ok(None);
    };
    if receipt.decision == "pass" && receipt.plan_ref == plan.plan_ref && receipt.task_id.as_deref() == Some(task_id) {
        Ok(Some(receipt_ref))
    } else {
        Ok(None)
    }
}

fn read_stored_receipt(root: &Path, receipt_ref: &str) -> Result<UpgradeReceipt> {
    validate_ref(receipt_ref, "upgrade stored receipt ref")?;
    let receipt = parse_upgrade_receipt(&read_preserves(&receipt_path(root, receipt_ref)?)?)?;
    if receipt.receipt_ref == receipt_ref {
        Ok(receipt)
    } else {
        Err(MoltenError::invalid_harness(format!(
            "upgrade stored receipt hash mismatch: expected {receipt_ref}, got {}",
            receipt.receipt_ref
        )))
    }
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
