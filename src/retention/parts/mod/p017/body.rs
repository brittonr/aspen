
pub fn evaluate_gc_lifecycle(input: RetentionGcLifecycleInput<'_>) -> RetentionGcLifecycleDecision {
    let mut diagnostics = Vec::with_capacity(RETENTION_GC_LIFECYCLE_DIAGNOSTIC_CAPACITY);
    let Some(plan) = input.plan else {
        diagnostics.push("retention-gc-lifecycle-plan-missing".to_string());
        return lifecycle_decision(diagnostics);
    };
    let Some(apply) = input.apply else {
        diagnostics.push("retention-gc-lifecycle-apply-missing".to_string());
        return lifecycle_decision(diagnostics);
    };
    let Some(execution) = input.execution else {
        diagnostics.push("retention-gc-lifecycle-execution-missing".to_string());
        return lifecycle_decision(diagnostics);
    };
    let Some(audit) = input.audit else {
        diagnostics.push("retention-gc-lifecycle-audit-missing".to_string());
        return lifecycle_decision(diagnostics);
    };

    lifecycle_decision_checks(plan, apply, execution, audit, &mut diagnostics);
    lifecycle_link_checks(plan, apply, execution, audit, &mut diagnostics);
    diagnostics.extend(audit.diagnostics.iter().cloned());
    lifecycle_decision(diagnostics)
}

fn lifecycle_decision_checks(
    plan: &GcPlan,
    apply: &GcApply,
    execution: &GcExecutionGate,
    audit: &GcAudit,
    diagnostics: &mut Vec<String>,
) {
    if plan.decision != "pass" {
        diagnostics.push("retention-gc-lifecycle-plan-not-pass".to_string());
    }
    if apply.decision != "pass" {
        diagnostics.push("retention-gc-lifecycle-apply-not-pass".to_string());
    }
    if execution.decision != "pass" {
        diagnostics.push("retention-gc-lifecycle-execution-not-pass".to_string());
    }
    if audit.decision != "pass" {
        diagnostics.push("retention-gc-lifecycle-audit-not-pass".to_string());
    }
}

fn lifecycle_link_checks(
    plan: &GcPlan,
    apply: &GcApply,
    execution: &GcExecutionGate,
    audit: &GcAudit,
    diagnostics: &mut Vec<String>,
) {
    if !same_lifecycle_scope_plan_apply(plan, apply) {
        diagnostics.push("retention-gc-lifecycle-plan-apply-scope-mismatch".to_string());
    }
    if !same_lifecycle_scope_apply_execution(apply, execution) {
        diagnostics.push("retention-gc-lifecycle-apply-execution-scope-mismatch".to_string());
    }
    if !same_lifecycle_scope_execution_audit(execution, audit) {
        diagnostics.push("retention-gc-lifecycle-execution-audit-scope-mismatch".to_string());
    }
    if apply.plan_ref != plan.plan_ref {
        diagnostics.push("retention-gc-lifecycle-apply-plan-mismatch".to_string());
    }
    if apply.recomputed_plan_ref != plan.plan_ref {
        diagnostics.push("retention-gc-lifecycle-recomputed-plan-mismatch".to_string());
    }
    if execution.apply_ref.as_deref() != Some(apply.apply_ref.as_str()) {
        diagnostics.push("retention-gc-lifecycle-execution-apply-mismatch".to_string());
    }
    if execution.plan_ref.as_deref() != Some(plan.plan_ref.as_str()) {
        diagnostics.push("retention-gc-lifecycle-execution-plan-mismatch".to_string());
    }
    if execution.recomputed_plan_ref.as_deref() != Some(apply.recomputed_plan_ref.as_str()) {
        diagnostics.push("retention-gc-lifecycle-execution-recomputed-plan-mismatch".to_string());
    }
    if execution.retention_receipt_ref != apply.retention_receipt_ref {
        diagnostics.push("retention-gc-lifecycle-execution-receipt-mismatch".to_string());
    }
    if execution.tombstone_ref != apply.tombstone_ref {
        diagnostics.push("retention-gc-lifecycle-execution-tombstone-mismatch".to_string());
    }
    if audit.plan_ref.as_deref() != Some(plan.plan_ref.as_str()) {
        diagnostics.push("retention-gc-lifecycle-audit-plan-mismatch".to_string());
    }
    if audit.apply_ref.as_deref() != Some(apply.apply_ref.as_str()) {
        diagnostics.push("retention-gc-lifecycle-audit-apply-mismatch".to_string());
    }
    if audit.execution_ref != execution.execution_ref {
        diagnostics.push("retention-gc-lifecycle-audit-execution-mismatch".to_string());
    }
    if audit.retention_receipt_ref != apply.retention_receipt_ref {
        diagnostics.push("retention-gc-lifecycle-audit-receipt-mismatch".to_string());
    }
    if audit.tombstone_ref != apply.tombstone_ref {
        diagnostics.push("retention-gc-lifecycle-audit-tombstone-mismatch".to_string());
    }
    if is_destructive_action(&plan.action) && apply.tombstone_ref.is_none() {
        diagnostics.push("retention-gc-lifecycle-tombstone-missing".to_string());
    }
}

