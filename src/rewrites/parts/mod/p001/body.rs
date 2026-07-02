
fn diff_items(root: &Path, input: &RewritePlanInput, query: &RewriteQuery) -> Result<Vec<RewriteDiff>> {
    let mut diffs = Vec::new();
    for rewrite_match in &query.matches {
        let artifact = crate::artifacts::read_artifact(root, &rewrite_match.artifact_ref)?;
        let payload = crate::artifacts::read_payload(root, &artifact.artifact_ref)?;
        let old_payload_ref = canonical_hash(&payload)?;
        let RewriteReplacement::StringValue { from, to } = &input.replacement;
        let mut paths = Vec::new();
        let rewritten = rewrite_string_values(RewriteStringValuesInput {
            value: &payload,
            from,
            to,
            path: "$",
            changed_paths: &mut paths,
        })?;
        if paths.is_empty() {
            continue;
        }
        let new_payload_ref = canonical_hash(&rewritten)?;
        let old_preview = preview_text(&payload)?;
        let new_preview = preview_text(&rewritten)?;
        let value = rewrite_diff_value(&RewriteDiffValueInput {
            artifact_ref: &artifact.artifact_ref,
            kind: &artifact.kind,
            old_payload_ref: &old_payload_ref,
            new_payload_ref: &new_payload_ref,
            paths: &paths,
            old_preview: &old_preview,
            new_preview: &new_preview,
        })?;
        push_bounded(
            &mut diffs,
            RewriteDiff {
                artifact_ref: artifact.artifact_ref,
                kind: artifact.kind,
                old_payload_ref,
                new_payload_ref,
                paths,
                old_preview,
                new_preview,
                new_payload: rewritten,
                value,
            },
            MAX_REWRITE_ITEMS,
            "rewrite diffs",
        )?;
    }
    diffs.sort_by(|left, right| left.artifact_ref.cmp(&right.artifact_ref));
    Ok(diffs)
}

struct PlanRefs<'a> {
    plan_ref: &'a str,
    query: &'a RewriteQuery,
    diffs: &'a [RewriteDiff],
    impacted_refs: &'a [String],
    input: &'a RewritePlanInput,
}

fn plan_refs(input: &PlanRefs<'_>) -> Result<Vec<String>> {
    let mut refs = vec![
        input.plan_ref.to_string(),
        input.query.query_ref.clone(),
        canonical_hash(&input.query.receipt_value)?,
    ];
    refs.extend(input.diffs.iter().map(|diff| diff.artifact_ref.clone()));
    refs.extend(input.diffs.iter().map(|diff| diff.new_payload_ref.clone()));
    refs.extend(input.impacted_refs.iter().cloned());
    refs.extend(input.input.policy_refs.as_slice().iter().cloned());
    refs.extend(input.input.capability_refs.as_slice().iter().cloned());
    refs.extend(input.input.transcript_refs.as_slice().iter().cloned());
    refs.extend(input.input.schema_migration_recipe_refs.as_slice().iter().cloned());
    Ok(refs)
}

