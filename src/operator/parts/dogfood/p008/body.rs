
fn release_bundle_mismatch_diagnostics(
    bundle: &ReleaseEvidenceBundle,
    observed: &ObservedReleaseBundleOutput,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    for diagnostic in [
        mismatch_diagnostic("output-path-ref", &bundle.output_path_ref, &observed.output_path_ref),
        mismatch_diagnostic("report-ref", &bundle.report_ref, &observed.report_ref),
        mismatch_diagnostic("release-gate-ref", &bundle.release_gate_ref, &observed.release_gate_ref),
        mismatch_diagnostic("replay-verify-ref", &bundle.replay_verify_ref, &observed.replay_verify_ref),
        mismatch_diagnostic("replay-index-ref", &bundle.replay_index_ref, &observed.replay_index_ref),
        mismatch_diagnostic("nix-evidence-ref", &bundle.nix_evidence_ref, &observed.nix_evidence_ref),
        mismatch_diagnostic("nix-verify-ref", &bundle.nix_verify_ref, &observed.nix_verify_ref),
        mismatch_diagnostic("summary-ref", &bundle.summary_ref, &observed.summary_ref),
        mismatch_diagnostic("nextest-marker-ref", &bundle.nextest_marker_ref, &observed.nextest_marker_ref),
        mismatch_diagnostic("nextest-check-path", &bundle.nextest_check_path, &observed.nextest_check_path),
    ]
    .into_iter()
    .flatten()
    {
        diagnostics.push_limited_value(
            diagnostic,
            MAX_OPERATOR_DIAGNOSTICS,
            "release evidence bundle verify diagnostics",
        )?;
    }
    for diagnostic in file_ref_mismatch_diagnostics(&bundle.member_refs, &observed.member_refs)? {
        diagnostics.push_limited_value(
            diagnostic,
            MAX_OPERATOR_DIAGNOSTICS,
            "release evidence bundle verify diagnostics",
        )?;
    }
    Ok(diagnostics)
}

fn release_bundle_signature_diagnostics(
    bundle: &ReleaseEvidenceBundle,
    input: &ReleaseEvidenceBundleVerifyInput<'_>,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if input.signed_member_values.is_empty() && !input.is_signed_members_required {
        return Ok(diagnostics);
    }
    let signable_members = release_bundle_signable_members(bundle)?;
    if input.is_signed_members_required && signable_members.is_empty() {
        diagnostics.push_limited_value(
            "release bundle has no required signed-member class".to_string(),
            MAX_OPERATOR_DIAGNOSTICS,
            "release evidence bundle signed member diagnostics",
        )?;
    }
    let mut signed_subject_refs: Vec<(usize, String)> = Vec::new();
    for (signed_index, signed_value) in input.signed_member_values.iter().enumerate() {
        match verify_release_bundle_signed_member(signed_value, input, None) {
            Ok(subject_ref) => {
                if signable_members.iter().any(|(_, member_ref)| member_ref == &subject_ref) {
                    if signed_subject_refs
                        .iter()
                        .any(|entry| entry.1.as_str() == subject_ref.as_str())
                    {
                        diagnostics.push_limited_value(
                            format!("duplicate signed member receipt for subject {subject_ref}"),
                            MAX_OPERATOR_DIAGNOSTICS,
                            "release evidence bundle signed member diagnostics",
                        )?;
                    }
                    signed_subject_refs.push_limited_value(
                        (signed_index, subject_ref),
                        MAX_OPERATOR_REFS,
                        "release evidence bundle signed member refs",
                    )?;
                } else {
                    diagnostics.push_limited_value(
                        format!("signed member subject {subject_ref} is not a signable bundle member"),
                        MAX_OPERATOR_DIAGNOSTICS,
                        "release evidence bundle signed member diagnostics",
                    )?;
                }
            }
            Err(error) => diagnostics.push_limited_value(
                format!("signed member verification failed: {error}"),
                MAX_OPERATOR_DIAGNOSTICS,
                "release evidence bundle signed member diagnostics",
            )?,
        }
    }
    if input.is_signed_members_required {
        for (name, member_ref) in &signable_members {
            if let Some((signed_index, _)) = signed_subject_refs.iter().find(|(_, subject_ref)| subject_ref == member_ref) {
                if let Err(error) = verify_release_bundle_signed_member(
                    &input.signed_member_values[*signed_index],
                    input,
                    Some(member_ref),
                ) {
                    diagnostics.push_limited_value(
                        format!("signed member receipt for {name} failed subject binding: {error}"),
                        MAX_OPERATOR_DIAGNOSTICS,
                        "release evidence bundle signed member diagnostics",
                    )?;
                }
            } else {
                diagnostics.push_limited_value(
                    format!("missing signed member receipt for {name}: {member_ref}"),
                    MAX_OPERATOR_DIAGNOSTICS,
                    "release evidence bundle signed member diagnostics",
                )?;
            }
        }
    }
    Ok(diagnostics)
}

