
fn validate_candidate_evidence_bindings(
    label: &'static str,
    bindings: &[CandidateEvidenceBinding<'_>],
    expected_source_ref: &str,
    decision: &str,
) -> Result<()> {
    if bindings.len() > MAX_PROD_REFS {
        return Err(MoltenError::invalid_harness(format!(
            "production readiness {label} binding count {} exceeds bound {MAX_PROD_REFS}",
            bindings.len()
        )));
    }
    if is_pass(decision) && bindings.is_empty() {
        return Err(MoltenError::invalid_harness(format!(
            "passing production readiness receipt requires at least one {label} candidate evidence binding"
        )));
    }
    for binding in bindings {
        validate_content_ref(binding.artifact_ref).map_err(|error| {
            MoltenError::invalid_harness(format!(
                "invalid production readiness {label} artifact ref {}: {error}",
                binding.artifact_ref
            ))
        })?;
        validate_content_ref(binding.source_ref).map_err(|error| {
            MoltenError::invalid_harness(format!(
                "invalid production readiness {label} candidate source ref {}: {error}",
                binding.source_ref
            ))
        })?;
        if binding.source_ref != expected_source_ref {
            return Err(MoltenError::invalid_harness(format!(
                "production readiness {label} candidate source mismatch: expected {expected_source_ref}, observed {}",
                binding.source_ref
            )));
        }
    }
    Ok(())
}

fn candidate_evidence_field(label: &'static str, bindings: &[CandidateEvidenceBinding<'_>]) -> IoValue {
    let values = bindings
        .iter()
        .map(|binding| {
            record(
                "candidate-evidence",
                vec![string(binding.artifact_ref), string(binding.source_ref)],
            )
        })
        .collect();
    record(label, vec![sequence(values)])
}

pub fn release_candidate_gate_value(input: &ReleaseCandidateGateInput<'_>) -> Result<IoValue> {
    let gate = ReleaseCandidateGate::new(input);
    gate.validate()?;
    gate.value()
}

fn decision_field(decision: &str) -> IoValue {
    record("decision", vec![string(decision)])
}

fn diagnostics_field(values: &[String]) -> Result<IoValue> {
    Ok(record("diagnostics", vec![sequence(string_values("diagnostic", values)?)]))
}

fn refs_field(label: &'static str, refs: &[String]) -> Result<IoValue> {
    Ok(record(label, vec![sequence(ref_values(label, refs)?)]))
}

fn texts_field(label: &'static str, values: &[String]) -> Result<IoValue> {
    Ok(record(label, vec![sequence(string_values(label, values)?)]))
}

fn checks_field(checks: Vec<IoValue>) -> IoValue {
    record("checks", vec![sequence(checks)])
}

fn check_value(name: &'static str, status: &'static str) -> IoValue {
    record("check", vec![string(name), string(status)])
}

fn pass_check(is_failed: bool) -> &'static str {
    if is_failed { "deny" } else { "pass" }
}
