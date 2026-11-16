
// r[impl molten.upgrades.capability_rooted_store]
// Every upgrades-store read, scan and write goes through the held store capability: relative locators,
// no-follow leaf access, and staged publication. The operator-selected root is the only ambient path.

/// Visits every regular member under `dir_name`, depth first. Links and other entry kinds are refused.
fn visit_store_members(
    root: &UpgradeStore,
    dir_name: &str,
    visit: &mut impl FnMut(&StoreLocator) -> Result<bool>,
) -> Result<()> {
    let store = root.root();
    let start = StoreLocator::parse(dir_name)?;
    if store.entry_kind_optional(&start)?.is_none() {
        return Ok(());
    }
    let mut pending_dirs = vec![start];
    let mut scanned_entries = 0usize;
    while let Some(current_dir) = pending_dirs.pop() {
        for member in store.list_entries(&current_dir)? {
            scanned_entries = scanned_entries
                .checked_add(1)
                .ok_or_else(|| MoltenError::invalid_harness("upgrade store scan count overflow"))?;
            ensure_count_at_most(scanned_entries, MAX_UPGRADE_POINTERS, "upgrade store scan entries")?;
            match member.kind {
                StoreMemberKind::Directory => {
                    push_bounded(&mut pending_dirs, member.path, MAX_UPGRADE_POINTERS, "upgrade store scan dirs")?;
                }
                StoreMemberKind::File => {
                    if !visit(&member.path)? {
                        return Ok(());
                    }
                }
                StoreMemberKind::Symlink | StoreMemberKind::Other => {
                    return Err(MoltenError::invalid_harness(format!(
                        "upgrade store member {} is not a regular file or directory",
                        member.path.display()
                    )));
                }
            }
        }
    }
    Ok(())
}

fn collect_upgrade_state_snapshot_entries(
    root: &UpgradeStore,
    dir_name: &str,
    entries: &mut impl crate::bounded::VecSink<(String, String)>,
) -> Result<()> {
    visit_store_members(root, dir_name, &mut |path| {
        let text = root.root().read_to_string(path)?;
        let content_ref = canonical_hash(&record("upgrade-state-file-v1", vec![string(&text)]))?;
        push_bounded(entries, (path.display(), content_ref), MAX_UPGRADE_POINTERS, "upgrade state snapshot entries")?;
        Ok(true)
    })
}

fn write_status(root: &UpgradeStore, plan: &UpgradePlan, task: &UpgradeTask, receipt_ref: &str) -> Result<()> {
    validate_ref(receipt_ref, "upgrade task status receipt ref")?;
    root.root().publish(&status_path(&plan.session_id, &task.task_id)?, receipt_ref.as_bytes())
}

fn read_status_receipt_ref(root: &UpgradeStore, plan: &UpgradePlan, task_id: &str) -> Result<Option<String>> {
    let path = status_path(&plan.session_id, task_id)?;
    if root.root().entry_kind_optional(&path)?.is_none() {
        return Ok(None);
    }
    let receipt_ref = root.root().read_to_string(&path)?;
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

fn read_stored_receipt(root: &UpgradeStore, receipt_ref: &str) -> Result<UpgradeReceipt> {
    validate_ref(receipt_ref, "upgrade stored receipt ref")?;
    let receipt = parse_upgrade_receipt(&read_store_preserves(root, &receipt_path(receipt_ref)?)?)?;
    if receipt.receipt_ref == receipt_ref {
        Ok(receipt)
    } else {
        Err(MoltenError::invalid_harness(format!(
            "upgrade stored receipt hash mismatch: expected {receipt_ref}, got {}",
            receipt.receipt_ref
        )))
    }
}

fn read_name_pointers(root: &UpgradeStore) -> Result<Vec<NamePointer>> {
    let mut pointers = Vec::new();
    visit_store_members(root, "names", &mut |path| {
        push_bounded(
            &mut pointers,
            parse_name_pointer(&read_store_preserves(root, path)?)?,
            MAX_UPGRADE_POINTERS,
            "upgrade name pointers",
        )?;
        Ok(true)
    })?;
    Ok(pointers)
}

fn store_text_contains_ref(root: &UpgradeStore, dir_name: &str, target_ref: &str) -> Result<bool> {
    let mut is_found = false;
    visit_store_members(root, dir_name, &mut |path| {
        is_found = root.root().read_to_string(path)?.contains(target_ref);
        Ok(!is_found)
    })?;
    Ok(is_found)
}

fn ensure_dirs(root: &UpgradeStore) -> Result<()> {
    for dir_name in ["plans", "receipts", "names", "status"] {
        root.root().create_dir_all(&StoreLocator::parse(dir_name)?)?;
    }
    Ok(())
}

/// Plans and receipts are content-addressed: publication never replaces a member, and an existing member
/// must already hold the same bytes.
fn publish_immutable(root: &UpgradeStore, path: &StoreLocator, value: &IoValue) -> Result<()> {
    let text = to_text(value)?;
    let store = root.root();
    if store.entry_kind_optional(path)?.is_none() {
        match store.publish_new(path, text.as_bytes()) {
            Ok(()) => return Ok(()),
            // A concurrent publisher may have created it; the comparison below decides.
            Err(error) if store.entry_kind_optional(path)?.is_none() => return Err(error),
            Err(_) => {}
        }
    }
    if store.read(path)? == text.as_bytes() {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "upgrade store member {} already holds different content",
            path.display()
        )))
    }
}

/// Name pointers are replaced in one step: across a crash a pointer holds its previous or its new value.
fn publish_mutable(root: &UpgradeStore, path: &StoreLocator, value: &IoValue) -> Result<()> {
    root.root().publish(path, to_text(value)?.as_bytes())
}

fn read_store_preserves(root: &UpgradeStore, path: &StoreLocator) -> Result<IoValue> {
    parse_text(&root.root().read_to_string(path)?)
}

fn store_receipt(root: &UpgradeStore, receipt_value: &IoValue) -> Result<()> {
    let receipt_ref = canonical_hash(receipt_value)?;
    publish_immutable(root, &receipt_path(&receipt_ref)?, receipt_value)
}

fn plan_path(plan_ref: &str) -> Result<StoreLocator> {
    StoreLocator::parse(&format!("plans/{}", filename_for_ref(plan_ref)?))
}

fn receipt_path(receipt_ref: &str) -> Result<StoreLocator> {
    StoreLocator::parse(&format!("receipts/{}", filename_for_ref(receipt_ref)?))
}

fn name_pointer_path(name: &str) -> Result<StoreLocator> {
    let key = canonical_hash(&record("upgrade-name-pointer-key", vec![string(name)]))?;
    StoreLocator::parse(&format!("names/{}", filename_for_ref(&key)?))
}

fn status_path(session_id: &str, task_id: &str) -> Result<StoreLocator> {
    let session = canonical_hash(&record("upgrade-session-status-key", vec![string(session_id)]))?;
    let task = canonical_hash(&record("upgrade-task-status-key", vec![string(task_id)]))?;
    StoreLocator::parse(&format!("status/{}/{}", filename_for_ref(&session)?, filename_for_ref(&task)?))
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
