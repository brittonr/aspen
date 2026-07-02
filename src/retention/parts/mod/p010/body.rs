
fn push_access_gates(gates: &mut impl VecSink<PlanGate>, input: &GateInputs<'_>) -> Result<()> {
    push_bounded(
        gates,
        requester_gate(input.input.evidence.requester_ref.as_deref())?,
        MAX_RETENTION_REFS,
        "retention GC plan gates",
    )?;
    push_bounded(
        gates,
        plan_gate(PlanGateBuildInput {
            name: "policy",
            is_required: true,
            required_refs: &input.input.evidence.policy_refs,
            admitted_refs: &input.policy.admitted_refs,
            diagnostics: diagnostics_with_missing(MissingDiagnosticInput {
                diagnostics: &input.policy.diagnostics,
                is_missing: input.input.evidence.policy_refs.is_empty(),
                missing_diagnostic: "retention-policy-missing",
            })?,
        })?,
        MAX_RETENTION_REFS,
        "retention GC plan gates",
    )?;
    push_bounded(
        gates,
        plan_gate(PlanGateBuildInput {
            name: "authority",
            is_required: is_destructive_action(input.input.action),
            required_refs: &input.input.evidence.authority_refs,
            admitted_refs: &input.authority.admitted_refs,
            diagnostics: diagnostics_with_missing(MissingDiagnosticInput {
                diagnostics: &input.authority.diagnostics,
                is_missing: is_destructive_action(input.input.action) && input.input.evidence.authority_refs.is_empty(),
                missing_diagnostic: "delete-authority-missing",
            })?,
        })?,
        MAX_RETENTION_REFS,
        "retention GC plan gates",
    )?;
    push_bounded(
        gates,
        plan_gate(PlanGateBuildInput {
            name: "supporting-evidence",
            is_required: is_destructive_action(input.input.action),
            required_refs: &input.input.evidence.evidence_refs,
            admitted_refs: &input.supporting.admitted_refs,
            diagnostics: diagnostics_with_missing(MissingDiagnosticInput {
                diagnostics: &input.supporting.diagnostics,
                is_missing: is_destructive_action(input.input.action) && input.input.evidence.evidence_refs.is_empty(),
                missing_diagnostic: "retention-evidence-missing",
            })?,
        })?,
        MAX_RETENTION_REFS,
        "retention GC plan gates",
    )?;
    Ok(())
}

fn push_index_gates(gates: &mut impl VecSink<PlanGate>, input: &GateInputs<'_>, index: &ReferenceIndex) -> Result<()> {
    push_bounded(
        gates,
        plan_gate(PlanGateBuildInput {
            name: "reference-index",
            is_required: input.input.evidence.is_reference_index_complete,
            required_refs: &input.input.evidence.reference_index_refs,
            admitted_refs: &input.reference_index.admitted_refs,
            diagnostics: reference_index_gate_diagnostics(input)?,
        })?,
        MAX_RETENTION_REFS,
        "retention GC plan gates",
    )?;
    push_bounded(
        gates,
        local_gate(LocalGateInput {
            input: input.input,
            index,
            has_delete_authority: input.has_delete_authority,
            has_remote_gc_clearance: input.has_remote_gc_clearance,
        })?,
        MAX_RETENTION_REFS,
        "retention GC plan gates",
    )?;
    Ok(())
}

fn push_external_gates(gates: &mut impl VecSink<PlanGate>, input: &GateInputs<'_>) -> Result<()> {
    push_bounded(
        gates,
        plan_gate(PlanGateBuildInput {
            name: "remote-gc",
            is_required: is_destructive_action(input.input.action) && !input.input.evidence.remote_refs.is_empty(),
            required_refs: &input.input.evidence.remote_gc_refs,
            admitted_refs: &input.remote_gc.admitted_refs,
            diagnostics: diagnostics_with_missing(MissingDiagnosticInput {
                diagnostics: &input.remote_gc.diagnostics,
                is_missing: is_destructive_action(input.input.action)
                    && !input.input.evidence.remote_refs.is_empty()
                    && input.input.evidence.remote_gc_refs.is_empty(),
                missing_diagnostic: "remote-gc-evidence-missing",
            })?,
        })?,
        MAX_RETENTION_REFS,
        "retention GC plan gates",
    )?;
    push_bounded(
        gates,
        plan_gate(PlanGateBuildInput {
            name: "remote-clearance",
            is_required: is_destructive_action(input.input.action)
                && (!input.input.evidence.remote_refs.is_empty() || !input.input.evidence.remote_peer_refs.is_empty()),
            required_refs: &input.input.evidence.remote_clearance_refs,
            admitted_refs: &input.remote_clearance.admitted_refs,
            diagnostics: remote_clearance_gate_diagnostics(RemoteClearanceGateInput {
                diagnostics: &input.remote_clearance.diagnostics,
                has_missing_refs: is_destructive_action(input.input.action)
                    && !input.input.evidence.remote_refs.is_empty()
                    && input.input.evidence.remote_clearance_refs.is_empty(),
                has_missing_peers: is_destructive_action(input.input.action)
                    && !input.input.evidence.remote_peer_refs.is_empty()
                    && input.input.evidence.remote_clearance_refs.is_empty(),
            })?,
        })?,
        MAX_RETENTION_REFS,
        "retention GC plan gates",
    )?;
    let empty_refs = Vec::new();
    push_bounded(
        gates,
        plan_gate(PlanGateBuildInput {
            name: "evidence-only-boundary",
            is_required: false,
            required_refs: &empty_refs,
            admitted_refs: &empty_refs,
            diagnostics: Vec::new(),
        })?,
        MAX_RETENTION_REFS,
        "retention GC plan gates",
    )?;
    Ok(())
}

