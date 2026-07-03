type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

fn value_to_iovalue(value: &preserves::Value<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

pub const TRUST_STATE_REVIEWED: &str = "reviewed";
pub const TRUST_STATE_REPRODUCIBLE_VERIFIED: &str = "reproducible-verified";
pub const TRUST_STATE_POLICY_TRUSTED: &str = "policy-trusted";
pub const TRUST_STATE_SANDBOX_ONLY: &str = "sandbox-only";

const TRUST_STATE_UNKNOWN: &str = "unknown";
const TRUST_STATE_SOURCE_KNOWN: &str = "source-known";
const TRUST_STATE_BUILDER_ATTESTED: &str = "builder-attested";
const TRUST_STATE_DENIED: &str = "denied";
const PROFILE_NODE_CONTROL: &str = "node-control";
const PROFILE_LOCAL_TEST: &str = "local-test";
const OPERATION_INSTALL_POLICY_ARTIFACT: &str = "install-policy-artifact";
const OPERATION_INSTALL_MIGRATION_RECIPE: &str = "install-migration-recipe";
const OPERATION_INSTALL_PRODUCTION_EXECUTABLE: &str = "install-production-executable";
const OPERATION_REMOTE_SYNC_EXECUTE: &str = "remote-sync-execute";
const MAX_PROVENANCE_REFS: usize = 64;
const MAX_BUILD_PARAMS: usize = 64;
const MAX_BUILD_PARAM_BYTES: usize = 256;

const _: () = assert!(MAX_PROVENANCE_REFS > 0);
const _: () = assert!(MAX_BUILD_PARAMS > 0);
const _: () = assert!(MAX_BUILD_PARAM_BYTES > 0);

#[derive(Debug, Clone, Copy)]
pub struct RecordInput<'a> {
    pub artifact_ref: &'a str,
    pub trust_state: &'a str,
    pub source_refs: &'a [String],
    pub dependency_closure_ref: &'a str,
    pub toolchain_refs: &'a [String],
    pub builder_ref: &'a str,
    pub review_refs: &'a [String],
    pub test_refs: &'a [String],
    pub source_gate_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub build_record_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct EvaluationInput<'a> {
    pub operation: &'a str,
    pub profile: &'a str,
    pub artifact_ref: &'a str,
    pub provenance_values: &'a [IoValue],
    pub build_verification_values: &'a [IoValue],
    pub prior_diagnostics: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BuildParam {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy)]
