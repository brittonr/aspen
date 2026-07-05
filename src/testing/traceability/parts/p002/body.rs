pub fn parse_verification_run_receipt(value: &IoValue) -> Result<VerificationRunReceipt> {
    let fields = value
        .collect_simple_record("verification-run-receipt-v1", Some(VERIFICATION_RUN_RECEIPT_ARITY))
        .ok_or_else(|| MoltenError::invalid_harness("expected <verification-run-receipt-v1 ...>"))?;
    require_schema(&fields[0], VERIFICATION_RUN_RECEIPT_SCHEMA, "verification run receipt")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    let requirement_id = record_string(&fields[2], "requirement")?;
    validate_requirement_id(&requirement_id)?;
    let coverage_kind = record_string(&fields[3], "coverage-kind")?;
    validate_coverage_kind(&coverage_kind)?;
    let target = record_string(&fields[4], "target")?;
    validate_text("verification target", &target)?;
    let argv = record_string_sequence(&fields[5], "argv")?;
    ensure_count_at_most(argv.len(), MAX_RECEIPT_ARGS, "verification argv")?;
    let profile_ref = record_ref(&fields[6], "profile")?;
    let toolchain_refs = record_ref_sequence(&fields[7], "toolchains")?;
    let exit_status = record_i64(&fields[8], "exit-status")?;
    let stdout_ref = record_ref(&fields[9], "stdout")?;
    let stderr_ref = record_ref(&fields[10], "stderr")?;
    let artifact_refs = record_ref_sequence(&fields[11], "artifacts")?;
    let diagnostics = record_string_sequence(&fields[12], "diagnostics")?;
    validate_verification_receipt_decision(&decision, &coverage_kind, exit_status, &diagnostics)?;
    Ok(VerificationRunReceipt {
        decision,
        requirement_id,
        coverage_kind,
        target,
        argv,
        profile_ref,
        toolchain_refs,
        exit_status,
        stdout_ref,
        stderr_ref,
        artifact_refs,
        diagnostics,
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        value: value.clone(),
    })
}

pub fn coverage_from_verification_receipts(sources: &[ReceiptCoverageSource]) -> Result<Vec<CoverageInput>> {
    let mut coverage = OrderedMap::<String, CoverageInput>::new();
    for source in sources {
        let receipt = parse_verification_run_receipt(&source.value)?;
        let evidence = evidence_from_verification_receipt(&receipt, source.target_exists)?;
        let entry = coverage.entry(receipt.requirement_id.clone()).or_insert_with(|| CoverageInput {
            requirement_id: receipt.requirement_id.clone(),
            positive: Vec::new(),
            negative: Vec::new(),
            exemption: None,
        });
        match receipt.coverage_kind.as_str() {
            "positive" => entry.positive.push(evidence),
            "negative" => entry.negative.push(evidence),
            other => return Err(MoltenError::invalid_harness(format!("unsupported coverage kind {other}"))),
        }
    }
    Ok(coverage.into_values().collect())
}

pub fn merge_coverage_inputs(inputs: Vec<CoverageInput>) -> Result<Vec<CoverageInput>> {
    let mut coverage = OrderedMap::<String, CoverageInput>::new();
    for input in inputs {
        validate_text("coverage requirement", &input.requirement_id)?;
        let entry = coverage.entry(input.requirement_id.clone()).or_insert_with(|| CoverageInput {
            requirement_id: input.requirement_id.clone(),
            positive: Vec::new(),
            negative: Vec::new(),
            exemption: None,
        });
        entry.positive.extend(input.positive);
        entry.negative.extend(input.negative);
        if input.exemption.is_some() {
            entry.exemption = input.exemption;
        }
    }
    Ok(coverage.into_values().collect())
}

pub fn build_aggregate_proof_manifest(input: &AggregateProofInput) -> Result<AggregateProofManifest> {
    validate_text("aggregate proof manifest id", &input.manifest_id)?;
    validate_ref(&input.subject_ref, "aggregate proof subject")?;
    ensure_count_at_most(input.obligations.len(), MAX_PROOF_OBLIGATIONS, "proof obligations")?;
    let mut diagnostics = aggregate_proof_diagnostics(input)?;
    diagnostics.sort();
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" }.to_string();
    let mut obligations = input.obligations.clone();
    obligations.sort_by(|left, right| left.id.cmp(&right.id));
    let value = aggregate_proof_value(input, &obligations, &decision, &diagnostics)?;
    let manifest_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(AggregateProofManifest {
        decision,
        manifest_id: input.manifest_id.clone(),
        subject_ref: input.subject_ref.clone(),
        obligations,
        diagnostics,
        manifest_ref,
        value,
    })
}

