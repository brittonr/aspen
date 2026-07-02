
fn install_job_execute_authority_context(
    registry: &Path,
    job_ref: &str,
    policy_refs: &[String],
    capability_refs: &[String],
) -> Result<String> {
    let subject_ref = dogfood_ref("target-peer-subject")?;
    let context_value = crate::authority::context_value(crate::authority::ContextValueInput {
        subject_ref: &subject_ref,
        capabilities: &[crate::authority::Capability {
            capability: "job:execute".to_string(),
            scope: job_ref.to_string(),
            attenuation: "scoped".to_string(),
        }],
        delegation_refs: &[],
        not_before: None,
        expires_at: None,
        revocation_refs: &[],
        key_refs: &[],
        policy_refs,
        evidence_refs: &[dogfood_ref("authority-evidence")?],
    })?;
    let context_ref = crate::preserves_rail::canonical_hash(&context_value)?;
    crate::artifacts::install_artifact(registry, &crate::artifacts::ArtifactInstallInput {
        kind: "authority-context".to_string(),
        payload: context_value,
        schema_refs: Vec::new(),
        dependency_refs: Vec::new(),
        effect_manifest_ref: None,
        policy_refs: policy_refs.to_vec(),
        evidence_refs: vec![dogfood_ref("authority-evidence")?],
        installer_ref: dogfood_ref("authority-installer")?,
        capability_refs: capability_refs.to_vec(),
    })?;
    Ok(context_ref)
}

fn install_clean_octet_gate(registry: &Path, policy_refs: &[String], capability_refs: &[String]) -> Result<String> {
    let gate_value = crate::octet_gate::synthetic_clean_octet_gate_receipt_for_tests()?;
    let gate_ref = crate::preserves_rail::canonical_hash(&gate_value)?;
    crate::artifacts::install_artifact(registry, &crate::artifacts::ArtifactInstallInput {
        kind: "octet-gate-receipt".to_string(),
        payload: gate_value,
        schema_refs: Vec::new(),
        dependency_refs: Vec::new(),
        effect_manifest_ref: None,
        policy_refs: policy_refs.to_vec(),
        evidence_refs: vec![dogfood_ref("octet-evidence")?],
        installer_ref: dogfood_ref("octet-installer")?,
        capability_refs: capability_refs.to_vec(),
    })?;
    Ok(gate_ref)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DogfoodRepro {
    report_ref: String,
    gate_ref: String,
    bundle_ref: String,
    verify_ref: String,
}

fn build_dogfood_repro() -> Result<DogfoodRepro> {
    let suite = crate::preserves_rail::parse_text(DOGFOOD_HARNESS_SUITE)?;
    let run = crate::harness::run_suite_value(&suite)?;
    let gate = crate::harness::receipt_value(&crate::harness::check_value(&run.report_value)?);
    let gate_ref = crate::preserves_rail::canonical_hash(&gate)?;
    let bundle = crate::harness::sealed_repro_bundle_value_with_command(&run.report_value, &[
        "molten".to_string(),
        "dogfood".to_string(),
        "local-node".to_string(),
    ])?;
    let bundle_ref = crate::preserves_rail::canonical_hash(&bundle)?;
    let verify = crate::harness::repro_verify_receipt_value(&bundle)?;
    let verify_ref = crate::preserves_rail::canonical_hash(&verify)?;
    Ok(DogfoodRepro {
        report_ref: run.report_ref,
        gate_ref,
        bundle_ref,
        verify_ref,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DogfoodEvidenceImportInput<'a> {
    ledger_root: &'a Path,
    workflow_value: &'a IoValue,
    step_values: &'a [IoValue],
    checkpoint_values: &'a [IoValue],
    report_value: &'a IoValue,
    release_gate_value: Option<&'a IoValue>,
    replay_verify_value: &'a IoValue,
    replay_index_value: &'a IoValue,
}

fn import_dogfood_evidence(input: DogfoodEvidenceImportInput<'_>) -> Result<Vec<String>> {
    let DogfoodEvidenceImportInput {
        ledger_root,
        workflow_value,
        step_values,
        checkpoint_values,
        report_value,
        release_gate_value,
        replay_verify_value,
        replay_index_value,
    } = input;
    let mut imports = Vec::new();
    for value in step_values
        .iter()
        .chain(checkpoint_values.iter())
        .chain(std::iter::once(workflow_value))
        .chain(std::iter::once(report_value))
        .chain(std::iter::once(replay_verify_value))
        .chain(std::iter::once(replay_index_value))
        .chain(release_gate_value)
    {
        let import = crate::ledger::import_artifact(ledger_root, value)?;
        imports.push_limited_value(
            crate::preserves_rail::canonical_hash(&import.receipt_value)?,
            MAX_OPERATOR_REFS,
            "dogfood ledger import refs",
        )?;
    }
    Ok(imports)
}

fn service_lifecycle_pass(value: &IoValue) -> bool {
    crate::service_records::parse_service_lifecycle_receipt(value).is_ok_and(|receipt| receipt.decision == "pass")
}

fn dirty_state_reason(path: &Path) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    if !path.is_dir() {
        return Ok(Some("dogfood state root exists but is not a directory".to_string()));
    }
    let mut entries = std::fs::read_dir(path).map_err(MoltenError::from)?;
    if entries.next().transpose().map_err(MoltenError::from)?.is_some() {
        Ok(Some("dogfood local-node requires a clean empty state root".to_string()))
    } else {
        Ok(None)
    }
}

fn dogfood_ref(label: &str) -> Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("operator-dogfood-ref", vec![
        crate::preserves_rail::string(label),
    ]))
}

