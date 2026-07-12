
fn parse_gc_apply_kind(value: &IoValue) -> Result<()> {
    parse_gc_apply(value).map(|_| ())
}

fn parse_gc_execution_kind(value: &IoValue) -> Result<()> {
    parse_gc_execution_gate(value).map(|_| ())
}

fn parse_gc_audit_kind(value: &IoValue) -> Result<()> {
    parse_gc_audit(value).map(|_| ())
}

fn parse_receipt_kind(value: &IoValue) -> Result<()> {
    parse_receipt(value).map(|_| ())
}

fn parse_tombstone_kind(value: &IoValue) -> Result<()> {
    parse_tombstone(value).map(|_| ())
}

fn validate_candidate_bundle_value_input(input: &CandidateBundleValueInput<'_>) -> Result<()> {
    require_ref(&input.explain.explain_ref, "retention bundle explain ref")?;
    validate_candidate_explain_value_input(&CandidateExplainValueInput {
        object_ref: &input.explain.object_ref,
        object_kind: input.explain.object_kind.as_deref(),
        retention_class: input.explain.retention_class.as_deref(),
        action: input.explain.action.as_deref(),
        subsystem: input.explain.subsystem.as_deref(),
        pin_refs: &input.explain.pin_refs,
        admission_refs: &input.explain.admission_refs,
        remote_clearance_refs: &input.explain.remote_clearance_refs,
        remote_clearance_import_refs: &input.explain.remote_clearance_import_refs,
        gc_plan_refs: &input.explain.gc_plan_refs,
        gc_apply_refs: &input.explain.gc_apply_refs,
        gc_execution_refs: &input.explain.gc_execution_refs,
        gc_audit_refs: &input.explain.gc_audit_refs,
        retention_receipt_refs: &input.explain.retention_receipt_refs,
        tombstone_refs: &input.explain.tombstone_refs,
        diagnostics: &input.explain.diagnostics,
    })?;
    validate_refs(input.artifact_refs, "retention bundle artifact ref")?;
    validate_diagnostics(input.diagnostics, "retention bundle diagnostics")
}

fn read_gc_plan_value(root: &CapabilityRetentionRoot, reference: &str) -> Result<IoValue> {
    Ok(read_gc_plan_with_root(root, reference)?.value)
}

fn read_apply_value(root: &CapabilityRetentionRoot, reference: &str) -> Result<IoValue> {
    Ok(read_gc_apply_with_root(root, reference)?.value)
}

fn read_gc_execution_value(root: &CapabilityRetentionRoot, reference: &str) -> Result<IoValue> {
    Ok(read_gc_execution_gate_with_root(root, reference)?.value)
}

fn read_gc_audit_value(root: &CapabilityRetentionRoot, reference: &str) -> Result<IoValue> {
    Ok(read_gc_audit_with_root(root, reference)?.value)
}

fn read_receipt_value(root: &CapabilityRetentionRoot, reference: &str) -> Result<IoValue> {
    Ok(read_receipt_with_root(root, reference)?.value)
}

fn read_tombstone_value(root: &CapabilityRetentionRoot, reference: &str) -> Result<IoValue> {
    Ok(read_tombstone_with_root(root, reference)?.value)
}

fn validate_candidate_explain_input<Root: ?Sized>(input: &CandidateExplainInput<'_, Root>) -> Result<()> {
    require_ref(input.object_ref, "retention candidate object ref")?;
    if let Some(object_kind) = input.object_kind {
        validate_name(object_kind, "retention candidate object kind")?;
    }
    if let Some(retention_class) = input.retention_class {
        validate_class(retention_class)?;
    }
    if let Some(action) = input.action {
        validate_action(action)?;
    }
    if let Some(subsystem) = input.subsystem {
        validate_name(subsystem, "retention candidate subsystem")?;
    }
    Ok(())
}

