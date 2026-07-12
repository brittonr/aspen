
pub fn validate_destructive_evidence(input: &DestructiveEvidence) -> Result<()> {
    if let Some(requester_ref) = input.requester_ref.as_ref() {
        require_ref(requester_ref, "retention requester ref")?;
    }
    validate_refs(&input.policy_refs, "retention policy ref")?;
    validate_refs(&input.authority_refs, "retention authority ref")?;
    validate_refs(&input.evidence_refs, "retention evidence ref")?;
    validate_refs(&input.retained_refs, "retention retained ref")?;
    validate_refs(&input.remote_peer_refs, "retention remote peer ref")?;
    validate_refs(&input.remote_refs, "retention remote ref")?;
    validate_refs(&input.reference_index_refs, "retention reference-index ref")?;
    validate_refs(&input.remote_gc_refs, "retention remote-gc ref")?;
    validate_refs(&input.remote_clearance_refs, "retention remote clearance ref")
}

fn validate_gc_plan_input<Root: ?Sized>(input: &GcPlanInput<'_, Root>) -> Result<()> {
    validate_name(input.subsystem, "retention GC plan subsystem")?;
    require_ref(input.object_ref, "retention GC plan object ref")?;
    validate_name(input.object_kind, "retention GC plan object kind")?;
    validate_class(input.retention_class)?;
    validate_action(input.action)?;
    validate_destructive_evidence(input.evidence)
}

struct MissingNote<'a> {
    emit: bool,
    message: &'a str,
}

