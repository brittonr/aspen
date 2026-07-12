
fn executions_for(root: &CapabilityRetentionRoot, filter: &CandidateFilter<'_>) -> Result<Vec<String>> {
    collect_matching_refs(
        root,
        GC_EXECUTE_DIR,
        parse_gc_execution_gate,
        |execute| {
            filter.matches_gc(
                &execute.subsystem,
                &execute.object_ref,
                &execute.object_kind,
                &execute.retention_class,
                &execute.action,
            )
        },
        |execute| execute.execution_ref.clone(),
        "retention candidate GC executions",
    )
}

fn audits_for(root: &CapabilityRetentionRoot, filter: &CandidateFilter<'_>) -> Result<Vec<String>> {
    collect_matching_refs(
        root,
        GC_AUDIT_DIR,
        parse_gc_audit,
        |audit| {
            filter.matches_gc(
                &audit.subsystem,
                &audit.object_ref,
                &audit.object_kind,
                &audit.retention_class,
                &audit.action,
            )
        },
        |audit| audit.audit_ref.clone(),
        "retention candidate GC audits",
    )
}

fn receipts_for(root: &CapabilityRetentionRoot, filter: &CandidateFilter<'_>) -> Result<Vec<String>> {
    collect_matching_refs(
        root,
        RECEIPT_DIR,
        parse_receipt,
        |receipt| {
            filter.matches_retention(
                &receipt.object_ref,
                &receipt.object_kind,
                &receipt.retention_class,
                &receipt.action,
            )
        },
        |receipt| receipt.receipt_ref.clone(),
        "retention candidate receipts",
    )
}

fn tombstones_for(root: &CapabilityRetentionRoot, filter: &CandidateFilter<'_>) -> Result<Vec<String>> {
    collect_matching_refs(
        root,
        TOMBSTONE_DIR,
        parse_tombstone,
        |tombstone| {
            filter.matches_retention(
                &tombstone.object_ref,
                &tombstone.object_kind,
                &tombstone.retention_class,
                &tombstone.action,
            )
        },
        |tombstone| tombstone.tombstone_ref.clone(),
        "retention candidate tombstones",
    )
}

fn candidate_explain_diagnostics(input: &CandidateExplainValueInput<'_>) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if input.pin_refs.is_empty()
        && input.admission_refs.is_empty()
        && input.remote_clearance_refs.is_empty()
        && input.remote_clearance_import_refs.is_empty()
        && input.gc_plan_refs.is_empty()
        && input.gc_apply_refs.is_empty()
        && input.gc_execution_refs.is_empty()
        && input.gc_audit_refs.is_empty()
        && input.retention_receipt_refs.is_empty()
        && input.tombstone_refs.is_empty()
    {
        push_bounded(
            &mut diagnostics,
            "retention-candidate-no-known-evidence".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention candidate explain diagnostics",
        )?;
    }
    if !input.pin_refs.is_empty() {
        push_bounded(
            &mut diagnostics,
            "active-pins-present".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention candidate explain diagnostics",
        )?;
    }
    diagnostics.sort();
    diagnostics.dedup();
    Ok(diagnostics)
}