fn validate_candidate_explain_value_input(input: &CandidateExplainValueInput<'_>) -> Result<()> {
    validate_candidate_explain_input(&CandidateExplainInput {
        root: Path::new("."),
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retention_class: input.retention_class,
        action: input.action,
        subsystem: input.subsystem,
    })?;
    validate_refs(input.pin_refs, "retention candidate pin ref")?;
    validate_refs(input.admission_refs, "retention candidate admission ref")?;
    validate_refs(input.remote_clearance_refs, "retention candidate remote clearance ref")?;
    validate_refs(input.remote_clearance_import_refs, "retention candidate remote clearance import ref")?;
    validate_refs(input.gc_plan_refs, "retention candidate GC plan ref")?;
    validate_refs(input.gc_apply_refs, "retention candidate GC apply ref")?;
    validate_refs(input.gc_execution_refs, "retention candidate GC execution ref")?;
    validate_refs(input.gc_audit_refs, "retention candidate GC audit ref")?;
    validate_refs(input.retention_receipt_refs, "retention candidate receipt ref")?;
    validate_refs(input.tombstone_refs, "retention candidate tombstone ref")?;
    validate_diagnostics(input.diagnostics, "retention candidate explain diagnostics")
}

impl CandidateFilter<'_> {
    fn matches_object(&self, object_ref: &str, object_kind: &str, retention_class: &str) -> bool {
        object_ref == self.object_ref
            && self.object_kind.is_none_or(|expected| expected == object_kind)
            && self.retention_class.is_none_or(|expected| expected == retention_class)
    }

    fn matches_retention(&self, object_ref: &str, object_kind: &str, retention_class: &str, action: &str) -> bool {
        self.matches_object(object_ref, object_kind, retention_class)
            && self.action.is_none_or(|expected| expected == action)
    }

    fn matches_gc(
        &self,
        subsystem: &str,
        object_ref: &str,
        object_kind: &str,
        retention_class: &str,
        action: &str,
    ) -> bool {
        self.matches_retention(object_ref, object_kind, retention_class, action)
            && self.subsystem.is_none_or(|expected| expected == subsystem)
    }
}

fn collect_matching_refs<T, Parse, Matches, Reference>(
    root: &CapabilityRetentionRoot,
    directory: &str,
    parse: Parse,
    matches: Matches,
    reference: Reference,
    label: &str,
) -> Result<Vec<String>>
where
    Parse: Fn(&IoValue) -> Result<T>,
    Matches: Fn(&T) -> bool,
    Reference: Fn(&T) -> String,
{
    let mut refs = Vec::new();
    let directory = capability_store_path(directory)?;
    if !root.root().try_exists(&directory)? {
        return Ok(refs);
    }
    for entry in root.root().list_entries(&directory)? {
        if entry.kind != crate::local_store::LocalStoreEntryKind::File {
            continue;
        }
        let value = read_store_value_with_root(root, &entry.path)?;
        let parsed = parse(&value)?;
        if matches(&parsed) {
            push_bounded(&mut refs, reference(&parsed), MAX_RETENTION_REFS, label)?;
        }
    }
    refs.sort();
    refs.dedup();
    Ok(refs)
}

fn optional_string_value(value: Option<&str>) -> IoValue {
    value.map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |text| crate::preserves_rail::record("some", vec![crate::preserves_rail::string(text)]),
    )
}

fn record_optional_string(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected {label} record")))?;
    optional_record_string(&fields[0], label)
}

fn optional_record_string(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let inner = crate::preserves_rail::value_to_iovalue(value);
    if inner.collect_simple_record("none", Some(0)).is_some() {
        Ok(None)
    } else {
        let some = inner
            .collect_simple_record("some", Some(1))
            .ok_or_else(|| MoltenError::invalid_harness(format!("expected optional string for {label}")))?;
        Ok(Some(required_string(&some[0], label)?))
    }
}

fn record_optional_ref_with_status(value: &Value<IoValue>, label: &str) -> Result<(Option<String>, String)> {
    let fields = value
        .collect_simple_record(label, Some(2))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected retention GC audit {label} record")))?;
    let inner = crate::preserves_rail::value_to_iovalue(&fields[0]);
    let reference = if inner.collect_simple_record("none", Some(0)).is_some() {
        None
    } else {
        let some = inner
            .collect_simple_record("some", Some(1))
            .ok_or_else(|| MoltenError::invalid_harness(format!("expected optional ref for {label}")))?;
        let reference = required_string(&some[0], label)?;
        require_ref(&reference, label)?;
        Some(reference)
    };
    let status = required_string(&fields[1], label)?;
    validate_audit_step_status(&status, label)?;
    Ok((reference, status))
}

