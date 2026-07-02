
fn execution_gate_parts(input: &GcExecutionGateInput<'_>) -> Result<ExecutionGateParts> {
    let mut parts = ExecutionGateParts::default();
    let Some(apply_ref) = input.apply_ref else {
        push_bounded(
            &mut parts.diagnostics,
            "retention-gc-execute-apply-missing".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC execution diagnostics",
        )?;
        return Ok(parts);
    };
    require_ref(apply_ref, "retention GC execution apply ref")?;
    match read_gc_apply(input.root, apply_ref) {
        Ok(apply) => {
            parts.plan_ref = Some(apply.plan_ref.clone());
            parts.recomputed_plan_ref = Some(apply.recomputed_plan_ref.clone());
            parts.retention_receipt_ref = apply.retention_receipt_ref.clone();
            parts.tombstone_ref = apply.tombstone_ref.clone();
            extend_bounded(
                &mut parts.diagnostics,
                execution_gate_apply_diagnostics(input, &apply)?,
                MAX_RETENTION_DIAGNOSTICS,
                "retention GC execution diagnostics",
            )?;
            if let Some(receipt_ref) = apply.retention_receipt_ref.as_ref() {
                extend_bounded(
                    &mut parts.diagnostics,
                    execution_gate_receipt_diagnostics(input.root, input, receipt_ref)?,
                    MAX_RETENTION_DIAGNOSTICS,
                    "retention GC execution diagnostics",
                )?;
            } else {
                push_bounded(
                    &mut parts.diagnostics,
                    "retention-gc-execute-retention-receipt-missing".to_string(),
                    MAX_RETENTION_DIAGNOSTICS,
                    "retention GC execution diagnostics",
                )?;
            }
            extend_bounded(
                &mut parts.diagnostics,
                execution_gate_tombstone_binding_diagnostics(input.root, input, &apply)?,
                MAX_RETENTION_DIAGNOSTICS,
                "retention GC execution diagnostics",
            )?;
        }
        Err(error) => push_bounded(
            &mut parts.diagnostics,
            format!("retention-gc-execute-apply-unreadable:{error}"),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC execution diagnostics",
        )?,
    }
    Ok(parts)
}

pub fn store_gc_execution_gate(input: GcExecutionGateInput<'_>) -> Result<GcExecutionGate> {
    ensure_store(input.root)?;
    validate_name(input.subsystem, "retention GC execution subsystem")?;
    validate_action(input.action)?;
    require_ref(input.object_ref, "retention GC execution object ref")?;
    validate_name(input.object_kind, "retention GC execution object kind")?;
    validate_class(input.retention_class)?;
    let mut parts = execution_gate_parts(&input)?;
    parts.diagnostics.sort();
    parts.diagnostics.dedup();
    let decision = if parts.diagnostics.is_empty() { "pass" } else { "deny" };
    let value = execution_gate_value(&ExecutionGateValueInput {
        decision,
        subsystem: input.subsystem,
        action: input.action,
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retention_class: input.retention_class,
        apply_ref: input.apply_ref,
        plan_ref: parts.plan_ref.as_deref(),
        recomputed_plan_ref: parts.recomputed_plan_ref.as_deref(),
        retention_receipt_ref: parts.retention_receipt_ref.as_deref(),
        tombstone_ref: parts.tombstone_ref.as_deref(),
        diagnostics: &parts.diagnostics,
    })?;
    let gate = parse_gc_execution_gate(&value)?;
    write_store_value(&gc_execute_path(input.root, &gate.execution_ref)?, &gate.value)?;
    Ok(gate)
}

fn execution_gate_apply_diagnostics(input: &GcExecutionGateInput<'_>, apply: &GcApply) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if apply.decision != "pass" {
        push_bounded(
            &mut diagnostics,
            "retention-gc-execute-apply-not-pass".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC execution diagnostics",
        )?;
        extend_bounded(
            &mut diagnostics,
            apply.diagnostics.iter().cloned(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC execution diagnostics",
        )?;
    }
    if apply.plan_ref != apply.recomputed_plan_ref {
        push_bounded(
            &mut diagnostics,
            "retention-gc-execute-apply-plan-drift".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC execution diagnostics",
        )?;
    }
    if apply.subsystem != input.subsystem
        || apply.action != input.action
        || apply.object_ref != input.object_ref
        || apply.object_kind != input.object_kind
        || apply.retention_class != input.retention_class
    {
        push_bounded(
            &mut diagnostics,
            "retention-gc-execute-apply-scope-mismatch".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC execution diagnostics",
        )?;
    }
    Ok(diagnostics)
}

fn execution_gate_receipt_diagnostics(
    root: &Path,
    input: &GcExecutionGateInput<'_>,
    receipt_ref: &str,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    match read_receipt(root, receipt_ref) {
        Ok(receipt) => {
            if receipt.decision != "pass" {
                push_bounded(
                    &mut diagnostics,
                    "retention-gc-execute-retention-receipt-not-pass".to_string(),
                    MAX_RETENTION_DIAGNOSTICS,
                    "retention GC execution diagnostics",
                )?;
            }
            if receipt.object_ref != input.object_ref
                || receipt.object_kind != input.object_kind
                || receipt.retention_class != input.retention_class
                || receipt.action != input.action
            {
                push_bounded(
                    &mut diagnostics,
                    "retention-gc-execute-retention-receipt-scope-mismatch".to_string(),
                    MAX_RETENTION_DIAGNOSTICS,
                    "retention GC execution diagnostics",
                )?;
            }
            if receipt.tombstone_ref.is_none() && is_destructive_action(input.action) {
                push_bounded(
                    &mut diagnostics,
                    "retention-gc-execute-retention-receipt-tombstone-missing".to_string(),
                    MAX_RETENTION_DIAGNOSTICS,
                    "retention GC execution diagnostics",
                )?;
            }
        }
        Err(error) => push_bounded(
            &mut diagnostics,
            format!("retention-gc-execute-retention-receipt-unreadable:{error}"),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC execution diagnostics",
        )?,
    }
    Ok(diagnostics)
}