fn same_lifecycle_scope_plan_apply(plan: &GcPlan, apply: &GcApply) -> bool {
    plan.subsystem == apply.subsystem
        && plan.action == apply.action
        && plan.object_ref == apply.object_ref
        && plan.object_kind == apply.object_kind
        && plan.retention_class == apply.retention_class
}

fn same_lifecycle_scope_apply_execution(apply: &GcApply, execution: &GcExecutionGate) -> bool {
    apply.subsystem == execution.subsystem
        && apply.action == execution.action
        && apply.object_ref == execution.object_ref
        && apply.object_kind == execution.object_kind
        && apply.retention_class == execution.retention_class
}

fn same_lifecycle_scope_execution_audit(execution: &GcExecutionGate, audit: &GcAudit) -> bool {
    execution.subsystem == audit.subsystem
        && execution.action == audit.action
        && execution.object_ref == audit.object_ref
        && execution.object_kind == audit.object_kind
        && execution.retention_class == audit.retention_class
}

fn lifecycle_decision(mut diagnostics: Vec<String>) -> RetentionGcLifecycleDecision {
    diagnostics.sort();
    diagnostics.dedup();
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    RetentionGcLifecycleDecision {
        decision: decision.to_string(),
        diagnostics,
    }
}

fn audit_value(input: &AuditValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_name(input.subsystem, "retention GC audit subsystem")?;
    validate_action(input.action)?;
    require_ref(input.object_ref, "retention GC audit object ref")?;
    validate_name(input.object_kind, "retention GC audit object kind")?;
    validate_class(input.retention_class)?;
    if let Some(plan_ref) = input.plan_ref {
        require_ref(plan_ref, "retention GC audit plan ref")?;
    }
    validate_audit_step_status(input.plan_decision, "retention GC audit plan decision")?;
    if let Some(apply_ref) = input.apply_ref {
        require_ref(apply_ref, "retention GC audit apply ref")?;
    }
    validate_audit_step_status(input.apply_decision, "retention GC audit apply decision")?;
    require_ref(input.execution_ref, "retention GC audit execution ref")?;
    validate_decision(input.execution_decision)?;
    if let Some(receipt_ref) = input.retention_receipt_ref {
        require_ref(receipt_ref, "retention GC audit receipt ref")?;
    }
    validate_audit_step_status(input.retention_receipt_decision, "retention GC audit receipt decision")?;
    if let Some(tombstone_ref) = input.tombstone_ref {
        require_ref(tombstone_ref, "retention GC audit tombstone ref")?;
    }
    validate_audit_step_status(input.tombstone_status, "retention GC audit tombstone status")?;
    Ok(crate::preserves_rail::record("retention-gc-audit-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RETENTION_GC_AUDIT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("mode", vec![crate::preserves_rail::string("audit")]),
        crate::preserves_rail::record("subsystem", vec![crate::preserves_rail::string(input.subsystem)]),
        crate::preserves_rail::record("action", vec![crate::preserves_rail::string(input.action)]),
        object_value(input.object_ref, input.object_kind),
        crate::preserves_rail::record("class", vec![crate::preserves_rail::string(input.retention_class)]),
        crate::preserves_rail::record("plan", vec![
            optional_ref_value(input.plan_ref),
            crate::preserves_rail::string(input.plan_decision),
        ]),
        crate::preserves_rail::record("apply", vec![
            optional_ref_value(input.apply_ref),
            crate::preserves_rail::string(input.apply_decision),
        ]),
        crate::preserves_rail::record("execution", vec![
            crate::preserves_rail::string(input.execution_ref),
            crate::preserves_rail::string(input.execution_decision),
        ]),
        crate::preserves_rail::record("retention-receipt", vec![
            optional_ref_value(input.retention_receipt_ref),
            crate::preserves_rail::string(input.retention_receipt_decision),
        ]),
        crate::preserves_rail::record("tombstone", vec![
            optional_ref_value(input.tombstone_ref),
            crate::preserves_rail::string(input.tombstone_status),
        ]),
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            ("audit-is-not-authority", "pass"),
            ("plan-link-bound", pass_or_deny(input.plan_ref.is_some())),
            ("apply-link-bound", pass_or_deny(input.apply_ref.is_some())),
            ("execution-link-bound", "pass"),
            ("retention-receipt-link-bound", pass_or_deny(input.retention_receipt_ref.is_some())),
            (
                "tombstone-link-bound",
                pass_or_deny(!is_destructive_action(input.action) || input.tombstone_ref.is_some()),
            ),
            ("normal-admission-still-required", "pass"),
            ("remote-clearance-import-still-required", "pass"),
        ]),
    ]))
}

