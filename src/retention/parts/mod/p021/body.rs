
pub fn parse_candidate_bundle(value: &IoValue) -> Result<CandidateBundle> {
    let fields = value
        .collect_simple_record("retention-candidate-bundle-v1", Some(13))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-candidate-bundle-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::RETENTION_CANDIDATE_BUNDLE_SCHEMA,
        "retention candidate bundle schema",
    )?;
    let explain_ref = record_ref(&fields[1], "explain")?;
    let object_fields = fields[2]
        .collect_simple_record("object", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected retention candidate bundle object"))?;
    let object_ref = required_string(&object_fields[0], "retention bundle object ref")?;
    require_ref(&object_ref, "retention bundle object ref")?;
    let object_kind = optional_record_string(&object_fields[1], "retention bundle object kind")?;
    if let Some(object_kind) = object_kind.as_deref() {
        validate_name(object_kind, "retention bundle object kind")?;
    }
    let filter_fields = fields[3]
        .collect_simple_record("filters", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("expected retention bundle filters"))?;
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
        validate_name(subsystem, "retention bundle subsystem")?;
    }
    let gc_plan_refs = record_ref_sequence(&fields[4], "gc-plans")?;
    let gc_apply_refs = record_ref_sequence(&fields[5], "gc-applies")?;
    let gc_execution_refs = record_ref_sequence(&fields[6], "gc-executes")?;
    let gc_audit_refs = record_ref_sequence(&fields[7], "gc-audits")?;
    let retention_receipt_refs = record_ref_sequence(&fields[8], "retention-receipts")?;
    let tombstone_refs = record_ref_sequence(&fields[9], "tombstones")?;
    let artifact_refs = record_ref_sequence(&fields[10], "artifacts")?;
    let diagnostics = record_string_sequence(&fields[11], "diagnostics")?;
    let checks = parse_checks(&fields[12])?;
    require_check(&checks, "bundle-is-not-authority", "retention candidate bundle")?;
    require_check(&checks, "read-only-export", "retention candidate bundle")?;
    require_check(&checks, "normal-admission-still-required", "retention candidate bundle")?;
    require_check(&checks, "plan-apply-execute-still-required", "retention candidate bundle")?;
    require_check(&checks, "remote-clearance-import-still-required", "retention candidate bundle")?;
    Ok(CandidateBundle {
        bundle_ref: crate::preserves_rail::canonical_hash(value)?,
        explain_ref,
        object_ref,
        object_kind,
        retention_class,
        action,
        subsystem,
        gc_plan_refs,
        gc_apply_refs,
        gc_execution_refs,
        gc_audit_refs,
        retention_receipt_refs,
        tombstone_refs,
        artifact_refs,
        diagnostics,
        value: value.clone(),
    })
}

pub fn verify_candidate_bundle(input: CandidateBundleVerifyInput<'_>) -> Result<CandidateBundleVerify> {
    let bundle_value = read_store_value(&input.bundle_dir.join("bundle.preserves"))?;
    let bundle = parse_candidate_bundle(&bundle_value)?;
    let explain_value = read_store_value(&input.bundle_dir.join("explain.preserves"))?;
    let explain = parse_candidate_explain(&explain_value)?;
    let mut diagnostics = Vec::new();
    push_bundle_scope_diagnostics(&bundle, &explain, &mut diagnostics)?;
    let expected_refs = candidate_bundle_expected_refs(&bundle)?;
    let expected_ref_set = push_expected_ref_notes(&bundle, &expected_refs, &mut diagnostics)?;
    let mut file_refs = Vec::new();
    scan_bundle_artifact_files(
        &input.bundle_dir.join("artifacts"),
        &expected_ref_set,
        &mut file_refs,
        &mut diagnostics,
    )?;
    verify_artifact_groups(input.bundle_dir, &bundle, &mut diagnostics)?;
    file_refs.sort();
    diagnostics.sort();
    diagnostics.dedup();
    push_file_ref_notes(&bundle, &file_refs, &mut diagnostics)?;
    diagnostics.sort();
    diagnostics.dedup();
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = candidate_bundle_verify_value(&CandidateBundleVerifyValueInput {
        bundle: &bundle,
        decision,
        file_refs: &file_refs,
        diagnostics: &diagnostics,
    })?;
    parse_candidate_bundle_verify(&value)
}

fn push_expected_ref_notes(
    bundle: &CandidateBundle,
    expected_refs: &[String],
    diagnostics: &mut impl VecSink<String>,
) -> Result<OrderedSet<String>> {
    push_duplicate_ref_diagnostics(&bundle.artifact_refs, "retention-bundle-duplicate-manifest-ref", diagnostics)?;
    push_duplicate_ref_diagnostics(expected_refs, "retention-bundle-duplicate-expected-ref", diagnostics)?;
    let manifest_refs = ref_set(&bundle.artifact_refs);
    let expected_ref_set = ref_set(expected_refs);
    for reference in expected_refs {
        if !manifest_refs.contains(reference) {
            push_bounded(
                diagnostics,
                format!("retention-bundle-manifest-missing-ref:{reference}"),
                MAX_RETENTION_DIAGNOSTICS,
                "retention bundle verify diagnostics",
            )?;
        }
    }
    for reference in &bundle.artifact_refs {
        if !expected_ref_set.contains(reference) {
            push_bounded(
                diagnostics,
                format!("retention-bundle-manifest-unreferenced-ref:{reference}"),
                MAX_RETENTION_DIAGNOSTICS,
                "retention bundle verify diagnostics",
            )?;
        }
    }
    Ok(expected_ref_set)
}

