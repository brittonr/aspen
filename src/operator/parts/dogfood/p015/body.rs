
fn gc_admissions(seed: GcSeed<'_>) -> Result<GcAdmissions> {
    let policy = store_gc_fixture(seed, crate::retention::ADMISSION_KIND_POLICY, "policy", &[])?;
    let authority = store_gc_fixture(seed, crate::retention::ADMISSION_KIND_AUTHORITY, "authority", &[])?;
    let support = store_gc_fixture(seed, crate::retention::ADMISSION_KIND_SUPPORTING_EVIDENCE, "support", &[])?;
    let index = store_gc_fixture(seed, crate::retention::ADMISSION_KIND_REFERENCE_INDEX, "index", &[])?;
    let remote_gc = store_gc_fixture(seed, crate::retention::ADMISSION_KIND_REMOTE_GC, "remote-gc", seed.remote_refs)?;
    let clearance_evidence = vec![support.admission_ref.clone()];
    let clearance =
        crate::retention::store_remote_gc_clearance(seed.root, &crate::retention::RemoteGcClearanceInput {
            decision: "pass",
            requester_ref: seed.requester_ref,
            peer_ref: seed.peer_ref,
            object_ref: seed.object_ref,
            object_kind: seed.object_kind,
            retention_class: seed.class,
            action: seed.action,
            remote_ref: seed.remote_ref,
            policy_ref: &policy.admission_ref,
            authority_ref: &authority.admission_ref,
            evidence_refs: &clearance_evidence,
            retained_refs: &[],
            is_current: true,
            revoked_refs: &[],
            diagnostics: &[],
        })?;
    let evidence = crate::retention::DestructiveEvidence {
        requester_ref: Some(seed.requester_ref.to_string()),
        policy_refs: vec![policy.admission_ref.clone()],
        authority_refs: vec![authority.admission_ref.clone()],
        evidence_refs: vec![support.admission_ref.clone()],
        retained_refs: Vec::new(),
        remote_peer_refs: vec![seed.peer_ref.to_string()],
        remote_refs: seed.remote_refs.to_vec(),
        reference_index_refs: vec![index.admission_ref.clone()],
        remote_gc_refs: vec![remote_gc.admission_ref.clone()],
        remote_clearance_refs: vec![clearance.clearance_ref.clone()],
        is_reference_index_complete: true,
    };
    Ok(GcAdmissions {
        policy,
        authority,
        support,
        index,
        remote_gc,
        clearance,
        evidence,
    })
}

fn gc_flow(
    input: GcWorkflowInput<'_>,
    seed: GcSeed<'_>,
    evidence: &crate::retention::DestructiveEvidence,
) -> Result<GcFlow> {
    let plan = crate::retention::store_gc_plan(crate::retention::GcPlanInput {
        root: input.root,
        subsystem: "ledger-gc",
        object_ref: seed.object_ref,
        object_kind: seed.object_kind,
        retention_class: seed.class,
        action: seed.action,
        evidence,
    })?;
    let apply = crate::retention::apply_gc_plan(crate::retention::GcApplyFromPlanInput {
        root: input.root,
        plan_ref: &plan.plan_ref,
    })?;
    let execution = crate::retention::store_gc_execution_gate(crate::retention::GcExecutionGateInput {
        root: input.root,
        subsystem: "ledger-gc",
        action: seed.action,
        object_ref: seed.object_ref,
        object_kind: seed.object_kind,
        retention_class: seed.class,
        apply_ref: Some(&apply.apply_ref),
    })?;
    let audit = crate::retention::audit_gc_execution(crate::retention::GcAuditInput {
        root: input.root,
        execution_ref: &execution.execution_ref,
    })?;
    let explain = crate::retention::explain_candidate(crate::retention::CandidateExplainInput {
        root: input.root,
        object_ref: seed.object_ref,
        object_kind: Some(seed.object_kind),
        retention_class: Some(seed.class),
        action: Some(seed.action),
        subsystem: Some("ledger-gc"),
    })?;
    let bundle = crate::retention::export_candidate_bundle(crate::retention::CandidateBundleExportInput {
        root: input.root,
        explain_value: &explain.value,
        out: input.bundle_dir,
        profile: crate::retention::CandidateBundleExportProfile::Public,
    })?;
    let profile_value = crate::preserves_rail::parse_text(
        &std::fs::read_to_string(input.bundle_dir.join("bundle-profile.preserves")).map_err(MoltenError::from)?,
    )?;
    let profile = crate::retention::parse_candidate_bundle_profile(&profile_value)?;
    let verify = crate::retention::verify_candidate_bundle(crate::retention::CandidateBundleVerifyInput {
        bundle_dir: input.bundle_dir,
    })?;
    Ok(GcFlow {
        plan,
        apply,
        execution,
        audit,
        explain,
        bundle,
        profile,
        verify,
    })
}

fn import_gc_values(root: &Path, admissions: &GcAdmissions, flow: &GcFlow) -> Result<Vec<String>> {
    let mut refs = Vec::new();
    for value in [
        &admissions.policy.value,
        &admissions.authority.value,
        &admissions.support.value,
        &admissions.index.value,
        &admissions.remote_gc.value,
        &admissions.clearance.value,
        &flow.plan.value,
        &flow.apply.value,
        &flow.execution.value,
        &flow.audit.value,
        &flow.explain.value,
        &flow.bundle.value,
        &flow.profile.value,
        &flow.verify.value,
    ] {
        let imported = crate::ledger::import_artifact(root, value)?;
        refs.push_limited_value(
            crate::preserves_rail::canonical_hash(&imported.receipt_value)?,
            MAX_OPERATOR_REFS,
            "retention dogfood ledger imports",
        )?;
    }
    Ok(refs)
}

