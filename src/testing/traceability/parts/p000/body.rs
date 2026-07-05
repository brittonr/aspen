type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;
type Value<T> = preserves::Value<T>;

type OrderedMap<K, V> = std::collections::BTreeMap<K, V>;
type OrderedSet<T> = std::collections::BTreeSet<T>;

use crate::bounded::VecSink;

const TRACEABILITY_MANIFEST_SCHEMA: &str = "molten.testing.requirement-traceability.manifest.v1";
const TRACEABILITY_GATE_SCHEMA: &str = "molten.testing.requirement-traceability.gate.v1";
const VERIFICATION_RUN_RECEIPT_SCHEMA: &str = "molten.testing.verification-run-receipt.v1";
const AGGREGATE_PROOF_MANIFEST_SCHEMA: &str = "molten.testing.aggregate-proof-manifest.v1";
const LAYERED_PROOF_MANIFEST_SCHEMA: &str = "molten.evidence.layered-proof-manifest.v1";
const DENY_PATH_MATRIX_SCHEMA: &str = "molten.evidence.proof-deny-path-matrix.v1";
const MAX_REQUIREMENTS: usize = 4096;
const MAX_COVERAGE_ITEMS: usize = 4096;
const MAX_SUMMARY_LINES: usize = 8192;
const MAX_RECEIPT_ARGS: usize = 128;
const MAX_RECEIPT_REFS: usize = 256;
const MAX_PROOF_OBLIGATIONS: usize = 512;
const MAX_PROOF_LAYERS: usize = 128;
const MAX_LAYER_DIRECT_DIAGNOSTICS_PER_LAYER: usize = 3;
const MAX_LAYER_LINK_DIAGNOSTICS_PER_CHILD: usize = 3;
const MAX_DENY_DIRECT_DIAGNOSTICS_PER_CASE: usize = 3;
const MAX_EVIDENCE_FIXED_DIAGNOSTICS_PER_ITEM: usize = 9;
const MAX_EXEMPTION_DIAGNOSTICS: usize = 1;
const MAX_STATUS_DIAGNOSTICS: usize = 2;
const MAX_AGGREGATE_DIAGNOSTICS_PER_OBLIGATION: usize = 3;
const MAX_LAYER_STACK_ITEMS: usize = MAX_PROOF_LAYERS * MAX_PROOF_LAYERS;
const MAX_EVIDENCE_DIAGNOSTICS_PER_ITEM: usize = MAX_RECEIPT_REFS + MAX_EVIDENCE_FIXED_DIAGNOSTICS_PER_ITEM;
const MAX_TRACEABILITY_ENTRY_DIAGNOSTICS: usize =
    (MAX_COVERAGE_ITEMS * MAX_EVIDENCE_DIAGNOSTICS_PER_ITEM * 2) + MAX_EXEMPTION_DIAGNOSTICS + MAX_STATUS_DIAGNOSTICS;