fn verify_artifact_groups(
    bundle_dir: &Path,
    bundle: &CandidateBundle,
    diagnostics: &mut impl VecSink<String>,
) -> Result<()> {
    let groups = [
        Group {
            dir_name: "gc-plans",
            refs: &bundle.gc_plan_refs,
            parse: parse_gc_plan_kind,
        },
        Group {
            dir_name: "gc-applies",
            refs: &bundle.gc_apply_refs,
            parse: parse_gc_apply_kind,
        },
        Group {
            dir_name: "gc-executes",
            refs: &bundle.gc_execution_refs,
            parse: parse_gc_execution_kind,
        },
        Group {
            dir_name: "gc-audits",
            refs: &bundle.gc_audit_refs,
            parse: parse_gc_audit_kind,
        },
        Group {
            dir_name: "receipts",
            refs: &bundle.retention_receipt_refs,
            parse: parse_receipt_kind,
        },
        Group {
            dir_name: "tombstones",
            refs: &bundle.tombstone_refs,
            parse: parse_tombstone_kind,
        },
    ];
    for group in groups {
        verify_bundle_artifact_group(
            BundleVerifyGroupInput {
                bundle_dir,
                dir_name: group.dir_name,
                refs: group.refs,
                parse: group.parse,
            },
            diagnostics,
        )?;
    }
    Ok(())
}

fn push_file_ref_notes(
    bundle: &CandidateBundle,
    file_refs: &[String],
    diagnostics: &mut impl VecSink<String>,
) -> Result<()> {
    let file_ref_set = ref_set(file_refs);
    let manifest_refs = ref_set(&bundle.artifact_refs);
    for reference in &bundle.artifact_refs {
        if !file_ref_set.contains(reference) {
            push_bounded(
                diagnostics,
                format!("retention-bundle-listed-ref-missing-file:{reference}"),
                MAX_RETENTION_DIAGNOSTICS,
                "retention bundle verify diagnostics",
            )?;
        }
    }
    for reference in file_refs {
        if !manifest_refs.contains(reference) {
            push_bounded(
                diagnostics,
                format!("retention-bundle-unlisted-file-ref:{reference}"),
                MAX_RETENTION_DIAGNOSTICS,
                "retention bundle verify diagnostics",
            )?;
        }
    }
    Ok(())
}

struct MismatchNote {
    is_same: bool,
    note: &'static str,
}

impl MismatchNote {
    fn new(is_same: bool, note: &'static str) -> Self {
        Self { is_same, note }
    }
}

fn push_mismatch_notes(checks: &[MismatchNote], diagnostics: &mut impl VecSink<String>) -> Result<()> {
    for check in checks {
        if check.is_same {
            continue;
        }
        push_bounded(
            diagnostics,
            check.note.to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention bundle verify diagnostics",
        )?;
    }
    Ok(())
}

fn push_bundle_scope_diagnostics(
    bundle: &CandidateBundle,
    explain: &CandidateExplain,
    diagnostics: &mut impl VecSink<String>,
) -> Result<()> {
    let checks = [
        MismatchNote::new(bundle.explain_ref == explain.explain_ref, "retention-bundle-explain-ref-mismatch"),
        MismatchNote::new(bundle.object_ref == explain.object_ref, "retention-bundle-object-mismatch"),
        MismatchNote::new(bundle.object_kind == explain.object_kind, "retention-bundle-kind-mismatch"),
        MismatchNote::new(bundle.retention_class == explain.retention_class, "retention-bundle-class-mismatch"),
        MismatchNote::new(bundle.action == explain.action, "retention-bundle-action-mismatch"),
        MismatchNote::new(bundle.subsystem == explain.subsystem, "retention-bundle-subsystem-mismatch"),
        MismatchNote::new(bundle.gc_plan_refs == explain.gc_plan_refs, "retention-bundle-plan-refs-mismatch"),
        MismatchNote::new(bundle.gc_apply_refs == explain.gc_apply_refs, "retention-bundle-apply-refs-mismatch"),
        MismatchNote::new(
            bundle.gc_execution_refs == explain.gc_execution_refs,
            "retention-bundle-execute-refs-mismatch",
        ),
        MismatchNote::new(bundle.gc_audit_refs == explain.gc_audit_refs, "retention-bundle-audit-refs-mismatch"),
        MismatchNote::new(
            bundle.retention_receipt_refs == explain.retention_receipt_refs,
            "retention-bundle-receipt-refs-mismatch",
        ),
        MismatchNote::new(bundle.tombstone_refs == explain.tombstone_refs, "retention-bundle-tombstone-refs-mismatch"),
    ];
    push_mismatch_notes(&checks, diagnostics)
}

fn candidate_bundle_expected_refs(bundle: &CandidateBundle) -> Result<Vec<String>> {
    let mut refs = Vec::new();
    push_ref_slice(&mut refs, &bundle.gc_plan_refs)?;
    push_ref_slice(&mut refs, &bundle.gc_apply_refs)?;
    push_ref_slice(&mut refs, &bundle.gc_execution_refs)?;
    push_ref_slice(&mut refs, &bundle.gc_audit_refs)?;
    push_ref_slice(&mut refs, &bundle.retention_receipt_refs)?;
    push_ref_slice(&mut refs, &bundle.tombstone_refs)?;
    Ok(refs)
}

fn push_ref_slice(values: &mut impl VecSink<String>, refs: &[String]) -> Result<()> {
    for reference in refs {
        push_bounded(values, reference.clone(), MAX_RETENTION_REFS, "retention bundle expected refs")?;
    }
    Ok(())
}