fn execution_gate_tombstone_binding_diagnostics(
    root: &Path,
    input: &GcExecutionGateInput<'_>,
    apply: &GcApply,
) -> Result<Vec<String>> {
    let Some(tombstone_ref) = apply.tombstone_ref.as_ref() else {
        let mut diagnostics = Vec::new();
        if is_destructive_action(input.action) {
            push_bounded(
                &mut diagnostics,
                "retention-gc-execute-tombstone-missing".to_string(),
                MAX_RETENTION_DIAGNOSTICS,
                "retention GC execution diagnostics",
            )?;
        }
        return Ok(diagnostics);
    };
    execution_gate_tombstone_diagnostics(root, input, tombstone_ref, apply.retention_receipt_ref.as_deref())
}

fn execution_gate_tombstone_diagnostics(
    root: &Path,
    input: &GcExecutionGateInput<'_>,
    tombstone_ref: &str,
    receipt_ref: Option<&str>,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    match read_tombstone(root, tombstone_ref) {
        Ok(tombstone) => {
            if tombstone.object_ref != input.object_ref
                || tombstone.object_kind != input.object_kind
                || tombstone.retention_class != input.retention_class
                || tombstone.action != input.action
            {
                push_bounded(
                    &mut diagnostics,
                    "retention-gc-execute-tombstone-scope-mismatch".to_string(),
                    MAX_RETENTION_DIAGNOSTICS,
                    "retention GC execution diagnostics",
                )?;
            }
            if let Some(expected_receipt_ref) = receipt_ref {
                let pending_receipt_ref = synthetic_ref("pending-retention-receipt")?;
                if tombstone.receipt_ref != expected_receipt_ref && tombstone.receipt_ref != pending_receipt_ref {
                    push_bounded(
                        &mut diagnostics,
                        "retention-gc-execute-tombstone-receipt-mismatch".to_string(),
                        MAX_RETENTION_DIAGNOSTICS,
                        "retention GC execution diagnostics",
                    )?;
                }
            }
        }
        Err(error) => push_bounded(
            &mut diagnostics,
            format!("retention-gc-execute-tombstone-unreadable:{error}"),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC execution diagnostics",
        )?,
    }
    Ok(diagnostics)
}

fn execution_gate_value(input: &ExecutionGateValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_name(input.subsystem, "retention GC execution subsystem")?;
    validate_action(input.action)?;
    require_ref(input.object_ref, "retention GC execution object ref")?;
    validate_name(input.object_kind, "retention GC execution object kind")?;
    validate_class(input.retention_class)?;
    if let Some(apply_ref) = input.apply_ref {
        require_ref(apply_ref, "retention GC execution apply ref")?;
    }
    if let Some(plan_ref) = input.plan_ref {
        require_ref(plan_ref, "retention GC execution plan ref")?;
    }
    if let Some(recomputed_plan_ref) = input.recomputed_plan_ref {
        require_ref(recomputed_plan_ref, "retention GC execution recomputed plan ref")?;
    }
    if let Some(receipt_ref) = input.retention_receipt_ref {
        require_ref(receipt_ref, "retention GC execution receipt ref")?;
    }
    if let Some(tombstone_ref) = input.tombstone_ref {
        require_ref(tombstone_ref, "retention GC execution tombstone ref")?;
    }
    Ok(crate::preserves_rail::record("retention-gc-execute-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RETENTION_GC_EXECUTE_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("mode", vec![crate::preserves_rail::string("execute-gate")]),
        crate::preserves_rail::record("subsystem", vec![crate::preserves_rail::string(input.subsystem)]),
        crate::preserves_rail::record("action", vec![crate::preserves_rail::string(input.action)]),
        object_value(input.object_ref, input.object_kind),
        crate::preserves_rail::record("class", vec![crate::preserves_rail::string(input.retention_class)]),
        crate::preserves_rail::record("apply", vec![optional_ref_value(input.apply_ref)]),
        crate::preserves_rail::record("plan", vec![optional_ref_value(input.plan_ref)]),
        crate::preserves_rail::record("recomputed-plan", vec![optional_ref_value(input.recomputed_plan_ref)]),
        crate::preserves_rail::record("retention-receipt", vec![optional_ref_value(input.retention_receipt_ref)]),
        crate::preserves_rail::record("tombstone", vec![optional_ref_value(input.tombstone_ref)]),
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            ("apply-ref-required", pass_or_deny(input.apply_ref.is_some())),
            ("apply-decision-pass", pass_or_deny(input.decision == "pass")),
            (
                "apply-plan-unchanged",
                pass_or_deny(input.plan_ref.is_some() && input.plan_ref == input.recomputed_plan_ref),
            ),
            ("retention-receipt-bound", pass_or_deny(input.retention_receipt_ref.is_some())),
            (
                "tombstone-bound",
                pass_or_deny(!is_destructive_action(input.action) || input.tombstone_ref.is_some()),
            ),
            ("execute-gate-is-not-authority", "pass"),
            ("normal-admission-still-required", "pass"),
            ("remote-clearance-import-still-required", "pass"),
        ]),
    ]))
}