fn gc_catalog(
    registry_root: &Path,
    ledger_root: &Path,
    object_ref: &str,
) -> Result<(crate::catalog_mcp::Call, String)> {
    let mcp_request = crate::catalog_mcp::mcp_request_value("search_retention_gc", vec![
        crate::preserves_rail::record("stage", vec![crate::preserves_rail::string("audit")]),
        crate::preserves_rail::record("object-ref", vec![crate::preserves_rail::string(object_ref)]),
        crate::preserves_rail::record("subsystem", vec![crate::preserves_rail::string("ledger-gc")]),
    ])?;
    let mcp_call = crate::catalog_mcp::call(registry_root, Some(ledger_root), &mcp_request)?;
    let catalog_receipt_ref = crate::preserves_rail::canonical_hash(&mcp_call.receipt_value)?;
    Ok((mcp_call, catalog_receipt_ref))
}

fn gc_bundle_diagnostics(flow: &GcFlow) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    append_dogfood_diagnostics(&mut diagnostics, "retention-bundle", &flow.bundle.diagnostics)?;
    append_dogfood_diagnostics(&mut diagnostics, "retention-bundle-profile", &flow.profile.diagnostics)?;
    append_dogfood_diagnostics(&mut diagnostics, "retention-bundle-verify", &flow.verify.diagnostics)?;
    Ok(diagnostics)
}

fn gc_artifact_refs(
    admissions: &GcAdmissions,
    flow: &GcFlow,
    response_ref: &str,
    ledger_import_refs: Vec<String>,
) -> Vec<String> {
    let mut refs = vec![
        admissions.policy.admission_ref.clone(),
        admissions.authority.admission_ref.clone(),
        admissions.support.admission_ref.clone(),
        admissions.index.admission_ref.clone(),
        admissions.remote_gc.admission_ref.clone(),
        admissions.clearance.clearance_ref.clone(),
        flow.plan.plan_ref.clone(),
        flow.apply.apply_ref.clone(),
        flow.execution.execution_ref.clone(),
        flow.audit.audit_ref.clone(),
        flow.explain.explain_ref.clone(),
        flow.bundle.bundle_ref.clone(),
        flow.profile.profile_ref.clone(),
        flow.verify.verify_ref.clone(),
        response_ref.to_string(),
    ];
    refs.extend(ledger_import_refs);
    refs
}

fn finish_gc_run(input: GcFinishInput) -> GcRun {
    let GcFinishInput {
        object_ref,
        flow,
        mcp_call,
        catalog_receipt_ref,
        artifact_refs,
        bundle_diagnostics,
    } = input;
    let GcFlow {
        plan,
        apply,
        execution,
        audit,
        explain,
        bundle,
        profile,
        verify,
    } = flow;
    GcRun {
        object_ref,
        plan_ref: plan.plan_ref,
        plan_decision: plan.decision,
        plan_diagnostics: plan.diagnostics,
        apply_ref: apply.apply_ref,
        apply_decision: apply.decision,
        apply_diagnostics: apply.diagnostics,
        execution_ref: execution.execution_ref,
        execution_decision: execution.decision,
        execution_diagnostics: execution.diagnostics,
        audit_ref: audit.audit_ref,
        audit_decision: audit.decision,
        audit_diagnostics: audit.diagnostics,
        explain_ref: explain.explain_ref,
        bundle_ref: bundle.bundle_ref,
        bundle_profile_ref: profile.profile_ref,
        bundle_verify_ref: verify.verify_ref,
        bundle_verify_decision: verify.decision,
        bundle_diagnostics,
        catalog_request_ref: mcp_call.request.request_ref,
        catalog_receipt_ref,
        catalog_response_ref: mcp_call.response_ref,
        catalog_decision: mcp_call.decision,
        artifact_refs,
    }
}

fn store_retention_admission_fixture(
    input: RetentionAdmissionFixtureInput<'_>,
) -> Result<crate::retention::EvidenceAdmission> {
    let bound_refs = vec![dogfood_ref(&format!("retention-{}-bound", input.label))?];
    crate::retention::store_evidence_admission(input.root, &crate::retention::EvidenceAdmissionInput {
        kind: input.kind,
        decision: "pass",
        requester_ref: input.requester_ref,
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retention_class: input.retention_class,
        action: input.action,
        bound_refs: &bound_refs,
        retained_refs: &[],
        remote_refs: input.remote_refs,
        is_reference_index_complete: true,
        is_current: true,
        revoked_refs: &[],
        diagnostics: &[],
    })
}

fn append_dogfood_diagnostics(sink: &mut impl PushLimited<String>, label: &str, diagnostics: &[String]) -> Result<()> {
    for diagnostic in diagnostics {
        sink.push_limited_value(
            format!("{label}:{diagnostic}"),
            MAX_OPERATOR_DIAGNOSTICS,
            "operator dogfood diagnostics",
        )?;
    }
    Ok(())
}