fn validate_audit_step_status(status: &str, label: &str) -> Result<()> {
    match status {
        "pass" | "deny" | "missing" | "present" | "not-required" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported {label}: {other}"))),
    }
}

pub fn parse_gc_audit(value: &IoValue) -> Result<GcAudit> {
    let fields = value
        .collect_simple_record("retention-gc-audit-v1", Some(14))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-gc-audit-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::RETENTION_GC_AUDIT_SCHEMA, "retention GC audit schema")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    let mode = record_string(&fields[2], "mode")?;
    if mode != "audit" {
        return Err(MoltenError::invalid_harness("retention GC audit mode must be audit"));
    }
    let subsystem = record_string(&fields[3], "subsystem")?;
    validate_name(&subsystem, "retention GC audit subsystem")?;
    let action = record_string(&fields[4], "action")?;
    validate_action(&action)?;
    let (object_ref, object_kind) = parse_object_value(&fields[5])?;
    let retention_class = record_string(&fields[6], "class")?;
    validate_class(&retention_class)?;
    let (plan_ref, plan_decision) = record_optional_ref_with_status(&fields[7], "plan")?;
    let (apply_ref, apply_decision) = record_optional_ref_with_status(&fields[8], "apply")?;
    let execution_fields = fields[9]
        .collect_simple_record("execution", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected retention GC audit execution record"))?;
    let execution_ref = required_string(&execution_fields[0], "retention GC audit execution ref")?;
    require_ref(&execution_ref, "retention GC audit execution ref")?;
    let execution_decision = required_string(&execution_fields[1], "retention GC audit execution decision")?;
    validate_decision(&execution_decision)?;
    let (retention_receipt_ref, retention_receipt_decision) =
        record_optional_ref_with_status(&fields[10], "retention-receipt")?;
    let (tombstone_ref, tombstone_status) = record_optional_ref_with_status(&fields[11], "tombstone")?;
    let diagnostics = record_string_sequence(&fields[12], "diagnostics")?;
    let checks = parse_checks(&fields[13])?;
    require_check(&checks, "audit-is-not-authority", "retention GC audit")?;
    require_check(&checks, "normal-admission-still-required", "retention GC audit")?;
    require_check(&checks, "remote-clearance-import-still-required", "retention GC audit")?;
    Ok(GcAudit {
        audit_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        subsystem,
        action,
        object_ref,
        object_kind,
        retention_class,
        plan_ref,
        plan_decision,
        apply_ref,
        apply_decision,
        execution_ref,
        execution_decision,
        retention_receipt_ref,
        retention_receipt_decision,
        tombstone_ref,
        tombstone_status,
        diagnostics,
        value: value.clone(),
    })
}

pub fn read_gc_audit(root: &Path, audit_ref: &str) -> Result<GcAudit> {
    require_ref(audit_ref, "retention GC audit ref")?;
    let value = read_store_value(&gc_audit_path(root, audit_ref)?)?;
    let audit = parse_gc_audit(&value)?;
    if audit.audit_ref != audit_ref {
        return Err(MoltenError::invalid_harness("stored retention GC audit ref mismatch"));
    }
    Ok(audit)
}

pub fn explain_candidate(input: CandidateExplainInput<'_>) -> Result<CandidateExplain> {
    validate_candidate_explain_input(&input)?;
    let filter = CandidateFilter {
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retention_class: input.retention_class,
        action: input.action,
        subsystem: input.subsystem,
    };
    let refs = MatchRefs::collect(input.root, &filter)?;
    let diagnostics = candidate_explain_diagnostics(&refs.value_input(input, &[]))?;
    let value = candidate_explain_value(&refs.value_input(input, &diagnostics))?;
    parse_candidate_explain(&value)
}

