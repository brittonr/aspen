
pub(super) fn release_candidate_gate(command: super::super::Command) -> super::super::Outcome<super::Emission> {
    let super::super::Command::ReleaseCandidateGate {
        decision,
        candidate,
        source_ref,
        rust_validation_bindings,
        nextest_bindings,
        nix_check_bindings,
        cairn_validation_bindings,
        octet_bindings,
        dogfood_bindings,
        bundle_verify_bindings,
        promotion_bindings,
        export_verify_bindings,
        source_gate_status,
        source_gate_caveats,
        pilot_decision_bindings,
        diagnostics,
        out,
    } = command
    else {
        return Err(super::super::wrong_handler("release-candidate-gate"));
    };
    let rust_validation_bindings = parse_candidate_evidence_bindings("Rust validation", rust_validation_bindings)?;
    let nextest_bindings = parse_candidate_evidence_bindings("nextest", nextest_bindings)?;
    let nix_check_bindings = parse_candidate_evidence_bindings("Nix check", nix_check_bindings)?;
    let cairn_validation_bindings = parse_candidate_evidence_bindings("Cairn validation", cairn_validation_bindings)?;
    let octet_bindings = parse_candidate_evidence_bindings("Octet", octet_bindings)?;
    let dogfood_bindings = parse_candidate_evidence_bindings("dogfood", dogfood_bindings)?;
    let bundle_verify_bindings = parse_candidate_evidence_bindings("release bundle verify", bundle_verify_bindings)?;
    let promotion_bindings = parse_candidate_evidence_bindings("promotion", promotion_bindings)?;
    let export_verify_bindings = parse_candidate_evidence_bindings("export verify", export_verify_bindings)?;
    let pilot_decision_bindings = parse_candidate_evidence_bindings("pilot decision", pilot_decision_bindings)?;
    let rust_validation_evidence = borrow_candidate_evidence_bindings(&rust_validation_bindings);
    let nextest_evidence = borrow_candidate_evidence_bindings(&nextest_bindings);
    let nix_check_evidence = borrow_candidate_evidence_bindings(&nix_check_bindings);
    let cairn_validation_evidence = borrow_candidate_evidence_bindings(&cairn_validation_bindings);
    let octet_evidence = borrow_candidate_evidence_bindings(&octet_bindings);
    let dogfood_evidence = borrow_candidate_evidence_bindings(&dogfood_bindings);
    let bundle_verify_evidence = borrow_candidate_evidence_bindings(&bundle_verify_bindings);
    let promotion_evidence = borrow_candidate_evidence_bindings(&promotion_bindings);
    let export_verify_evidence = borrow_candidate_evidence_bindings(&export_verify_bindings);
    let pilot_decision_evidence = borrow_candidate_evidence_bindings(&pilot_decision_bindings);
    Ok(super::Emission {
        value: molten::prod_readiness::release_candidate_gate_value(
            &molten::prod_readiness::ReleaseCandidateGateInput {
                decision: &decision,
                candidate: &candidate,
                source_ref: &source_ref,
                rust_validation_evidence: &rust_validation_evidence,
                nextest_evidence: &nextest_evidence,
                nix_check_evidence: &nix_check_evidence,
                cairn_validation_evidence: &cairn_validation_evidence,
                octet_evidence: &octet_evidence,
                dogfood_evidence: &dogfood_evidence,
                bundle_verify_evidence: &bundle_verify_evidence,
                promotion_evidence: &promotion_evidence,
                export_verify_evidence: &export_verify_evidence,
                source_gate_status: &source_gate_status,
                source_gate_caveats: &source_gate_caveats,
                pilot_decision_evidence: &pilot_decision_evidence,
                diagnostics: &diagnostics,
            },
        )?,
        out,
        kind: "release-candidate-gate",
        subject: candidate,
        decision,
    })
}

struct OwnedCandidateEvidenceBinding {
    artifact_ref: String,
    source_ref: String,
}

fn parse_candidate_evidence_bindings(
    label: &'static str,
    values: Vec<String>,
) -> super::super::Outcome<Vec<OwnedCandidateEvidenceBinding>> {
    const BINDING_SEPARATOR: char = '@';
    values
        .into_iter()
        .map(|value| {
            let Some((artifact_ref, source_ref)) = value.split_once(BINDING_SEPARATOR) else {
                return Err(molten::error::MoltenError::invalid_harness(format!(
                    "production readiness {label} binding must use ARTIFACT_REF@SOURCE_REF"
                )));
            };
            if artifact_ref.trim().is_empty() || source_ref.trim().is_empty() {
                return Err(molten::error::MoltenError::invalid_harness(format!(
                    "production readiness {label} binding members must not be empty"
                )));
            }
            Ok(OwnedCandidateEvidenceBinding {
                artifact_ref: artifact_ref.to_string(),
                source_ref: source_ref.to_string(),
            })
        })
        .collect()
}

fn borrow_candidate_evidence_bindings(
    bindings: &[OwnedCandidateEvidenceBinding],
) -> Vec<molten::prod_readiness::CandidateEvidenceBinding<'_>> {
    bindings
        .iter()
        .map(|binding| molten::prod_readiness::CandidateEvidenceBinding {
            artifact_ref: &binding.artifact_ref,
            source_ref: &binding.source_ref,
        })
        .collect()
}
