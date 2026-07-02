
fn observed_nix_or_fallback(
    output_path: &Path,
    evidence: &NixDogfoodEvidence,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<NixObservation> {
    let output_path_string = output_path.display().to_string();
    let fallback_output_path_ref = raw_text_ref("molten.operator.nix-dogfood-output-path.v1", &output_path_string);
    match observe_nix_dogfood_output(output_path) {
        Ok(observed) => Ok(NixObservation {
            observed,
            is_output_observed: true,
        }),
        Err(error) => {
            diagnostics.push_limited_value(
                format!("Nix dogfood output observation failed: {error}"),
                MAX_OPERATOR_DIAGNOSTICS,
                "Nix dogfood verify diagnostics",
            )?;
            Ok(NixObservation {
                observed: fallback_nix_output(output_path_string, fallback_output_path_ref, evidence),
                is_output_observed: false,
            })
        }
    }
}

pub fn verify_nix_dogfood_evidence(input: &NixDogfoodVerifyInput<'_>) -> Result<NixDogfoodVerifyReceipt> {
    let evidence = parse_nix_dogfood_evidence(input.evidence_value)?;
    let mut diagnostics = Vec::new();
    let NixObservation {
        observed,
        is_output_observed,
    } = observed_nix_or_fallback(input.output_path, &evidence, &mut diagnostics)?;
    for diagnostic in [
        mismatch_diagnostic("output-path-ref", &evidence.output_path_ref, &observed.output_path_ref),
        mismatch_diagnostic("report-ref", &evidence.report_ref, &observed.report_ref),
        mismatch_diagnostic("release-gate-ref", &evidence.release_gate_ref, &observed.release_gate_ref),
        mismatch_diagnostic("replay-verify-ref", &evidence.replay_verify_ref, &observed.replay_verify_ref),
        mismatch_diagnostic("replay-index-ref", &evidence.replay_index_ref, &observed.replay_index_ref),
        mismatch_diagnostic("summary-ref", &evidence.summary_ref, &observed.summary_ref),
        mismatch_diagnostic("nextest-marker-ref", &evidence.nextest_marker_ref, &observed.nextest_marker_ref),
        mismatch_diagnostic("nextest-check-path", &evidence.nextest_check_path, &observed.nextest_check_path),
    ]
    .into_iter()
    .flatten()
    {
        diagnostics.push_limited_value(diagnostic, MAX_OPERATOR_DIAGNOSTICS, "Nix dogfood verify diagnostics")?;
    }
    for diagnostic in file_ref_mismatch_diagnostics(&evidence.file_refs, &observed.file_refs)? {
        diagnostics.push_limited_value(diagnostic, MAX_OPERATOR_DIAGNOSTICS, "Nix dogfood verify diagnostics")?;
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = crate::preserves_rail::record("nix-dogfood-release-verify-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::OPERATOR_NIX_DOGFOOD_VERIFY_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(decision)]),
        crate::preserves_rail::record("evidence", vec![crate::preserves_rail::string(&evidence.evidence_ref)]),
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
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value_from_pairs(&[
            ("dogfood-report-pass", status(is_output_observed)),
            ("release-gate-ref-bound", status(evidence.release_gate_ref == observed.release_gate_ref)),
            ("replay-verify-ref-bound", status(evidence.replay_verify_ref == observed.replay_verify_ref)),
            ("replay-index-ref-bound", status(evidence.replay_index_ref == observed.replay_index_ref)),
            ("replay-index-is-evidence-only", "pass"),
            ("nix-output-path-bound", status(evidence.output_path_ref == observed.output_path_ref)),
            ("nextest-dependency-bound", status(evidence.nextest_marker_ref == observed.nextest_marker_ref)),
            ("release-evidence-only", "pass"),
            ("no-text-oracle", "pass"),
        ]),
    ]);
    parse_nix_dogfood_verify_receipt(&value)
}

pub fn parse_nix_dogfood_verify_receipt(value: &IoValue) -> Result<NixDogfoodVerifyReceipt> {
    let fields = value
        .collect_simple_record("nix-dogfood-release-verify-receipt-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <nix-dogfood-release-verify-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::OPERATOR_NIX_DOGFOOD_VERIFY_RECEIPT_SCHEMA,
        "Nix dogfood verify receipt",
    )?;
    let output_path = crate::preserves_rail::value_to_iovalue(&fields[3]);
    let output_fields = simple_record(&output_path, "output-path", 2)?;
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "replay-verify-ref-bound", "Nix dogfood verify receipt")?;
    require_check(&checks, "replay-index-ref-bound", "Nix dogfood verify receipt")?;
    require_check(&checks, "replay-index-is-evidence-only", "Nix dogfood verify receipt")?;
    require_check(&checks, "release-evidence-only", "Nix dogfood verify receipt")?;
    require_check(&checks, "no-text-oracle", "Nix dogfood verify receipt")?;
    Ok(NixDogfoodVerifyReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        evidence_ref: record_ref(&fields[2], "evidence")?,
        output_path_ref: required_ref(&output_fields[1], "Nix dogfood verify output path ref")?,
        report_ref: record_ref(&fields[4], "report")?,
        release_gate_ref: record_ref(&fields[5], "release-gate")?,
        replay_verify_ref: record_ref(&fields[6], "replay-verify")?,
        replay_index_ref: record_ref(&fields[7], "replay-index")?,
        diagnostics: record_string_sequence(&fields[8], "diagnostics")?,
        checks,
        value: value.clone(),
    })
}