impl MatchRefs {
    fn collect(root: &Path, filter: &CandidateFilter<'_>) -> Result<Self> {
        let pin_refs = pins_for(root, filter)?;
        let admission_refs = admissions_for(root, filter)?;
        let remote_clearance_refs = clearances_for(root, filter)?;
        let remote_clearance_import_refs = imports_for(root, &remote_clearance_refs)?;
        let gc_plan_refs = plans_for(root, filter)?;
        let gc_apply_refs = applies_for(root, filter)?;
        let gc_execution_refs = executions_for(root, filter)?;
        let gc_audit_refs = audits_for(root, filter)?;
        let retention_receipt_refs = receipts_for(root, filter)?;
        let tombstone_refs = tombstones_for(root, filter)?;
        Ok(Self {
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
        })
    }

    fn value_input<'a>(
        &'a self,
        input: CandidateExplainInput<'a>,
        diagnostics: &'a [String],
    ) -> CandidateExplainValueInput<'a> {
        CandidateExplainValueInput {
            object_ref: input.object_ref,
            object_kind: input.object_kind,
            retention_class: input.retention_class,
            action: input.action,
            subsystem: input.subsystem,
            pin_refs: &self.pin_refs,
            admission_refs: &self.admission_refs,
            remote_clearance_refs: &self.remote_clearance_refs,
            remote_clearance_import_refs: &self.remote_clearance_import_refs,
            gc_plan_refs: &self.gc_plan_refs,
            gc_apply_refs: &self.gc_apply_refs,
            gc_execution_refs: &self.gc_execution_refs,
            gc_audit_refs: &self.gc_audit_refs,
            retention_receipt_refs: &self.retention_receipt_refs,
            tombstone_refs: &self.tombstone_refs,
            diagnostics,
        }
    }
}

fn pins_for(root: &Path, filter: &CandidateFilter<'_>) -> Result<Vec<String>> {
    collect_matching_refs(
        &pins_dir(root),
        parse_pin,
        |pin| filter.matches_object(&pin.object_ref, &pin.object_kind, &pin.retention_class),
        |pin| pin.pin_ref.clone(),
        "retention candidate pins",
    )
}

fn admissions_for(root: &Path, filter: &CandidateFilter<'_>) -> Result<Vec<String>> {
    collect_matching_refs(
        &admissions_dir(root),
        parse_evidence_admission,
        |admission| {
            filter.matches_retention(
                &admission.object_ref,
                &admission.object_kind,
                &admission.retention_class,
                &admission.action,
            )
        },
        |admission| admission.admission_ref.clone(),
        "retention candidate admissions",
    )
}

fn clearances_for(root: &Path, filter: &CandidateFilter<'_>) -> Result<Vec<String>> {
    collect_matching_refs(
        &remote_clearances_dir(root),
        parse_remote_gc_clearance,
        |clearance| {
            filter.matches_retention(
                &clearance.object_ref,
                &clearance.object_kind,
                &clearance.retention_class,
                &clearance.action,
            )
        },
        |clearance| clearance.clearance_ref.clone(),
        "retention candidate remote clearances",
    )
}

fn imports_for(root: &Path, remote_clearance_refs: &[String]) -> Result<Vec<String>> {
    collect_matching_refs(
        &remote_clearance_imports_dir(root),
        parse_remote_gc_clearance_import,
        |import| import.clearance_ref.as_ref().is_some_and(|reference| remote_clearance_refs.contains(reference)),
        |import| import.import_ref.clone(),
        "retention candidate remote clearance imports",
    )
}

fn plans_for(root: &Path, filter: &CandidateFilter<'_>) -> Result<Vec<String>> {
    collect_matching_refs(
        &gc_plans_dir(root),
        parse_gc_plan,
        |plan| {
            filter.matches_gc(&plan.subsystem, &plan.object_ref, &plan.object_kind, &plan.retention_class, &plan.action)
        },
        |plan| plan.plan_ref.clone(),
        "retention candidate GC plans",
    )
}

fn applies_for(root: &Path, filter: &CandidateFilter<'_>) -> Result<Vec<String>> {
    collect_matching_refs(
        &gc_applies_dir(root),
        parse_gc_apply,
        |apply| {
            filter.matches_gc(
                &apply.subsystem,
                &apply.object_ref,
                &apply.object_kind,
                &apply.retention_class,
                &apply.action,
            )
        },
        |apply| apply.apply_ref.clone(),
        "retention candidate GC applies",
    )
}
