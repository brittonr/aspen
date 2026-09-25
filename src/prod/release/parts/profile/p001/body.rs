
fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn field_string(label: &'static str, value: &str) -> IoValue {
    record(label, vec![string(value)])
}

fn field_sequence(label: &'static str, values: Vec<IoValue>) -> IoValue {
    record(label, vec![crate::preserves_rail::sequence(values)])
}

fn string(value: &str) -> IoValue {
    crate::preserves_rail::string(value)
}

fn string_values(values: &[String]) -> Result<Vec<IoValue>> {
    ensure_diagnostic_bound(values.len())?;
    Ok(values.iter().map(|value| string(value)).collect())
}

fn bool_value(value: bool) -> IoValue {
    crate::preserves_rail::bool_value(value)
}

fn validate_ref(reference: &str, label: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(reference)
        .map_err(|error| MoltenError::invalid_harness(format!("invalid {label} ref {reference}: {error}")))
}

fn validate_text(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("{label} must not be empty")))
    } else {
        Ok(())
    }
}

fn ensure_ref_bound(count: usize, label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_REFS, label)
}

fn ensure_hash_bound(count: usize, label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_HASHES, label)
}

fn ensure_caveat_bound(count: usize, label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_CAVEATS, label)
}

fn ensure_diagnostic_bound(count: usize) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_DIAGNOSTICS, "release profile diagnostics")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local_ref(label: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
    }

    fn release_refs() -> ReleaseEvidenceRefs {
        ReleaseEvidenceRefs {
            source_gate_ref: Some(local_ref("source-gate")),
            policy_ref: Some(local_ref("policy")),
            octet_ref: Some(local_ref("octet")),
            cairn_ref: Some(local_ref("cairn")),
            stack_provenance_ref: Some(local_ref("stack-provenance")),
            production_profile_ref: Some(local_ref("production-profile")),
        }
    }

    fn release_input() -> ReleaseProfileInput {
        let generated = local_ref("generated-profile");
        ReleaseProfileInput {
            profile_id: "release-candidate".to_string(),
            tier: "release".to_string(),
            candidate_ref: Some(local_ref("candidate")),
            evidence_refs: release_refs(),
            freshness: ReleaseProfileFreshness {
                expected_generated_export_ref: Some(generated.clone()),
                actual_generated_export_ref: Some(generated),
            },
            stack_provenance_required: true,
            accepted_valence_policy_hashes: vec![
                "8f5174292fe31f8fc364dc8f49560b21581f2cf01e54ae3fe8820c6d90d62f65".to_string(),
            ],
            caveats: vec!["release review only".to_string()],
        }
    }

    // r[verify molten.prod_ops.release_profile.tiers]
    // r[verify molten.prod_ops.release_profile.no_placeholder_refs]
    // r[verify molten.prod_ops.release_profile.freshness]
    // r[verify molten.prod_ops.release_profile.fixtures]
    // r[verify molten.prod_ops.release_profile.candidate_binding]
    // r[verify molten.evidence.stack_provenance.release_required]
    // r[verify molten.evidence.stack_provenance.non_placeholder_hashes]
    #[test]
    fn release_profile_accepts_development_pilot_and_release_tiers() {
        let mut development = release_input();
        development.tier = "development".to_string();
        development.stack_provenance_required = false;
        development.evidence_refs = ReleaseEvidenceRefs {
            source_gate_ref: None,
            policy_ref: None,
            octet_ref: None,
            cairn_ref: None,
            stack_provenance_ref: None,
            production_profile_ref: None,
        };
        development.accepted_valence_policy_hashes.clear();
        assert_eq!(validate_release_profile(&development).expect("development").decision, DECISION_PASS);

        let mut pilot = release_input();
        pilot.tier = "pilot".to_string();
        pilot.stack_provenance_required = false;
        assert_eq!(validate_release_profile(&pilot).expect("pilot").decision, DECISION_PASS);

        let release = validate_release_profile(&release_input()).expect("release");
        assert_eq!(release.decision, DECISION_PASS);
        assert!(
            crate::preserves_rail::to_text(&release.value)
                .expect("release profile text")
                .contains("release-profile-validation-v1")
        );
    }

    #[test]
    fn release_profile_denies_missing_and_placeholder_candidate_refs() {
        let mut missing = release_input();
        missing.candidate_ref = None;
        let missing_validation = validate_release_profile(&missing).expect("missing candidate validation");
        assert_eq!(missing_validation.decision, DECISION_DENY);
        assert!(
            missing_validation
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "missing-release-candidate-ref")
        );

        let mut placeholder = release_input();
        placeholder.candidate_ref =
            Some("blake3:0000000000000000000000000000000000000000000000000000000000000000".to_string());
        let placeholder_validation = validate_release_profile(&placeholder).expect("placeholder candidate validation");
        assert_eq!(placeholder_validation.decision, DECISION_DENY);
        assert!(
            placeholder_validation
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "placeholder-release-candidate-ref")
        );
    }

    #[test]
    fn release_profile_denies_zero_dummy_and_optional_stack_provenance() {
        let mut input = release_input();
        input.evidence_refs.source_gate_ref =
            Some("blake3:0000000000000000000000000000000000000000000000000000000000000000".to_string());
        input.evidence_refs.policy_ref =
            Some("blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string());
        input.stack_provenance_required = false;
        input.accepted_valence_policy_hashes =
            vec!["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()];
        let validation = validate_release_profile(&input).expect("validation");
        assert_eq!(validation.decision, DECISION_DENY);
        assert!(validation.diagnostics.iter().any(|diagnostic| diagnostic == "placeholder-release-ref:source-gate"));
        assert!(validation.diagnostics.iter().any(|diagnostic| diagnostic == "placeholder-release-ref:policy"));
        assert!(validation.diagnostics.iter().any(|diagnostic| diagnostic == "release-stack-provenance-optional"));
        assert!(
            validation
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.starts_with("placeholder-valence-policy-hash"))
        );
    }

    #[test]
    fn release_profile_denies_stale_generated_export_and_missing_evidence() {
        let mut input = release_input();
        input.evidence_refs.octet_ref = None;
        input.freshness.actual_generated_export_ref = Some(local_ref("stale-generated-profile"));
        let validation = validate_release_profile(&input).expect("validation");
        assert_eq!(validation.decision, DECISION_DENY);
        assert!(validation.diagnostics.iter().any(|diagnostic| diagnostic == "missing-release-ref:octet"));
        assert!(validation.diagnostics.iter().any(|diagnostic| diagnostic.starts_with("stale-generated-profile:")));
    }

    #[test]
    fn release_profile_denies_unsupported_tier_and_missing_valence_policy_hash() {
        let mut input = release_input();
        input.tier = "production-ish".to_string();
        input.accepted_valence_policy_hashes.clear();
        let validation = validate_release_profile(&input).expect("validation");
        assert_eq!(validation.decision, DECISION_DENY);
        assert!(
            validation
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "unsupported-release-profile-tier:production-ish")
        );
    }
}
