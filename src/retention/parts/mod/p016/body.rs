
pub fn parse_gc_execution_gate(value: &IoValue) -> Result<GcExecutionGate> {
    let fields = value
        .collect_simple_record("retention-gc-execute-v1", Some(14))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-gc-execute-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::RETENTION_GC_EXECUTE_SCHEMA, "retention GC execution schema")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    let mode = record_string(&fields[2], "mode")?;
    if mode != "execute-gate" {
        return Err(MoltenError::invalid_harness("retention GC execution mode must be execute-gate"));
    }
    let subsystem = record_string(&fields[3], "subsystem")?;
    validate_name(&subsystem, "retention GC execution subsystem")?;
    let action = record_string(&fields[4], "action")?;
    validate_action(&action)?;
    let (object_ref, object_kind) = parse_object_value(&fields[5])?;
    let retention_class = record_string(&fields[6], "class")?;
    validate_class(&retention_class)?;
    let apply_ref = record_optional_ref(&fields[7], "apply")?;
    let plan_ref = record_optional_ref(&fields[8], "plan")?;
    let recomputed_plan_ref = record_optional_ref(&fields[9], "recomputed-plan")?;
    let retention_receipt_ref = record_optional_ref(&fields[10], "retention-receipt")?;
    let tombstone_ref = record_optional_ref(&fields[11], "tombstone")?;
    let diagnostics = record_string_sequence(&fields[12], "diagnostics")?;
    let checks = parse_checks(&fields[13])?;
    require_check(&checks, "execute-gate-is-not-authority", "retention GC execution")?;
    require_check(&checks, "normal-admission-still-required", "retention GC execution")?;
    require_check(&checks, "remote-clearance-import-still-required", "retention GC execution")?;
    Ok(GcExecutionGate {
        execution_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        subsystem,
        action,
        object_ref,
        object_kind,
        retention_class,
        apply_ref,
        plan_ref,
        recomputed_plan_ref,
        retention_receipt_ref,
        tombstone_ref,
        diagnostics,
        value: value.clone(),
    })
}

pub fn audit_gc_execution(input: GcAuditInput<'_>) -> Result<GcAudit> {
    ensure_store(input.root)?;
    let execution = read_gc_execution_gate(input.root, input.execution_ref)?;
    let execution_scope = gc_audit_scope(
        &execution.subsystem,
        &execution.action,
        &execution.object_ref,
        &execution.object_kind,
        &execution.retention_class,
    );
    let facts = audit_facts(input.root, &execution, &execution_scope)?;
    let decision = if facts.diagnostics.is_empty() { "pass" } else { "deny" };
    let value = audit_value(&AuditValueInput {
        decision,
        subsystem: &execution.subsystem,
        action: &execution.action,
        object_ref: &execution.object_ref,
        object_kind: &execution.object_kind,
        retention_class: &execution.retention_class,
        plan_ref: facts.plan_ref.as_deref(),
        plan_decision: &facts.plan_decision,
        apply_ref: execution.apply_ref.as_deref(),
        apply_decision: &facts.apply_decision,
        execution_ref: &execution.execution_ref,
        execution_decision: &execution.decision,
        retention_receipt_ref: execution.retention_receipt_ref.as_deref(),
        retention_receipt_decision: &facts.retention_receipt_decision,
        tombstone_ref: execution.tombstone_ref.as_deref(),
        tombstone_status: &facts.tombstone_status,
        diagnostics: &facts.diagnostics,
    })?;
    let audit = parse_gc_audit(&value)?;
    write_store_value(&gc_audit_path(input.root, &audit.audit_ref)?, &audit.value)?;
    Ok(audit)
}