pub fn release_evidence_bundle_value(input: &ReleaseEvidenceBundleInput<'_>) -> Result<IoValue> {
    let observed = observe_release_bundle_output(input.output_path)?;
    Ok(crate::preserves_rail::record("release-evidence-bundle-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::OPERATOR_RELEASE_EVIDENCE_BUNDLE_SCHEMA),
        crate::preserves_rail::record("output-path", vec![
            crate::preserves_rail::string(observed.output_path.as_str()),
            crate::preserves_rail::string(&observed.output_path_ref),
        ]),
        crate::preserves_rail::record("members", vec![file_refs_sequence(&observed.member_refs)]),
        crate::preserves_rail::record("dogfood", vec![
            crate::preserves_rail::string(&observed.report_ref),
            crate::preserves_rail::string(&observed.release_gate_ref),
        ]),
        crate::preserves_rail::record("replay", vec![
            crate::preserves_rail::string(&observed.replay_verify_ref),
            crate::preserves_rail::string(&observed.replay_index_ref),
        ]),
        crate::preserves_rail::record("nix", vec![
            crate::preserves_rail::string(&observed.nix_evidence_ref),
            crate::preserves_rail::string(&observed.nix_verify_ref),
        ]),
        crate::preserves_rail::record("nextest", vec![
            crate::preserves_rail::string(&observed.nextest_marker_ref),
            crate::preserves_rail::string(observed.nextest_check_path.as_str()),
        ]),
        checks_value_from_pairs(&[
            ("dogfood-report-pass", "pass"),
            ("release-gate-pass", "pass"),
            ("replay-verify-bound", "pass"),
            ("replay-index-bound", "pass"),
            ("replay-index-is-evidence-only", "pass"),
            ("nix-verify-pass", "pass"),
            ("bundle-members-bound", "pass"),
            ("nextest-dependency-bound", "pass"),
            ("release-evidence-only", "pass"),
            ("no-text-oracle", "pass"),
        ]),
    ]))
}

