
pub fn apply_gc_plan(input: GcApplyFromPlanInput<'_>) -> Result<GcApply> {
    let root = open_capability_retention_root(input.root)?;
    apply_gc_plan_with_root(GcApplyFromPlanInput {
        root: &root,
        plan_ref: input.plan_ref,
    })
}

pub fn apply_gc_plan_with_root(input: GcApplyFromPlanInput<'_, CapabilityRetentionRoot>) -> Result<GcApply> {
    ensure_store_with_root(input.root)?;
    let original = read_gc_plan_with_root(input.root, input.plan_ref)?;
    let recomputed = store_gc_plan_with_root(GcPlanInput {
        root: input.root,
        subsystem: &original.subsystem,
        object_ref: &original.object_ref,
        object_kind: &original.object_kind,
        retention_class: &original.retention_class,
        action: &original.action,
        evidence: &original.evidence,
    })?;
    let admission = admit_destructive_evidence_with_root(DestructiveAdmissionInput {
        root: input.root,
        evidence: &original.evidence,
        object_ref: &original.object_ref,
        object_kind: &original.object_kind,
        retention_class: &original.retention_class,
        action: &original.action,
    })?;
    let outcome = apply_outcome(input.root, &original, &recomputed, &admission)?;
    let decision = if outcome.diagnostics.is_empty() { "pass" } else { "deny" };
    let mut admission_refs = admission.admitted_refs;
    admission_refs.sort();
    admission_refs.dedup();
    let value = apply_value(&ApplyValueInput {
        decision,
        subsystem: &original.subsystem,
        action: &original.action,
        object_ref: &original.object_ref,
        object_kind: &original.object_kind,
        retention_class: &original.retention_class,
        requester_ref: original.requester_ref.as_deref(),
        plan_ref: &original.plan_ref,
        recomputed_plan_ref: &recomputed.plan_ref,
        retention_receipt_ref: outcome.retention_receipt_ref.as_deref(),
        tombstone_ref: outcome.tombstone_ref.as_deref(),
        admission_refs: &admission_refs,
        diagnostics: &outcome.diagnostics,
    })?;
    let apply = parse_gc_apply(&value)?;
    write_store_value_with_root(input.root, &capability_ref_path(GC_APPLY_DIR, &apply.apply_ref)?, &apply.value)?;
    Ok(apply)
}

fn apply_outcome(
    root: &CapabilityRetentionRoot,
    original: &GcPlan,
    recomputed: &GcPlan,
    admission: &DestructiveAdmission,
) -> Result<ApplyOutcome> {
    let diagnostics = apply_diagnostics(original, recomputed, admission)?;
    if diagnostics.is_empty() {
        apply_success_outcome(root, original, admission)
    } else {
        Ok(ApplyOutcome {
            retention_receipt_ref: None,
            tombstone_ref: None,
            diagnostics,
        })
    }
}

fn push_apply_diagnostic(diagnostics: &mut impl VecSink<String>, diagnostic: &str, details: &[String]) -> Result<()> {
    push_bounded(diagnostics, diagnostic.to_string(), MAX_RETENTION_DIAGNOSTICS, APPLY_DIAGNOSTICS)?;
    extend_bounded(diagnostics, details.iter().cloned(), MAX_RETENTION_DIAGNOSTICS, APPLY_DIAGNOSTICS)
}

fn apply_diagnostics(original: &GcPlan, recomputed: &GcPlan, admission: &DestructiveAdmission) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if original.decision != "pass" {
        push_apply_diagnostic(&mut diagnostics, "retention-gc-apply-plan-not-pass", &original.diagnostics)?;
    }
    if recomputed.plan_ref != original.plan_ref {
        push_apply_diagnostic(&mut diagnostics, "retention-gc-apply-plan-drift", &[])?;
    }
    if recomputed.decision != "pass" {
        push_apply_diagnostic(
            &mut diagnostics,
            "retention-gc-apply-recomputed-plan-not-pass",
            &recomputed.diagnostics,
        )?;
    }
    if admission.decision != "pass" {
        push_apply_diagnostic(&mut diagnostics, "retention-gc-apply-admission-not-pass", &admission.diagnostics)?;
    }
    diagnostics.sort();
    diagnostics.dedup();
    Ok(diagnostics)
}