fn push_missing_notes<S>(diagnostics: &mut S, notes: &[MissingNote<'_>]) -> Result<()>
where S: VecSink<String> {
    for note in notes {
        if note.emit {
            push_bounded(
                diagnostics,
                note.message.to_string(),
                MAX_RETENTION_DIAGNOSTICS,
                "retention destructive evidence diagnostics",
            )?;
        }
    }
    Ok(())
}

pub fn destructive_evidence_diagnostics(input: &DestructiveEvidence, action: &str) -> Result<Vec<String>> {
    validate_destructive_evidence(input)?;
    validate_action(action)?;
    let is_destructive = is_destructive_action(action);
    let mut diagnostics = Vec::new();
    let notes = [
        MissingNote {
            emit: input.requester_ref.is_none(),
            message: "retention-requester-missing",
        },
        MissingNote {
            emit: input.policy_refs.is_empty(),
            message: "retention-policy-missing",
        },
        MissingNote {
            emit: is_destructive && input.authority_refs.is_empty(),
            message: "delete-authority-missing",
        },
        MissingNote {
            emit: is_destructive && input.evidence_refs.is_empty(),
            message: "retention-evidence-missing",
        },
        MissingNote {
            emit: !input.is_reference_index_complete,
            message: "incomplete-reference-proof",
        },
        MissingNote {
            emit: is_destructive && input.is_reference_index_complete && input.reference_index_refs.is_empty(),
            message: "reference-index-evidence-missing",
        },
        MissingNote {
            emit: !input.retained_refs.is_empty(),
            message: "retained-dependencies-present",
        },
        MissingNote {
            emit: is_destructive && !input.remote_refs.is_empty() && input.remote_gc_refs.is_empty(),
            message: "remote-gc-evidence-missing",
        },
        MissingNote {
            emit: is_destructive
                && (!input.remote_refs.is_empty() || !input.remote_peer_refs.is_empty())
                && input.remote_clearance_refs.is_empty(),
            message: "remote-clearance-evidence-missing",
        },
    ];
    push_missing_notes(&mut diagnostics, &notes)?;
    Ok(diagnostics)
}

pub fn destructive_evidence_value(input: &DestructiveEvidence) -> Result<IoValue> {
    validate_destructive_evidence(input)?;
    let requester_value = input
        .requester_ref
        .as_deref()
        .map(crate::preserves_rail::string)
        .unwrap_or_else(|| crate::preserves_rail::record("none", Vec::new()));
    Ok(crate::preserves_rail::record("retention-evidence-summary-v1", vec![
        crate::preserves_rail::record("requester", vec![requester_value]),
        crate::preserves_rail::record("policy", vec![strings_sequence(&input.policy_refs)]),
        crate::preserves_rail::record("authority", vec![strings_sequence(&input.authority_refs)]),
        crate::preserves_rail::record("evidence", vec![strings_sequence(&input.evidence_refs)]),
        crate::preserves_rail::record("retained", vec![strings_sequence(&input.retained_refs)]),
        crate::preserves_rail::record("remote-peer", vec![strings_sequence(&input.remote_peer_refs)]),
        crate::preserves_rail::record("remote", vec![strings_sequence(&input.remote_refs)]),
        crate::preserves_rail::record("reference-index", vec![strings_sequence(&input.reference_index_refs)]),
        crate::preserves_rail::record("remote-gc", vec![strings_sequence(&input.remote_gc_refs)]),
        crate::preserves_rail::record("remote-clearance", vec![strings_sequence(&input.remote_clearance_refs)]),
        crate::preserves_rail::record("reference-index-complete", vec![crate::preserves_rail::string(pass_or_deny(
            input.is_reference_index_complete,
        ))]),
        checks_value(&[
            ("requester-bound", pass_or_deny(input.requester_ref.is_some())),
            ("policy-bound", pass_or_deny(!input.policy_refs.is_empty())),
            ("authority-bound", pass_or_deny(!input.authority_refs.is_empty())),
            ("evidence-bound", pass_or_deny(!input.evidence_refs.is_empty())),
            ("reference-index-bound", pass_or_deny(!input.reference_index_refs.is_empty())),
            ("remote-gc-bound", pass_or_deny(input.remote_refs.is_empty() || !input.remote_gc_refs.is_empty())),
            (
                "remote-clearance-bound",
                pass_or_deny(
                    (input.remote_refs.is_empty() && input.remote_peer_refs.is_empty())
                        || !input.remote_clearance_refs.is_empty(),
                ),
            ),
        ]),
    ]))
}

pub fn store_gc_plan(input: GcPlanInput<'_>) -> Result<GcPlan> {
    let root = open_capability_retention_root(input.root)?;
    store_gc_plan_with_root(GcPlanInput {
        root: &root,
        subsystem: input.subsystem,
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retention_class: input.retention_class,
        action: input.action,
        evidence: input.evidence,
    })
}

pub fn store_gc_plan_with_root(input: GcPlanInput<'_, CapabilityRetentionRoot>) -> Result<GcPlan> {
    ensure_store_with_root(input.root)?;
    validate_gc_plan_input(&input)?;
    let index = reference_index_for_object_with_root(ReferenceIndexForObjectInput {
        root: input.root,
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retained_refs: input.evidence.retained_refs.as_slice(),
        remote_refs: input.evidence.remote_refs.as_slice(),
        is_complete: input.evidence.is_reference_index_complete,
    })?;
    let gate_inputs = gate_inputs(&input)?;
    let gates = retention_plan_gates(&gate_inputs, &index)?;
    let mut diagnostics = Vec::new();
    for gate in &gates {
        extend_bounded(
            &mut diagnostics,
            gate.diagnostics.iter().cloned(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC plan diagnostics",
        )?;
    }
    diagnostics.sort();
    diagnostics.dedup();
    let decision = if gates.iter().all(|gate| gate.decision == "pass") && diagnostics.is_empty() {
        "pass"
    } else {
        "deny"
    };
    let evidence_value = destructive_evidence_value(input.evidence)?;
    let value = gc_plan_value(&GcPlanValueInput {
        decision,
        subsystem: input.subsystem,
        action: input.action,
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retention_class: input.retention_class,
        requester_ref: input.evidence.requester_ref.as_deref(),
        index: &index,
        evidence_value: &evidence_value,
        gates: &gates,
        diagnostics: &diagnostics,
    })?;
    let plan = parse_gc_plan(&value)?;
    write_store_value_with_root(input.root, &capability_ref_path(GC_PLAN_DIR, &plan.plan_ref)?, &plan.value)?;
    Ok(plan)
}

pub fn gc_plan_value(input: &GcPlanValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_name(input.subsystem, "retention GC plan subsystem")?;
    validate_action(input.action)?;
    require_ref(input.object_ref, "retention GC plan object ref")?;
    validate_name(input.object_kind, "retention GC plan object kind")?;
    validate_class(input.retention_class)?;
    if let Some(requester_ref) = input.requester_ref {
        require_ref(requester_ref, "retention GC plan requester ref")?;
    }
    parse_destructive_evidence_summary(input.evidence_value)?;
    let gate_values = input.gates.iter().map(plan_gate_value).collect::<Result<Vec<_>>>()?;
    Ok(crate::preserves_rail::record("retention-gc-plan-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RETENTION_GC_PLAN_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("mode", vec![crate::preserves_rail::string("dry-run")]),
        crate::preserves_rail::record("subsystem", vec![crate::preserves_rail::string(input.subsystem)]),
        crate::preserves_rail::record("action", vec![crate::preserves_rail::string(input.action)]),
        object_value(input.object_ref, input.object_kind),
        crate::preserves_rail::record("class", vec![crate::preserves_rail::string(input.retention_class)]),
        crate::preserves_rail::record("requester", vec![optional_ref_value(input.requester_ref)]),
        crate::preserves_rail::record("index", vec![
            crate::preserves_rail::string(&input.index.index_ref),
            input.index.value.clone(),
        ]),
        crate::preserves_rail::record("retention-evidence", vec![input.evidence_value.clone()]),
        crate::preserves_rail::record("gates", vec![crate::preserves_rail::sequence(gate_values)]),
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            ("canonical-ref-binding", "pass"),
            ("dry-run-only", "pass"),
            ("no-retention-receipt-written", "pass"),
            ("no-tombstone-written", "pass"),
            ("plan-is-not-authority", "pass"),
            ("remote-clearance-import-still-required", "pass"),
        ]),
    ]))
}