fn requester_gate(requester_ref: Option<&str>) -> Result<PlanGate> {
    let required_refs = requester_ref.map(|reference| vec![reference.to_string()]).unwrap_or_default();
    let diagnostics = if requester_ref.is_some() {
        Vec::new()
    } else {
        vec!["retention-requester-missing".to_string()]
    };
    plan_gate(PlanGateBuildInput {
        name: "requester",
        is_required: true,
        required_refs: &required_refs,
        admitted_refs: &required_refs,
        diagnostics,
    })
}

fn local_gate(input: LocalGateInput<'_>) -> Result<PlanGate> {
    let requester_ref = match input.input.evidence.requester_ref.as_ref() {
        Some(reference) => reference.clone(),
        None => synthetic_ref("retention-gc-plan-missing-requester")?,
    };
    let local_input = EvaluationInput {
        root: input.input.root,
        object_ref: input.input.object_ref,
        object_kind: input.input.object_kind,
        retention_class: input.input.retention_class,
        action: input.input.action,
        requester_ref: &requester_ref,
        is_reference_index_complete: input.input.evidence.is_reference_index_complete,
        retained_refs: &input.input.evidence.retained_refs,
        remote_refs: &input.input.evidence.remote_refs,
        policy_refs: &input.input.evidence.policy_refs,
        evidence_refs: &input.input.evidence.evidence_refs,
        has_delete_authority: input.has_delete_authority,
        has_remote_gc_clearance: input.has_remote_gc_clearance,
    };
    let diagnostics = evaluation_diagnostics(&local_input, input.index)?;
    let required_refs = vec![input.index.index_ref.clone()];
    let admitted_refs = if diagnostics.is_empty() {
        required_refs.clone()
    } else {
        Vec::new()
    };
    plan_gate(PlanGateBuildInput {
        name: "local-retention",
        is_required: true,
        required_refs: &required_refs,
        admitted_refs: &admitted_refs,
        diagnostics,
    })
}

fn diagnostics_with_missing(input: MissingDiagnosticInput<'_>) -> Result<Vec<String>> {
    let mut diagnostics = input.diagnostics.to_vec();
    if input.is_missing {
        push_bounded(
            &mut diagnostics,
            input.missing_diagnostic.to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC plan gate diagnostics",
        )?;
    }
    diagnostics.sort();
    diagnostics.dedup();
    Ok(diagnostics)
}

fn reference_index_gate_diagnostics(input: &GateInputs<'_>) -> Result<Vec<String>> {
    let mut diagnostics = input.reference_index.diagnostics.clone();
    if !input.input.evidence.is_reference_index_complete {
        push_bounded(
            &mut diagnostics,
            "incomplete-reference-proof".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC plan reference-index diagnostics",
        )?;
    }
    if input.input.evidence.is_reference_index_complete && input.input.evidence.reference_index_refs.is_empty() {
        push_bounded(
            &mut diagnostics,
            "reference-index-evidence-missing".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC plan reference-index diagnostics",
        )?;
    }
    diagnostics.sort();
    diagnostics.dedup();
    Ok(diagnostics)
}

fn remote_clearance_gate_diagnostics(input: RemoteClearanceGateInput<'_>) -> Result<Vec<String>> {
    let mut diagnostics = input.diagnostics.to_vec();
    if input.has_missing_refs || input.has_missing_peers {
        push_bounded(
            &mut diagnostics,
            "remote-clearance-evidence-missing".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC plan remote-clearance diagnostics",
        )?;
    }
    diagnostics.sort();
    diagnostics.dedup();
    Ok(diagnostics)
}

fn plan_gate(input: PlanGateBuildInput<'_>) -> Result<PlanGate> {
    validate_name(input.name, "retention GC plan gate name")?;
    validate_refs(input.required_refs, "retention GC plan required ref")?;
    validate_refs(input.admitted_refs, "retention GC plan admitted ref")?;
    let is_pass = input.diagnostics.is_empty() && (!input.is_required || !input.admitted_refs.is_empty());
    Ok(PlanGate {
        name: input.name.to_string(),
        decision: pass_or_deny(is_pass).to_string(),
        required_refs: input.required_refs.to_vec(),
        admitted_refs: input.admitted_refs.to_vec(),
        diagnostics: input.diagnostics,
    })
}

fn plan_gate_value(input: &PlanGate) -> Result<IoValue> {
    validate_name(&input.name, "retention GC plan gate name")?;
    validate_decision(&input.decision)?;
    validate_refs(&input.required_refs, "retention GC plan gate required ref")?;
    validate_refs(&input.admitted_refs, "retention GC plan gate admitted ref")?;
    Ok(crate::preserves_rail::record("gate", vec![
        crate::preserves_rail::record("name", vec![crate::preserves_rail::string(&input.name)]),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(&input.decision)]),
        crate::preserves_rail::record("required", vec![strings_sequence(&input.required_refs)]),
        crate::preserves_rail::record("admitted", vec![strings_sequence(&input.admitted_refs)]),
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(&input.diagnostics)]),
    ]))
}

fn parse_plan_gates(value: &Value<IoValue>) -> Result<Vec<PlanGate>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = value
        .collect_simple_record("gates", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected retention GC plan gates"))?;
    let entries = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("expected retention GC plan gate sequence"))?;
    let mut gates = Vec::with_capacity(entries.len());
    for entry in entries.iter() {
        let gate_value = crate::preserves_rail::value_to_iovalue(entry);
        push_bounded(&mut gates, parse_plan_gate(&gate_value)?, MAX_RETENTION_REFS, "retention GC plan gates")?;
    }
    Ok(gates)
}