fn apply_success_outcome(
    root: &CapabilityRetentionRoot,
    original: &GcPlan,
    admission: &DestructiveAdmission,
) -> Result<ApplyOutcome> {
    let requester_ref = destructive_requester_ref(&original.evidence, "retention-gc-apply-missing-requester")?;
    let evaluation = evaluate_with_root(EvaluationInput {
        root,
        object_ref: &original.object_ref,
        object_kind: &original.object_kind,
        retention_class: &original.retention_class,
        action: &original.action,
        requester_ref: &requester_ref,
        is_reference_index_complete: original.evidence.is_reference_index_complete,
        retained_refs: &original.evidence.retained_refs,
        remote_refs: &original.evidence.remote_refs,
        policy_refs: &original.evidence.policy_refs,
        evidence_refs: &original.evidence.evidence_refs,
        has_delete_authority: admission.has_delete_authority,
        has_remote_gc_clearance: admission.has_remote_gc_clearance,
    })?;
    let retention_receipt_ref = Some(evaluation.receipt.receipt_ref.clone());
    let tombstone_ref = evaluation.tombstone.as_ref().map(|created| created.tombstone_ref.clone());
    let mut diagnostics = Vec::new();
    if evaluation.receipt.decision != "pass" {
        push_apply_diagnostic(
            &mut diagnostics,
            "retention-gc-apply-retention-receipt-not-pass",
            &evaluation.receipt.diagnostics,
        )?;
    }
    diagnostics.sort();
    diagnostics.dedup();
    Ok(ApplyOutcome {
        retention_receipt_ref,
        tombstone_ref,
        diagnostics,
    })
}

fn apply_value(input: &ApplyValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_name(input.subsystem, "retention GC apply subsystem")?;
    validate_action(input.action)?;
    require_ref(input.object_ref, "retention GC apply object ref")?;
    validate_name(input.object_kind, "retention GC apply object kind")?;
    validate_class(input.retention_class)?;
    if let Some(requester_ref) = input.requester_ref {
        require_ref(requester_ref, "retention GC apply requester ref")?;
    }
    require_ref(input.plan_ref, "retention GC apply plan ref")?;
    require_ref(input.recomputed_plan_ref, "retention GC apply recomputed plan ref")?;
    if let Some(receipt_ref) = input.retention_receipt_ref {
        require_ref(receipt_ref, "retention GC apply receipt ref")?;
    }
    if let Some(tombstone_ref) = input.tombstone_ref {
        require_ref(tombstone_ref, "retention GC apply tombstone ref")?;
    }
    validate_refs(input.admission_refs, "retention GC apply admission ref")?;
    let is_plan_unchanged = input.plan_ref == input.recomputed_plan_ref;
    let is_plan_passed = input.decision == "pass";
    let is_tombstone_bound =
        !is_destructive_action(input.action) || input.decision != "pass" || input.tombstone_ref.is_some();
    Ok(crate::preserves_rail::record("retention-gc-apply-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RETENTION_GC_APPLY_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("mode", vec![crate::preserves_rail::string("apply")]),
        crate::preserves_rail::record("subsystem", vec![crate::preserves_rail::string(input.subsystem)]),
        crate::preserves_rail::record("action", vec![crate::preserves_rail::string(input.action)]),
        object_value(input.object_ref, input.object_kind),
        crate::preserves_rail::record("class", vec![crate::preserves_rail::string(input.retention_class)]),
        crate::preserves_rail::record("requester", vec![optional_ref_value(input.requester_ref)]),
        crate::preserves_rail::record("plan", vec![crate::preserves_rail::string(input.plan_ref)]),
        crate::preserves_rail::record("recomputed-plan", vec![crate::preserves_rail::string(
            input.recomputed_plan_ref,
        )]),
        crate::preserves_rail::record("retention-receipt", vec![optional_ref_value(input.retention_receipt_ref)]),
        crate::preserves_rail::record("tombstone", vec![optional_ref_value(input.tombstone_ref)]),
        crate::preserves_rail::record("admission", vec![strings_sequence(input.admission_refs)]),
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            ("plan-ref-bound", "pass"),
            ("plan-recomputed-before-mutation", "pass"),
            ("plan-unchanged", pass_or_deny(is_plan_unchanged)),
            ("plan-decision-pass", pass_or_deny(is_plan_passed)),
            ("normal-admission-run", "pass"),
            (
                "retention-receipt-bound",
                pass_or_deny(input.decision != "pass" || input.retention_receipt_ref.is_some()),
            ),
            ("tombstone-bound", pass_or_deny(is_tombstone_bound)),
            (
                "deny-before-mutation",
                pass_or_deny(input.decision == "pass" || input.retention_receipt_ref.is_none()),
            ),
            ("plan-is-not-authority", "pass"),
            ("remote-clearance-import-still-required", "pass"),
        ]),
    ]))
}

