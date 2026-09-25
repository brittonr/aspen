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
/// Each covered requirement can add a missing-positive and a missing-negative diagnostic.
const MISSING_COVERAGE_DIAGNOSTICS_PER_REQUIREMENT: usize = 2;
const MINIMUM_CI_TOTAL_FOR_PASS: u64 = 1;
const ZERO_COUNT: u64 = 0;
const PROFILE_METADATA_ARTIFACT: &str = "profile-metadata";
const FILTER_READBACK_ARTIFACT: &str = "filter-readback";
const JUNIT_ARTIFACT: &str = "junit";
const CANONICAL_TEST_RUN_ARTIFACT: &str = "canonical-test-run-receipt";
const NEXTEST_JUNIT_RELATIVE_PATH: &str = "junit.xml";
const NEXTEST_COMMAND_PREFIX: &str = "cargo nextest run --profile ";
const NEXTEST_PARTITION_SELECTOR: &str = "test(";
const NEXTEST_ALL_FILTER: &str = "all()";

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
    pub filter_expression: String,
    pub retry_policy: String,
    pub expected_artifacts: Vec<String>,
    pub expected_junit_path: String,
    pub cost_class: String,
    pub caveats: Vec<String>,
    pub excluded_partitions: Vec<String>,
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