pub fn parse_gc_plan(value: &IoValue) -> Result<GcPlan> {
    let fields = value
        .collect_simple_record("retention-gc-plan-v1", Some(13))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-gc-plan-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::RETENTION_GC_PLAN_SCHEMA, "retention GC plan schema")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    let mode = record_string(&fields[2], "mode")?;
    if mode != "dry-run" {
        return Err(MoltenError::invalid_harness("retention GC plan mode must be dry-run"));
    }
    let subsystem = record_string(&fields[3], "subsystem")?;
    validate_name(&subsystem, "retention GC plan subsystem")?;
    let action = record_string(&fields[4], "action")?;
    validate_action(&action)?;
    let (object_ref, object_kind) = parse_object_value(&fields[5])?;
    let retention_class = record_string(&fields[6], "class")?;
    validate_class(&retention_class)?;
    let requester_ref = record_optional_ref(&fields[7], "requester")?;
    let (index_ref, index) = parse_embedded_reference_index(&fields[8])?;
    if index.object_ref != object_ref || index.object_kind != object_kind {
        return Err(MoltenError::invalid_harness("retention GC plan index scope mismatch"));
    }
    let evidence_value = parse_embedded_destructive_evidence_summary(&fields[9])?;
    let evidence = parse_destructive_evidence_summary_to_evidence(&evidence_value)?;
    if requester_ref != evidence.requester_ref {
        return Err(MoltenError::invalid_harness("retention GC plan requester evidence mismatch"));
    }
    let gates = parse_plan_gates(&fields[10])?;
    let diagnostics = record_string_sequence(&fields[11], "diagnostics")?;
    let checks = parse_checks(&fields[12])?;
    require_check(&checks, "dry-run-only", "retention GC plan")?;
    require_check(&checks, "plan-is-not-authority", "retention GC plan")?;
    require_check(&checks, "remote-clearance-import-still-required", "retention GC plan")?;
    let evidence_ref = crate::preserves_rail::canonical_hash(&evidence_value)?;
    require_ref(&evidence_ref, "retention GC plan evidence summary ref")?;
    Ok(GcPlan {
        plan_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        subsystem,
        action,
        object_ref,
        object_kind,
        retention_class,
        requester_ref,
        index_ref,
        evidence,
        gates,
        diagnostics,
        value: value.clone(),
    })
}

pub fn read_gc_plan(root: &Path, plan_ref: &str) -> Result<GcPlan> {
    let root = open_capability_retention_root(root)?;
    read_gc_plan_with_root(&root, plan_ref)
}

pub fn read_gc_plan_with_root(root: &CapabilityRetentionRoot, plan_ref: &str) -> Result<GcPlan> {
    require_ref(plan_ref, "retention GC plan ref")?;
    let value = read_store_value_with_root(root, &capability_ref_path(GC_PLAN_DIR, plan_ref)?)?;
    let plan = parse_gc_plan(&value)?;
    if plan.plan_ref != plan_ref {
        return Err(MoltenError::invalid_harness("stored retention GC plan ref mismatch"));
    }
    Ok(plan)
}
