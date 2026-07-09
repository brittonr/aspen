const REPLAY_FRESHNESS_IDENTITY_SCHEMA: &str = "molten.determinism.replay-freshness.identity.v1";
const REPLAY_FRESHNESS_VALIDATION_SCHEMA: &str = "molten.determinism.replay-freshness.validation.v1";
const REPLAY_ROLLUP_LEGACY_FIELD_COUNT: usize = 10;
const REPLAY_ROLLUP_IDENTITY_FIELD_COUNT: usize = 11;
const REPLAY_ROLLUP_SCHEMA_INDEX: usize = 0;
const REPLAY_ROLLUP_DECISION_INDEX: usize = 1;
const REPLAY_ROLLUP_TOTAL_COUNT_INDEX: usize = 2;
const REPLAY_ROLLUP_PASS_COUNT_INDEX: usize = 3;
const REPLAY_ROLLUP_DENY_COUNT_INDEX: usize = 4;
const REPLAY_ROLLUP_RECEIPT_REFS_INDEX: usize = 5;
const REPLAY_ROLLUP_IDENTITY_REFS_INDEX: usize = 6;
const REPLAY_ROLLUP_LEGACY_DIVERGENCE_INDEX: usize = 6;
const REPLAY_ROLLUP_DIVERGENCE_INDEX: usize = 7;
const REPLAY_ROLLUP_LEGACY_FIRST_DIVERGENCE_INDEX: usize = 7;
const REPLAY_ROLLUP_FIRST_DIVERGENCE_INDEX: usize = 8;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayRunIdentity {
    pub artifact_ref: String,
    pub dependency_closure_ref: String,
    pub initial_state_ref: String,
    pub schema_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub revocation_refs: Vec<String>,
    pub handler_profile_ref: String,
    pub seed_or_effect_log_ref: String,
    pub runtime_refs: Vec<String>,
    pub tool_refs: Vec<String>,
    pub replay_profile: String,
}

#[derive(Clone, Debug)]
pub struct ReplayFreshnessInput {
    pub subject_ref: String,
    pub evidence_ref: String,
    pub expected_identity: ReplayRunIdentity,
    pub evidence_identity: ReplayRunIdentity,
}

#[derive(Clone, Debug)]
pub struct ReplayFreshnessReceipt {
    pub value: IoValue,
    pub freshness_ref: String,
    pub decision: String,
    pub expected_identity_ref: String,
    pub evidence_identity_ref: String,
    pub replay_profile: String,
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug)]
struct ParsedReplayVerify {
    receipt_ref: String,
    decision: String,
    divergence: String,
    first_divergence_ref: Option<String>,
    identity_refs: Vec<String>,
    report_refs: Vec<String>,
    final_state_refs: Vec<String>,
}

#[derive(Clone, Debug)]
struct ParsedReplayRollup {
    rollup_ref: String,
    decision: String,
    total_count: u64,
    pass_count: u64,
    deny_count: u64,
    receipt_refs: Vec<String>,
    identity_refs: Vec<String>,
    divergence_counts: OrderedMap<String, u64>,
    first_divergence_refs: Vec<String>,
}

