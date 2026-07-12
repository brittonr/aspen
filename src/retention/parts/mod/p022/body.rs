
fn push_duplicate_ref_diagnostics(refs: &[String], prefix: &str, diagnostics: &mut impl VecSink<String>) -> Result<()> {
    let mut seen = OrderedSet::new();
    let mut duplicates = OrderedSet::new();
    for reference in refs {
        if !seen.insert(reference.clone()) {
            duplicates.insert(reference.clone());
        }
    }
    for reference in duplicates {
        push_bounded(
            diagnostics,
            format!("{prefix}:{reference}"),
            MAX_RETENTION_DIAGNOSTICS,
            "retention bundle verify diagnostics",
        )?;
    }
    Ok(())
}

fn ref_set(refs: &[String]) -> OrderedSet<String> {
    refs.iter().cloned().collect()
}

fn scan_bundle_artifact_files(
    bundle_root: &CapabilityBundleRoot,
    expected_refs: &OrderedSet<String>,
    file_refs: &mut impl VecSink<String>,
    diagnostics: &mut impl VecSink<String>,
) -> Result<()> {
    let artifact_dir = bundle_path("artifacts")?;
    if !bundle_root.root().try_exists(&artifact_dir)? {
        push_bounded(
            diagnostics,
            "retention-bundle-artifacts-dir-missing".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention bundle verify diagnostics",
        )?;
        return Ok(());
    }
    let mut seen_files = OrderedSet::new();
    for entry in bundle_root.root().list_entries(&artifact_dir)? {
        if entry.kind != crate::local_store::LocalStoreEntryKind::Directory {
            push_bounded(
                diagnostics,
                "retention-bundle-unexpected-artifact-root-entry".to_string(),
                MAX_RETENTION_DIAGNOSTICS,
                "retention bundle verify diagnostics",
            )?;
            continue;
        }
        if !bundle_artifact_dirs().contains(&entry.name.as_str()) {
            push_bounded(
                diagnostics,
                "retention-bundle-unexpected-artifact-dir".to_string(),
                MAX_RETENTION_DIAGNOSTICS,
                "retention bundle verify diagnostics",
            )?;
            continue;
        }
        scan_bundle_artifact_group_files(
            BundleArtifactGroupScanInput {
                bundle_root,
                dir_name: &entry.name,
                expected_refs,
            },
            file_refs,
            diagnostics,
            &mut seen_files,
        )?;
    }
    Ok(())
}

fn scan_bundle_artifact_group_files(
    input: BundleArtifactGroupScanInput<'_>,
    file_refs: &mut impl VecSink<String>,
    diagnostics: &mut impl VecSink<String>,
    seen_files: &mut OrderedSet<String>,
) -> Result<()> {
    let group_dir = bundle_path(&format!("artifacts/{}", input.dir_name))?;
    for entry in input.bundle_root.root().list_entries(&group_dir)? {
        if entry.kind != crate::local_store::LocalStoreEntryKind::File || !entry.name.ends_with(".preserves") {
            push_bounded(
                diagnostics,
                format!("retention-bundle-unexpected-artifact-entry:{}", input.dir_name),
                MAX_RETENTION_DIAGNOSTICS,
                "retention bundle verify diagnostics",
            )?;
            continue;
        }
        match read_bundle_value(input.bundle_root, &entry.path) {
            Ok(value) => {
                let actual_ref = crate::preserves_rail::canonical_hash(&value)?;
                if !seen_files.insert(actual_ref.clone()) {
                    push_bounded(
                        diagnostics,
                        format!("retention-bundle-duplicate-file-ref:{actual_ref}"),
                        MAX_RETENTION_DIAGNOSTICS,
                        "retention bundle verify diagnostics",
                    )?;
                }
                if !input.expected_refs.contains(&actual_ref) {
                    push_bounded(
                        diagnostics,
                        format!("retention-bundle-unreferenced-file:{}:{actual_ref}", input.dir_name),
                        MAX_RETENTION_DIAGNOSTICS,
                        "retention bundle verify diagnostics",
                    )?;
                }
                push_bounded(file_refs, actual_ref, MAX_RETENTION_REFS, "retention bundle file refs")?;
            }
            Err(_) => push_bounded(
                diagnostics,
                format!("retention-bundle-unreadable-file:{}", input.dir_name),
                MAX_RETENTION_DIAGNOSTICS,
                "retention bundle verify diagnostics",
            )?,
        }
    }
    Ok(())
}

fn verify_bundle_artifact_group(
    input: BundleVerifyGroupInput<'_>,
    diagnostics: &mut impl VecSink<String>,
) -> Result<()> {
    for reference in input.refs {
        let path = bundle_artifact_path(input.dir_name, reference)?;
        if !input.bundle_root.root().try_exists(&path)? {
            push_bounded(
                diagnostics,
                format!("retention-bundle-missing-file:{}:{reference}", input.dir_name),
                MAX_RETENTION_DIAGNOSTICS,
                "retention bundle verify diagnostics",
            )?;
            continue;
        }
        let value = match read_bundle_value(input.bundle_root, &path) {
            Ok(value) => value,
            Err(_) => {
                push_bounded(
                    diagnostics,
                    format!("retention-bundle-unreadable-file:{}", input.dir_name),
                    MAX_RETENTION_DIAGNOSTICS,
                    "retention bundle verify diagnostics",
                )?;
                continue;
            }
        };
        let actual_ref = crate::preserves_rail::canonical_hash(&value)?;
        if &actual_ref != reference {
            push_bounded(
                diagnostics,
                format!("retention-bundle-tampered-file:{}:{reference}:{actual_ref}", input.dir_name),
                MAX_RETENTION_DIAGNOSTICS,
                "retention bundle verify diagnostics",
            )?;
            continue;
        }
        if (input.parse)(&value).is_err() {
            push_bounded(
                diagnostics,
                format!("retention-bundle-kind-mismatch:{}:{reference}", input.dir_name),
                MAX_RETENTION_DIAGNOSTICS,
                "retention bundle verify diagnostics",
            )?;
        }
    }
    Ok(())
}

