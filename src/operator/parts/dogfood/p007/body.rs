
#[derive(Debug, Clone, PartialEq, Eq)]
struct ObservedNixDogfoodOutput {
    output_path: String,
    output_path_ref: String,
    report_ref: String,
    release_gate_ref: String,
    replay_verify_ref: String,
    replay_index_ref: String,
    summary_ref: String,
    nextest_marker_ref: String,
    nextest_check_path: String,
    file_refs: Vec<(String, String)>,
}

struct OutputBindingRefs<'a> {
    report: &'a DogfoodReport,
    release_gate: &'a ReleaseGateReceipt,
    replay_verify_ref: &'a str,
    replay_index_ref: &'a str,
    replay_index_receipt_refs: &'a [String],
}

fn require_observed_bindings(input: &OutputBindingRefs<'_>) -> Result<()> {
    if !input
        .replay_index_receipt_refs
        .iter()
        .any(|reference| reference.as_str() == input.replay_verify_ref)
    {
        return Err(MoltenError::invalid_harness(format!(
            "Nix dogfood replay index {} does not bind replay verify {}",
            input.replay_index_ref, input.replay_verify_ref
        )));
    }
    if input.report.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "Nix dogfood evidence requires pass report {}; decision is {}",
            input.report.report_ref, input.report.decision
        )));
    }
    if input.release_gate.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "Nix dogfood evidence requires pass release gate {}; decision is {}",
            input.release_gate.receipt_ref, input.release_gate.decision
        )));
    }
    if input.release_gate.report_ref != input.report.report_ref {
        return Err(MoltenError::invalid_harness(format!(
            "Nix dogfood release gate report ref {} does not match report {}",
            input.release_gate.report_ref, input.report.report_ref
        )));
    }
    if !input
        .release_gate
        .replay_index_refs
        .iter()
        .any(|reference| reference.as_str() == input.replay_index_ref)
    {
        return Err(MoltenError::invalid_harness(format!(
            "Nix dogfood release gate does not bind replay index {}",
            input.replay_index_ref
        )));
    }
    Ok(())
}

fn observed_file_refs(entries: [(&str, &str); 6]) -> Result<Vec<(String, String)>> {
    let mut file_refs = Vec::new();
    for (path, reference) in entries {
        file_refs.push_limited_value(
            (path.to_string(), reference.to_string()),
            MAX_OPERATOR_REFS,
            "Nix dogfood file refs",
        )?;
    }
    Ok(file_refs)
}