const VERIFICATION_RUN_RECEIPT_ARITY: usize = 13;
const _: () = assert!(MAX_REQUIREMENTS > 0);
const _: () = assert!(MAX_COVERAGE_ITEMS >= MAX_REQUIREMENTS);
const _: () = assert!(MAX_SUMMARY_LINES >= MAX_REQUIREMENTS);
const _: () = assert!(MAX_RECEIPT_ARGS > 0);
const _: () = assert!(MAX_RECEIPT_REFS > 0);
const _: () = assert!(MAX_PROOF_OBLIGATIONS > 0);
const _: () = assert!(MAX_PROOF_LAYERS > 0);
const _: () = assert!(MAX_LAYER_DIRECT_DIAGNOSTICS_PER_LAYER > 0);
const _: () = assert!(MAX_LAYER_LINK_DIAGNOSTICS_PER_CHILD > 0);
const _: () = assert!(MAX_DENY_DIRECT_DIAGNOSTICS_PER_CASE > 0);
const _: () = assert!(MAX_EVIDENCE_FIXED_DIAGNOSTICS_PER_ITEM > 0);
const _: () = assert!(MAX_EXEMPTION_DIAGNOSTICS > 0);
const _: () = assert!(MAX_STATUS_DIAGNOSTICS > 0);
const _: () = assert!(MAX_AGGREGATE_DIAGNOSTICS_PER_OBLIGATION > 0);
const _: () = assert!(MAX_LAYER_STACK_ITEMS >= MAX_PROOF_LAYERS);
const _: () = assert!(MAX_TRACEABILITY_ENTRY_DIAGNOSTICS >= MAX_COVERAGE_ITEMS);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecSource {
    pub source: String,
    pub markdown: String,
    pub changed: bool,
    pub default_kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequirementInput {
    pub id: String,
    pub source: String,
    pub kind: String,
    pub changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationEvidence {
    pub target: String,
    pub command: String,
    pub artifact_ref: String,
    pub artifact_refs: Vec<String>,
    pub target_exists: bool,
    pub artifact_present: bool,
    pub source: String,
    pub receipt_ref: Option<String>,
    pub expected_decision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageInput {
    pub requirement_id: String,
    pub positive: Vec<VerificationEvidence>,
    pub negative: Vec<VerificationEvidence>,
    pub exemption: Option<CoverageExemption>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageExemption {
    pub class: String,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceabilityInput {
    pub requirements: Vec<RequirementInput>,
    pub coverage: Vec<CoverageInput>,
    pub require_receipt_backed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceabilityEntry {
    pub requirement_id: String,
    pub source: String,
    pub kind: String,
    pub changed: bool,
    pub status: String,
    pub diagnostics: Vec<String>,
    pub positive: Vec<VerificationEvidence>,
    pub negative: Vec<VerificationEvidence>,
    pub exemption: Option<CoverageExemption>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceabilityManifest {
    pub decision: String,
    pub entries: Vec<TraceabilityEntry>,
    pub summary: TraceabilitySummary,
    pub manifest_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TraceabilitySummary {
    pub covered: Vec<String>,
    pub exempt: Vec<String>,
    pub missing_positive: Vec<String>,
    pub missing_negative: Vec<String>,
    pub stale_reference: Vec<String>,
    pub unsupported: Vec<String>,
    pub compatibility_only: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationRunInput {
    pub requirement_id: String,
    pub coverage_kind: String,
    pub target: String,
    pub argv: Vec<String>,
    pub profile_ref: String,
    pub toolchain_refs: Vec<String>,
    pub exit_status: i64,
    pub stdout_ref: String,
    pub stderr_ref: String,
    pub artifact_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationRunReceipt {
    pub decision: String,
    pub requirement_id: String,
    pub coverage_kind: String,
    pub target: String,
    pub argv: Vec<String>,
    pub profile_ref: String,
    pub toolchain_refs: Vec<String>,
    pub exit_status: i64,
    pub stdout_ref: String,
    pub stderr_ref: String,
    pub artifact_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptCoverageSource {
    pub value: IoValue,
    pub target_exists: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofObligationInput {
    pub id: String,
    pub class: String,
    pub subject_ref: String,
    pub prerequisite_refs: Vec<String>,
    pub receipt_refs: Vec<String>,
    pub decision: String,
    pub requirement_ids: Vec<String>,
    pub coverage_kind: Option<String>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateProofInput {
    pub manifest_id: String,
    pub subject_ref: String,
    pub required_obligation_ids: Vec<String>,
    pub obligations: Vec<ProofObligationInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateProofManifest {
    pub decision: String,
    pub manifest_id: String,
    pub subject_ref: String,
    pub obligations: Vec<ProofObligationInput>,
    pub diagnostics: Vec<String>,
    pub manifest_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofLayerInput {
    pub id: String,
    pub role: String,
    pub subject_ref: String,
    pub decision: String,
    pub child_ids: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayeredProofInput {
    pub subject_ref: String,
    pub layers: Vec<ProofLayerInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayeredProofManifest {
    pub decision: String,
    pub subject_ref: String,
    pub layers: Vec<ProofLayerInput>,
    pub diagnostics: Vec<String>,
    pub manifest_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DenyPathCaseInput {
    pub class: String,
    pub fixture_ref: String,
    pub expected_decision: String,
    pub before_state_ref: Option<String>,
    pub after_state_ref: Option<String>,
    pub no_mutation_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DenyPathMatrixInput {
    pub gate: String,
    pub subject_ref: String,
    pub cases: Vec<DenyPathCaseInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DenyPathMatrix {
    pub decision: String,
    pub gate: String,
    pub subject_ref: String,
    pub diagnostics: Vec<String>,
    pub matrix_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofReadback {
    pub decision: String,
    pub entries: Vec<ProofReadbackEntry>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofReadbackEntry {
    pub requirement_id: String,
    pub status: String,
    pub positive_receipt_refs: Vec<String>,
    pub negative_receipt_refs: Vec<String>,
    pub artifact_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub caveats: Vec<String>,
}