fn bundle_artifact_dirs() -> &'static [&'static str] {
    &[
        "gc-plans",
        "gc-applies",
        "gc-executes",
        "gc-audits",
        "receipts",
        "tombstones",
    ]
}

fn candidate_bundle_verify_value(input: &CandidateBundleVerifyValueInput<'_>) -> Result<IoValue> {
    validate_candidate_bundle_verify_value_input(input)?;
    Ok(crate::preserves_rail::record("retention-candidate-bundle-verify-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RETENTION_CANDIDATE_BUNDLE_VERIFY_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("bundle", vec![crate::preserves_rail::string(&input.bundle.bundle_ref)]),
        crate::preserves_rail::record("explain", vec![crate::preserves_rail::string(&input.bundle.explain_ref)]),
        crate::preserves_rail::record("object", vec![
            crate::preserves_rail::string(&input.bundle.object_ref),
            optional_string_value(input.bundle.object_kind.as_deref()),
        ]),
        crate::preserves_rail::record("filters", vec![
            crate::preserves_rail::record("class", vec![optional_string_value(
                input.bundle.retention_class.as_deref(),
            )]),
            crate::preserves_rail::record("action", vec![optional_string_value(input.bundle.action.as_deref())]),
            crate::preserves_rail::record("subsystem", vec![optional_string_value(input.bundle.subsystem.as_deref())]),
        ]),
        crate::preserves_rail::record("artifacts", vec![strings_sequence(&input.bundle.artifact_refs)]),
        crate::preserves_rail::record("files", vec![strings_sequence(input.file_refs)]),
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            ("verify-is-not-authority", "pass"),
            ("read-only-verify", "pass"),
            ("normal-admission-still-required", "pass"),
            ("plan-apply-execute-still-required", "pass"),
            ("remote-clearance-import-still-required", "pass"),
        ]),
    ]))
}

pub fn parse_candidate_bundle_verify(value: &IoValue) -> Result<CandidateBundleVerify> {
    let fields = value
        .collect_simple_record("retention-candidate-bundle-verify-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-candidate-bundle-verify-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::RETENTION_CANDIDATE_BUNDLE_VERIFY_SCHEMA,
        "retention candidate bundle verify schema",
    )?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    let bundle_ref = record_ref(&fields[2], "bundle")?;
    let explain_ref = record_ref(&fields[3], "explain")?;
    let object_fields = fields[4]
        .collect_simple_record("object", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected retention bundle verify object"))?;
    let object_ref = required_string(&object_fields[0], "retention bundle verify object ref")?;
    require_ref(&object_ref, "retention bundle verify object ref")?;
    let object_kind = optional_record_string(&object_fields[1], "retention bundle verify object kind")?;
    if let Some(object_kind) = object_kind.as_deref() {
        validate_name(object_kind, "retention bundle verify object kind")?;
    }
    let filter_fields = fields[5]
        .collect_simple_record("filters", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("expected retention bundle verify filters"))?;
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
        validate_name(subsystem, "retention bundle verify subsystem")?;
    }
    let artifact_refs = record_ref_sequence(&fields[6], "artifacts")?;
    let file_refs = record_ref_sequence(&fields[7], "files")?;
    let diagnostics = record_string_sequence(&fields[8], "diagnostics")?;
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "verify-is-not-authority", "retention candidate bundle verify")?;
    require_check(&checks, "read-only-verify", "retention candidate bundle verify")?;
    require_check(&checks, "normal-admission-still-required", "retention candidate bundle verify")?;
    require_check(&checks, "plan-apply-execute-still-required", "retention candidate bundle verify")?;
    require_check(&checks, "remote-clearance-import-still-required", "retention candidate bundle verify")?;
    Ok(CandidateBundleVerify {
        verify_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        bundle_ref,
        explain_ref,
        object_ref,
        object_kind,
        retention_class,
        action,
        subsystem,
        artifact_refs,
        file_refs,
        diagnostics,
        value: value.clone(),
    })
}

fn validate_candidate_bundle_verify_value_input(input: &CandidateBundleVerifyValueInput<'_>) -> Result<()> {
    validate_decision(input.decision)?;
    require_ref(&input.bundle.bundle_ref, "retention bundle verify bundle ref")?;
    require_ref(&input.bundle.explain_ref, "retention bundle verify explain ref")?;
    validate_refs(&input.bundle.artifact_refs, "retention bundle verify artifact ref")?;
    validate_refs(input.file_refs, "retention bundle verify file ref")?;
    validate_diagnostics(input.diagnostics, "retention bundle verify diagnostics")
}

fn parse_gc_plan_kind(value: &IoValue) -> Result<()> {
    parse_gc_plan(value).map(|_| ())
}