fn observe_nix_dogfood_output(output_path: &Path) -> Result<ObservedNixDogfoodOutput> {
    let output_path_string = output_path.display().to_string();
    let output_path_ref = raw_text_ref("molten.operator.nix-dogfood-output-path.v1", &output_path_string);
    let report_text = read_output_text(output_path, "dogfood-report.preserves")?;
    let release_gate_text = read_output_text(output_path, "release-gate.preserves")?;
    let replay_verify_text = read_output_text(output_path, "replay-verify.preserves")?;
    let replay_index_text = read_output_text(output_path, "replay-evidence-index.preserves")?;
    let summary_text = read_output_text(output_path, "dogfood-summary.txt")?;
    let nextest_text = read_output_text(output_path, "after-nextest.txt")?;
    let report_value = crate::preserves_rail::parse_text(&report_text)?;
    let release_gate_value = crate::preserves_rail::parse_text(&release_gate_text)?;
    let replay_verify_value = crate::preserves_rail::parse_text(&replay_verify_text)?;
    let replay_index_value = crate::preserves_rail::parse_text(&replay_index_text)?;
    let report = parse_dogfood_report(&report_value)?;
    let release_gate = parse_release_gate_receipt(&release_gate_value)?;
    let replay_verify_ref = parse_release_replay_verify(&replay_verify_value)?;
    let replay_index_ref = parse_release_replay_index(&replay_index_value)?;
    let replay_index_receipt_refs = parse_release_replay_index_receipt_refs(&replay_index_value)?;
    require_observed_bindings(&OutputBindingRefs {
        report: &report,
        release_gate: &release_gate,
        replay_verify_ref: &replay_verify_ref,
        replay_index_ref: &replay_index_ref,
        replay_index_receipt_refs: &replay_index_receipt_refs,
    })?;
    let nextest_check_path = nextest_text.trim().to_string();
    if nextest_check_path.is_empty() {
        return Err(MoltenError::invalid_harness("Nix dogfood after-nextest marker is empty"));
    }
    let summary_ref = raw_text_ref("molten.operator.nix-dogfood-summary.v1", &summary_text);
    let nextest_marker_ref = raw_text_ref("molten.operator.nix-dogfood-nextest-marker.v1", &nextest_text);
    let file_refs = observed_file_refs([
        ("dogfood-report.preserves", report.report_ref.as_str()),
        ("release-gate.preserves", release_gate.receipt_ref.as_str()),
        ("replay-verify.preserves", replay_verify_ref.as_str()),
        ("replay-evidence-index.preserves", replay_index_ref.as_str()),
        ("dogfood-summary.txt", summary_ref.as_str()),
        ("after-nextest.txt", nextest_marker_ref.as_str()),
    ])?;
    Ok(ObservedNixDogfoodOutput {
        output_path: output_path_string,
        output_path_ref,
        report_ref: report.report_ref,
        release_gate_ref: release_gate.receipt_ref,
        replay_verify_ref,
        replay_index_ref,
        summary_ref,
        nextest_marker_ref,
        nextest_check_path,
        file_refs,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ObservedReleaseBundleOutput {
    output_path: String,
    output_path_ref: String,
    report_ref: String,
    release_gate_ref: String,
    replay_verify_ref: String,
    replay_index_ref: String,
    nix_evidence_ref: String,
    nix_verify_ref: String,
    summary_ref: String,
    nextest_marker_ref: String,
    nextest_check_path: String,
    member_refs: Vec<(String, String)>,
}

fn parse_release_replay_verify(value: &IoValue) -> Result<String> {
    let fields = value
        .collect_simple_record("deterministic-replay-verify-v1", Some(13))
        .ok_or_else(|| MoltenError::invalid_harness("expected <deterministic-replay-verify-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::DETERMINISTIC_REPLAY_VERIFY_SCHEMA, "release replay verify")?;
    let decision = required_string(&fields[1], "release replay verify decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "release replay verify decision is {decision}; expected pass"
        )));
    }
    let divergence = record_string(&fields[10], "divergence")?;
    if divergence != "none" {
        return Err(MoltenError::invalid_harness(format!(
            "release replay verify divergence is {divergence}; expected none"
        )));
    }
    crate::preserves_rail::canonical_hash(value)
}

fn parse_release_replay_index(value: &IoValue) -> Result<String> {
    let fields = value
        .collect_simple_record("deterministic-replay-index-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <deterministic-replay-index-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::DETERMINISTIC_REPLAY_INDEX_SCHEMA, "release replay index")?;
    let decision = record_string(&fields[1], "decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "release replay index decision is {decision}; expected pass"
        )));
    }
    let checks = parse_replay_index_checks(&fields[14])?;
    require_check(&checks, "evidence-only", "release replay index")?;
    require_check(&checks, "no-authority-grant", "release replay index")?;
    crate::preserves_rail::canonical_hash(value)
}

fn parse_release_replay_index_receipt_refs(value: &IoValue) -> Result<Vec<String>> {
    let fields = value
        .collect_simple_record("deterministic-replay-index-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <deterministic-replay-index-v1 ...>"))?;
    record_ref_sequence(&fields[7], "receipt-refs")
}

fn parse_replay_index_checks(value: &Value<IoValue>) -> Result<Vec<(String, String)>> {
    let items = required_sequence(value, "release replay index checks")?;
    ensure_count_at_most(items.len(), MAX_OPERATOR_REFS, "release replay index checks")?;
    let mut checks = Vec::new();
    for item in items.iter() {
        let item = crate::preserves_rail::value_to_iovalue(item);
        let fields = simple_record(&item, "check", 2)?;
        let name = required_string(&fields[0], "release replay index check name")?;
        let status = required_string(&fields[1], "release replay index check status")?;
        checks.push_limited_value((name, status), MAX_OPERATOR_REFS, "release replay index checks")?;
    }
    Ok(checks)
}