fn release_bundle_signable_members(bundle: &ReleaseEvidenceBundle) -> Result<Vec<(String, String)>> {
    let mut signable_members = Vec::new();
    for (name, member_ref) in &bundle.member_refs {
        if name.ends_with(".preserves") {
            signable_members.push_limited_value(
                (name.clone(), member_ref.clone()),
                MAX_OPERATOR_REFS,
                "release bundle signable member refs",
            )?;
        }
    }
    Ok(signable_members)
}

fn verify_release_bundle_signed_member(
    signed_value: &IoValue,
    input: &ReleaseEvidenceBundleVerifyInput<'_>,
    expected_subject_ref: Option<&str>,
) -> Result<String> {
    if input.signed_keys.is_empty() && input.signed_key_revocations.is_empty() {
        let signed = verify_signed_receipt_with_policy(signed_value, &VerifySignedReceiptPolicy {
            required_purpose: input.signed_purpose,
            trust_root: input.signed_trust_root,
            key: input.signed_key,
            expected_signer: input.signed_signer,
            expected_subject_ref,
        })?;
        Ok(signed.subject_ref)
    } else {
        let signed = verify_signed_receipt_with_keyring_policy(signed_value, &VerifySignedReceiptKeyringPolicy {
            required_purpose: input.signed_purpose,
            trust_root: input.signed_trust_root,
            expected_signer: input.signed_signer,
            expected_subject_ref,
            required_key_ref: input.signed_key_ref,
            required_key_id: input.signed_key_id,
            keys: input.signed_keys,
            revocations: input.signed_key_revocations,
        })?;
        Ok(signed.receipt.subject_ref)
    }
}

pub fn release_export_file_ref(name: &str, bytes: &[u8]) -> String {
    raw_bytes_ref("molten.operator.release-export.file.v1", name, bytes)
}

pub fn release_export_member_names() -> &'static [&'static str] {
    &[
        "dogfood-report.preserves",
        "dogfood-report.signed.preserves",
        "release-gate.preserves",
        "release-gate.signed.preserves",
        "replay-verify.preserves",
        "replay-verify.signed.preserves",
        "replay-evidence-index.preserves",
        "replay-evidence-index.signed.preserves",
        "dogfood-summary.txt",
        "after-nextest.txt",
        "nix-dogfood-evidence.preserves",
        "nix-dogfood-evidence.signed.preserves",
        "nix-dogfood-verify.preserves",
        "nix-dogfood-verify.signed.preserves",
        "nix-dogfood-verify.txt",
        "release-evidence-bundle.preserves",
        "release-evidence-bundle-verify.preserves",
        "release-evidence-bundle-verify.txt",
        "release-promotion-gate.preserves",
        "release-promotion-gate.txt",
        "release-promotion-gate.signed.preserves",
        "release-promotion-gate-signed-verify.txt",
        "release-promotion-summary.preserves",
        "release-promotion-summary.txt",
        "signed-keyring-import.txt",
    ]
}

fn observe_release_export_members(output_path: &Path) -> Result<Vec<(String, String)>> {
    // r[impl molten.filesystem_materialization.root]
    let policy = release_materialization_policy()?;
    let source = crate::materialization::SourceDirectoryRoot::open_existing(output_path)?;
    let mut members = Vec::new();
    for name in release_export_member_names() {
        let path = crate::materialization::MaterializationPath::parse(name, policy.max_path_bytes)?;
        let bytes = source.read_path(&path, policy.max_member_bytes)?;
        members.push_limited_value(
            (name.to_string(), release_export_file_ref(name, &bytes)),
            MAX_OPERATOR_REFS,
            "release export members",
        )?;
    }
    let keyring_path = crate::materialization::MaterializationPath::parse("signed-keyring", policy.max_path_bytes)?;
    let keyring = source.open_subdir(&keyring_path)?;
    for relative in keyring.list_regular_files_recursive(&policy)? {
        let name = format!("signed-keyring/{}", relative.as_str());
        let bytes = keyring.read_path(&relative, policy.max_member_bytes)?;
        members.push_limited_value(
            (name.clone(), release_export_file_ref(&name, &bytes)),
            MAX_OPERATOR_REFS,
            "release export members",
        )?;
    }
    Ok(members)
}

