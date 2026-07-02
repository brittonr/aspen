
impl ReportParts {
    fn collect(input: &DogfoodReportInput<'_>, workflow: &OperatorWorkflow) -> Result<Self> {
        let checkpoint_refs = input
            .checkpoint_values
            .iter()
            .map(crate::preserves_rail::canonical_hash)
            .collect::<Result<Vec<_>>>()?;
        ensure_count_at_most(checkpoint_refs.len(), MAX_OPERATOR_STEPS, "dogfood checkpoints")?;
        let diagnostics = input.diagnostics.to_vec();
        ensure_count_at_most(diagnostics.len(), MAX_OPERATOR_DIAGNOSTICS, "dogfood report diagnostics")?;
        let mut parts = Self {
            checkpoint_refs,
            step_receipts: Vec::new(),
            diagnostics,
        };
        parts.add_step_notes(workflow)?;
        parts.add_summary_notes(input, workflow)?;
        Ok(parts)
    }

    fn add_step_notes(&mut self, workflow: &OperatorWorkflow) -> Result<()> {
        for step in &workflow.steps {
            if let Some(receipt_ref) = step.receipt_ref.as_ref() {
                self.step_receipts.push_limited_value(
                    (step.name.clone(), receipt_ref.clone()),
                    MAX_OPERATOR_STEPS,
                    "dogfood step receipts",
                )?;
            }
            for diagnostic in &step.diagnostics {
                self.push_note(format!("dogfood step {} diagnostic: {diagnostic}", step.name))?;
            }
            if step.mandatory && step.receipt_ref.is_none() {
                self.push_note(format!("mandatory dogfood step {} lacks canonical receipt", step.name))?;
            }
            if step.mandatory && step.decision != "pass" {
                self.push_note(format!("mandatory dogfood step {} decision is {}", step.name, step.decision))?;
            }
            if step.mandatory && !matches!(step.replay_status.as_str(), "deterministic" | "recorded") {
                self.push_note(format!(
                    "mandatory dogfood step {} has non-release replay status {}",
                    step.name, step.replay_status
                ))?;
            }
        }
        Ok(())
    }

    fn add_summary_notes(&mut self, input: &DogfoodReportInput<'_>, workflow: &OperatorWorkflow) -> Result<()> {
        if self.checkpoint_refs.len() < workflow.steps.len() {
            self.push_note(format!(
                "dogfood workflow has {} steps but only {} checkpoints",
                workflow.steps.len(),
                self.checkpoint_refs.len()
            ))?;
        }
        if !workflow_check_pass(&workflow.checks, "no-hidden-bypass") {
            self.push_note("dogfood workflow contains hidden or unreceipted operator bypass")?;
        }
        if !workflow_check_pass(&workflow.checks, "explicit-operator-authority") {
            self.push_note("dogfood workflow lacks current explicit operator policy/capability refs")?;
        }
        if input.gate_receipt_refs.is_empty() {
            self.push_note("dogfood report requires at least one gate receipt")?;
        }
        if input.repro_bundle_refs.is_empty() {
            self.push_note("dogfood report requires a sealed/redacted repro bundle ref")?;
        }
        Ok(())
    }

    fn push_note(&mut self, note: impl Into<String>) -> Result<()> {
        self.diagnostics
            .push_limited_value(note.into(), MAX_OPERATOR_DIAGNOSTICS, "dogfood report diagnostics")
    }
}

