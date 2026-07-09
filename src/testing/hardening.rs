type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;
type OrderedMap<K, V> = std::collections::BTreeMap<K, V>;
type OrderedSet<T> = std::collections::BTreeSet<T>;

const DECISION_PASS: &str = "pass";
const DECISION_DENY: &str = "deny";
const EVIDENCE_ONLY_CAVEAT: &str = "receipt is evidence-only and does not grant authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation, deployment, or release trust";
const DIAGNOSTIC_VIEW_CAVEAT: &str =
    "rendered text, JUnit, JSON, markdown, and terminal output are diagnostic views over canonical artifacts";
const BOUNDARY_COVERAGE_GATE_SCHEMA: &str = "molten.testing.boundary-coverage-gate.v1";
const EVIDENCE_MATRIX_SCHEMA: &str = "molten.testing.evidence-matrix.v1";
const CI_TEST_RUN_RECEIPT_SCHEMA: &str = "molten.testing.ci-test-run-receipt.v1";
const TAMPER_MATRIX_SCHEMA: &str = "molten.testing.tamper-negative-matrix.v1";
const HEGEL_COUNTEREXAMPLE_SCHEMA: &str = "molten.testing.hegel-counterexample-fixture.v1";
const HEGEL_PROMOTION_SCHEMA: &str = "molten.testing.hegel-counterexample-promotion.v1";
const REPLAY_SMOKE_SCHEMA: &str = "molten.testing.replay-smoke-gate.v1";
const NEXTEST_PROFILE_MATRIX_SCHEMA: &str = "molten.testing.nextest-profile-matrix.v1";
const CLI_RECEIPT_FIRST_SCHEMA: &str = "molten.testing.cli-receipt-first-gate.v1";
const MAX_ITEMS: usize = 4096;
const MAX_REFS: usize = 256;
const MINIMUM_CI_TOTAL_FOR_PASS: u64 = 1;
const ZERO_COUNT: u64 = 0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundaryObservationInput {
    pub class: String,
    pub polarity: String,
    pub requirement_id: String,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundaryRequirementInput {
    pub class: String,
    pub polarity: String,
    pub requirement_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundaryCoverageExemptionInput {
    pub class: String,
    pub reason: String,
    pub evidence_ref: String,
    pub scope: String,
    pub caveat: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundaryCoverageGateInput {
    pub report_ref: String,
    pub suite_ref: String,
    pub required: Vec<BoundaryRequirementInput>,
    pub observed: Vec<BoundaryObservationInput>,
    pub exemptions: Vec<BoundaryCoverageExemptionInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundaryCoverageGate {
    pub decision: String,
    pub observed_classes: Vec<String>,
    pub missing_classes: Vec<String>,
    pub diagnostics: Vec<String>,
    pub gate_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceMatrixEntryInput {
    pub requirement_id: String,
    pub coverage_kind: String,
    pub evidence_scope: String,
    pub target: String,
    pub command: String,
    pub artifact_refs: Vec<String>,
    pub receipt_ref: Option<String>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceMatrixExemptionInput {
    pub requirement_id: String,
    pub reason: String,
    pub evidence_ref: String,
    pub scope: String,
    pub review_note: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceMatrixInput {
    pub requirements: Vec<crate::requirement_traceability::RequirementInput>,
    pub entries: Vec<EvidenceMatrixEntryInput>,
    pub exemptions: Vec<EvidenceMatrixExemptionInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceMatrixManifest {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub missing_positive: Vec<String>,
    pub missing_negative: Vec<String>,
    pub manifest_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiTestCounts {
    pub total: u64,
    pub passed: u64,
    pub failed: u64,
    pub skipped: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiTestRunInput {
    pub source_ref: String,
    pub profile_id: String,
    pub command_surface: String,
    pub nextest_config_ref: String,
    pub cargo_metadata_ref: String,
    pub binaries_metadata_ref: String,
    pub junit_ref: String,
    pub counts: CiTestCounts,
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiTestRunReceipt {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TamperFamilyInput {
    pub family: String,
    pub control_ref: String,
    pub parser: String,
    pub gate: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TamperCaseInput {
    pub family: String,
    pub mutation: String,
    pub fixture_ref: String,
    pub expected_diagnostic: String,
    pub decision: String,
    pub pass_evidence_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TamperMatrixInput {
    pub subject_ref: String,
    pub families: Vec<TamperFamilyInput>,
    pub cases: Vec<TamperCaseInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TamperMatrix {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub generated_cases: Vec<TamperCaseInput>,
    pub matrix_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HegelCounterexampleInput {
    pub property_id: String,
    pub requirement_ids: Vec<String>,
    pub generator_profile_ref: String,
    pub generation_seed: String,
    pub shrink_path: Vec<String>,
    pub shrunk_input_ref: String,
    pub replay_identity_ref: String,
    pub trace_refs: Vec<String>,
    pub receipt_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub confidentiality: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HegelCounterexampleFixture {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub fixture_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HegelPromotionInput {
    pub source_fixture_ref: String,
    pub new_suite_entry_ref: String,
    pub review_ref: String,
    pub property_id: String,
    pub reason: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaySmokeRunInput {
    pub role: String,
    pub report_ref: String,
    pub final_state_ref: String,
    pub effect_log_ref: String,
    pub trace_ref: String,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaySmokeInput {
    pub suite_id: String,
    pub eligibility: String,
    pub runs: Vec<ReplaySmokeRunInput>,
    pub variance: Vec<String>,
    pub diagnostic_caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaySmokeGate {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub gate_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticProfileInput {
    pub profile_id: String,
    pub evidence_scope: String,
    pub command_surface: String,
    pub retry_policy: String,
    pub expected_artifacts: Vec<String>,
    pub cost_class: String,
    pub caveats: Vec<String>,
    pub platform_required: bool,
    pub platform_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextestProfileMatrixInput {
    pub profiles: Vec<SemanticProfileInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextestProfileMatrix {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub matrix_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliReceiptFirstInput {
    pub command: String,
    pub evidence_bearing: bool,
    pub canonical_artifact_refs: Vec<String>,
    pub rendered_output_kinds: Vec<String>,
    pub negative_case: bool,
    pub failure_artifact_ref: Option<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliReceiptFirstGate {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub gate_ref: String,
    pub value: IoValue,
}

// r[impl molten.testing.boundary_coverage.gate]
// r[impl molten.testing.boundary_coverage.positive_negative]
// r[impl molten.testing.boundary_coverage.exemptions]
// r[impl molten.testing.evidence_matrix.checked_in_manifest]
// r[impl molten.testing.evidence_matrix.changed_requirement_gate]
// r[impl molten.testing.evidence_matrix.receipt_backed_entries]
// r[impl molten.testing.evidence_matrix.exemptions]
// r[impl molten.testing.ci_run_receipt.canonical_receipt]
// r[impl molten.testing.ci_run_receipt.junit_view_only]
// r[impl molten.testing.ci_run_receipt.nix_nextest_binding]
// r[impl molten.testing.ci_run_receipt.deny_on_missing_metadata]
// r[impl molten.testing.tamper_matrix.generated_cases]
// r[impl molten.testing.tamper_matrix.coverage]
// r[impl molten.testing.tamper_matrix.fail_closed]
// r[impl molten.testing.hegel_counterexample.replay_fixture]
// r[impl molten.testing.hegel_counterexample.promotion]
// r[impl molten.testing.hegel_counterexample.redaction]
// r[impl molten.testing.replay_smoke.all_evidence_suites]
// r[impl molten.testing.replay_smoke.fresh_rerun]
// r[impl molten.testing.replay_smoke.non_replayable_excluded]
// r[impl molten.testing.nextest.semantic_profiles]
// r[impl molten.testing.nextest.risk_scope]
// r[impl molten.testing.nextest.nix_outputs]
// r[impl molten.testing.nextest.exploratory_exclusion]
// r[impl molten.testing.cli_receipt_first.normative_artifacts]
// r[impl molten.testing.cli_receipt_first.stdout_diagnostic_only]
// r[impl molten.testing.cli_receipt_first.negative_fail_closed]
pub fn build_boundary_coverage_gate(input: &BoundaryCoverageGateInput) -> Result<BoundaryCoverageGate> {
    validate_ref(&input.report_ref, "boundary report")?;
    validate_ref(&input.suite_ref, "boundary suite")?;
    ensure_bound(input.required.len(), "boundary requirements")?;
    ensure_bound(input.observed.len(), "boundary observations")?;
    ensure_bound(input.exemptions.len(), "boundary exemptions")?;
    let mut diagnostics = Vec::new();
    let mut observed = OrderedSet::new();
    for item in &input.observed {
        validate_boundary_observation(item, &mut diagnostics)?;
        observed.insert(boundary_key(&item.class, &item.polarity));
    }
    let mut exemptions = OrderedSet::new();
    for item in &input.exemptions {
        validate_boundary_exemption(item, &mut diagnostics)?;
        exemptions.insert(item.class.clone());
    }
    let mut missing = Vec::new();
    for requirement in &input.required {
        validate_boundary_requirement(requirement)?;
        let key = boundary_key(&requirement.class, &requirement.polarity);
        if observed.contains(&key) || exemptions.contains(&requirement.class) {
            continue;
        }
        missing.push(key.clone());
        diagnostics.push(format!("missing-boundary:{key}:{}", requirement.requirement_id));
    }
    diagnostics.sort();
    diagnostics.dedup();
    let observed_classes = observed.into_iter().collect::<Vec<_>>();
    let decision = decision_for(&diagnostics);
    let value = boundary_gate_value(input, decision, &observed_classes, &missing, &diagnostics)?;
    let gate_ref = hash(&value)?;
    Ok(BoundaryCoverageGate {
        decision: decision.to_string(),
        observed_classes,
        missing_classes: missing,
        diagnostics,
        gate_ref,
        value,
    })
}

pub fn build_evidence_matrix_manifest(input: &EvidenceMatrixInput) -> Result<EvidenceMatrixManifest> {
    ensure_bound(input.requirements.len(), "matrix requirements")?;
    ensure_bound(input.entries.len(), "matrix entries")?;
    ensure_bound(input.exemptions.len(), "matrix exemptions")?;
    let requirement_map = requirement_map(&input.requirements)?;
    let mut diagnostics = Vec::new();
    let mut duplicate_keys = OrderedSet::new();
    let mut positive = OrderedSet::new();
    let mut negative = OrderedSet::new();
    for entry in &input.entries {
        validate_matrix_entry(entry, &requirement_map, &mut diagnostics)?;
        let key = format!("{}|{}|{}|{}", entry.requirement_id, entry.coverage_kind, entry.evidence_scope, entry.target);
        if !duplicate_keys.insert(key.clone()) {
            diagnostics.push(format!("duplicate-entry:{key}"));
        }
        match entry.coverage_kind.as_str() {
            "positive" => {
                positive.insert(entry.requirement_id.clone());
            }
            "negative" => {
                negative.insert(entry.requirement_id.clone());
            }
            _ => {}
        }
    }
    let mut exempt = OrderedSet::new();
    for exemption in &input.exemptions {
        validate_matrix_exemption(exemption, &requirement_map, &mut diagnostics)?;
        exempt.insert(exemption.requirement_id.clone());
    }
    let mut missing_positive = Vec::new();
    let mut missing_negative = Vec::new();
    for requirement in requirement_map.values() {
        if !requires_matrix_coverage(requirement) || exempt.contains(&requirement.id) {
            continue;
        }
        if !positive.contains(&requirement.id) {
            missing_positive.push(requirement.id.clone());
            diagnostics.push(format!("missing-positive:{}", requirement.id));
        }
        if !negative.contains(&requirement.id) {
            missing_negative.push(requirement.id.clone());
            diagnostics.push(format!("missing-negative:{}", requirement.id));
        }
    }
    diagnostics.sort();
    diagnostics.dedup();
    let decision = decision_for(&diagnostics);
    let value = evidence_matrix_value(input, decision, &missing_positive, &missing_negative, &diagnostics)?;
    let manifest_ref = hash(&value)?;
    Ok(EvidenceMatrixManifest {
        decision: decision.to_string(),
        diagnostics,
        missing_positive,
        missing_negative,
        manifest_ref,
        value,
    })
}

pub fn build_ci_test_run_receipt(input: &CiTestRunInput) -> Result<CiTestRunReceipt> {
    validate_ci_input(input)?;
    let mut diagnostics = input.diagnostics.clone();
    ci_diagnostics(input, &mut diagnostics)?;
    diagnostics.sort();
    diagnostics.dedup();
    let decision = if diagnostics.is_empty() {
        input.decision.as_str()
    } else {
        DECISION_DENY
    };
    let value = ci_test_run_value(input, decision, &diagnostics)?;
    let receipt_ref = hash(&value)?;
    Ok(CiTestRunReceipt {
        decision: decision.to_string(),
        diagnostics,
        receipt_ref,
        value,
    })
}

pub fn build_tamper_matrix(input: &TamperMatrixInput) -> Result<TamperMatrix> {
    validate_ref(&input.subject_ref, "tamper subject")?;
    ensure_bound(input.families.len(), "tamper families")?;
    ensure_bound(input.cases.len(), "tamper cases")?;
    let mut diagnostics = Vec::new();
    let families = family_map(&input.families, &mut diagnostics)?;
    let mut seen = OrderedSet::new();
    for case in &input.cases {
        validate_tamper_case(case, &families, &mut diagnostics)?;
        let key = format!("{}|{}", case.family, case.mutation);
        if !seen.insert(key.clone()) {
            diagnostics.push(format!("duplicate-tamper-case:{key}"));
        }
    }
    for family in families.keys() {
        for mutation in required_tamper_mutations() {
            let key = format!("{family}|{mutation}");
            if !seen.contains(&key) {
                diagnostics.push(format!("missing-tamper-case:{key}"));
            }
        }
    }
    diagnostics.sort();
    diagnostics.dedup();
    let decision = decision_for(&diagnostics);
    let value = tamper_matrix_value(input, decision, &diagnostics)?;
    let matrix_ref = hash(&value)?;
    Ok(TamperMatrix {
        decision: decision.to_string(),
        diagnostics,
        generated_cases: input.cases.clone(),
        matrix_ref,
        value,
    })
}

pub fn build_hegel_counterexample_fixture(input: &HegelCounterexampleInput) -> Result<HegelCounterexampleFixture> {
    let mut validation_diagnostics = Vec::new();
    validate_text("hegel property id", &input.property_id)?;
    validate_requirement_ids(&input.requirement_ids)?;
    validate_ref(&input.generator_profile_ref, "hegel generator profile")?;
    validate_ref(&input.shrunk_input_ref, "hegel shrunk input")?;
    validate_ref(&input.replay_identity_ref, "hegel replay identity")?;
    validate_ref_list("hegel trace refs", &input.trace_refs)?;
    validate_ref_list("hegel receipt refs", &input.receipt_refs)?;
    validate_text("hegel confidentiality", &input.confidentiality)?;
    if input.generation_seed.trim().is_empty() {
        validation_diagnostics.push("missing-seed".to_string());
    }
    if input.shrink_path.is_empty() {
        validation_diagnostics.push("missing-shrink-path".to_string());
    }
    if input.diagnostics.is_empty() {
        validation_diagnostics.push("missing-diagnostics".to_string());
    }
    if input.confidentiality == "sensitive" {
        validation_diagnostics.push("sensitive-input-not-redacted".to_string());
    }
    validation_diagnostics.sort();
    validation_diagnostics.dedup();
    let decision = decision_for(&validation_diagnostics);
    let mut output_diagnostics = validation_diagnostics;
    output_diagnostics.extend(input.diagnostics.clone());
    output_diagnostics.sort();
    output_diagnostics.dedup();
    let value = hegel_fixture_value(input, decision, &output_diagnostics)?;
    let fixture_ref = hash(&value)?;
    Ok(HegelCounterexampleFixture {
        decision: decision.to_string(),
        diagnostics: output_diagnostics,
        fixture_ref,
        value,
    })
}

pub fn build_hegel_promotion_record(input: &HegelPromotionInput) -> Result<CiTestRunReceipt> {
    validate_ref(&input.source_fixture_ref, "hegel source fixture")?;
    validate_ref(&input.new_suite_entry_ref, "hegel new suite entry")?;
    validate_ref(&input.review_ref, "hegel review")?;
    validate_text("hegel property id", &input.property_id)?;
    validate_text("hegel reason", &input.reason)?;
    let mut diagnostics = Vec::new();
    if !matches!(input.status.as_str(), "regression-pass" | "known-deny") {
        diagnostics.push(format!("unsupported-promotion-status:{}", input.status));
    }
    let decision = decision_for(&diagnostics);
    let value = hegel_promotion_value(input, decision, &diagnostics)?;
    let receipt_ref = hash(&value)?;
    Ok(CiTestRunReceipt {
        decision: decision.to_string(),
        diagnostics,
        receipt_ref,
        value,
    })
}

pub fn build_replay_smoke_gate(input: &ReplaySmokeInput) -> Result<ReplaySmokeGate> {
    validate_text("replay smoke suite id", &input.suite_id)?;
    validate_replay_eligibility(&input.eligibility)?;
    ensure_bound(input.runs.len(), "replay smoke runs")?;
    let mut diagnostics = Vec::new();
    for variance in &input.variance {
        validate_variance(variance)?;
    }
    for caveat in &input.diagnostic_caveats {
        validate_text("replay smoke caveat", caveat)?;
    }
    if input.eligibility == "deterministic" {
        replay_deterministic_diagnostics(input, &mut diagnostics)?;
    } else if input.diagnostic_caveats.is_empty() {
        diagnostics.push("non-replayable-without-diagnostic".to_string());
    }
    diagnostics.sort();
    diagnostics.dedup();
    let decision = decision_for(&diagnostics);
    let value = replay_smoke_value(input, decision, &diagnostics)?;
    let gate_ref = hash(&value)?;
    Ok(ReplaySmokeGate {
        decision: decision.to_string(),
        diagnostics,
        gate_ref,
        value,
    })
}

pub fn build_nextest_profile_matrix(input: &NextestProfileMatrixInput) -> Result<NextestProfileMatrix> {
    ensure_bound(input.profiles.len(), "nextest profiles")?;
    let mut diagnostics = Vec::new();
    let mut seen = OrderedSet::new();
    for profile in &input.profiles {
        validate_profile(profile, &mut diagnostics)?;
        if !seen.insert(profile.profile_id.clone()) {
            diagnostics.push(format!("duplicate-profile:{}", profile.profile_id));
        }
    }
    for required in required_semantic_profiles() {
        if !seen.contains(*required) {
            diagnostics.push(format!("missing-profile:{required}"));
        }
    }
    diagnostics.sort();
    diagnostics.dedup();
    let decision = decision_for(&diagnostics);
    let value = nextest_profile_matrix_value(input, decision, &diagnostics)?;
    let matrix_ref = hash(&value)?;
    Ok(NextestProfileMatrix {
        decision: decision.to_string(),
        diagnostics,
        matrix_ref,
        value,
    })
}

pub fn build_cli_receipt_first_gate(input: &CliReceiptFirstInput) -> Result<CliReceiptFirstGate> {
    validate_text("cli command", &input.command)?;
    ensure_bound(input.canonical_artifact_refs.len(), "cli artifact refs")?;
    ensure_bound(input.rendered_output_kinds.len(), "cli rendered output kinds")?;
    let mut diagnostics = input.diagnostics.clone();
    for reference in &input.canonical_artifact_refs {
        validate_ref(reference, "cli canonical artifact")?;
    }
    for kind in &input.rendered_output_kinds {
        validate_rendered_output_kind(kind)?;
    }
    if input.evidence_bearing && input.canonical_artifact_refs.is_empty() {
        diagnostics.push("missing-canonical-artifact".to_string());
    }
    if input.negative_case {
        match input.failure_artifact_ref.as_ref() {
            Some(reference) => validate_ref(reference, "cli failure artifact")?,
            None => diagnostics.push("missing-negative-failure-artifact".to_string()),
        }
    }
    diagnostics.sort();
    diagnostics.dedup();
    let decision = decision_for(&diagnostics);
    let value = cli_receipt_first_value(input, decision, &diagnostics)?;
    let gate_ref = hash(&value)?;
    Ok(CliReceiptFirstGate {
        decision: decision.to_string(),
        diagnostics,
        gate_ref,
        value,
    })
}

fn validate_boundary_observation(item: &BoundaryObservationInput, diagnostics: &mut Vec<String>) -> Result<()> {
    validate_boundary_class(&item.class, diagnostics);
    validate_boundary_polarity(&item.polarity)?;
    validate_text("boundary requirement id", &item.requirement_id)?;
    if let Err(error) = validate_ref(&item.evidence_ref, "boundary evidence") {
        diagnostics.push(format!("stale-evidence-ref:{}:{error}", item.class));
    }
    Ok(())
}

fn validate_boundary_requirement(item: &BoundaryRequirementInput) -> Result<()> {
    let mut diagnostics = Vec::new();
    validate_boundary_class(&item.class, &mut diagnostics);
    if !diagnostics.is_empty() {
        return Err(MoltenError::invalid_harness(diagnostics.join(",")));
    }
    validate_boundary_polarity(&item.polarity)?;
    validate_text("boundary requirement id", &item.requirement_id)
}

fn validate_boundary_exemption(item: &BoundaryCoverageExemptionInput, diagnostics: &mut Vec<String>) -> Result<()> {
    validate_boundary_class(&item.class, diagnostics);
    validate_text("boundary exemption reason", &item.reason)?;
    validate_text("boundary exemption scope", &item.scope)?;
    validate_text("boundary exemption caveat", &item.caveat)?;
    if item.caveat != "diagnostic-only" {
        diagnostics.push(format!("exemption-caveat-not-diagnostic-only:{}", item.class));
    }
    if let Err(error) = validate_ref(&item.evidence_ref, "boundary exemption evidence") {
        diagnostics.push(format!("exemption-without-evidence:{}:{error}", item.class));
    }
    Ok(())
}

fn validate_boundary_class(class: &str, diagnostics: &mut Vec<String>) {
    if !allowed_boundary_classes().contains(&class) {
        diagnostics.push(format!("unsupported-boundary-class:{class}"));
    }
}

fn validate_boundary_polarity(polarity: &str) -> Result<()> {
    match polarity {
        "positive" | "negative" | "diagnostic" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported boundary polarity {other}"))),
    }
}

fn boundary_key(class: &str, polarity: &str) -> String {
    format!("{polarity}:{class}")
}

fn validate_matrix_entry(
    entry: &EvidenceMatrixEntryInput,
    requirements: &OrderedMap<String, crate::requirement_traceability::RequirementInput>,
    diagnostics: &mut Vec<String>,
) -> Result<()> {
    validate_text("matrix requirement id", &entry.requirement_id)?;
    if !requirements.contains_key(&entry.requirement_id) {
        diagnostics.push(format!("stale-requirement-id:{}", entry.requirement_id));
    }
    validate_matrix_coverage_kind(&entry.coverage_kind, diagnostics)?;
    validate_evidence_scope(&entry.evidence_scope, diagnostics)?;
    validate_text("matrix target", &entry.target)?;
    validate_text("matrix command", &entry.command)?;
    if entry.artifact_refs.is_empty() {
        diagnostics.push(format!("missing-artifact-ref:{}", entry.requirement_id));
    }
    validate_ref_list_with_diagnostics("matrix artifact", &entry.artifact_refs, diagnostics)?;
    if let Some(reference) = entry.receipt_ref.as_ref() {
        validate_ref_with_diagnostics(reference, "matrix receipt", diagnostics);
    }
    for caveat in &entry.caveats {
        validate_text("matrix caveat", caveat)?;
    }
    Ok(())
}

fn validate_matrix_exemption(
    exemption: &EvidenceMatrixExemptionInput,
    requirements: &OrderedMap<String, crate::requirement_traceability::RequirementInput>,
    diagnostics: &mut Vec<String>,
) -> Result<()> {
    validate_text("matrix exemption requirement", &exemption.requirement_id)?;
    if !requirements.contains_key(&exemption.requirement_id) {
        diagnostics.push(format!("stale-exemption-requirement:{}", exemption.requirement_id));
    }
    validate_text("matrix exemption reason", &exemption.reason)?;
    validate_ref_with_diagnostics(&exemption.evidence_ref, "matrix exemption evidence", diagnostics);
    validate_text("matrix exemption scope", &exemption.scope)?;
    validate_text("matrix exemption review note", &exemption.review_note)
}

fn validate_matrix_coverage_kind(kind: &str, diagnostics: &mut Vec<String>) -> Result<()> {
    match kind {
        "positive" | "negative" => Ok(()),
        other => {
            diagnostics.push(format!("unsupported-coverage-kind:{other}"));
            Ok(())
        }
    }
}

fn validate_evidence_scope(scope: &str, diagnostics: &mut Vec<String>) -> Result<()> {
    match scope {
        "unit" | "property" | "cli" | "integration" | "vm" | "dogfood" | "exemption" => Ok(()),
        other => {
            diagnostics.push(format!("unsupported-evidence-scope:{other}"));
            Ok(())
        }
    }
}

fn requires_matrix_coverage(requirement: &crate::requirement_traceability::RequirementInput) -> bool {
    requirement.changed || requirement.kind == "evidence"
}

fn validate_ci_input(input: &CiTestRunInput) -> Result<()> {
    validate_ref(&input.source_ref, "ci source")?;
    validate_semantic_profile_id(&input.profile_id)?;
    validate_text("ci command surface", &input.command_surface)?;
    validate_ref(&input.nextest_config_ref, "ci nextest config")?;
    validate_ref(&input.cargo_metadata_ref, "ci cargo metadata")?;
    validate_ref(&input.binaries_metadata_ref, "ci binaries metadata")?;
    validate_ref(&input.junit_ref, "ci junit")?;
    validate_decision(&input.decision)?;
    for diagnostic in &input.diagnostics {
        validate_text("ci diagnostic", diagnostic)?;
    }
    for caveat in &input.caveats {
        validate_text("ci caveat", caveat)?;
    }
    Ok(())
}

fn ci_diagnostics(input: &CiTestRunInput, diagnostics: &mut Vec<String>) -> Result<()> {
    let observed = input
        .counts
        .passed
        .checked_add(input.counts.failed)
        .and_then(|count| count.checked_add(input.counts.skipped))
        .ok_or_else(|| MoltenError::invalid_harness("ci counts overflow"))?;
    if observed > input.counts.total {
        diagnostics.push("mismatched-counts".to_string());
    }
    if input.decision == DECISION_PASS && input.counts.total < MINIMUM_CI_TOTAL_FOR_PASS {
        diagnostics.push("missing-test-counts".to_string());
    }
    if input.decision == DECISION_PASS && input.counts.failed > ZERO_COUNT {
        diagnostics.push("failed-tests-with-pass-decision".to_string());
    }
    if input.profile_id == "exploratory" && input.decision == DECISION_PASS {
        diagnostics.push("exploratory-pass-is-diagnostic-only".to_string());
    }
    Ok(())
}

fn family_map(
    families: &[TamperFamilyInput],
    diagnostics: &mut Vec<String>,
) -> Result<OrderedMap<String, TamperFamilyInput>> {
    let mut output = OrderedMap::new();
    for family in families {
        validate_text("tamper family", &family.family)?;
        validate_ref(&family.control_ref, "tamper control")?;
        validate_text("tamper parser", &family.parser)?;
        validate_text("tamper gate", &family.gate)?;
        if output.insert(family.family.clone(), family.clone()).is_some() {
            diagnostics.push(format!("duplicate-family:{}", family.family));
        }
    }
    Ok(output)
}

fn validate_tamper_case(
    case: &TamperCaseInput,
    families: &OrderedMap<String, TamperFamilyInput>,
    diagnostics: &mut Vec<String>,
) -> Result<()> {
    validate_text("tamper case family", &case.family)?;
    if !families.contains_key(&case.family) {
        diagnostics.push(format!("unknown-family:{}", case.family));
    }
    validate_tamper_mutation(&case.mutation, diagnostics);
    validate_ref(&case.fixture_ref, "tamper fixture")?;
    validate_text("tamper expected diagnostic", &case.expected_diagnostic)?;
    validate_decision(&case.decision)?;
    if case.decision != DECISION_DENY {
        diagnostics.push(format!("tamper-case-not-deny:{}:{}", case.family, case.mutation));
    }
    if case.pass_evidence_ref.is_some() {
        diagnostics.push(format!("tamper-case-emits-pass-evidence:{}:{}", case.family, case.mutation));
    }
    Ok(())
}

fn validate_tamper_mutation(mutation: &str, diagnostics: &mut Vec<String>) {
    if !required_tamper_mutations().contains(&mutation) {
        diagnostics.push(format!("unsupported-mutation:{mutation}"));
    }
}

fn replay_deterministic_diagnostics(input: &ReplaySmokeInput, diagnostics: &mut Vec<String>) -> Result<()> {
    let mut roles = OrderedMap::new();
    for run in &input.runs {
        validate_replay_run(run)?;
        if roles.insert(run.role.clone(), run).is_some() {
            diagnostics.push(format!("duplicate-replay-role:{}", run.role));
        }
    }
    for role in ["fresh", "replay", "fresh-rerun"] {
        if !roles.contains_key(role) {
            diagnostics.push(format!("missing-replay-role:{role}"));
        }
    }
    let Some(fresh) = roles.get("fresh") else {
        return Ok(());
    };
    for role in ["replay", "fresh-rerun"] {
        if let Some(run) = roles.get(role) {
            if run.report_ref != fresh.report_ref {
                diagnostics.push(format!("report-ref-mismatch:{role}"));
            }
            if run.final_state_ref != fresh.final_state_ref {
                diagnostics.push(format!("final-state-ref-mismatch:{role}"));
            }
            if run.effect_log_ref != fresh.effect_log_ref {
                diagnostics.push(format!("effect-log-ref-mismatch:{role}"));
            }
            if run.trace_ref != fresh.trace_ref && !input.variance.iter().any(|item| item == "trace-ref") {
                diagnostics.push(format!("trace-ref-mismatch:{role}"));
            }
            for diagnostic in &run.diagnostics {
                diagnostics.push(format!("run-diagnostic:{role}:{diagnostic}"));
            }
        }
    }
    if fresh.effect_log_ref == placeholder_ref()? {
        diagnostics.push("missing-effect-log".to_string());
    }
    Ok(())
}

fn validate_replay_run(run: &ReplaySmokeRunInput) -> Result<()> {
    match run.role.as_str() {
        "fresh" | "replay" | "fresh-rerun" => {}
        other => return Err(MoltenError::invalid_harness(format!("unsupported replay smoke role {other}"))),
    }
    validate_ref(&run.report_ref, "replay report")?;
    validate_ref(&run.final_state_ref, "replay final state")?;
    validate_ref(&run.effect_log_ref, "replay effect log")?;
    validate_ref(&run.trace_ref, "replay trace")?;
    for diagnostic in &run.diagnostics {
        validate_text("replay run diagnostic", diagnostic)?;
    }
    Ok(())
}

fn validate_replay_eligibility(value: &str) -> Result<()> {
    match value {
        "deterministic" | "exploratory" | "live-only" | "vm-unavailable" | "diagnostic-only" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported replay eligibility {other}"))),
    }
}

fn validate_variance(value: &str) -> Result<()> {
    match value {
        "temporary-root" | "runtime-path" | "store-path" | "diagnostic-log" | "rendered-output" | "trace-ref" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported replay variance {other}"))),
    }
}

fn validate_profile(profile: &SemanticProfileInput, diagnostics: &mut Vec<String>) -> Result<()> {
    validate_semantic_profile_id(&profile.profile_id)?;
    validate_evidence_scope(&profile.evidence_scope, diagnostics)?;
    validate_text("profile command surface", &profile.command_surface)?;
    validate_retry_policy(&profile.retry_policy, diagnostics)?;
    if profile.expected_artifacts.is_empty() {
        diagnostics.push(format!("missing-expected-artifacts:{}", profile.profile_id));
    }
    for artifact in &profile.expected_artifacts {
        validate_text("profile expected artifact", artifact)?;
    }
    validate_cost_class(&profile.cost_class, diagnostics)?;
    for caveat in &profile.caveats {
        validate_text("profile caveat", caveat)?;
    }
    if profile.platform_required && !profile.platform_available {
        diagnostics.push(format!("required-platform-unavailable:{}", profile.profile_id));
    }
    if profile.profile_id == "exploratory" && profile.retry_policy == "retry-pass" {
        return Ok(());
    }
    if profile.retry_policy == "retry-pass" {
        diagnostics.push(format!("retry-pass-not-deterministic:{}", profile.profile_id));
    }
    Ok(())
}

fn validate_semantic_profile_id(profile_id: &str) -> Result<()> {
    match profile_id {
        "fast-core"
        | "harness"
        | "cli"
        | "distributed-simulation"
        | "vm-platform"
        | "dogfood-soak"
        | "ci"
        | "deterministic"
        | "exploratory" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported semantic profile {other}"))),
    }
}

fn validate_retry_policy(policy: &str, diagnostics: &mut Vec<String>) -> Result<()> {
    match policy {
        "zero-retry" | "retry-diagnostic" | "retry-pass" => Ok(()),
        other => {
            diagnostics.push(format!("unsupported-retry-policy:{other}"));
            Ok(())
        }
    }
}

fn validate_cost_class(class: &str, diagnostics: &mut Vec<String>) -> Result<()> {
    match class {
        "fast" | "moderate" | "expensive" | "platform" | "soak" => Ok(()),
        other => {
            diagnostics.push(format!("unsupported-cost-class:{other}"));
            Ok(())
        }
    }
}

fn validate_rendered_output_kind(kind: &str) -> Result<()> {
    match kind {
        "stdout" | "stderr" | "markdown" | "json" | "junit" | "terminal-summary" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported rendered output kind {other}"))),
    }
}

fn requirement_map(
    requirements: &[crate::requirement_traceability::RequirementInput],
) -> Result<OrderedMap<String, crate::requirement_traceability::RequirementInput>> {
    let mut output = OrderedMap::new();
    for requirement in requirements {
        validate_text("requirement id", &requirement.id)?;
        if output.insert(requirement.id.clone(), requirement.clone()).is_some() {
            return Err(MoltenError::invalid_harness(format!("duplicate requirement {}", requirement.id)));
        }
    }
    Ok(output)
}

fn allowed_boundary_classes() -> &'static [&'static str] {
    &[
        "envelope-send",
        "envelope-receive",
        "dataspace-assert",
        "dataspace-retract",
        "dataspace-observe",
        "policy-pass",
        "policy-denial",
        "capability-pass",
        "capability-denial",
        "effect-request",
        "effect-response",
        "hostcall-request",
        "hostcall-denial",
        "resource-pass",
        "resource-exhaustion",
        "replay-pass",
        "replay-divergence",
        "redaction-pass",
        "redaction-denial",
        "adapter-pass",
        "adapter-denial",
        "adapter-failure",
        "pass-evidence-gate",
        "diagnostic-only-rejection",
    ]
}

fn required_tamper_mutations() -> &'static [&'static str] {
    &[
        "missing-required-field",
        "stale-content-ref",
        "wrong-artifact-kind",
        "malformed-content-ref",
        "duplicate-member",
        "tampered-embedded-receipt",
        "noncanonical-value",
        "diagnostic-only-as-pass",
        "missing-child-receipt",
        "unsupported-schema-version",
    ]
}

fn required_semantic_profiles() -> &'static [&'static str] {
    &[
        "fast-core",
        "harness",
        "cli",
        "distributed-simulation",
        "vm-platform",
        "dogfood-soak",
    ]
}

fn boundary_gate_value(
    input: &BoundaryCoverageGateInput,
    decision: &str,
    observed: &[String],
    missing: &[String],
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("boundary-coverage-gate-v1", vec![
        string(BOUNDARY_COVERAGE_GATE_SCHEMA),
        field_string("decision", decision),
        field_string("report-ref", &input.report_ref),
        field_string("suite-ref", &input.suite_ref),
        field_sequence("required", boundary_requirement_values(&input.required)?),
        field_sequence("observed", boundary_observation_values(&input.observed)?),
        field_sequence("observed-classes", string_values(observed)?),
        field_sequence("missing", string_values(missing)?),
        field_sequence("exemptions", boundary_exemption_values(&input.exemptions)?),
        field_sequence("diagnostics", string_values(diagnostics)?),
        field_sequence(
            "caveats",
            string_values(&[EVIDENCE_ONLY_CAVEAT.to_string(), DIAGNOSTIC_VIEW_CAVEAT.to_string()])?,
        ),
    ]))
}

fn evidence_matrix_value(
    input: &EvidenceMatrixInput,
    decision: &str,
    missing_positive: &[String],
    missing_negative: &[String],
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("evidence-matrix-v1", vec![
        string(EVIDENCE_MATRIX_SCHEMA),
        field_string("decision", decision),
        field_sequence("entries", matrix_entry_values(&input.entries)?),
        field_sequence("exemptions", matrix_exemption_values(&input.exemptions)?),
        field_sequence("missing-positive", string_values(missing_positive)?),
        field_sequence("missing-negative", string_values(missing_negative)?),
        field_sequence("diagnostics", string_values(diagnostics)?),
        field_sequence(
            "caveats",
            string_values(&[EVIDENCE_ONLY_CAVEAT.to_string(), DIAGNOSTIC_VIEW_CAVEAT.to_string()])?,
        ),
    ]))
}

fn ci_test_run_value(input: &CiTestRunInput, decision: &str, diagnostics: &[String]) -> Result<IoValue> {
    Ok(record("ci-test-run-receipt-v1", vec![
        string(CI_TEST_RUN_RECEIPT_SCHEMA),
        field_string("decision", decision),
        field_string("source-ref", &input.source_ref),
        field_string("profile-id", &input.profile_id),
        field_string("command-surface", &input.command_surface),
        field_string("nextest-config-ref", &input.nextest_config_ref),
        field_string("cargo-metadata-ref", &input.cargo_metadata_ref),
        field_string("binaries-metadata-ref", &input.binaries_metadata_ref),
        field_string("junit-ref", &input.junit_ref),
        record("counts", vec![
            field_u64("total", input.counts.total),
            field_u64("passed", input.counts.passed),
            field_u64("failed", input.counts.failed),
            field_u64("skipped", input.counts.skipped),
        ]),
        field_sequence("diagnostics", string_values(diagnostics)?),
        field_sequence("caveats", string_values(&input.caveats)?),
    ]))
}

fn tamper_matrix_value(input: &TamperMatrixInput, decision: &str, diagnostics: &[String]) -> Result<IoValue> {
    Ok(record("tamper-negative-matrix-v1", vec![
        string(TAMPER_MATRIX_SCHEMA),
        field_string("decision", decision),
        field_string("subject", &input.subject_ref),
        field_sequence("families", tamper_family_values(&input.families)?),
        field_sequence("cases", tamper_case_values(&input.cases)?),
        field_sequence("diagnostics", string_values(diagnostics)?),
        field_sequence("caveats", string_values(&[EVIDENCE_ONLY_CAVEAT.to_string()])?),
    ]))
}

fn hegel_fixture_value(input: &HegelCounterexampleInput, decision: &str, diagnostics: &[String]) -> Result<IoValue> {
    Ok(record("hegel-counterexample-fixture-v1", vec![
        string(HEGEL_COUNTEREXAMPLE_SCHEMA),
        field_string("decision", decision),
        field_string("property-id", &input.property_id),
        field_sequence("requirements", string_values(&input.requirement_ids)?),
        field_string("generator-profile-ref", &input.generator_profile_ref),
        field_string("generation-seed", &input.generation_seed),
        field_sequence("shrink-path", string_values(&input.shrink_path)?),
        field_string("shrunk-input-ref", &input.shrunk_input_ref),
        field_string("replay-identity-ref", &input.replay_identity_ref),
        field_sequence("trace-refs", string_values(&input.trace_refs)?),
        field_sequence("receipt-refs", string_values(&input.receipt_refs)?),
        field_string("confidentiality", &input.confidentiality),
        field_sequence("diagnostics", string_values(diagnostics)?),
        field_sequence("caveats", string_values(&[EVIDENCE_ONLY_CAVEAT.to_string()])?),
    ]))
}

fn hegel_promotion_value(input: &HegelPromotionInput, decision: &str, diagnostics: &[String]) -> Result<IoValue> {
    Ok(record("hegel-counterexample-promotion-v1", vec![
        string(HEGEL_PROMOTION_SCHEMA),
        field_string("decision", decision),
        field_string("source-fixture-ref", &input.source_fixture_ref),
        field_string("new-suite-entry-ref", &input.new_suite_entry_ref),
        field_string("review-ref", &input.review_ref),
        field_string("property-id", &input.property_id),
        field_string("reason", &input.reason),
        field_string("status", &input.status),
        field_sequence("diagnostics", string_values(diagnostics)?),
    ]))
}

fn replay_smoke_value(input: &ReplaySmokeInput, decision: &str, diagnostics: &[String]) -> Result<IoValue> {
    Ok(record("replay-smoke-gate-v1", vec![
        string(REPLAY_SMOKE_SCHEMA),
        field_string("decision", decision),
        field_string("suite-id", &input.suite_id),
        field_string("eligibility", &input.eligibility),
        field_sequence("runs", replay_run_values(&input.runs)?),
        field_sequence("variance", string_values(&input.variance)?),
        field_sequence("diagnostic-caveats", string_values(&input.diagnostic_caveats)?),
        field_sequence("diagnostics", string_values(diagnostics)?),
        field_sequence(
            "caveats",
            string_values(&[EVIDENCE_ONLY_CAVEAT.to_string(), DIAGNOSTIC_VIEW_CAVEAT.to_string()])?,
        ),
    ]))
}

fn nextest_profile_matrix_value(
    input: &NextestProfileMatrixInput,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("nextest-profile-matrix-v1", vec![
        string(NEXTEST_PROFILE_MATRIX_SCHEMA),
        field_string("decision", decision),
        field_sequence("profiles", profile_values(&input.profiles)?),
        field_sequence("diagnostics", string_values(diagnostics)?),
        field_sequence(
            "caveats",
            string_values(&[EVIDENCE_ONLY_CAVEAT.to_string(), DIAGNOSTIC_VIEW_CAVEAT.to_string()])?,
        ),
    ]))
}

fn cli_receipt_first_value(input: &CliReceiptFirstInput, decision: &str, diagnostics: &[String]) -> Result<IoValue> {
    Ok(record("cli-receipt-first-gate-v1", vec![
        string(CLI_RECEIPT_FIRST_SCHEMA),
        field_string("decision", decision),
        field_string("command", &input.command),
        record("evidence-bearing", vec![bool_value(input.evidence_bearing)]),
        field_sequence("canonical-artifacts", string_values(&input.canonical_artifact_refs)?),
        field_sequence("rendered-output-kinds", string_values(&input.rendered_output_kinds)?),
        record("negative-case", vec![bool_value(input.negative_case)]),
        field_string("failure-artifact-ref", input.failure_artifact_ref.as_deref().unwrap_or("none")),
        field_sequence("diagnostics", string_values(diagnostics)?),
        field_sequence(
            "caveats",
            string_values(&[DIAGNOSTIC_VIEW_CAVEAT.to_string(), EVIDENCE_ONLY_CAVEAT.to_string()])?,
        ),
    ]))
}

fn boundary_requirement_values(values: &[BoundaryRequirementInput]) -> Result<Vec<IoValue>> {
    values
        .iter()
        .map(|item| {
            Ok(record("boundary-requirement", vec![
                field_string("class", &item.class),
                field_string("polarity", &item.polarity),
                field_string("requirement", &item.requirement_id),
            ]))
        })
        .collect()
}

fn boundary_observation_values(values: &[BoundaryObservationInput]) -> Result<Vec<IoValue>> {
    values
        .iter()
        .map(|item| {
            Ok(record("boundary-observation", vec![
                field_string("class", &item.class),
                field_string("polarity", &item.polarity),
                field_string("requirement", &item.requirement_id),
                field_string("evidence-ref", &item.evidence_ref),
            ]))
        })
        .collect()
}

fn boundary_exemption_values(values: &[BoundaryCoverageExemptionInput]) -> Result<Vec<IoValue>> {
    values
        .iter()
        .map(|item| {
            Ok(record("boundary-exemption", vec![
                field_string("class", &item.class),
                field_string("reason", &item.reason),
                field_string("evidence-ref", &item.evidence_ref),
                field_string("scope", &item.scope),
                field_string("caveat", &item.caveat),
            ]))
        })
        .collect()
}

fn matrix_entry_values(values: &[EvidenceMatrixEntryInput]) -> Result<Vec<IoValue>> {
    values
        .iter()
        .map(|item| {
            Ok(record("entry", vec![
                field_string("requirement", &item.requirement_id),
                field_string("coverage-kind", &item.coverage_kind),
                field_string("evidence-scope", &item.evidence_scope),
                field_string("target", &item.target),
                field_string("command", &item.command),
                field_sequence("artifact-refs", string_values(&item.artifact_refs)?),
                field_string("receipt-ref", item.receipt_ref.as_deref().unwrap_or("none")),
                field_sequence("caveats", string_values(&item.caveats)?),
            ]))
        })
        .collect()
}

fn matrix_exemption_values(values: &[EvidenceMatrixExemptionInput]) -> Result<Vec<IoValue>> {
    values
        .iter()
        .map(|item| {
            Ok(record("exemption", vec![
                field_string("requirement", &item.requirement_id),
                field_string("reason", &item.reason),
                field_string("evidence-ref", &item.evidence_ref),
                field_string("scope", &item.scope),
                field_string("review-note", &item.review_note),
            ]))
        })
        .collect()
}

fn tamper_family_values(values: &[TamperFamilyInput]) -> Result<Vec<IoValue>> {
    values
        .iter()
        .map(|item| {
            Ok(record("family", vec![
                field_string("family", &item.family),
                field_string("control-ref", &item.control_ref),
                field_string("parser", &item.parser),
                field_string("gate", &item.gate),
            ]))
        })
        .collect()
}

fn tamper_case_values(values: &[TamperCaseInput]) -> Result<Vec<IoValue>> {
    values
        .iter()
        .map(|item| {
            Ok(record("case", vec![
                field_string("family", &item.family),
                field_string("mutation", &item.mutation),
                field_string("fixture-ref", &item.fixture_ref),
                field_string("expected-diagnostic", &item.expected_diagnostic),
                field_string("decision", &item.decision),
                field_string("pass-evidence-ref", item.pass_evidence_ref.as_deref().unwrap_or("none")),
            ]))
        })
        .collect()
}

fn replay_run_values(values: &[ReplaySmokeRunInput]) -> Result<Vec<IoValue>> {
    values
        .iter()
        .map(|item| {
            Ok(record("run", vec![
                field_string("role", &item.role),
                field_string("report-ref", &item.report_ref),
                field_string("final-state-ref", &item.final_state_ref),
                field_string("effect-log-ref", &item.effect_log_ref),
                field_string("trace-ref", &item.trace_ref),
                field_sequence("diagnostics", string_values(&item.diagnostics)?),
            ]))
        })
        .collect()
}

fn profile_values(values: &[SemanticProfileInput]) -> Result<Vec<IoValue>> {
    values
        .iter()
        .map(|item| {
            Ok(record("profile", vec![
                field_string("profile-id", &item.profile_id),
                field_string("evidence-scope", &item.evidence_scope),
                field_string("command-surface", &item.command_surface),
                field_string("retry-policy", &item.retry_policy),
                field_sequence("expected-artifacts", string_values(&item.expected_artifacts)?),
                field_string("cost-class", &item.cost_class),
                field_sequence("caveats", string_values(&item.caveats)?),
                record("platform-required", vec![bool_value(item.platform_required)]),
                record("platform-available", vec![bool_value(item.platform_available)]),
            ]))
        })
        .collect()
}

fn validate_requirement_ids(ids: &[String]) -> Result<()> {
    ensure_bound(ids.len(), "requirement ids")?;
    for id in ids {
        validate_text("requirement id", id)?;
    }
    Ok(())
}

fn validate_ref_list(label: &str, refs: &[String]) -> Result<()> {
    ensure_ref_bound(refs.len(), label)?;
    for reference in refs {
        validate_ref(reference, label)?;
    }
    Ok(())
}

fn validate_ref_list_with_diagnostics(label: &str, refs: &[String], diagnostics: &mut Vec<String>) -> Result<()> {
    ensure_ref_bound(refs.len(), label)?;
    for reference in refs {
        validate_ref_with_diagnostics(reference, label, diagnostics);
    }
    Ok(())
}

fn validate_ref_with_diagnostics(reference: &str, label: &str, diagnostics: &mut Vec<String>) {
    if let Err(error) = validate_ref(reference, label) {
        diagnostics.push(format!("stale-ref:{reference}:{error}"));
    }
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

fn validate_decision(decision: &str) -> Result<()> {
    match decision {
        DECISION_PASS | DECISION_DENY => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported decision {other}"))),
    }
}

fn ensure_bound(count: usize, label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_ITEMS, label)
}

fn ensure_ref_bound(count: usize, label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_REFS, label)
}

fn decision_for(diagnostics: &[String]) -> &'static str {
    if diagnostics.is_empty() {
        DECISION_PASS
    } else {
        DECISION_DENY
    }
}

fn placeholder_ref() -> Result<String> {
    Ok(crate::preserves_rail::content_ref_from_bytes(b"missing"))
}

fn hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn field_string(label: &'static str, value: &str) -> IoValue {
    record(label, vec![string(value)])
}

fn field_u64(label: &'static str, value: u64) -> IoValue {
    record(label, vec![IoValue::new(value)])
}

fn field_sequence(label: &'static str, values: Vec<IoValue>) -> IoValue {
    record(label, vec![sequence(values)])
}

fn string_values(values: &[String]) -> Result<Vec<IoValue>> {
    ensure_bound(values.len(), "string values")?;
    for value in values {
        validate_text("string value", value)?;
    }
    Ok(values.iter().map(string).collect())
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

fn bool_value(value: bool) -> IoValue {
    crate::preserves_rail::bool_value(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    const REQUIREMENT_ID: &str = "molten.testing.hardening.fixture";
    const SECOND_REQUIREMENT_ID: &str = "molten.testing.hardening.negative";
    const PASS_COUNT: u64 = 12;
    const FAIL_COUNT: u64 = 1;
    const SKIP_COUNT: u64 = 0;

    fn local_ref(label: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
    }

    fn requirement(id: &str, changed: bool) -> crate::requirement_traceability::RequirementInput {
        crate::requirement_traceability::RequirementInput {
            id: id.to_string(),
            source: "cairn/specs/testing-harness/spec.md".to_string(),
            kind: "evidence".to_string(),
            changed,
        }
    }

    fn matrix_entry(id: &str, kind: &str) -> EvidenceMatrixEntryInput {
        EvidenceMatrixEntryInput {
            requirement_id: id.to_string(),
            coverage_kind: kind.to_string(),
            evidence_scope: "cli".to_string(),
            target: format!("tests/{kind}.rs"),
            command: format!("cargo test {kind}"),
            artifact_refs: vec![local_ref(&format!("{id}-{kind}"))],
            receipt_ref: Some(local_ref(&format!("{id}-{kind}-receipt"))),
            caveats: vec!["evidence-only".to_string()],
        }
    }

    fn all_tamper_cases(family: &str) -> Vec<TamperCaseInput> {
        required_tamper_mutations()
            .iter()
            .map(|mutation| TamperCaseInput {
                family: family.to_string(),
                mutation: (*mutation).to_string(),
                fixture_ref: local_ref(&format!("{family}-{mutation}")),
                expected_diagnostic: format!("deny-{mutation}"),
                decision: DECISION_DENY.to_string(),
                pass_evidence_ref: None,
            })
            .collect()
    }

    fn replay_run(role: &str, label: &str) -> ReplaySmokeRunInput {
        ReplaySmokeRunInput {
            role: role.to_string(),
            report_ref: local_ref(&format!("{label}-report")),
            final_state_ref: local_ref(&format!("{label}-state")),
            effect_log_ref: local_ref(&format!("{label}-effects")),
            trace_ref: local_ref(&format!("{label}-trace")),
            diagnostics: Vec::new(),
        }
    }

    fn semantic_profile(id: &str, scope: &str, cost: &str) -> SemanticProfileInput {
        SemanticProfileInput {
            profile_id: id.to_string(),
            evidence_scope: scope.to_string(),
            command_surface: format!("cargo nextest run --profile {id}"),
            retry_policy: "zero-retry".to_string(),
            expected_artifacts: vec!["profile-metadata".to_string(), "junit".to_string()],
            cost_class: cost.to_string(),
            caveats: vec!["evidence-only".to_string()],
            platform_required: false,
            platform_available: true,
        }
    }

    // r[verify molten.testing.boundary_coverage.gate]
    // r[verify molten.testing.boundary_coverage.positive_negative]
    // r[verify molten.testing.boundary_coverage.exemptions]
    // r[verify molten.testing.evidence_matrix.checked_in_manifest]
    // r[verify molten.testing.evidence_matrix.changed_requirement_gate]
    // r[verify molten.testing.evidence_matrix.receipt_backed_entries]
    // r[verify molten.testing.evidence_matrix.exemptions]
    // r[verify molten.testing.ci_run_receipt.canonical_receipt]
    // r[verify molten.testing.ci_run_receipt.junit_view_only]
    // r[verify molten.testing.ci_run_receipt.nix_nextest_binding]
    // r[verify molten.testing.ci_run_receipt.deny_on_missing_metadata]
    // r[verify molten.testing.tamper_matrix.generated_cases]
    // r[verify molten.testing.tamper_matrix.coverage]
    // r[verify molten.testing.tamper_matrix.fail_closed]
    // r[verify molten.testing.hegel_counterexample.replay_fixture]
    // r[verify molten.testing.hegel_counterexample.promotion]
    // r[verify molten.testing.hegel_counterexample.redaction]
    // r[verify molten.testing.replay_smoke.all_evidence_suites]
    // r[verify molten.testing.replay_smoke.fresh_rerun]
    // r[verify molten.testing.replay_smoke.non_replayable_excluded]
    // r[verify molten.testing.nextest.semantic_profiles]
    // r[verify molten.testing.nextest.risk_scope]
    // r[verify molten.testing.nextest.nix_outputs]
    // r[verify molten.testing.nextest.exploratory_exclusion]
    // r[verify molten.testing.cli_receipt_first.normative_artifacts]
    // r[verify molten.testing.cli_receipt_first.stdout_diagnostic_only]
    // r[verify molten.testing.cli_receipt_first.negative_fail_closed]
    #[test]
    fn boundary_coverage_gate_passes_complete_positive_and_negative_boundaries() {
        let input = BoundaryCoverageGateInput {
            report_ref: local_ref("report"),
            suite_ref: local_ref("suite"),
            required: vec![
                BoundaryRequirementInput {
                    class: "policy-pass".to_string(),
                    polarity: "positive".to_string(),
                    requirement_id: REQUIREMENT_ID.to_string(),
                },
                BoundaryRequirementInput {
                    class: "policy-denial".to_string(),
                    polarity: "negative".to_string(),
                    requirement_id: REQUIREMENT_ID.to_string(),
                },
            ],
            observed: vec![
                BoundaryObservationInput {
                    class: "policy-pass".to_string(),
                    polarity: "positive".to_string(),
                    requirement_id: REQUIREMENT_ID.to_string(),
                    evidence_ref: local_ref("policy-pass"),
                },
                BoundaryObservationInput {
                    class: "policy-denial".to_string(),
                    polarity: "negative".to_string(),
                    requirement_id: REQUIREMENT_ID.to_string(),
                    evidence_ref: local_ref("policy-denial"),
                },
            ],
            exemptions: Vec::new(),
        };
        let gate = build_boundary_coverage_gate(&input).expect("boundary gate");
        assert_eq!(gate.decision, DECISION_PASS);
        assert!(gate.missing_classes.is_empty());
    }

    #[test]
    fn boundary_coverage_gate_denies_missing_denial_and_bad_exemption() {
        let input = BoundaryCoverageGateInput {
            report_ref: local_ref("report"),
            suite_ref: local_ref("suite"),
            required: vec![BoundaryRequirementInput {
                class: "policy-denial".to_string(),
                polarity: "negative".to_string(),
                requirement_id: REQUIREMENT_ID.to_string(),
            }],
            observed: vec![BoundaryObservationInput {
                class: "unsupported".to_string(),
                polarity: "positive".to_string(),
                requirement_id: REQUIREMENT_ID.to_string(),
                evidence_ref: "not-a-ref".to_string(),
            }],
            exemptions: vec![BoundaryCoverageExemptionInput {
                class: "policy-denial".to_string(),
                reason: "vm-unavailable".to_string(),
                evidence_ref: "missing".to_string(),
                scope: "local".to_string(),
                caveat: "pass".to_string(),
            }],
        };
        let gate = build_boundary_coverage_gate(&input).expect("boundary gate");
        assert_eq!(gate.decision, DECISION_DENY);
        assert!(gate.diagnostics.iter().any(|diagnostic| diagnostic.contains("unsupported-boundary-class")));
        assert!(gate.diagnostics.iter().any(|diagnostic| diagnostic.contains("exemption-without-evidence")));
    }

    #[test]
    fn evidence_matrix_manifest_accepts_positive_and_negative_entries() {
        let manifest = build_evidence_matrix_manifest(&EvidenceMatrixInput {
            requirements: vec![requirement(REQUIREMENT_ID, true)],
            entries: vec![
                matrix_entry(REQUIREMENT_ID, "positive"),
                matrix_entry(REQUIREMENT_ID, "negative"),
            ],
            exemptions: Vec::new(),
        })
        .expect("evidence matrix");
        assert_eq!(manifest.decision, DECISION_PASS);
        assert!(manifest.missing_positive.is_empty());
        assert!(manifest.missing_negative.is_empty());
    }

    #[test]
    fn evidence_matrix_manifest_denies_missing_negative_and_stale_id() {
        let manifest = build_evidence_matrix_manifest(&EvidenceMatrixInput {
            requirements: vec![requirement(REQUIREMENT_ID, true)],
            entries: vec![
                matrix_entry(REQUIREMENT_ID, "positive"),
                matrix_entry(SECOND_REQUIREMENT_ID, "negative"),
            ],
            exemptions: Vec::new(),
        })
        .expect("evidence matrix");
        assert_eq!(manifest.decision, DECISION_DENY);
        assert_eq!(manifest.missing_negative, vec![REQUIREMENT_ID.to_string()]);
        assert!(manifest.diagnostics.iter().any(|diagnostic| diagnostic.contains("stale-requirement-id")));
    }

    #[test]
    fn ci_test_run_receipt_binds_metadata_and_counts() {
        let receipt = build_ci_test_run_receipt(&CiTestRunInput {
            source_ref: local_ref("source"),
            profile_id: "ci".to_string(),
            command_surface: "cargo nextest run --profile ci".to_string(),
            nextest_config_ref: local_ref("nextest"),
            cargo_metadata_ref: local_ref("cargo"),
            binaries_metadata_ref: local_ref("binaries"),
            junit_ref: local_ref("junit"),
            counts: CiTestCounts {
                total: PASS_COUNT,
                passed: PASS_COUNT,
                failed: SKIP_COUNT,
                skipped: SKIP_COUNT,
            },
            decision: DECISION_PASS.to_string(),
            diagnostics: Vec::new(),
            caveats: vec![EVIDENCE_ONLY_CAVEAT.to_string()],
        })
        .expect("ci receipt");
        assert_eq!(receipt.decision, DECISION_PASS);
        assert!(receipt.diagnostics.is_empty());
    }

    #[test]
    fn ci_test_run_receipt_denies_mismatched_counts_and_exploratory_pass() {
        let receipt = build_ci_test_run_receipt(&CiTestRunInput {
            source_ref: local_ref("source"),
            profile_id: "exploratory".to_string(),
            command_surface: "cargo nextest run --profile exploratory".to_string(),
            nextest_config_ref: local_ref("nextest"),
            cargo_metadata_ref: local_ref("cargo"),
            binaries_metadata_ref: local_ref("binaries"),
            junit_ref: local_ref("junit"),
            counts: CiTestCounts {
                total: PASS_COUNT,
                passed: PASS_COUNT,
                failed: FAIL_COUNT,
                skipped: SKIP_COUNT,
            },
            decision: DECISION_PASS.to_string(),
            diagnostics: Vec::new(),
            caveats: Vec::new(),
        })
        .expect("ci receipt");
        assert_eq!(receipt.decision, DECISION_DENY);
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic == "mismatched-counts"));
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic == "exploratory-pass-is-diagnostic-only"));
    }

    #[test]
    fn tamper_matrix_requires_generated_negative_cases() {
        let family = TamperFamilyInput {
            family: "harness-report".to_string(),
            control_ref: local_ref("control"),
            parser: "parse_report".to_string(),
            gate: "gate-report".to_string(),
        };
        let matrix = build_tamper_matrix(&TamperMatrixInput {
            subject_ref: local_ref("subject"),
            families: vec![family],
            cases: all_tamper_cases("harness-report"),
        })
        .expect("tamper matrix");
        assert_eq!(matrix.decision, DECISION_PASS);
        assert_eq!(matrix.generated_cases.len(), required_tamper_mutations().len());
    }

    #[test]
    fn tamper_matrix_denies_pass_evidence_for_negative_case() {
        let family = TamperFamilyInput {
            family: "release-bundle".to_string(),
            control_ref: local_ref("control"),
            parser: "parse_release".to_string(),
            gate: "verify_release".to_string(),
        };
        let mut cases = all_tamper_cases("release-bundle");
        cases[0].decision = DECISION_PASS.to_string();
        cases[0].pass_evidence_ref = Some(local_ref("bad-pass"));
        let matrix = build_tamper_matrix(&TamperMatrixInput {
            subject_ref: local_ref("subject"),
            families: vec![family],
            cases,
        })
        .expect("tamper matrix");
        assert_eq!(matrix.decision, DECISION_DENY);
        assert!(matrix.diagnostics.iter().any(|diagnostic| diagnostic.contains("tamper-case-emits-pass-evidence")));
    }

    #[test]
    fn hegel_counterexample_fixture_binds_replay_identity_and_redacted_input() {
        let fixture = build_hegel_counterexample_fixture(&HegelCounterexampleInput {
            property_id: "property:roundtrip".to_string(),
            requirement_ids: vec![REQUIREMENT_ID.to_string()],
            generator_profile_ref: local_ref("generator"),
            generation_seed: "seed-1".to_string(),
            shrink_path: vec!["remove-field".to_string()],
            shrunk_input_ref: local_ref("input"),
            replay_identity_ref: local_ref("replay"),
            trace_refs: vec![local_ref("trace")],
            receipt_refs: vec![local_ref("receipt")],
            diagnostics: vec!["expected-failure".to_string()],
            confidentiality: "redacted".to_string(),
        })
        .expect("hegel fixture");
        assert_eq!(fixture.decision, DECISION_PASS);
        assert!(fixture.diagnostics.iter().any(|diagnostic| diagnostic == "expected-failure"));
    }

    #[test]
    fn hegel_counterexample_fixture_denies_missing_seed_and_sensitive_export() {
        let fixture = build_hegel_counterexample_fixture(&HegelCounterexampleInput {
            property_id: "property:roundtrip".to_string(),
            requirement_ids: vec![REQUIREMENT_ID.to_string()],
            generator_profile_ref: local_ref("generator"),
            generation_seed: String::new(),
            shrink_path: Vec::new(),
            shrunk_input_ref: local_ref("input"),
            replay_identity_ref: local_ref("replay"),
            trace_refs: Vec::new(),
            receipt_refs: Vec::new(),
            diagnostics: Vec::new(),
            confidentiality: "sensitive".to_string(),
        })
        .expect("hegel fixture");
        assert_eq!(fixture.decision, DECISION_DENY);
        assert!(fixture.diagnostics.iter().any(|diagnostic| diagnostic == "missing-seed"));
        assert!(fixture.diagnostics.iter().any(|diagnostic| diagnostic == "sensitive-input-not-redacted"));
    }

    #[test]
    fn hegel_promotion_requires_reviewed_status() {
        let record = build_hegel_promotion_record(&HegelPromotionInput {
            source_fixture_ref: local_ref("source"),
            new_suite_entry_ref: local_ref("suite"),
            review_ref: local_ref("review"),
            property_id: "property:roundtrip".to_string(),
            reason: "fixed-bug".to_string(),
            status: "regression-pass".to_string(),
        })
        .expect("promotion");
        assert_eq!(record.decision, DECISION_PASS);
    }

    #[test]
    fn replay_smoke_gate_passes_stable_run_replay_fresh_refs() {
        let runs = vec![
            replay_run("fresh", "same"),
            replay_run("replay", "same"),
            replay_run("fresh-rerun", "same"),
        ];
        let gate = build_replay_smoke_gate(&ReplaySmokeInput {
            suite_id: "suite:deterministic".to_string(),
            eligibility: "deterministic".to_string(),
            runs,
            variance: Vec::new(),
            diagnostic_caveats: Vec::new(),
        })
        .expect("replay smoke");
        assert_eq!(gate.decision, DECISION_PASS);
    }

    #[test]
    fn replay_smoke_gate_denies_changed_effect_response_and_non_replayable_pass_misuse() {
        let mut replay = replay_run("replay", "same");
        replay.effect_log_ref = local_ref("changed-effect");
        let gate = build_replay_smoke_gate(&ReplaySmokeInput {
            suite_id: "suite:deterministic".to_string(),
            eligibility: "deterministic".to_string(),
            runs: vec![replay_run("fresh", "same"), replay, replay_run("fresh-rerun", "same")],
            variance: Vec::new(),
            diagnostic_caveats: Vec::new(),
        })
        .expect("replay smoke");
        assert_eq!(gate.decision, DECISION_DENY);
        assert!(gate.diagnostics.iter().any(|diagnostic| diagnostic == "effect-log-ref-mismatch:replay"));
        let non_replayable = build_replay_smoke_gate(&ReplaySmokeInput {
            suite_id: "suite:live".to_string(),
            eligibility: "live-only".to_string(),
            runs: Vec::new(),
            variance: Vec::new(),
            diagnostic_caveats: Vec::new(),
        })
        .expect("non replayable smoke");
        assert_eq!(non_replayable.decision, DECISION_DENY);
        assert!(
            non_replayable
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "non-replayable-without-diagnostic")
        );
    }

    #[test]
    fn nextest_profile_matrix_accepts_semantic_profiles() {
        let matrix = build_nextest_profile_matrix(&NextestProfileMatrixInput {
            profiles: vec![
                semantic_profile("fast-core", "unit", "fast"),
                semantic_profile("harness", "integration", "moderate"),
                semantic_profile("cli", "cli", "moderate"),
                semantic_profile("distributed-simulation", "integration", "moderate"),
                semantic_profile("vm-platform", "vm", "platform"),
                semantic_profile("dogfood-soak", "dogfood", "soak"),
            ],
        })
        .expect("profile matrix");
        assert_eq!(matrix.decision, DECISION_PASS);
    }

    #[test]
    fn nextest_profile_matrix_denies_missing_profile_and_unavailable_platform() {
        let mut profile = semantic_profile("vm-platform", "vm", "platform");
        profile.platform_required = true;
        profile.platform_available = false;
        let matrix = build_nextest_profile_matrix(&NextestProfileMatrixInput {
            profiles: vec![profile],
        })
        .expect("profile matrix");
        assert_eq!(matrix.decision, DECISION_DENY);
        assert!(
            matrix
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "required-platform-unavailable:vm-platform")
        );
        assert!(matrix.diagnostics.iter().any(|diagnostic| diagnostic == "missing-profile:fast-core"));
    }

    #[test]
    fn cli_receipt_first_gate_accepts_canonical_receipt_assertion() {
        let gate = build_cli_receipt_first_gate(&CliReceiptFirstInput {
            command: "molten test gate check".to_string(),
            evidence_bearing: true,
            canonical_artifact_refs: vec![local_ref("gate-receipt")],
            rendered_output_kinds: vec!["stdout".to_string()],
            negative_case: false,
            failure_artifact_ref: None,
            diagnostics: Vec::new(),
        })
        .expect("cli gate");
        assert_eq!(gate.decision, DECISION_PASS);
    }

    #[test]
    fn cli_receipt_first_gate_denies_stdout_only_and_missing_negative_artifact() {
        let gate = build_cli_receipt_first_gate(&CliReceiptFirstInput {
            command: "molten test gate check".to_string(),
            evidence_bearing: true,
            canonical_artifact_refs: Vec::new(),
            rendered_output_kinds: vec!["stdout".to_string()],
            negative_case: true,
            failure_artifact_ref: None,
            diagnostics: Vec::new(),
        })
        .expect("cli gate");
        assert_eq!(gate.decision, DECISION_DENY);
        assert!(gate.diagnostics.iter().any(|diagnostic| diagnostic == "missing-canonical-artifact"));
        assert!(gate.diagnostics.iter().any(|diagnostic| diagnostic == "missing-negative-failure-artifact"));
    }
}
