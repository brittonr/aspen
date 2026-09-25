
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