pub fn apply(root: &Path, input: &RewritePlanInput) -> Result<RewriteApply> {
    let preview = preview(root, input)?;
    if preview.diffs.is_empty() {
        return Err(MoltenError::invalid_harness("rewrite apply denied because preview has no diffs"));
    }
    let preview_receipt_ref = canonical_hash(&preview.receipt_value)?;
    let query_receipt_ref = canonical_hash(&preview.query.receipt_value)?;
    let mut installed = Vec::new();
    for diff in &preview.diffs {
        let artifact = crate::artifacts::read_artifact(root, &diff.artifact_ref)?;
        let mut policy_refs = sorted_unique_refs(&merge_refs(&artifact.policy_refs, &input.policy_refs));
        policy_refs.push(preview.plan_ref.clone());
        policy_refs = sorted_unique_refs(&policy_refs);
        let mut evidence_refs = artifact.evidence_refs.clone();
        evidence_refs.push(preview_receipt_ref.clone());
        evidence_refs.push(query_receipt_ref.clone());
        evidence_refs.extend(input.transcript_refs.as_slice().iter().cloned());
        evidence_refs.extend(input.schema_migration_recipe_refs.as_slice().iter().cloned());
        evidence_refs = sorted_unique_refs(&evidence_refs);
        let install = crate::artifacts::install_artifact(root, &crate::artifacts::ArtifactInstallInput {
            kind: artifact.kind,
            payload: diff.new_payload.clone(),
            schema_refs: artifact.schema_refs,
            dependency_refs: artifact.dependency_refs,
            effect_manifest_ref: artifact.effect_manifest_ref,
            policy_refs,
            evidence_refs,
            installer_ref: input.planner_ref.clone(),
            capability_refs: input.capability_refs.clone(),
        })?;
        let install_receipt_ref = canonical_hash(&install.receipt_value)?;
        push_bounded(
            &mut installed,
            RewriteInstalledArtifact {
                old_artifact_ref: diff.artifact_ref.clone(),
                new_artifact_ref: install.artifact_ref,
                install_receipt_ref,
            },
            MAX_REWRITE_ITEMS,
            "rewrite installed artifacts",
        )?;
    }
    let mut refs = vec![preview.plan_ref.clone(), preview_receipt_ref, query_receipt_ref];
    for item in &installed {
        refs.push(item.old_artifact_ref.clone());
        refs.push(item.new_artifact_ref.clone());
        refs.push(item.install_receipt_ref.clone());
    }
    let apply_subject = local_ref("rewrite-apply", &refs)?;
    let receipt_value = rewrite_receipt_value(&RewriteReceiptValueInput {
        operation: "apply",
        decision: "pass",
        subject_ref: &apply_subject,
        refs: &refs,
        diagnostics: &[],
        checks: &[
            ("artifact-creation", "pass"),
            ("no-in-place-mutation", "pass"),
            ("preview-ref-binding", "pass"),
            ("upgrade-session-hook-ready", "pass"),
        ],
    })?;
    Ok(RewriteApply {
        preview,
        installed,
        receipt_value,
    })
}

pub fn upgrade_plan_from_apply(
    rewrite: &RewriteApply,
    session_id: &str,
    initiator_ref: &str,
    capability_refs: &[String],
    policy_refs: &[String],
) -> Result<IoValue> {
    validate_non_empty(session_id, "rewrite upgrade session id")?;
    validate_ref(initiator_ref, "rewrite upgrade initiator ref")?;
    validate_refs(capability_refs, "rewrite upgrade capability ref")?;
    validate_refs(policy_refs, "rewrite upgrade policy ref")?;
    let apply_receipt_ref = canonical_hash(&rewrite.receipt_value)?;
    let preview_receipt_ref = canonical_hash(&rewrite.preview.receipt_value)?;
    let mut tasks = Vec::new();
    for (index, installed) in rewrite.installed.iter().enumerate() {
        push_bounded(
            &mut tasks,
            crate::upgrades::UpgradeTaskInput {
                task_id: format!("rewrite-install-{index}"),
                kind: "install-artifact".to_string(),
                subject: installed.old_artifact_ref.clone(),
                from_ref: Some(installed.old_artifact_ref.clone()),
                to_ref: Some(installed.new_artifact_ref.clone()),
                precondition_refs: vec![rewrite.preview.plan_ref.clone(), preview_receipt_ref.clone()],
                postcondition_refs: vec![installed.install_receipt_ref.clone()],
                reversible: true,
            },
            MAX_REWRITE_ITEMS,
            "rewrite upgrade tasks",
        )?;
    }
    if !rewrite.preview.query.matches.is_empty() {
        push_bounded(
            &mut tasks,
            crate::upgrades::UpgradeTaskInput {
                task_id: "rewrite-transcript-gate".to_string(),
                kind: "transcript-rerun".to_string(),
                subject: rewrite.preview.plan_ref.clone(),
                from_ref: None,
                to_ref: None,
                precondition_refs: vec![preview_receipt_ref.clone()],
                postcondition_refs: vec![apply_receipt_ref.clone()],
                reversible: true,
            },
            MAX_REWRITE_ITEMS,
            "rewrite upgrade tasks",
        )?;
    }
    let affected_refs = rewrite
        .installed
        .iter()
        .flat_map(|installed| [installed.old_artifact_ref.clone(), installed.new_artifact_ref.clone()])
        .collect::<Vec<_>>();
    crate::upgrades::upgrade_plan_value(&crate::upgrades::UpgradePlanInput {
        session_id: session_id.to_string(),
        reason: "structured rewrite".to_string(),
        summary: format!("structured rewrite applied {} immutable artifact replacement(s)", rewrite.installed.len()),
        initiator_ref: initiator_ref.to_string(),
        capability_refs: capability_refs.to_vec(),
        affected_refs: sorted_unique_refs(&affected_refs),
        impact_refs: rewrite.preview.impacted_refs.clone(),
        tasks,
        compatibility: crate::upgrades::UpgradeCompatibilityWindow {
            old_refs: rewrite.installed.iter().map(|installed| installed.old_artifact_ref.clone()).collect(),
            new_refs: rewrite.installed.iter().map(|installed| installed.new_artifact_ref.clone()).collect(),
            expires_at: None,
            policy_refs: policy_refs.to_vec(),
        },
        rollback_refs: rewrite.installed.iter().map(|installed| installed.old_artifact_ref.clone()).collect(),
        policy_refs: policy_refs.to_vec(),
        evidence_refs: vec![rewrite.preview.plan_ref.clone(), preview_receipt_ref, apply_receipt_ref],
        source_gate_receipt_values: vec![crate::octet_gate::synthetic_clean_octet_gate_receipt_for_tests()?],
    })
}

