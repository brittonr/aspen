type IoValue = preserves::IOValue;
type Result<T> = crate::error::Result<T>;
type MoltenError = crate::error::MoltenError;
type OrderedSet<T> = std::collections::BTreeSet<T>;

const RELEASE_PROFILE_SCHEMA: &str = "molten.prod-ops.release-profile-validation.v1";
const STACK_PROVENANCE_SCHEMA: &str = "molten.evidence.stack-provenance-release-gate.v1";
const DECISION_PASS: &str = "pass";
const DECISION_DENY: &str = "deny";
const MAX_REFS: usize = 128;
const MAX_HASHES: usize = 64;
const MAX_CAVEATS: usize = 128;
const MAX_DIAGNOSTICS: usize = 4096;
const BLAKE3_PREFIX: &str = "blake3:";
const BLAKE3_DIGEST_HEX_LEN: usize = 64;
const ZERO_DIGIT: char = '0';
const DUMMY_A_DIGIT: char = 'a';
const DUMMY_F_DIGIT: char = 'f';
const FIXTURE_MARKER: &str = "fixture";
const PLACEHOLDER_MARKER: &str = "placeholder";
const EVIDENCE_ONLY_CAVEAT: &str = "release profile validation is release-review evidence only and does not grant runtime authority, policy admission, provenance trust, source-gate acceptance, resource rights, transport trust, retention clearance, destructive-operation permission, deployment trust, or release eligibility by itself";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseEvidenceRefs {
    pub source_gate_ref: Option<String>,
    pub policy_ref: Option<String>,
    pub octet_ref: Option<String>,
    pub cairn_ref: Option<String>,
    pub stack_provenance_ref: Option<String>,
    pub production_profile_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseProfileFreshness {
    pub expected_generated_export_ref: Option<String>,
    pub actual_generated_export_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseProfileInput {
    pub profile_id: String,
    pub tier: String,
    pub evidence_refs: ReleaseEvidenceRefs,
    pub freshness: ReleaseProfileFreshness,
    pub stack_provenance_required: bool,
    pub accepted_valence_policy_hashes: Vec<String>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseProfileValidation {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub validation_ref: String,
    pub value: IoValue,
}

// r[impl molten.prod_ops.release_profile.tiers]
// r[impl molten.prod_ops.release_profile.no_placeholder_refs]
// r[impl molten.prod_ops.release_profile.freshness]
// r[impl molten.prod_ops.release_profile.fixtures]
// r[impl molten.evidence.stack_provenance.release_required]
// r[impl molten.evidence.stack_provenance.non_placeholder_hashes]
pub fn validate_release_profile(input: &ReleaseProfileInput) -> Result<ReleaseProfileValidation> {
    validate_text("release profile id", &input.profile_id)?;
    ensure_ref_bound(optional_ref_count(&input.evidence_refs), "release evidence refs")?;
    ensure_hash_bound(input.accepted_valence_policy_hashes.len(), "accepted valence policy hashes")?;
    ensure_caveat_bound(input.caveats.len(), "release profile caveats")?;
    let mut diagnostics = Vec::new();
    validate_tier(&input.tier, &mut diagnostics)?;
    validate_caveats(&input.caveats)?;
    validate_refs(input, &mut diagnostics);
    validate_stack_provenance(input, &mut diagnostics);
    validate_freshness(input, &mut diagnostics);
    validate_valence_policy_hashes(input, &mut diagnostics)?;
    diagnostics.sort();
    diagnostics.dedup();
    ensure_diagnostic_bound(diagnostics.len())?;
    let decision = if diagnostics.is_empty() {
        DECISION_PASS
    } else {
        DECISION_DENY
    };
    let value = release_profile_value(input, decision, &diagnostics)?;
    let validation_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(ReleaseProfileValidation {
        decision: decision.to_string(),
        diagnostics,
        validation_ref,
        value,
    })
}

fn validate_tier(tier: &str, diagnostics: &mut Vec<String>) -> Result<()> {
    match tier {
        "development" | "pilot" | "release" => Ok(()),
        other => {
            diagnostics.push(format!("unsupported-release-profile-tier:{other}"));
            Ok(())
        }
    }
}

fn validate_refs(input: &ReleaseProfileInput, diagnostics: &mut Vec<String>) {
    let release = input.tier == "release";
    for (field, reference) in evidence_ref_pairs(&input.evidence_refs) {
        match reference {
            Some(reference) => {
                validate_ref_with_diagnostics(field, reference, diagnostics);
                if release && is_placeholder_ref(reference) {
                    diagnostics.push(format!("placeholder-release-ref:{field}"));
                }
            }
            None if release => diagnostics.push(format!("missing-release-ref:{field}")),
            None => {}
        }
    }
}

fn validate_stack_provenance(input: &ReleaseProfileInput, diagnostics: &mut Vec<String>) {
    if input.tier != "release" {
        return;
    }
    if !input.stack_provenance_required {
        diagnostics.push("release-stack-provenance-optional".to_string());
    }
    if input.evidence_refs.stack_provenance_ref.is_none() {
        diagnostics.push("missing-release-stack-provenance".to_string());
    }
}

fn validate_freshness(input: &ReleaseProfileInput, diagnostics: &mut Vec<String>) {
    if input.tier != "release" {
        return;
    }
    match (
        input.freshness.expected_generated_export_ref.as_ref(),
        input.freshness.actual_generated_export_ref.as_ref(),
    ) {
        (Some(expected), Some(actual)) => {
            validate_ref_with_diagnostics("expected-generated-export", expected, diagnostics);
            validate_ref_with_diagnostics("actual-generated-export", actual, diagnostics);
            if expected != actual {
                diagnostics.push(format!("stale-generated-profile:expected={expected}:actual={actual}"));
            }
            if is_placeholder_ref(expected) || is_placeholder_ref(actual) {
                diagnostics.push("placeholder-generated-profile-ref".to_string());
            }
        }
        (None, _) => diagnostics.push("missing-expected-generated-export-ref".to_string()),
        (_, None) => diagnostics.push("missing-actual-generated-export-ref".to_string()),
    }
}

fn validate_valence_policy_hashes(input: &ReleaseProfileInput, diagnostics: &mut Vec<String>) -> Result<()> {
    let mut seen = OrderedSet::new();
    if input.tier == "release" && input.accepted_valence_policy_hashes.is_empty() {
        diagnostics.push("missing-valence-policy-hash".to_string());
    }
    for hash in &input.accepted_valence_policy_hashes {
        validate_text("valence policy hash", hash)?;
        if !seen.insert(hash.clone()) {
            diagnostics.push(format!("duplicate-valence-policy-hash:{hash}"));
        }
        if is_placeholder_hash(hash) {
            diagnostics.push(format!("placeholder-valence-policy-hash:{hash}"));
        }
    }
    Ok(())
}

fn validate_caveats(caveats: &[String]) -> Result<()> {
    for caveat in caveats {
        validate_text("release profile caveat", caveat)?;
    }
    Ok(())
}

fn evidence_ref_pairs(refs: &ReleaseEvidenceRefs) -> [(&'static str, Option<&String>); 6] {
    [
        ("source-gate", refs.source_gate_ref.as_ref()),
        ("policy", refs.policy_ref.as_ref()),
        ("octet", refs.octet_ref.as_ref()),
        ("cairn", refs.cairn_ref.as_ref()),
        ("stack-provenance", refs.stack_provenance_ref.as_ref()),
        ("production-profile", refs.production_profile_ref.as_ref()),
    ]
}

fn optional_ref_count(refs: &ReleaseEvidenceRefs) -> usize {
    evidence_ref_pairs(refs).iter().filter(|(_, reference)| reference.is_some()).count()
}

fn is_placeholder_ref(reference: &str) -> bool {
    if !reference.starts_with(BLAKE3_PREFIX) {
        return true;
    }
    let digest = &reference[BLAKE3_PREFIX.len()..];
    is_placeholder_hash(digest) || contains_placeholder_marker(reference)
}

fn is_placeholder_hash(hash: &str) -> bool {
    hash.len() != BLAKE3_DIGEST_HEX_LEN
        || all_same(hash, ZERO_DIGIT)
        || all_same(hash, DUMMY_A_DIGIT)
        || all_same(hash, DUMMY_F_DIGIT)
        || contains_placeholder_marker(hash)
}

fn all_same(value: &str, needle: char) -> bool {
    value.chars().all(|character| character == needle)
}

fn contains_placeholder_marker(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains(FIXTURE_MARKER) || lower.contains(PLACEHOLDER_MARKER)
}

fn validate_ref_with_diagnostics(label: &str, reference: &str, diagnostics: &mut Vec<String>) {
    if let Err(error) = validate_ref(reference, label) {
        diagnostics.push(format!("stale-release-ref:{label}:{reference}:{error}"));
    }
}

fn release_profile_value(input: &ReleaseProfileInput, decision: &str, diagnostics: &[String]) -> Result<IoValue> {
    Ok(record("release-profile-validation-v1", vec![
        string(RELEASE_PROFILE_SCHEMA),
        string(STACK_PROVENANCE_SCHEMA),
        field_string("decision", decision),
        field_string("profile-id", &input.profile_id),
        field_string("tier", &input.tier),
        evidence_refs_value(&input.evidence_refs),
        freshness_value(&input.freshness),
        record("stack-provenance-required", vec![bool_value(input.stack_provenance_required)]),
        field_sequence("accepted-valence-policy-hashes", string_values(&input.accepted_valence_policy_hashes)?),
        field_sequence("diagnostics", string_values(diagnostics)?),
        field_sequence("caveats", string_values(&release_caveats(&input.caveats))?),
    ]))
}

fn evidence_refs_value(refs: &ReleaseEvidenceRefs) -> IoValue {
    record(
        "evidence-refs",
        evidence_ref_pairs(refs)
            .iter()
            .map(|(field, reference)| field_string(field, reference.map(String::as_str).unwrap_or("none")))
            .collect(),
    )
}

fn freshness_value(freshness: &ReleaseProfileFreshness) -> IoValue {
    record("freshness", vec![
        field_string(
            "expected-generated-export-ref",
            freshness.expected_generated_export_ref.as_deref().unwrap_or("none"),
        ),
        field_string("actual-generated-export-ref", freshness.actual_generated_export_ref.as_deref().unwrap_or("none")),
    ])
}

fn release_caveats(caveats: &[String]) -> Vec<String> {
    let mut output = caveats.to_vec();
    output.push(EVIDENCE_ONLY_CAVEAT.to_string());
    output
}

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