pub fn parse_release_evidence_bundle(value: &IoValue) -> Result<ReleaseEvidenceBundle> {
    let fields = value
        .collect_simple_record("release-evidence-bundle-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <release-evidence-bundle-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::OPERATOR_RELEASE_EVIDENCE_BUNDLE_SCHEMA,
        "release evidence bundle",
    )?;
    let output_path = crate::preserves_rail::value_to_iovalue(&fields[1]);
    let output_fields = simple_record(&output_path, "output-path", 2)?;
    let dogfood = crate::preserves_rail::value_to_iovalue(&fields[3]);
    let dogfood_fields = simple_record(&dogfood, "dogfood", 2)?;
    let replay = crate::preserves_rail::value_to_iovalue(&fields[4]);
    let replay_fields = simple_record(&replay, "replay", 2)?;
    let nix = crate::preserves_rail::value_to_iovalue(&fields[5]);
    let nix_fields = simple_record(&nix, "nix", 2)?;
    let nextest = crate::preserves_rail::value_to_iovalue(&fields[6]);
    let nextest_fields = simple_record(&nextest, "nextest", 2)?;
    let checks = parse_checks(&fields[7])?;
    require_check(&checks, "bundle-members-bound", "release evidence bundle")?;
    require_check(&checks, "replay-verify-bound", "release evidence bundle")?;
    require_check(&checks, "replay-index-bound", "release evidence bundle")?;
    require_check(&checks, "replay-index-is-evidence-only", "release evidence bundle")?;
    require_check(&checks, "release-evidence-only", "release evidence bundle")?;
    require_check(&checks, "no-text-oracle", "release evidence bundle")?;
    Ok(ReleaseEvidenceBundle {
        bundle_ref: crate::preserves_rail::canonical_hash(value)?,
        output_path: required_string(&output_fields[0], "release evidence output path")?,
        output_path_ref: required_ref(&output_fields[1], "release evidence output path ref")?,
        report_ref: required_ref(&dogfood_fields[0], "release evidence report ref")?,
        release_gate_ref: required_ref(&dogfood_fields[1], "release evidence release gate ref")?,
        replay_verify_ref: required_ref(&replay_fields[0], "release evidence replay verify ref")?,
        replay_index_ref: required_ref(&replay_fields[1], "release evidence replay index ref")?,
        nix_evidence_ref: required_ref(&nix_fields[0], "release evidence Nix evidence ref")?,
        nix_verify_ref: required_ref(&nix_fields[1], "release evidence Nix verify ref")?,
        summary_ref: member_ref(&fields[2], "dogfood-summary.txt")?,
        nextest_marker_ref: required_ref(&nextest_fields[0], "release evidence nextest marker ref")?,
        nextest_check_path: required_string(&nextest_fields[1], "release evidence nextest check path")?,
        member_refs: record_file_refs(&fields[2], "members")?,
        checks,
        value: value.clone(),
    })
}

struct BundleObservation {
    observed: ObservedReleaseBundleOutput,
    is_output_observed: bool,
}

fn fallback_output(
    output_path: String,
    output_path_ref: String,
    bundle: &ReleaseEvidenceBundle,
) -> ObservedReleaseBundleOutput {
    ObservedReleaseBundleOutput {
        output_path,
        output_path_ref,
        report_ref: bundle.report_ref.clone(),
        release_gate_ref: bundle.release_gate_ref.clone(),
        replay_verify_ref: bundle.replay_verify_ref.clone(),
        replay_index_ref: bundle.replay_index_ref.clone(),
        nix_evidence_ref: bundle.nix_evidence_ref.clone(),
        nix_verify_ref: bundle.nix_verify_ref.clone(),
        summary_ref: bundle.summary_ref.clone(),
        nextest_marker_ref: bundle.nextest_marker_ref.clone(),
        nextest_check_path: bundle.nextest_check_path.clone(),
        member_refs: bundle.member_refs.clone(),
    }
}

fn observed_or_fallback(
    output_path: &Path,
    bundle: &ReleaseEvidenceBundle,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<BundleObservation> {
    let output_path_string = output_path.display().to_string();
    let fallback_output_path_ref = raw_text_ref("molten.operator.nix-dogfood-output-path.v1", &output_path_string);
    match observe_release_bundle_output(output_path) {
        Ok(observed) => Ok(BundleObservation {
            observed,
            is_output_observed: true,
        }),
        Err(error) => {
            diagnostics.push_limited_value(
                format!("release evidence bundle output observation failed: {error}"),
                MAX_OPERATOR_DIAGNOSTICS,
                "release evidence bundle verify diagnostics",
            )?;
            Ok(BundleObservation {
                observed: fallback_output(output_path_string, fallback_output_path_ref, bundle),
                is_output_observed: false,
            })
        }
    }
}