fn candidate_explain_value(input: &CandidateExplainValueInput<'_>) -> Result<IoValue> {
    validate_candidate_explain_value_input(input)?;
    Ok(crate::preserves_rail::record("retention-candidate-explain-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RETENTION_CANDIDATE_EXPLAIN_SCHEMA),
        crate::preserves_rail::record("object", vec![
            crate::preserves_rail::string(input.object_ref),
            optional_string_value(input.object_kind),
        ]),
        crate::preserves_rail::record("filters", vec![
            crate::preserves_rail::record("class", vec![optional_string_value(input.retention_class)]),
            crate::preserves_rail::record("action", vec![optional_string_value(input.action)]),
            crate::preserves_rail::record("subsystem", vec![optional_string_value(input.subsystem)]),
        ]),
        crate::preserves_rail::record("pins", vec![strings_sequence(input.pin_refs)]),
        crate::preserves_rail::record("admissions", vec![strings_sequence(input.admission_refs)]),
        crate::preserves_rail::record("remote-clearances", vec![strings_sequence(input.remote_clearance_refs)]),
        crate::preserves_rail::record("remote-clearance-imports", vec![strings_sequence(
            input.remote_clearance_import_refs,
        )]),
        crate::preserves_rail::record("gc-plans", vec![strings_sequence(input.gc_plan_refs)]),
        crate::preserves_rail::record("gc-applies", vec![strings_sequence(input.gc_apply_refs)]),
        crate::preserves_rail::record("gc-executes", vec![strings_sequence(input.gc_execution_refs)]),
        crate::preserves_rail::record("gc-audits", vec![strings_sequence(input.gc_audit_refs)]),
        crate::preserves_rail::record("retention-receipts", vec![strings_sequence(input.retention_receipt_refs)]),
        crate::preserves_rail::record("tombstones", vec![strings_sequence(input.tombstone_refs)]),
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            ("read-only-explain", "pass"),
            ("catalog-discovery-only", "pass"),
            ("normal-admission-still-required", "pass"),
            ("plan-apply-execute-still-required", "pass"),
            ("remote-clearance-import-still-required", "pass"),
        ]),
    ]))
}