pub fn parse_gc_apply(value: &IoValue) -> Result<GcApply> {
    let fields = value
        .collect_simple_record("retention-gc-apply-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-gc-apply-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::RETENTION_GC_APPLY_SCHEMA, "retention GC apply schema")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    let mode = record_string(&fields[2], "mode")?;
    if mode != "apply" {
        return Err(MoltenError::invalid_harness("retention GC apply mode must be apply"));
    }
    let subsystem = record_string(&fields[3], "subsystem")?;
    validate_name(&subsystem, "retention GC apply subsystem")?;
    let action = record_string(&fields[4], "action")?;
    validate_action(&action)?;
    let (object_ref, object_kind) = parse_object_value(&fields[5])?;
    let retention_class = record_string(&fields[6], "class")?;
    validate_class(&retention_class)?;
    let requester_ref = record_optional_ref(&fields[7], "requester")?;
    let plan_ref = record_ref(&fields[8], "plan")?;
    let recomputed_plan_ref = record_ref(&fields[9], "recomputed-plan")?;
    let retention_receipt_ref = record_optional_ref(&fields[10], "retention-receipt")?;
    let tombstone_ref = record_optional_ref(&fields[11], "tombstone")?;
    let admission_refs = record_ref_sequence(&fields[12], "admission")?;
    let diagnostics = record_string_sequence(&fields[13], "diagnostics")?;
    let checks = parse_checks(&fields[14])?;
    require_check(&checks, "plan-ref-bound", "retention GC apply")?;
    require_check(&checks, "plan-recomputed-before-mutation", "retention GC apply")?;
    require_check(&checks, "normal-admission-run", "retention GC apply")?;
    require_check(&checks, "plan-is-not-authority", "retention GC apply")?;
    require_check(&checks, "remote-clearance-import-still-required", "retention GC apply")?;
    Ok(GcApply {
        apply_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        subsystem,
        action,
        object_ref,
        object_kind,
        retention_class,
        requester_ref,
        plan_ref,
        recomputed_plan_ref,
        retention_receipt_ref,
        tombstone_ref,
        admission_refs,
        diagnostics,
        value: value.clone(),
    })
}

pub fn read_gc_apply(root: &Path, apply_ref: &str) -> Result<GcApply> {
    let root = open_capability_retention_root(root)?;
    read_gc_apply_with_root(&root, apply_ref)
}

pub fn read_gc_apply_with_root(root: &CapabilityRetentionRoot, apply_ref: &str) -> Result<GcApply> {
    require_ref(apply_ref, "retention GC apply ref")?;
    let value = read_store_value_with_root(root, &capability_ref_path(GC_APPLY_DIR, apply_ref)?)?;
    let apply = parse_gc_apply(&value)?;
    if apply.apply_ref != apply_ref {
        return Err(MoltenError::invalid_harness("stored retention GC apply ref mismatch"));
    }
    Ok(apply)
}

pub fn read_gc_execution_gate(root: &Path, execution_ref: &str) -> Result<GcExecutionGate> {
    let root = open_capability_retention_root(root)?;
    read_gc_execution_gate_with_root(&root, execution_ref)
}

pub fn read_gc_execution_gate_with_root(
    root: &CapabilityRetentionRoot,
    execution_ref: &str,
) -> Result<GcExecutionGate> {
    require_ref(execution_ref, "retention GC execution ref")?;
    let value = read_store_value_with_root(root, &capability_ref_path(GC_EXECUTE_DIR, execution_ref)?)?;
    let gate = parse_gc_execution_gate(&value)?;
    if gate.execution_ref != execution_ref {
        return Err(MoltenError::invalid_harness("stored retention GC execution ref mismatch"));
    }
    Ok(gate)
}

#[derive(Debug, Default)]
struct ExecutionGateParts {
    plan_ref: Option<String>,
    recomputed_plan_ref: Option<String>,
    retention_receipt_ref: Option<String>,
    tombstone_ref: Option<String>,
    diagnostics: Vec<String>,
}