pub struct BuildRecordInput<'a> {
    pub expected_artifact_ref: &'a str,
    pub source_refs: &'a [String],
    pub dependency_closure_ref: &'a str,
    pub toolchain_refs: &'a [String],
    pub build_params: &'a [BuildParam],
    pub builder_ref: &'a str,
    pub nix_derivation_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct BuildVerificationInput<'a> {
    pub build_record_value: &'a IoValue,
    pub actual_artifact_ref: &'a str,
    pub prior_diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProvenanceProfileThreshold<'a> {
    pub operation: &'a str,
    pub profile: &'a str,
    pub minimum_trust_state: &'static str,
    pub reproducible_build_verification_required: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct EvidenceOnlyBoundaryInput<'a> {
    pub operation: &'a str,
    pub provenance_receipt_refs: &'a [String],
    pub authority_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub source_gate_refs: &'a [String],
    pub transport_refs: &'a [String],
    pub retention_refs: &'a [String],
    pub execution_refs: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceOnlyBoundaryDecision {
    pub decision: String,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct ReceiptValueInput<'a> {
    decision: &'a str,
    operation: &'a str,
    profile: &'a str,
    artifact_ref: &'a str,
    trust_state: &'a str,
    record_ref: Option<&'a str>,
    build_verification_refs: &'a [String],
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct BuildVerifyReceiptValueInput<'a> {
    decision: &'a str,
    expected_artifact_ref: &'a str,
    actual_artifact_ref: &'a str,
    build_record_ref: &'a str,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrustAdmission {
    Admitted,
    Denied,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BuildChecks {
    receipts: Vec<BuildVerificationReceipt>,
    refs: Vec<String>,
    diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecordMatch {
    record: Option<Record>,
    diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub record_ref: String,
    pub artifact_ref: String,
    pub trust_state: String,
    pub source_refs: Vec<String>,
    pub dependency_closure_ref: String,
    pub toolchain_refs: Vec<String>,
    pub builder_ref: String,
    pub review_refs: Vec<String>,
    pub test_refs: Vec<String>,
    pub source_gate_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub build_record_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evaluation {
    pub decision: String,
    pub receipt_ref: String,
    pub receipt_value: IoValue,
    pub matched_record_ref: Option<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildRecord {
    pub record_ref: String,
    pub expected_artifact_ref: String,
    pub source_refs: Vec<String>,
    pub dependency_closure_ref: String,
    pub toolchain_refs: Vec<String>,
    pub build_params: Vec<BuildParam>,
    pub builder_ref: String,
    pub nix_derivation_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildVerification {
    pub decision: String,
    pub receipt_ref: String,
    pub receipt_value: IoValue,
    pub build_record_ref: String,
    pub expected_artifact_ref: String,
    pub actual_artifact_ref: String,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildVerificationReceipt {
    pub decision: String,
    pub receipt_ref: String,
    pub expected_artifact_ref: String,
    pub actual_artifact_ref: String,
    pub build_record_ref: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildVerificationBinding {
    pub provenance_record_ref: String,
    pub artifact_ref: String,
    pub matched_receipt_ref: Option<String>,
    pub matched_build_record_ref: Option<String>,
    pub is_bound: bool,
    pub diagnostics: Vec<String>,
}

pub fn record_value(input: &RecordInput<'_>) -> Result<IoValue> {
    validate_ref(input.artifact_ref, "provenance artifact ref")?;
    validate_trust_state(input.trust_state)?;
    validate_refs(input.source_refs, "provenance source ref")?;
    validate_ref(input.dependency_closure_ref, "provenance dependency closure ref")?;
    validate_refs(input.toolchain_refs, "provenance toolchain ref")?;
    validate_ref(input.builder_ref, "provenance builder ref")?;
    validate_refs(input.review_refs, "provenance review ref")?;
    validate_refs(input.test_refs, "provenance test ref")?;
    validate_refs(input.source_gate_refs, "provenance source-gate ref")?;
    validate_refs(input.policy_refs, "provenance policy ref")?;
    validate_refs(input.build_record_refs, "provenance build record ref")?;
    Ok(record("provenance-record-v1", vec![
        string(crate::preserves_rail::PROVENANCE_RECORD_SCHEMA),
        record("artifact", vec![string(input.artifact_ref)]),
        record("trust-state", vec![string(input.trust_state)]),
        record("source", vec![refs_sequence(input.source_refs)]),
        record("dependency-closure", vec![string(input.dependency_closure_ref)]),
        record("toolchain", vec![refs_sequence(input.toolchain_refs)]),
        record("builder", vec![string(input.builder_ref)]),
        record("review", vec![refs_sequence(input.review_refs)]),
        record("tests", vec![refs_sequence(input.test_refs)]),
        record("source-gates", vec![refs_sequence(input.source_gate_refs)]),
        record("policy", vec![refs_sequence(input.policy_refs)]),
        record("build-records", vec![refs_sequence(input.build_record_refs)]),
    ]))
}

pub fn synthetic_reviewed_record(artifact_ref: &str) -> Result<IoValue> {
    let source_refs = vec![synthetic_ref("source", artifact_ref)?];
    let toolchain_refs = vec![synthetic_ref("toolchain", env!("CARGO_PKG_VERSION"))?];
    let review_refs = vec![synthetic_ref("review", artifact_ref)?];
    let test_refs = vec![synthetic_ref("tests", artifact_ref)?];
    let source_gate_refs = vec![synthetic_ref("source-gate", artifact_ref)?];
    let policy_refs = vec![synthetic_ref("policy", artifact_ref)?];
    let dependency_closure_ref = synthetic_ref("dependency-closure", artifact_ref)?;
    let builder_ref = synthetic_ref("builder", "local-synthetic-reviewed")?;
    record_value(&RecordInput {
        artifact_ref,
        trust_state: TRUST_STATE_REVIEWED,
        source_refs: &source_refs,
        dependency_closure_ref: &dependency_closure_ref,
        toolchain_refs: &toolchain_refs,
        builder_ref: &builder_ref,
        review_refs: &review_refs,
        test_refs: &test_refs,
        source_gate_refs: &source_gate_refs,
        policy_refs: &policy_refs,
        build_record_refs: &[],
    })
}

pub fn build_record_value(input: &BuildRecordInput<'_>) -> Result<IoValue> {
    validate_ref(input.expected_artifact_ref, "provenance expected artifact ref")?;
    validate_refs(input.source_refs, "provenance build source ref")?;
    validate_ref(input.dependency_closure_ref, "provenance build dependency closure ref")?;
    validate_refs(input.toolchain_refs, "provenance build toolchain ref")?;
    validate_build_params(input.build_params)?;
    validate_ref(input.builder_ref, "provenance build builder ref")?;
    validate_refs(input.nix_derivation_refs, "provenance build nix derivation ref")?;
    validate_refs(input.policy_refs, "provenance build policy ref")?;
    validate_refs(input.evidence_refs, "provenance build evidence ref")?;
    Ok(record("provenance-build-record-v1", vec![
        string(crate::preserves_rail::PROVENANCE_BUILD_RECORD_SCHEMA),
        record("expected-artifact", vec![string(input.expected_artifact_ref)]),
        record("source", vec![refs_sequence(input.source_refs)]),
        record("dependency-closure", vec![string(input.dependency_closure_ref)]),
        record("toolchain", vec![refs_sequence(input.toolchain_refs)]),
        record("build-params", vec![build_params_sequence(input.build_params)]),
        record("builder", vec![string(input.builder_ref)]),
        record("nix-derivations", vec![refs_sequence(input.nix_derivation_refs)]),
        record("policy", vec![refs_sequence(input.policy_refs)]),
        record("evidence", vec![refs_sequence(input.evidence_refs)]),
    ]))
}