fn audit_facts(root: &Path, execution: &GcExecutionGate, scope: &GcAuditScope<'_>) -> Result<AuditFacts> {
    let mut diagnostics = execution_notes(execution)?;
    let ApplyStatus {
        decision: apply_decision,
        plan_ref,
        diagnostics: apply_diagnostics,
    } = apply_status(root, execution, scope)?;
    extend_diag(&mut diagnostics, apply_diagnostics)?;
    let PlanStatus {
        decision: plan_decision,
        diagnostics: plan_diagnostics,
    } = plan_status(root, plan_ref.as_deref(), scope)?;
    extend_diag(&mut diagnostics, plan_diagnostics)?;
    let ReceiptStatus {
        decision: retention_receipt_decision,
        diagnostics: receipt_diagnostics,
    } = receipt_status(root, execution, scope)?;
    extend_diag(&mut diagnostics, receipt_diagnostics)?;
    let TombstoneStatus {
        status: tombstone_status,
        diagnostics: tombstone_diagnostics,
    } = tombstone_status(root, execution, scope)?;
    extend_diag(&mut diagnostics, tombstone_diagnostics)?;
    diagnostics.sort();
    diagnostics.dedup();
    Ok(AuditFacts {
        apply_decision,
        plan_ref,
        plan_decision,
        retention_receipt_decision,
        tombstone_status,
        diagnostics,
    })
}

fn execution_notes(execution: &GcExecutionGate) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if execution.decision != "pass" {
        push_diag(&mut diagnostics, "retention-gc-audit-execution-not-pass")?;
        extend_bounded(
            &mut diagnostics,
            execution.diagnostics.iter().cloned(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC audit diagnostics",
        )?;
    }
    Ok(diagnostics)
}

fn apply_status(root: &Path, execution: &GcExecutionGate, scope: &GcAuditScope<'_>) -> Result<ApplyStatus> {
    let mut diagnostics = Vec::new();
    let mut decision = "missing".to_string();
    let mut plan_ref = execution.plan_ref.clone();
    if let Some(apply_ref) = execution.apply_ref.as_ref() {
        let apply = read_gc_apply(root, apply_ref)?;
        decision.clone_from(&apply.decision);
        if apply.decision != "pass" {
            push_diag(&mut diagnostics, "retention-gc-audit-apply-not-pass")?;
        }
        if !same_gc_scope(
            scope,
            &gc_audit_scope(
                &apply.subsystem,
                &apply.action,
                &apply.object_ref,
                &apply.object_kind,
                &apply.retention_class,
            ),
        ) {
            push_diag(&mut diagnostics, "retention-gc-audit-apply-scope-mismatch")?;
        }
        if execution.plan_ref.as_deref().is_some_and(|reference| reference != apply.plan_ref) {
            push_diag(&mut diagnostics, "retention-gc-audit-execution-apply-plan-mismatch")?;
        }
        if execution
            .retention_receipt_ref
            .as_deref()
            .is_some_and(|reference| apply.retention_receipt_ref.as_deref() != Some(reference))
        {
            push_diag(&mut diagnostics, "retention-gc-audit-execution-apply-receipt-mismatch")?;
        }
        if execution
            .tombstone_ref
            .as_deref()
            .is_some_and(|reference| apply.tombstone_ref.as_deref() != Some(reference))
        {
            push_diag(&mut diagnostics, "retention-gc-audit-execution-apply-tombstone-mismatch")?;
        }
        plan_ref.get_or_insert(apply.plan_ref.clone());
    } else {
        push_diag(&mut diagnostics, "retention-gc-audit-apply-missing")?;
    }
    Ok(ApplyStatus {
        decision,
        plan_ref,
        diagnostics,
    })
}

fn plan_status(root: &Path, plan_ref: Option<&str>, scope: &GcAuditScope<'_>) -> Result<PlanStatus> {
    let mut diagnostics = Vec::new();
    let mut decision = "missing".to_string();
    if let Some(reference) = plan_ref {
        let plan = read_gc_plan(root, reference)?;
        decision.clone_from(&plan.decision);
        if plan.decision != "pass" {
            push_diag(&mut diagnostics, "retention-gc-audit-plan-not-pass")?;
        }
        if !same_gc_scope(
            scope,
            &gc_audit_scope(&plan.subsystem, &plan.action, &plan.object_ref, &plan.object_kind, &plan.retention_class),
        ) {
            push_diag(&mut diagnostics, "retention-gc-audit-plan-scope-mismatch")?;
        }
    } else {
        push_diag(&mut diagnostics, "retention-gc-audit-plan-missing")?;
    }
    Ok(PlanStatus { decision, diagnostics })
}