pub fn dogfood_report_value(input: &DogfoodReportInput<'_>) -> Result<IoValue> {
    let workflow = parse_operator_workflow(input.workflow_value)?;
    validate_refs(input.gate_receipt_refs, "dogfood gate receipt ref")?;
    validate_refs(input.repro_bundle_refs, "dogfood repro bundle ref")?;
    validate_ref(input.final_state_ref, "dogfood final state ref")?;
    let parts = ReportParts::collect(input, &workflow)?;
    let decision = if parts.diagnostics.is_empty() { "pass" } else { "deny" };
    Ok(crate::preserves_rail::record("dogfood-report-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::OPERATOR_DOGFOOD_REPORT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(decision)]),
        crate::preserves_rail::record("workflow", vec![crate::preserves_rail::string(&workflow.workflow_ref)]),
        crate::preserves_rail::record("checkpoints", vec![refs_sequence(&parts.checkpoint_refs)]),
        crate::preserves_rail::record("step-receipts", vec![step_receipts_sequence(&parts.step_receipts)]),
        crate::preserves_rail::record("gate-receipts", vec![refs_sequence(input.gate_receipt_refs)]),
        crate::preserves_rail::record("repro-bundles", vec![refs_sequence(input.repro_bundle_refs)]),
        crate::preserves_rail::record("final-state", vec![crate::preserves_rail::string(input.final_state_ref)]),
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(&parts.diagnostics)]),
        checks_value_from_pairs(&[
            ("canonical-report", "pass"),
            (
                "deterministic-or-recorded",
                status(parts.diagnostics.iter().all(|item| !item.contains("replay status"))),
            ),
            ("final-state-bound", "pass"),
            ("redaction-gate", status(!input.repro_bundle_refs.is_empty())),
            ("no-text-oracle", "pass"),
            (
                "no-hidden-bypass",
                status(workflow.checks.iter().any(|(name, status)| name == "no-hidden-bypass" && status == "pass")),
            ),
        ]),
    ]))
}

pub fn parse_dogfood_report(value: &IoValue) -> Result<DogfoodReport> {
    let fields = value
        .collect_simple_record("dogfood-report-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <dogfood-report-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::OPERATOR_DOGFOOD_REPORT_SCHEMA, "dogfood report")?;
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "canonical-report", "dogfood report")?;
    require_check(&checks, "final-state-bound", "dogfood report")?;
    require_check(&checks, "no-text-oracle", "dogfood report")?;
    Ok(DogfoodReport {
        report_ref: crate::preserves_rail::canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        workflow_ref: record_ref(&fields[2], "workflow")?,
        checkpoint_refs: record_ref_sequence(&fields[3], "checkpoints")?,
        step_receipts: record_step_receipts(&fields[4], "step-receipts")?,
        gate_receipts: record_ref_sequence(&fields[5], "gate-receipts")?,
        repro_bundles: record_ref_sequence(&fields[6], "repro-bundles")?,
        final_state_ref: record_ref(&fields[7], "final-state")?,
        diagnostics: record_string_sequence(&fields[8], "diagnostics")?,
        checks,
        value: value.clone(),
    })
}

pub fn release_gate_receipt_value(input: &ReleaseGateInput<'_>) -> Result<IoValue> {
    let report = parse_dogfood_report(input.report_value)?;
    if report.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "dogfood release gate requires pass report {}; decision is {}",
            report.report_ref, report.decision
        )));
    }
    validate_ref(input.node_startup_ref, "dogfood release gate startup ref")?;
    validate_ref(input.node_shutdown_ref, "dogfood release gate shutdown ref")?;
    require_non_empty_refs(input.harness_gate_refs, "dogfood release harness gate ref")?;
    require_non_empty_refs(input.catalog_query_refs, "dogfood release catalog query ref")?;
    require_non_empty_refs(input.repro_verify_refs, "dogfood release repro verify ref")?;
    require_non_empty_refs(input.replay_index_refs, "dogfood release replay index ref")?;
    require_non_empty_refs(input.gc_refs, "dogfood release retention GC ref")?;
    require_non_empty_refs(input.validation_command_refs, "dogfood release validation command ref")?;
    Ok(crate::preserves_rail::record("release-gate-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::OPERATOR_RELEASE_GATE_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
        crate::preserves_rail::record("report", vec![crate::preserves_rail::string(&report.report_ref)]),
        crate::preserves_rail::record("node", vec![
            crate::preserves_rail::string(input.node_startup_ref),
            crate::preserves_rail::string(input.node_shutdown_ref),
        ]),
        crate::preserves_rail::record("harness-gates", vec![refs_sequence(input.harness_gate_refs)]),
        crate::preserves_rail::record("catalog-queries", vec![refs_sequence(input.catalog_query_refs)]),
        crate::preserves_rail::record("repro-verifies", vec![refs_sequence(input.repro_verify_refs)]),
        crate::preserves_rail::record("replay-indexes", vec![refs_sequence(input.replay_index_refs)]),
        crate::preserves_rail::record("retention-gc", vec![refs_sequence(input.gc_refs)]),
        crate::preserves_rail::record("validation-commands", vec![refs_sequence(input.validation_command_refs)]),
        checks_value_from_pairs(&[
            ("dogfood-report-pass", "pass"),
            ("deterministic-or-recorded-only", "pass"),
            ("redaction-gate-bound", "pass"),
            ("startup-shutdown-bound", "pass"),
            ("catalog-mcp-bound", "pass"),
            ("replay-evidence-index-bound", "pass"),
            ("replay-index-is-evidence-only", "pass"),
            ("retention-gc-review-bound", "pass"),
            ("retention-gc-is-evidence-only", "pass"),
            ("no-text-oracle", "pass"),
        ]),
    ]))
}