pub fn validate_replay_freshness(input: ReplayFreshnessInput) -> Result<ReplayFreshnessReceipt> {
    validate_content_ref(&input.subject_ref)?;
    validate_content_ref(&input.evidence_ref)?;
    let expected_identity = replay_freshness_identity_value(input.expected_identity.clone())?;
    let evidence_identity = replay_freshness_identity_value(input.evidence_identity.clone())?;
    let expected_identity_ref = canonical_hash(&expected_identity)?;
    let evidence_identity_ref = canonical_hash(&evidence_identity)?;
    let diagnostics = first_freshness_diagnostic(&input.expected_identity, &input.evidence_identity)
        .into_iter()
        .collect::<Vec<_>>();
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = record("replay-freshness-validation-v1", vec![
        string(REPLAY_FRESHNESS_VALIDATION_SCHEMA),
        record("decision", vec![string(decision)]),
        record("subject-ref", vec![string(&input.subject_ref)]),
        record("evidence-ref", vec![string(&input.evidence_ref)]),
        record("expected-identity-ref", vec![string(&expected_identity_ref)]),
        record("evidence-identity-ref", vec![string(&evidence_identity_ref)]),
        record("replay-profile", vec![string(&input.expected_identity.replay_profile)]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
        sequence(freshness_checks(decision, diagnostics.is_empty())),
    ]);
    let freshness_ref = canonical_hash(&value)?;
    Ok(ReplayFreshnessReceipt {
        value,
        freshness_ref,
        decision: decision.to_string(),
        expected_identity_ref,
        evidence_identity_ref,
        replay_profile: input.expected_identity.replay_profile,
        diagnostics,
    })
}

pub fn validate_release_replay_freshness(input: ReplayFreshnessInput) -> Result<ReplayFreshnessReceipt> {
    validate_replay_freshness(input)
}

pub fn replay_freshness_catalog_terms(receipt: ReplayFreshnessReceipt) -> Vec<String> {
    let mut terms = vec![
        format!("replay-freshness:{}", receipt.decision),
        format!("freshness-ref:{}", receipt.freshness_ref),
        format!("expected-identity-ref:{}", receipt.expected_identity_ref),
        format!("evidence-identity-ref:{}", receipt.evidence_identity_ref),
        format!("replay-profile:{}", receipt.replay_profile),
        "evidence-only-no-authority".to_string(),
    ];
    for diagnostic in receipt.diagnostics {
        terms.push(format!("stale-component:{diagnostic}"));
    }
    terms
}

fn replay_freshness_identity_value(identity: ReplayRunIdentity) -> Result<IoValue> {
    validate_replay_identity(identity.clone())?;
    Ok(record("replay-run-identity-v1", vec![
        string(REPLAY_FRESHNESS_IDENTITY_SCHEMA),
        record("artifact-ref", vec![string(identity.artifact_ref)]),
        record("dependency-closure-ref", vec![string(identity.dependency_closure_ref)]),
        record("initial-state-ref", vec![string(identity.initial_state_ref)]),
        record("schema-refs", vec![sequence(identity.schema_refs.iter().map(string).collect())]),
        record("policy-refs", vec![sequence(identity.policy_refs.iter().map(string).collect())]),
        record("capability-refs", vec![sequence(identity.capability_refs.iter().map(string).collect())]),
        record("revocation-refs", vec![sequence(identity.revocation_refs.iter().map(string).collect())]),
        record("handler-profile-ref", vec![string(identity.handler_profile_ref)]),
        record("seed-or-effect-log-ref", vec![string(identity.seed_or_effect_log_ref)]),
        record("runtime-refs", vec![sequence(identity.runtime_refs.iter().map(string).collect())]),
        record("tool-refs", vec![sequence(identity.tool_refs.iter().map(string).collect())]),
        record("replay-profile", vec![string(identity.replay_profile)]),
    ]))
}

fn validate_replay_identity(identity: ReplayRunIdentity) -> Result<()> {
    validate_content_ref(&identity.artifact_ref)?;
    validate_content_ref(&identity.dependency_closure_ref)?;
    validate_content_ref(&identity.initial_state_ref)?;
    validate_ref_list(&identity.schema_refs, "schema refs")?;
    validate_ref_list(&identity.policy_refs, "policy refs")?;
    validate_ref_list(&identity.capability_refs, "capability refs")?;
    validate_ref_list(&identity.revocation_refs, "revocation refs")?;
    validate_content_ref(&identity.handler_profile_ref)?;
    validate_content_ref(&identity.seed_or_effect_log_ref)?;
    validate_ref_list(&identity.runtime_refs, "runtime refs")?;
    validate_ref_list(&identity.tool_refs, "tool refs")?;
    validate_replay_profile(&identity.replay_profile)
}

fn validate_ref_list(refs: &[String], label: &'static str) -> Result<()> {
    if refs.is_empty() {
        return Err(crate::error::MoltenError::invalid_harness(format!("{label} must not be empty")));
    }
    for reference in refs {
        validate_content_ref(reference)?;
    }
    Ok(())
}

fn validate_replay_profile(profile: &str) -> Result<()> {
    if profile.is_empty() {
        return Err(crate::error::MoltenError::invalid_harness("replay profile cannot be empty"));
    }
    if profile.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_') {
        Ok(())
    } else {
        Err(crate::error::MoltenError::invalid_harness("replay profile must be a lowercase ascii token"))
    }
}

fn first_freshness_diagnostic(expected: &ReplayRunIdentity, evidence: &ReplayRunIdentity) -> Option<String> {
    let checks = [
        ("artifact-ref", expected.artifact_ref == evidence.artifact_ref),
        (
            "dependency-closure-ref",
            expected.dependency_closure_ref == evidence.dependency_closure_ref,
        ),
        ("initial-state-ref", expected.initial_state_ref == evidence.initial_state_ref),
        ("schema-refs", expected.schema_refs == evidence.schema_refs),
        ("policy-refs", expected.policy_refs == evidence.policy_refs),
        ("capability-refs", expected.capability_refs == evidence.capability_refs),
        ("revocation-refs", expected.revocation_refs == evidence.revocation_refs),
        (
            "handler-profile-ref",
            expected.handler_profile_ref == evidence.handler_profile_ref,
        ),
        (
            "seed-or-effect-log-ref",
            expected.seed_or_effect_log_ref == evidence.seed_or_effect_log_ref,
        ),
        ("runtime-refs", expected.runtime_refs == evidence.runtime_refs),
        ("tool-refs", expected.tool_refs == evidence.tool_refs),
        ("replay-profile", expected.replay_profile == evidence.replay_profile),
    ];
    checks
        .into_iter()
        .find_map(|(component, matches)| (!matches).then(|| format!("{component} mismatch")))
}

fn freshness_checks(decision: &str, identities_match: bool) -> Vec<IoValue> {
    vec![
        record("check", vec![string("run-identity-bound"), string(if identities_match { "pass" } else { "deny" })]),
        record("check", vec![string("stale-replay-denied"), string(decision)]),
        record("check", vec![string("evidence-only-no-authority"), string("pass")]),
    ]
}