pub fn coverage_from_aggregate_proof(
    manifest: &AggregateProofManifest,
    target_exists: bool,
) -> Result<Vec<CoverageInput>> {
    let mut coverage = OrderedMap::<String, CoverageInput>::new();
    for obligation in &manifest.obligations {
        let Some(kind) = obligation.coverage_kind.as_deref() else {
            continue;
        };
        validate_coverage_kind(kind)?;
        for requirement_id in &obligation.requirement_ids {
            let entry = coverage.entry(requirement_id.clone()).or_insert_with(|| CoverageInput {
                requirement_id: requirement_id.clone(),
                positive: Vec::new(),
                negative: Vec::new(),
                exemption: None,
            });
            let evidence = VerificationEvidence {
                target: format!("aggregate-proof:{}", manifest.manifest_id),
                command: "molten test traceability scan --receipt aggregate-proof".to_string(),
                artifact_ref: manifest.manifest_ref.clone(),
                artifact_refs: obligation.receipt_refs.clone(),
                target_exists,
                artifact_present: crate::preserves_rail::validate_content_ref(&manifest.manifest_ref).is_ok(),
                source: "aggregate-proof".to_string(),
                receipt_ref: Some(manifest.manifest_ref.clone()),
                expected_decision: obligation_expected_decision(&obligation.class)?.to_string(),
            };
            match kind {
                "positive" => entry.positive.push(evidence),
                "negative" => entry.negative.push(evidence),
                other => return Err(MoltenError::invalid_harness(format!("unsupported aggregate proof kind {other}"))),
            }
        }
    }
    Ok(coverage.into_values().collect())
}

pub fn build_layered_proof_manifest(input: &LayeredProofInput) -> Result<LayeredProofManifest> {
    validate_ref(&input.subject_ref, "layered proof subject")?;
    ensure_count_at_most(input.layers.len(), MAX_PROOF_LAYERS, "proof layers")?;
    let mut diagnostics = layered_proof_diagnostics(input)?;
    diagnostics.sort();
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" }.to_string();
    let mut layers = input.layers.clone();
    layers.sort_by(|left, right| left.id.cmp(&right.id));
    let value = layered_proof_value(&input.subject_ref, &layers, &decision, &diagnostics)?;
    let manifest_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(LayeredProofManifest {
        decision,
        subject_ref: input.subject_ref.clone(),
        layers,
        diagnostics,
        manifest_ref,
        value,
    })
}

pub fn build_deny_path_matrix(input: &DenyPathMatrixInput) -> Result<DenyPathMatrix> {
    validate_text("deny path gate", &input.gate)?;
    validate_ref(&input.subject_ref, "deny path subject")?;
    ensure_count_at_most(input.cases.len(), MAX_COVERAGE_ITEMS, "deny path cases")?;
    let mut diagnostics = deny_path_diagnostics(input)?;
    diagnostics.sort();
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" }.to_string();
    let value = deny_path_matrix_value(input, &decision, &diagnostics)?;
    let matrix_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(DenyPathMatrix {
        decision,
        gate: input.gate.clone(),
        subject_ref: input.subject_ref.clone(),
        diagnostics,
        matrix_ref,
        value,
    })
}

pub fn build_proof_readback(manifest: &TraceabilityManifest) -> Result<ProofReadback> {
    let mut entries = Vec::with_capacity(manifest.entries.len());
    for entry in &manifest.entries {
        let positive_receipt_refs = receipt_refs(&entry.positive);
        let negative_receipt_refs = receipt_refs(&entry.negative);
        let artifact_refs = artifact_refs_for_entry(entry)?;
        let mut caveats = vec![
            "readback is non-normative".to_string(),
            "canonical receipts control pass or deny".to_string(),
        ];
        if entry
            .positive
            .iter()
            .chain(entry.negative.iter())
            .any(|evidence| evidence.source == "compatibility")
        {
            caveats.push("compatibility-only coverage must not be treated as receipt-backed proof".to_string());
        }
        entries.push(ProofReadbackEntry {
            requirement_id: entry.requirement_id.clone(),
            status: entry.status.clone(),
            positive_receipt_refs,
            negative_receipt_refs,
            artifact_refs,
            diagnostics: entry.diagnostics.clone(),
            caveats,
        });
    }
    entries.sort_by(|left, right| left.requirement_id.cmp(&right.requirement_id));
    Ok(ProofReadback {
        decision: manifest.decision.clone(),
        entries,
        caveats: vec![
            "summary is a rendered view over canonical traceability and proof receipts".to_string(),
            "readbacks do not grant authority, policy, provenance, resource, transport, source-gate, retention, or destructive-operation trust".to_string(),
        ],
    })
}

pub fn render_proof_readback(readback: &ProofReadback) -> Result<String> {
    let mut lines = vec![format!("proof-readback decision={}", readback.decision)];
    for caveat in &readback.caveats {
        validate_text("readback caveat", caveat)?;
        lines.push(format!("caveat: {caveat}"));
    }
    for entry in &readback.entries {
        validate_requirement_id(&entry.requirement_id)?;
        lines.push(format!("requirement {} status={}", entry.requirement_id, entry.status));
        lines.push(format!("  positive-receipts: {}", display_group(&entry.positive_receipt_refs)));
        lines.push(format!("  negative-receipts: {}", display_group(&entry.negative_receipt_refs)));
        lines.push(format!("  artifact-refs: {}", display_group(&entry.artifact_refs)));
        lines.push(format!("  diagnostics: {}", display_group(&entry.diagnostics)));
        lines.push(format!("  caveats: {}", display_group(&entry.caveats)));
    }
    Ok(lines.join("\n"))
}