pub fn parse_release_gate_receipt(value: &IoValue) -> Result<ReleaseGateReceipt> {
    let fields = value
        .collect_simple_record("release-gate-receipt-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected <release-gate-receipt-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::OPERATOR_RELEASE_GATE_RECEIPT_SCHEMA, "operator release gate")?;
    let checks = parse_checks(&fields[10])?;
    require_check(&checks, "dogfood-report-pass", "operator release gate")?;
    require_check(&checks, "replay-evidence-index-bound", "operator release gate")?;
    require_check(&checks, "replay-index-is-evidence-only", "operator release gate")?;
    require_check(&checks, "no-text-oracle", "operator release gate")?;
    let node = crate::preserves_rail::value_to_iovalue(&fields[3]);
    let node_fields = simple_record(&node, "node", 2)?;
    Ok(ReleaseGateReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        report_ref: record_ref(&fields[2], "report")?,
        startup_ref: required_ref(&node_fields[0], "release gate startup ref")?,
        shutdown_ref: required_ref(&node_fields[1], "release gate shutdown ref")?,
        harness_gate_refs: record_ref_sequence(&fields[4], "harness-gates")?,
        catalog_query_refs: record_ref_sequence(&fields[5], "catalog-queries")?,
        repro_verify_refs: record_ref_sequence(&fields[6], "repro-verifies")?,
        replay_index_refs: record_ref_sequence(&fields[7], "replay-indexes")?,
        gc_refs: record_ref_sequence(&fields[8], "retention-gc")?,
        validation_command_refs: record_ref_sequence(&fields[9], "validation-commands")?,
        checks,
        value: value.clone(),
    })
}

