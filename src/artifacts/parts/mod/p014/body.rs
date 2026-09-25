
fn identity_input_from_artifact(artifact: &ArtifactRecord) -> ArtifactIdentityInput<'_> {
    ArtifactIdentityInput {
        kind: &artifact.kind,
        identity_domain: &artifact.domain,
        canonical_payload_ref: canonical_payload_ref(&artifact.payload),
        canonicalizer: canonicalizer_for_kind(&artifact.kind),
        artifact_ref: Some(&artifact.artifact_ref),
        schema_refs: &artifact.schema_refs,
        dependency_summary_refs: &artifact.dependency_refs,
        effect_manifest_ref: artifact.effect_manifest_ref.as_deref(),
        policy_refs: &artifact.policy_refs,
        provenance_refs: &artifact.evidence_refs,
        hash_algorithm: ARTIFACT_IDENTITY_HASH_ALGORITHM,
    }
}

fn canonical_payload_ref(payload: &ArtifactPayloadRef) -> &str {
    match payload {
        ArtifactPayloadRef::Inline { value_ref, .. } => value_ref,
        ArtifactPayloadRef::ContentRef { manifest_ref, .. } => manifest_ref,
    }
}

// r[impl molten.artifacts.normalized_payload_boundary]
fn canonicalizer_for_kind(kind: &str) -> &'static str {
    if SUPPORTED_ARTIFACT_KINDS.contains(&kind) {
        PRESERVES_VALUE_CANONICALIZER
    } else {
        "unsupported-opaque"
    }
}

// r[impl molten.artifacts.domain_separated_identity]
// r[impl molten.artifacts.install_rejects_noncanonical]
fn artifact_identity_diagnostics(input: &ArtifactIdentityInput<'_>) -> Result<Vec<String>> {
    validate_kind(input.kind)?;
    validate_refs(input.schema_refs, "artifact identity schema ref")?;
    validate_refs(input.dependency_summary_refs, "artifact identity dependency summary ref")?;
    if let Some(effect_manifest_ref) = input.effect_manifest_ref {
        validate_ref(effect_manifest_ref, "artifact identity effect manifest ref")?;
    }
    if let Some(artifact_ref) = input.artifact_ref {
        validate_ref(artifact_ref, "artifact identity artifact ref")?;
    }
    validate_refs(input.policy_refs, "artifact identity policy ref")?;
    validate_refs(input.provenance_refs, "artifact identity provenance ref")?;

    let mut diagnostics = Vec::new();
    if !SUPPORTED_ARTIFACT_KINDS.contains(&input.kind) {
        push_bounded(
            &mut diagnostics,
            format!("unsupported artifact kind {} has no reviewed canonicalizer", input.kind),
            MAX_ARTIFACT_DIAGNOSTICS,
            "artifact identity diagnostics",
        )?;
    }
    if input.identity_domain != domain_for_kind(input.kind) {
        push_bounded(
            &mut diagnostics,
            format!("identity domain {} does not match artifact kind {}", input.identity_domain, input.kind),
            MAX_ARTIFACT_DIAGNOSTICS,
            "artifact identity diagnostics",
        )?;
    }
    if let Err(error) = validate_ref(input.canonical_payload_ref, "artifact identity canonical payload ref") {
        push_bounded(
            &mut diagnostics,
            format!("missing or invalid canonical payload ref: {error}"),
            MAX_ARTIFACT_DIAGNOSTICS,
            "artifact identity diagnostics",
        )?;
    }
    if input.hash_algorithm != ARTIFACT_IDENTITY_HASH_ALGORITHM {
        push_bounded(
            &mut diagnostics,
            format!(
                "unsupported hash algorithm {}; Molten-owned artifact identity requires {}",
                input.hash_algorithm, ARTIFACT_IDENTITY_HASH_ALGORITHM
            ),
            MAX_ARTIFACT_DIAGNOSTICS,
            "artifact identity diagnostics",
        )?;
    }
    if input.canonicalizer.is_empty()
        || matches!(input.canonicalizer, RAW_SOURCE_CANONICALIZER | RENDERED_LOG_CANONICALIZER)
    {
        push_bounded(
            &mut diagnostics,
            format!("artifact kind {} lacks a reviewed canonical payload boundary", input.kind),
            MAX_ARTIFACT_DIAGNOSTICS,
            "artifact identity diagnostics",
        )?;
    }
    Ok(diagnostics)
}