pub fn parse_candidate_explain(value: &IoValue) -> Result<CandidateExplain> {
    let fields = value
        .collect_simple_record("retention-candidate-explain-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-candidate-explain-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::RETENTION_CANDIDATE_EXPLAIN_SCHEMA,
        "retention candidate explain schema",
    )?;
    let object_fields = fields[1]
        .collect_simple_record("object", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected retention candidate object"))?;
    let object_ref = required_string(&object_fields[0], "retention candidate object ref")?;
    require_ref(&object_ref, "retention candidate object ref")?;
    let object_kind = optional_record_string(&object_fields[1], "retention candidate object kind")?;
    if let Some(object_kind) = object_kind.as_deref() {
        validate_name(object_kind, "retention candidate object kind")?;
    }
    let filter_fields = fields[2]
        .collect_simple_record("filters", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("expected retention candidate filters"))?;
    let retention_class = record_optional_string(&filter_fields[0], "class")?;
    if let Some(retention_class) = retention_class.as_deref() {
        validate_class(retention_class)?;
    }
    let action = record_optional_string(&filter_fields[1], "action")?;
    if let Some(action) = action.as_deref() {
        validate_action(action)?;
    }
    let subsystem = record_optional_string(&filter_fields[2], "subsystem")?;
    if let Some(subsystem) = subsystem.as_deref() {
        validate_name(subsystem, "retention candidate subsystem")?;
    }
    let pin_refs = record_ref_sequence(&fields[3], "pins")?;
    let admission_refs = record_ref_sequence(&fields[4], "admissions")?;
    let remote_clearance_refs = record_ref_sequence(&fields[5], "remote-clearances")?;
    let remote_clearance_import_refs = record_ref_sequence(&fields[6], "remote-clearance-imports")?;
    let gc_plan_refs = record_ref_sequence(&fields[7], "gc-plans")?;
    let gc_apply_refs = record_ref_sequence(&fields[8], "gc-applies")?;
    let gc_execution_refs = record_ref_sequence(&fields[9], "gc-executes")?;
    let gc_audit_refs = record_ref_sequence(&fields[10], "gc-audits")?;
    let retention_receipt_refs = record_ref_sequence(&fields[11], "retention-receipts")?;
    let tombstone_refs = record_ref_sequence(&fields[12], "tombstones")?;
    let diagnostics = record_string_sequence(&fields[13], "diagnostics")?;
    let checks = parse_checks(&fields[14])?;
    require_check(&checks, "read-only-explain", "retention candidate explain")?;
    require_check(&checks, "normal-admission-still-required", "retention candidate explain")?;
    require_check(&checks, "plan-apply-execute-still-required", "retention candidate explain")?;
    require_check(&checks, "remote-clearance-import-still-required", "retention candidate explain")?;
    Ok(CandidateExplain {
        explain_ref: crate::preserves_rail::canonical_hash(value)?,
        object_ref,
        object_kind,
        retention_class,
        action,
        subsystem,
        pin_refs,
        admission_refs,
        remote_clearance_refs,
        remote_clearance_import_refs,
        gc_plan_refs,
        gc_apply_refs,
        gc_execution_refs,
        gc_audit_refs,
        retention_receipt_refs,
        tombstone_refs,
        diagnostics,
        value: value.clone(),
    })
}

pub fn export_candidate_bundle(input: CandidateBundleExportInput<'_>) -> Result<CandidateBundle> {
    let retention_root = open_capability_retention_root(input.root)?;
    let bundle_root = CapabilityBundleRoot::open(input.out)?;
    export_candidate_bundle_with_roots(&retention_root, &bundle_root, input.explain_value, input.profile)
}

pub fn export_candidate_bundle_with_roots(
    retention_root: &CapabilityRetentionRoot,
    bundle_root: &CapabilityBundleRoot,
    explain_value: &IoValue,
    profile: CandidateBundleExportProfile,
) -> Result<CandidateBundle> {
    let explain = parse_candidate_explain(explain_value)?;
    bundle_root.root().create_dir_all(&bundle_path("artifacts")?)?;
    write_bundle_value(bundle_root, &bundle_path("explain.preserves")?, &explain.value)?;
    let (artifact_refs, diagnostics) = export_groups(retention_root, bundle_root, &explain)?;
    let value = candidate_bundle_value(&CandidateBundleValueInput {
        explain: &explain,
        artifact_refs: &artifact_refs,
        diagnostics: &diagnostics,
    })?;
    write_bundle_value(bundle_root, &bundle_path("bundle.preserves")?, &value)?;
    let bundle = parse_candidate_bundle(&value)?;
    let profile_value = profile_candidate_bundle(bundle_root, profile, &bundle)?;
    write_bundle_value(bundle_root, &bundle_path(BUNDLE_PROFILE_FILE)?, &profile_value.value)?;
    if profile == CandidateBundleExportProfile::Diagnostic {
        write_candidate_bundle_redacted_view(bundle_root, &bundle)?;
    }
    Ok(bundle)
}

fn export_groups(
    root: &CapabilityRetentionRoot,
    bundle_root: &CapabilityBundleRoot,
    explain: &CandidateExplain,
) -> Result<(Vec<String>, Vec<String>)> {
    let groups = [
        GroupSpec {
            dir_name: "gc-plans",
            refs: &explain.gc_plan_refs,
            read: read_gc_plan_value,
        },
        GroupSpec {
            dir_name: "gc-applies",
            refs: &explain.gc_apply_refs,
            read: read_apply_value,
        },
        GroupSpec {
            dir_name: "gc-executes",
            refs: &explain.gc_execution_refs,
            read: read_gc_execution_value,
        },
        GroupSpec {
            dir_name: "gc-audits",
            refs: &explain.gc_audit_refs,
            read: read_gc_audit_value,
        },
        GroupSpec {
            dir_name: "receipts",
            refs: &explain.retention_receipt_refs,
            read: read_receipt_value,
        },
        GroupSpec {
            dir_name: "tombstones",
            refs: &explain.tombstone_refs,
            read: read_tombstone_value,
        },
    ];
    let mut artifact_refs = Vec::new();
    let mut diagnostics = Vec::new();
    for group in groups {
        export_bundle_artifact_group(
            BundleArtifactGroupInput {
                root,
                bundle_root,
                dir_name: group.dir_name,
                refs: group.refs,
                read: group.read,
            },
            &mut artifact_refs,
            &mut diagnostics,
        )?;
    }
    artifact_refs.sort();
    artifact_refs.dedup();
    diagnostics.sort();
    diagnostics.dedup();
    Ok((artifact_refs, diagnostics))
}
