
fn install_refs(
    input: &ArtifactInstallInput,
    artifact: &ArtifactRecord,
    identity_receipt_ref: &str,
    chunk_receipt_ref: Option<&String>,
) -> Result<Vec<String>> {
    let mut refs = Vec::new();
    push_bounded(&mut refs, artifact.artifact_ref.clone(), MAX_ARTIFACT_REF_LIST, "artifact install refs")?;
    push_bounded(&mut refs, identity_receipt_ref.to_string(), MAX_ARTIFACT_REF_LIST, "artifact install refs")?;
    push_bounded(&mut refs, input.installer_ref.clone(), MAX_ARTIFACT_REF_LIST, "artifact install refs")?;
    extend_cloned_bounded(&mut refs, &input.capability_refs, MAX_ARTIFACT_REF_LIST, "artifact install refs")?;
    extend_cloned_bounded(&mut refs, &input.dependency_refs, MAX_ARTIFACT_REF_LIST, "artifact install refs")?;
    extend_cloned_bounded(&mut refs, &input.schema_refs, MAX_ARTIFACT_REF_LIST, "artifact install refs")?;
    extend_cloned_bounded(&mut refs, &input.policy_refs, MAX_ARTIFACT_REF_LIST, "artifact install refs")?;
    extend_cloned_bounded(&mut refs, &input.evidence_refs, MAX_ARTIFACT_REF_LIST, "artifact install refs")?;
    if let Some(effect_manifest_ref) = input.effect_manifest_ref.as_ref() {
        push_bounded(&mut refs, effect_manifest_ref.clone(), MAX_ARTIFACT_REF_LIST, "artifact install refs")?;
    }
    if let Some(chunk_receipt_ref) = chunk_receipt_ref {
        push_bounded(&mut refs, chunk_receipt_ref.clone(), MAX_ARTIFACT_REF_LIST, "artifact install refs")?;
    }
    Ok(refs)
}

fn install_diagnostics(missing_dependencies: &[String]) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    for dependency in missing_dependencies {
        push_bounded(
            &mut diagnostics,
            format!("missing dependency {dependency}"),
            MAX_ARTIFACT_DIAGNOSTICS,
            "artifact install diagnostics",
        )?;
    }
    Ok(diagnostics)
}

fn install_receipt_value(
    artifact: &ArtifactRecord,
    decision: &str,
    refs: &[String],
    diagnostics: &[String],
    missing_dependencies: &[String],
) -> Result<IoValue> {
    artifact_receipt_value(&ArtifactReceiptValueInput {
        operation: "install",
        decision,
        subject_ref: &artifact.artifact_ref,
        name: None,
        refs,
        diagnostics,
        checks: &[
            ("domain-separated-identity", "pass"),
            ("canonical-payload-ref", "pass"),
            ("dependency-closure", dependency_check(missing_dependencies)),
            ("policy-admission", "pass"),
            ("capability-admission", "pass"),
            ("names-are-metadata", "pass"),
        ],
    })
}

fn dependency_check(missing_dependencies: &[String]) -> &'static str {
    if missing_dependencies.is_empty() {
        "pass"
    } else {
        "fail"
    }
}