fn artifact_identity_subject_value(input: &ArtifactIdentityInput<'_>) -> Result<IoValue> {
    validate_ref(input.canonical_payload_ref, "artifact identity canonical payload ref")?;
    validate_refs(input.schema_refs, "artifact identity schema ref")?;
    validate_refs(input.dependency_summary_refs, "artifact identity dependency summary ref")?;
    if let Some(effect_manifest_ref) = input.effect_manifest_ref {
        validate_ref(effect_manifest_ref, "artifact identity effect manifest ref")?;
    }
    validate_refs(input.policy_refs, "artifact identity policy ref")?;
    validate_refs(input.provenance_refs, "artifact identity provenance ref")?;
    Ok(record("artifact-identity-subject-v1", vec![
        record("kind", vec![string(input.kind)]),
        record("domain", vec![string(input.identity_domain)]),
        record("canonical-payload", vec![string(input.canonical_payload_ref)]),
        record("canonicalizer", vec![string(input.canonicalizer)]),
        record("schemas", vec![refs_sequence(input.schema_refs)]),
        record("dependency-summaries", vec![refs_sequence(input.dependency_summary_refs)]),
        record("effects", vec![optional_ref_value(input.effect_manifest_ref)]),
        record("policy", vec![refs_sequence(input.policy_refs)]),
        record("provenance", vec![refs_sequence(input.provenance_refs)]),
        record("hash-algorithm", vec![string(input.hash_algorithm)]),
    ]))
}

fn artifact_identity_ref(input: &ArtifactIdentityInput<'_>) -> Result<String> {
    canonical_hash(&artifact_identity_subject_value(input)?)
}

fn artifact_identity_receipt_value(
    input: &ArtifactIdentityInput<'_>,
    decision: &str,
    artifact_ref: Option<&str>,
    diagnostics: &[String],
) -> Result<IoValue> {
    validate_refs(input.schema_refs, "artifact identity schema ref")?;
    validate_refs(input.dependency_summary_refs, "artifact identity dependency summary ref")?;
    if let Some(effect_manifest_ref) = input.effect_manifest_ref {
        validate_ref(effect_manifest_ref, "artifact identity effect manifest ref")?;
    }
    if let Some(artifact_ref) = artifact_ref {
        validate_ref(artifact_ref, "artifact identity artifact ref")?;
    }
    validate_refs(input.policy_refs, "artifact identity policy ref")?;
    validate_refs(input.provenance_refs, "artifact identity provenance ref")?;
    Ok(record("artifact-identity-receipt-v1", vec![
        string(crate::preserves_rail::ARTIFACT_IDENTITY_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("kind", vec![string(input.kind)]),
        record("domain", vec![string(input.identity_domain)]),
        record("canonicalizer", vec![string(input.canonicalizer)]),
        record("canonical-payload", vec![string(input.canonical_payload_ref)]),
        record("artifact-ref", vec![optional_ref_value(artifact_ref)]),
        record("schemas", vec![refs_sequence(input.schema_refs)]),
        record("dependency-summaries", vec![refs_sequence(input.dependency_summary_refs)]),
        record("effects", vec![optional_ref_value(input.effect_manifest_ref)]),
        record("policy", vec![refs_sequence(input.policy_refs)]),
        record("provenance", vec![refs_sequence(input.provenance_refs)]),
        record("hash-algorithm", vec![string(input.hash_algorithm)]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
        checks_value_from_pairs(&artifact_identity_checks(input)),
    ]))
}

fn artifact_identity_checks(input: &ArtifactIdentityInput<'_>) -> [(&'static str, &'static str); MAX_ARTIFACT_IDENTITY_CHECKS] {
    [
        ("supported-kind", check_status(SUPPORTED_ARTIFACT_KINDS.contains(&input.kind))),
        ("canonical-payload-ref", check_status(validate_ref(input.canonical_payload_ref, "artifact identity canonical payload ref").is_ok())),
        ("domain-separated-identity", check_status(input.identity_domain == domain_for_kind(input.kind))),
        ("blake3-identity", check_status(input.hash_algorithm == ARTIFACT_IDENTITY_HASH_ALGORITHM)),
        (
            "normalized-payload-boundary",
            check_status(!input.canonicalizer.is_empty()
                && !matches!(input.canonicalizer, RAW_SOURCE_CANONICALIZER | RENDERED_LOG_CANONICALIZER)),
        ),
        ("identity-is-not-authority", "pass"),
    ]
}

const MAX_ARTIFACT_IDENTITY_CHECKS: usize = 6;

fn check_status(condition: bool) -> &'static str {
    if condition {
        "pass"
    } else {
        "fail"
    }
}