fn read_output_text(output_path: &Path, name: &str) -> Result<String> {
    let policy = release_materialization_policy()?;
    let source = crate::materialization::SourceDirectoryRoot::open_existing(output_path)?;
    let path = crate::materialization::MaterializationPath::parse(name, policy.max_path_bytes)?;
    let bytes = source.read_path(&path, policy.max_member_bytes)?;
    String::from_utf8(bytes)
        .map_err(|error| MoltenError::invalid_harness(format!("release output {name} is not UTF-8: {error}")))
}

fn release_materialization_policy() -> Result<crate::materialization::MaterializationPolicy> {
    crate::materialization::MaterializationPolicy::bounded(
        "operator-release-directory-v1",
        crate::materialization::ReplacementPolicy::NoReplace,
    )
}

fn raw_text_ref(domain: &str, text: &str) -> String {
    let mut bytes = Vec::with_capacity(domain.len().saturating_add(text.len()).saturating_add(1));
    bytes.extend_from_slice(domain.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(text.as_bytes());
    crate::preserves_rail::content_ref_from_bytes(&bytes)
}

fn raw_bytes_ref(domain: &str, name: &str, payload: &[u8]) -> String {
    let mut bytes =
        Vec::with_capacity(domain.len().saturating_add(name.len()).saturating_add(payload.len()).saturating_add(2));
    bytes.extend_from_slice(domain.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(name.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(payload);
    crate::preserves_rail::content_ref_from_bytes(&bytes)
}

fn mismatch_diagnostic(label: &str, expected: &str, actual: &str) -> Option<String> {
    if expected == actual {
        None
    } else {
        Some(format!("{label} mismatch: evidence={expected} observed={actual}"))
    }
}

fn file_ref_mismatch_diagnostics(expected: &[(String, String)], observed: &[(String, String)]) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    for diagnostic in duplicate_file_ref_diagnostics(expected, "evidence")? {
        diagnostics.push_limited_value(
            diagnostic,
            MAX_OPERATOR_DIAGNOSTICS,
            "Nix dogfood verify diagnostics",
        )?;
    }
    for diagnostic in duplicate_file_ref_diagnostics(observed, "observed output")? {
        diagnostics.push_limited_value(
            diagnostic,
            MAX_OPERATOR_DIAGNOSTICS,
            "Nix dogfood verify diagnostics",
        )?;
    }
    if expected.len() != observed.len() {
        diagnostics.push_limited_value(
            format!("file ref count mismatch: evidence={} observed={}", expected.len(), observed.len()),
            MAX_OPERATOR_DIAGNOSTICS,
            "Nix dogfood verify diagnostics",
        )?;
    }
    for (expected_name, expected_ref) in expected {
        match observed.iter().find(|(observed_name, _)| observed_name == expected_name) {
            Some((_, observed_ref)) => {
                if let Some(diagnostic) = mismatch_diagnostic(expected_name, expected_ref, observed_ref) {
                    diagnostics.push_limited_value(
                        diagnostic,
                        MAX_OPERATOR_DIAGNOSTICS,
                        "Nix dogfood verify diagnostics",
                    )?;
                }
            }
            None => diagnostics.push_limited_value(
                format!("file ref missing from observed output: {expected_name}"),
                MAX_OPERATOR_DIAGNOSTICS,
                "Nix dogfood verify diagnostics",
            )?,
        }
    }
    for (observed_name, _) in observed {
        if !expected.iter().any(|(expected_name, _)| expected_name == observed_name) {
            diagnostics.push_limited_value(
                format!("unexpected observed file ref: {observed_name}"),
                MAX_OPERATOR_DIAGNOSTICS,
                "Nix dogfood verify diagnostics",
            )?;
        }
    }
    Ok(diagnostics)
}

fn duplicate_file_ref_diagnostics(refs: &[(String, String)], label: &str) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    let mut seen_names = Vec::new();
    for (name, _) in refs {
        if seen_names.iter().any(|seen_name: &String| seen_name == name) {
            diagnostics.push_limited_value(
                format!("duplicate file ref path in {label}: {name}"),
                MAX_OPERATOR_DIAGNOSTICS,
                "duplicate file ref diagnostics",
            )?;
        } else {
            seen_names.push_limited_value(name.clone(), MAX_OPERATOR_REFS, "duplicate file ref names")?;
        }
    }
    Ok(diagnostics)
}