pub fn nix_dogfood_release_evidence_value(input: &NixDogfoodEvidenceInput<'_>) -> Result<IoValue> {
    let observed = observe_nix_dogfood_output(input.output_path)?;
    Ok(crate::preserves_rail::record("nix-dogfood-release-evidence-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::OPERATOR_NIX_DOGFOOD_EVIDENCE_SCHEMA),
        crate::preserves_rail::record("output-path", vec![
            crate::preserves_rail::string(observed.output_path.as_str()),
            crate::preserves_rail::string(&observed.output_path_ref),
        ]),
        crate::preserves_rail::record("report", vec![crate::preserves_rail::string(&observed.report_ref)]),
        crate::preserves_rail::record("release-gate", vec![crate::preserves_rail::string(&observed.release_gate_ref)]),
        crate::preserves_rail::record("replay-verify", vec![crate::preserves_rail::string(
            &observed.replay_verify_ref,
        )]),
        crate::preserves_rail::record("replay-index", vec![crate::preserves_rail::string(&observed.replay_index_ref)]),
        crate::preserves_rail::record("summary", vec![crate::preserves_rail::string(&observed.summary_ref)]),
        crate::preserves_rail::record("nextest", vec![
            crate::preserves_rail::string(&observed.nextest_marker_ref),
            crate::preserves_rail::string(observed.nextest_check_path.as_str()),
        ]),
        crate::preserves_rail::record("files", vec![file_refs_sequence(&observed.file_refs)]),
        checks_value_from_pairs(&[
            ("dogfood-report-pass", "pass"),
            ("release-gate-ref-bound", "pass"),
            ("replay-verify-ref-bound", "pass"),
            ("replay-index-ref-bound", "pass"),
            ("replay-index-is-evidence-only", "pass"),
            ("nix-output-path-bound", "pass"),
            ("nextest-dependency-bound", "pass"),
            ("release-evidence-only", "pass"),
            ("no-text-oracle", "pass"),
        ]),
    ]))
}

pub fn parse_nix_dogfood_evidence(value: &IoValue) -> Result<NixDogfoodEvidence> {
    let fields = value
        .collect_simple_record("nix-dogfood-release-evidence-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <nix-dogfood-release-evidence-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::OPERATOR_NIX_DOGFOOD_EVIDENCE_SCHEMA, "Nix dogfood evidence")?;
    let output_path = crate::preserves_rail::value_to_iovalue(&fields[1]);
    let output_fields = simple_record(&output_path, "output-path", 2)?;
    let nextest = crate::preserves_rail::value_to_iovalue(&fields[7]);
    let nextest_fields = simple_record(&nextest, "nextest", 2)?;
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "replay-verify-ref-bound", "Nix dogfood evidence")?;
    require_check(&checks, "replay-index-ref-bound", "Nix dogfood evidence")?;
    require_check(&checks, "replay-index-is-evidence-only", "Nix dogfood evidence")?;
    require_check(&checks, "release-evidence-only", "Nix dogfood evidence")?;
    require_check(&checks, "no-text-oracle", "Nix dogfood evidence")?;
    Ok(NixDogfoodEvidence {
        evidence_ref: crate::preserves_rail::canonical_hash(value)?,
        output_path: required_string(&output_fields[0], "Nix dogfood output path")?,
        output_path_ref: required_ref(&output_fields[1], "Nix dogfood output path ref")?,
        report_ref: record_ref(&fields[2], "report")?,
        release_gate_ref: record_ref(&fields[3], "release-gate")?,
        replay_verify_ref: record_ref(&fields[4], "replay-verify")?,
        replay_index_ref: record_ref(&fields[5], "replay-index")?,
        summary_ref: record_ref(&fields[6], "summary")?,
        nextest_marker_ref: required_ref(&nextest_fields[0], "Nix dogfood nextest marker ref")?,
        nextest_check_path: required_string(&nextest_fields[1], "Nix dogfood nextest check path")?,
        file_refs: record_file_refs(&fields[8], "files")?,
        checks,
        value: value.clone(),
    })
}

struct NixObservation {
    observed: ObservedNixDogfoodOutput,
    is_output_observed: bool,
}

fn fallback_nix_output(
    output_path: String,
    output_path_ref: String,
    evidence: &NixDogfoodEvidence,
) -> ObservedNixDogfoodOutput {
    ObservedNixDogfoodOutput {
        output_path,
        output_path_ref,
        report_ref: evidence.report_ref.clone(),
        release_gate_ref: evidence.release_gate_ref.clone(),
        replay_verify_ref: evidence.replay_verify_ref.clone(),
        replay_index_ref: evidence.replay_index_ref.clone(),
        summary_ref: evidence.summary_ref.clone(),
        nextest_marker_ref: evidence.nextest_marker_ref.clone(),
        nextest_check_path: evidence.nextest_check_path.clone(),
        file_refs: evidence.file_refs.clone(),
    }
}