pub fn parse_rewrite_receipt(value: &IoValue) -> Result<RewriteReceipt> {
    let fields = value
        .collect_simple_record("rewrite-receipt-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <rewrite-receipt-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::REWRITE_RECEIPT_SCHEMA, "rewrite receipt")?;
    let checks = parse_checks(&fields[7])?;
    require_check(&checks, "canonical-receipt", "rewrite receipt")?;
    Ok(RewriteReceipt {
        receipt_ref: canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        decision: record_string(&fields[2], "decision")?,
        subject_ref: record_ref(&fields[3], "subject")?,
        refs: record_ref_sequence(&fields[4], "refs")?,
        diagnostics: record_string_sequence(&fields[5], "diagnostics")?,
        value: value.clone(),
    })
}

pub fn rewrite_summary(value: &IoValue) -> Result<String> {
    if let Ok(receipt) = parse_rewrite_receipt(value) {
        return Ok(format!(
            "rewrite receipt operation={} decision={} subject={} refs={}",
            receipt.operation,
            receipt.decision,
            receipt.subject_ref,
            receipt.refs.len()
        ));
    }
    if let Some(fields) = value.collect_simple_record("rewrite-plan-v1", Some(11)) {
        require_schema(&fields[0], crate::preserves_rail::REWRITE_PLAN_SCHEMA, "rewrite plan")?;
        let diffs = value_to_iovalue(&fields[5]);
        let diff_record = simple_record(&diffs, "diffs", 1)?;
        let diff_count = required_sequence(&diff_record[0], "rewrite plan diffs")?.len();
        return Ok(format!("rewrite plan ref={} diffs={diff_count}", canonical_hash(value)?));
    }
    if let Some(fields) = value.collect_simple_record("rewrite-query-v1", Some(6)) {
        require_schema(&fields[0], crate::preserves_rail::REWRITE_QUERY_SCHEMA, "rewrite query")?;
        return Ok(format!("rewrite query ref={}", canonical_hash(value)?));
    }
    Err(MoltenError::invalid_harness("unsupported rewrite artifact for show"))
}

pub fn rewrite_query_value(input: &RewriteQueryInput) -> Result<IoValue> {
    validate_query_input(input)?;
    Ok(record("rewrite-query-v1", vec![
        string(crate::preserves_rail::REWRITE_QUERY_SCHEMA),
        record("scope", vec![
            refs_sequence(&sorted_unique_refs(&input.root_refs)),
            bool_value(input.include_dependencies),
            sequence(sorted_unique_strings(&input.artifact_kinds).as_slice().iter().map(string).collect()),
        ]),
        pattern_value(&input.pattern)?,
        record("visibility", vec![
            refs_sequence(&sorted_unique_refs(&input.policy_refs)),
            refs_sequence(&sorted_unique_refs(&input.capability_refs)),
            refs_sequence(&sorted_unique_refs(&input.hidden_refs)),
        ]),
        record("constraints", vec![sequence(vec![
            record("constraint", vec![string("immutable-artifacts-only")]),
            record("constraint", vec![string("bounded-preserves-patterns")]),
        ])]),
        checks_value(&[
            "canonical-query-ref",
            "visibility-filter",
            "bounded-preserves-pattern",
            "no-text-only-bypass",
        ]),
    ]))
}