fn receipt_status(root: &Path, execution: &GcExecutionGate, scope: &GcAuditScope<'_>) -> Result<ReceiptStatus> {
    let mut diagnostics = Vec::new();
    let mut decision = "missing".to_string();
    if let Some(receipt_ref) = execution.retention_receipt_ref.as_ref() {
        let receipt = read_receipt(root, receipt_ref)?;
        decision.clone_from(&receipt.decision);
        if receipt.decision != "pass" {
            push_diag(&mut diagnostics, "retention-gc-audit-retention-receipt-not-pass")?;
        }
        if !same_audit_scope(
            &scope.retention,
            &audit_scope(&receipt.action, &receipt.object_ref, &receipt.object_kind, &receipt.retention_class),
        ) {
            push_diag(&mut diagnostics, "retention-gc-audit-retention-receipt-scope-mismatch")?;
        }
    } else {
        push_diag(&mut diagnostics, "retention-gc-audit-retention-receipt-missing")?;
    }
    Ok(ReceiptStatus { decision, diagnostics })
}

fn tombstone_status(root: &Path, execution: &GcExecutionGate, scope: &GcAuditScope<'_>) -> Result<TombstoneStatus> {
    let mut diagnostics = Vec::new();
    let mut status = "missing".to_string();
    if let Some(tombstone_ref) = execution.tombstone_ref.as_ref() {
        let tombstone = read_tombstone(root, tombstone_ref)?;
        status = "present".to_string();
        if !same_audit_scope(
            &scope.retention,
            &audit_scope(&tombstone.action, &tombstone.object_ref, &tombstone.object_kind, &tombstone.retention_class),
        ) {
            push_diag(&mut diagnostics, "retention-gc-audit-tombstone-scope-mismatch")?;
        }
        if let Some(receipt_ref) = execution.retention_receipt_ref.as_ref()
            && tombstone.receipt_ref != *receipt_ref
        {
            push_diag(&mut diagnostics, "retention-gc-audit-tombstone-receipt-mismatch")?;
        }
    } else if is_destructive_action(&execution.action) {
        push_diag(&mut diagnostics, "retention-gc-audit-tombstone-missing")?;
    } else {
        status = "not-required".to_string();
    }
    Ok(TombstoneStatus { status, diagnostics })
}

fn push_diag(diagnostics: &mut impl VecSink<String>, note: &str) -> Result<()> {
    push_bounded(diagnostics, note.to_string(), MAX_RETENTION_DIAGNOSTICS, "retention GC audit diagnostics")
}

fn extend_diag(diagnostics: &mut impl VecSink<String>, incoming: Vec<String>) -> Result<()> {
    extend_bounded(diagnostics, incoming, MAX_RETENTION_DIAGNOSTICS, "retention GC audit diagnostics")
}

fn gc_audit_scope<'a>(
    subsystem: &'a str,
    action: &'a str,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
) -> GcAuditScope<'a> {
    GcAuditScope {
        subsystem,
        retention: audit_scope(action, object_ref, object_kind, retention_class),
    }
}

fn audit_scope<'a>(
    action: &'a str,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
) -> AuditScope<'a> {
    AuditScope {
        action,
        object_ref,
        object_kind,
        retention_class,
    }
}

fn same_gc_scope(left: &GcAuditScope<'_>, right: &GcAuditScope<'_>) -> bool {
    left.subsystem == right.subsystem && same_audit_scope(&left.retention, &right.retention)
}

fn same_audit_scope(left: &AuditScope<'_>, right: &AuditScope<'_>) -> bool {
    left.action == right.action
        && left.object_ref == right.object_ref
        && left.object_kind == right.object_kind
        && left.retention_class == right.retention_class
}