fn validate_workflow_id(value: &str) -> Result<()> {
    validate_non_empty(value, "operator workflow id")?;
    if !value.starts_with("dogfood:") {
        return Err(MoltenError::invalid_harness(format!("operator workflow id {value} must start with dogfood:")));
    }
    Ok(())
}

fn validate_step_name(value: &str) -> Result<()> {
    validate_non_empty(value, "operator step name")?;
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_')
    {
        return Err(MoltenError::invalid_harness(format!("unsupported operator step name {value}")));
    }
    Ok(())
}

fn validate_decision(value: &str) -> Result<()> {
    match value {
        "pass" | "deny" | "diagnostic" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported operator decision {value}"))),
    }
}

fn validate_replay_status(value: &str) -> Result<()> {
    match value {
        "deterministic" | "recorded" | "diagnostic" | "non-replayable" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported operator replay status {value}"))),
    }
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}

fn validate_ref(value: &str, field: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
        .map_err(|error| MoltenError::invalid_harness(format!("{field} must be a canonical content ref: {error}")))
}

fn validate_optional_ref(value: Option<&str>, field: &str) -> Result<()> {
    if let Some(value) = value {
        validate_ref(value, field)
    } else {
        Ok(())
    }
}

fn validate_refs(values: &[String], field: &str) -> Result<()> {
    for value in values {
        validate_ref(value, field)?;
    }
    Ok(())
}

fn require_non_empty_refs(values: &[String], field: &str) -> Result<()> {
    if values.is_empty() {
        return Err(MoltenError::invalid_harness(format!("{field} must not be empty")));
    }
    validate_refs(values, field)
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count > maximum {
        Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds {maximum}")))
    } else {
        Ok(())
    }
}

trait PushLimited<T> {
    fn push_limited_value(&mut self, value: T, maximum: usize, label: &str) -> Result<()>;
}

impl<T> PushLimited<T> for Vec<T> {
    fn push_limited_value(&mut self, value: T, maximum: usize, label: &str) -> Result<()> {
        ensure_count_at_most(self.len().saturating_add(1), maximum, label)?;
        self.push(value);
        Ok(())
    }
}

fn status(value: bool) -> &'static str {
    if value { "pass" } else { "fail" }
}

fn refs_sequence(refs: &[String]) -> IoValue {
    crate::preserves_rail::sequence(refs.iter().map(crate::preserves_rail::string).collect())
}

fn strings_sequence(values: &[String]) -> IoValue {
    crate::preserves_rail::sequence(values.iter().map(crate::preserves_rail::string).collect())
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |value| crate::preserves_rail::record("some", vec![crate::preserves_rail::string(value)]),
    )
}

fn checks_value_from_pairs(checks: &[(&str, &str)]) -> IoValue {
    crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(
        checks
            .iter()
            .map(|(name, status)| {
                crate::preserves_rail::record("check", vec![
                    crate::preserves_rail::string(name),
                    crate::preserves_rail::string(status),
                ])
            })
            .collect(),
    )])
}

fn step_receipts_sequence(receipts: &[(String, String)]) -> IoValue {
    crate::preserves_rail::sequence(
        receipts
            .iter()
            .map(|(name, reference)| {
                crate::preserves_rail::record("step", vec![
                    crate::preserves_rail::string(name),
                    crate::preserves_rail::string(reference),
                ])
            })
            .collect(),
    )
}

fn file_refs_sequence(refs: &[(String, String)]) -> IoValue {
    crate::preserves_rail::sequence(
        refs.iter()
            .map(|(name, reference)| {
                crate::preserves_rail::record("file", vec![
                    crate::preserves_rail::string(name),
                    crate::preserves_rail::string(reference),
                ])
            })
            .collect(),
    )
}