fn observe_release_bundle_output(output_path: &Path) -> Result<ObservedReleaseBundleOutput> {
    let observed_nix = observe_nix_dogfood_output(output_path)?;
    let nix_evidence_value =
        crate::preserves_rail::parse_text(&read_output_text(output_path, "nix-dogfood-evidence.preserves")?)?;
    let nix_verify_value =
        crate::preserves_rail::parse_text(&read_output_text(output_path, "nix-dogfood-verify.preserves")?)?;
    let nix_evidence = parse_nix_dogfood_evidence(&nix_evidence_value)?;
    let nix_verify = parse_nix_dogfood_verify_receipt(&nix_verify_value)?;
    ensure_nix_release_artifacts_match(&observed_nix, &nix_evidence, &nix_verify)?;
    let mut member_refs = observed_nix.file_refs.clone();
    member_refs.push_limited_value(
        ("nix-dogfood-evidence.preserves".to_string(), nix_evidence.evidence_ref.clone()),
        MAX_OPERATOR_REFS,
        "release evidence bundle members",
    )?;
    member_refs.push_limited_value(
        ("nix-dogfood-verify.preserves".to_string(), nix_verify.receipt_ref.clone()),
        MAX_OPERATOR_REFS,
        "release evidence bundle members",
    )?;
    Ok(ObservedReleaseBundleOutput {
        output_path: observed_nix.output_path,
        output_path_ref: observed_nix.output_path_ref,
        report_ref: observed_nix.report_ref,
        release_gate_ref: observed_nix.release_gate_ref,
        replay_verify_ref: observed_nix.replay_verify_ref,
        replay_index_ref: observed_nix.replay_index_ref,
        nix_evidence_ref: nix_evidence.evidence_ref,
        nix_verify_ref: nix_verify.receipt_ref,
        summary_ref: observed_nix.summary_ref,
        nextest_marker_ref: observed_nix.nextest_marker_ref,
        nextest_check_path: observed_nix.nextest_check_path,
        member_refs,
    })
}

fn ensure_nix_release_artifacts_match(
    observed: &ObservedNixDogfoodOutput,
    evidence: &NixDogfoodEvidence,
    verify: &NixDogfoodVerifyReceipt,
) -> Result<()> {
    if let Some(mismatch) = [
        mismatch_diagnostic("Nix evidence output-path-ref", &evidence.output_path_ref, &observed.output_path_ref),
        mismatch_diagnostic("Nix evidence report-ref", &evidence.report_ref, &observed.report_ref),
        mismatch_diagnostic("Nix evidence release-gate-ref", &evidence.release_gate_ref, &observed.release_gate_ref),
        mismatch_diagnostic("Nix evidence replay-verify-ref", &evidence.replay_verify_ref, &observed.replay_verify_ref),
        mismatch_diagnostic("Nix evidence replay-index-ref", &evidence.replay_index_ref, &observed.replay_index_ref),
        mismatch_diagnostic("Nix evidence summary-ref", &evidence.summary_ref, &observed.summary_ref),
        mismatch_diagnostic(
            "Nix evidence nextest-marker-ref",
            &evidence.nextest_marker_ref,
            &observed.nextest_marker_ref,
        ),
        mismatch_diagnostic(
            "Nix evidence nextest-check-path",
            &evidence.nextest_check_path,
            &observed.nextest_check_path,
        ),
        mismatch_diagnostic("Nix verify evidence-ref", &verify.evidence_ref, &evidence.evidence_ref),
        mismatch_diagnostic("Nix verify output-path-ref", &verify.output_path_ref, &observed.output_path_ref),
        mismatch_diagnostic("Nix verify report-ref", &verify.report_ref, &observed.report_ref),
        mismatch_diagnostic("Nix verify release-gate-ref", &verify.release_gate_ref, &observed.release_gate_ref),
        mismatch_diagnostic("Nix verify replay-verify-ref", &verify.replay_verify_ref, &observed.replay_verify_ref),
        mismatch_diagnostic("Nix verify replay-index-ref", &verify.replay_index_ref, &observed.replay_index_ref),
    ]
    .into_iter()
    .flatten()
    .next()
    {
        return Err(MoltenError::invalid_harness(mismatch));
    }
    if verify.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "Nix dogfood verify receipt {} decision is {}",
            verify.receipt_ref, verify.decision
        )));
    }
    Ok(())
}