pub fn parse_receipt(value: &IoValue) -> Result<Receipt> {
    crate::preserves_rail::validate_boundary_schema(value, &crate::preserves_rail::RETENTION_RECEIPT_BOUNDARY_SCHEMA)?;
    let fields = value
        .collect_simple_record("retention-receipt-v1", Some(14))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-receipt-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::RETENTION_RECEIPT_SCHEMA, "retention receipt schema")?;
    let decision = record_string(&fields[1], "decision")?;
    let action = record_string(&fields[2], "action")?;
    let (object_ref, object_kind) = parse_object_value(&fields[3])?;
    let retention_class = record_string(&fields[4], "class")?;
    let requester_ref = record_ref(&fields[5], "requester")?;
    let index_ref = record_ref(&fields[6], "index")?;
    let pin_refs = record_ref_sequence(&fields[7], "pins")?;
    let retained_refs = record_ref_sequence(&fields[8], "retained")?;
    let remote_refs = record_ref_sequence(&fields[9], "remote")?;
    let tombstone_ref = record_optional_ref(&fields[10], "tombstone")?;
    let diagnostics = record_string_sequence(&fields[11], "diagnostics")?;
    let checks = parse_checks(&fields[13])?;
    require_check(&checks, "reference-index-bound", "retention receipt")?;
    validate_action(&action)?;
    validate_class(&retention_class)?;
    Ok(Receipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        action,
        object_ref,
        object_kind,
        retention_class,
        requester_ref,
        index_ref,
        pin_refs,
        retained_refs,
        remote_refs,
        tombstone_ref,
        diagnostics,
        value: value.clone(),
    })
}

pub fn parse_tombstone(value: &IoValue) -> Result<Tombstone> {
    let fields = value
        .collect_simple_record("retention-tombstone-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-tombstone-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::RETENTION_TOMBSTONE_SCHEMA, "retention tombstone schema")?;
    let (object_ref, object_kind) = parse_object_value(&fields[1])?;
    let retention_class = record_string(&fields[2], "class")?;
    let action = record_string(&fields[3], "action")?;
    let receipt_ref = record_ref(&fields[4], "receipt")?;
    let policy_refs = record_ref_sequence(&fields[5], "policy")?;
    let evidence_refs = record_ref_sequence(&fields[6], "evidence")?;
    require_check(&parse_checks(&fields[8])?, "audit-visible-tombstone", "retention tombstone")?;
    validate_class(&retention_class)?;
    validate_action(&action)?;
    Ok(Tombstone {
        tombstone_ref: crate::preserves_rail::canonical_hash(value)?,
        object_ref,
        object_kind,
        retention_class,
        action,
        receipt_ref,
        policy_refs,
        evidence_refs,
        value: value.clone(),
    })
}

pub fn read_receipt(root: &Path, receipt_ref: &str) -> Result<Receipt> {
    let root = open_capability_retention_root(root)?;
    read_receipt_with_root(&root, receipt_ref)
}

pub fn read_receipt_with_root(root: &CapabilityRetentionRoot, receipt_ref: &str) -> Result<Receipt> {
    require_ref(receipt_ref, "retention receipt ref")?;
    let value = read_store_value_with_root(root, &capability_ref_path(RECEIPT_DIR, receipt_ref)?)?;
    parse_receipt(&value)
}

pub fn read_tombstone(root: &Path, tombstone_ref: &str) -> Result<Tombstone> {
    let root = open_capability_retention_root(root)?;
    read_tombstone_with_root(&root, tombstone_ref)
}

pub fn read_tombstone_with_root(root: &CapabilityRetentionRoot, tombstone_ref: &str) -> Result<Tombstone> {
    require_ref(tombstone_ref, "retention tombstone ref")?;
    let value = read_store_value_with_root(root, &capability_ref_path(TOMBSTONE_DIR, tombstone_ref)?)?;
    let tombstone = parse_tombstone(&value)?;
    if tombstone.tombstone_ref != tombstone_ref {
        return Err(MoltenError::invalid_harness("stored retention tombstone ref mismatch"));
    }
    Ok(tombstone)
}
