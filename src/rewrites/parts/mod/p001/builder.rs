/// Apply a rewrite with its upgrade plan only after admitting operator refs and fresh source evidence.
// r[impl molten.local_rewrite.upgrade_hook_authority]
// r[impl molten.upgrades.authenticated_source_gate]
pub fn apply_with_upgrade_plan(
    root: &Path,
    input: &RewritePlanInput,
    upgrade: &RewriteUpgradePlanInput<'_>,
) -> Result<(RewriteApply, IoValue)> {
    validate_non_empty(upgrade.session_id, "rewrite upgrade session id")?;
    validate_ref(upgrade.initiator_ref, "rewrite upgrade initiator ref")?;
    require_refs(upgrade.capability_refs, "rewrite upgrade capability ref")?;
    require_refs(upgrade.policy_refs, "rewrite upgrade policy ref")?;
    let validation = crate::octet_gate::validate_octet_source_gate(&crate::octet_gate::OctetSourceGateValidationInput {
        consumer: "upgrade-plan".to_string(),
        subject_ref: upgrade.source_gate.binding_ref().to_string(),
        receipt_value: Some(upgrade.source_gate.gate_value().clone()),
        source_scope: Vec::new(),
    })?;
    if validation.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "rewrite upgrade source gate validation failed: {}",
            validation.diagnostics.join("; "),
        )));
    }
    let applied = apply(root, input)?;
    let plan = upgrade_plan_from_apply(&applied, upgrade)?;
    Ok((applied, plan))
}

// r[impl molten.local_rewrite.upgrade_hook_authority]
// r[impl molten.upgrades.authenticated_source_gate]
fn upgrade_plan_from_apply(rewrite: &RewriteApply, input: &RewriteUpgradePlanInput<'_>) -> Result<IoValue> {
    let RewriteUpgradePlanInput {
        session_id,
        initiator_ref,
        capability_refs,
        policy_refs,
        source_gate,
    } = *input;
    let apply_receipt_ref = canonical_hash(&rewrite.receipt_value)?;
    let preview_receipt_ref = canonical_hash(&rewrite.preview.receipt_value)?;
    let tasks = rewrite_upgrade_tasks(rewrite, &preview_receipt_ref, &apply_receipt_ref)?;
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
        evidence_refs: vec![
            rewrite.preview.plan_ref.clone(),
            preview_receipt_ref,
            apply_receipt_ref,
            source_gate.binding_ref().to_string(),
            source_gate.validation_ref().to_string(),
        ],
        source_gate_receipt_values: vec![source_gate.gate_value().clone()],
    })
}

fn rewrite_upgrade_tasks(
    rewrite: &RewriteApply,
    preview_receipt_ref: &str,
    apply_receipt_ref: &str,
) -> Result<Vec<crate::upgrades::UpgradeTaskInput>> {
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
                precondition_refs: vec![rewrite.preview.plan_ref.clone(), preview_receipt_ref.to_string()],
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
                precondition_refs: vec![preview_receipt_ref.to_string()],
                postcondition_refs: vec![apply_receipt_ref.to_string()],
                reversible: true,
            },
            MAX_REWRITE_ITEMS,
            "rewrite upgrade tasks",
        )?;
    }
    Ok(tasks)
}