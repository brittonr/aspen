use preserves::IOValue;

use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::PROVENANCE_BUILD_RECORD_SCHEMA;
use crate::preserves_rail::PROVENANCE_BUILD_VERIFY_RECEIPT_SCHEMA;
use crate::preserves_rail::PROVENANCE_RECEIPT_SCHEMA;
use crate::preserves_rail::PROVENANCE_RECORD_SCHEMA;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::value_to_iovalue;

pub const TRUST_STATE_REVIEWED: &str = "reviewed";
pub const TRUST_STATE_REPRODUCIBLE_VERIFIED: &str = "reproducible-verified";
pub const TRUST_STATE_POLICY_TRUSTED: &str = "policy-trusted";
pub const TRUST_STATE_SANDBOX_ONLY: &str = "sandbox-only";

const TRUST_STATE_UNKNOWN: &str = "unknown";
const TRUST_STATE_SOURCE_KNOWN: &str = "source-known";
const TRUST_STATE_BUILDER_ATTESTED: &str = "builder-attested";
const TRUST_STATE_DENIED: &str = "denied";
const PROFILE_LOCAL_TEST: &str = "local-test";
const MAX_PROVENANCE_REFS: usize = 64;
const MAX_BUILD_PARAMS: usize = 64;
const MAX_BUILD_PARAM_BYTES: usize = 256;

const _: () = assert!(MAX_PROVENANCE_REFS > 0);
const _: () = assert!(MAX_BUILD_PARAMS > 0);
const _: () = assert!(MAX_BUILD_PARAM_BYTES > 0);

#[derive(Debug, Clone, Copy)]
pub struct ProvenanceRecordInput<'a> {
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
pub struct ProvenanceEvaluationInput<'a> {
    pub operation: &'a str,
    pub profile: &'a str,
    pub artifact_ref: &'a str,
    pub provenance_values: &'a [IOValue],
    pub build_verification_values: &'a [IOValue],
    pub prior_diagnostics: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BuildParam {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy)]
pub struct ProvenanceBuildRecordInput<'a> {
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
pub struct ProvenanceBuildVerificationInput<'a> {
    pub build_record_value: &'a IOValue,
    pub actual_artifact_ref: &'a str,
    pub prior_diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct ProvenanceReceiptValueInput<'a> {
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
pub struct ProvenanceRecord {
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
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceEvaluation {
    pub decision: String,
    pub receipt_ref: String,
    pub receipt_value: IOValue,
    pub matched_record_ref: Option<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceBuildRecord {
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
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceBuildVerification {
    pub decision: String,
    pub receipt_ref: String,
    pub receipt_value: IOValue,
    pub build_record_ref: String,
    pub expected_artifact_ref: String,
    pub actual_artifact_ref: String,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceBuildVerificationReceipt {
    pub decision: String,
    pub receipt_ref: String,
    pub expected_artifact_ref: String,
    pub actual_artifact_ref: String,
    pub build_record_ref: String,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

pub fn provenance_record_value(input: &ProvenanceRecordInput<'_>) -> Result<IOValue> {
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
        string(PROVENANCE_RECORD_SCHEMA),
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

pub fn synthetic_reviewed_provenance_record(artifact_ref: &str) -> Result<IOValue> {
    let source_refs = vec![synthetic_ref("source", artifact_ref)?];
    let toolchain_refs = vec![synthetic_ref("toolchain", env!("CARGO_PKG_VERSION"))?];
    let review_refs = vec![synthetic_ref("review", artifact_ref)?];
    let test_refs = vec![synthetic_ref("tests", artifact_ref)?];
    let source_gate_refs = vec![synthetic_ref("source-gate", artifact_ref)?];
    let policy_refs = vec![synthetic_ref("policy", artifact_ref)?];
    let dependency_closure_ref = synthetic_ref("dependency-closure", artifact_ref)?;
    let builder_ref = synthetic_ref("builder", "local-synthetic-reviewed")?;
    provenance_record_value(&ProvenanceRecordInput {
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

pub fn provenance_build_record_value(input: &ProvenanceBuildRecordInput<'_>) -> Result<IOValue> {
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
        string(PROVENANCE_BUILD_RECORD_SCHEMA),
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

pub fn verify_provenance_build(input: &ProvenanceBuildVerificationInput<'_>) -> Result<ProvenanceBuildVerification> {
    validate_ref(input.actual_artifact_ref, "provenance actual artifact ref")?;
    let build_record = parse_provenance_build_record(input.build_record_value)?;
    let mut diagnostics = Vec::with_capacity(input.prior_diagnostics.len().saturating_add(2));
    diagnostics.extend(input.prior_diagnostics.iter().cloned());
    if build_record.expected_artifact_ref != input.actual_artifact_ref {
        diagnostics.push(format!(
            "build artifact mismatch: expected {}, got {}",
            build_record.expected_artifact_ref, input.actual_artifact_ref
        ));
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = build_verify_receipt_value(&BuildVerifyReceiptValueInput {
        decision,
        expected_artifact_ref: &build_record.expected_artifact_ref,
        actual_artifact_ref: input.actual_artifact_ref,
        build_record_ref: &build_record.record_ref,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = canonical_hash(&receipt_value)?;
    Ok(ProvenanceBuildVerification {
        decision: decision.to_string(),
        receipt_ref,
        receipt_value,
        build_record_ref: build_record.record_ref,
        expected_artifact_ref: build_record.expected_artifact_ref,
        actual_artifact_ref: input.actual_artifact_ref.to_string(),
        diagnostics,
    })
}

pub fn evaluate_provenance(input: &ProvenanceEvaluationInput<'_>) -> Result<ProvenanceEvaluation> {
    validate_ref(input.artifact_ref, "provenance evaluation artifact ref")?;
    validate_profile(input.profile)?;
    ensure_ref_bound(input.provenance_values.len(), MAX_PROVENANCE_REFS, "provenance values")?;
    ensure_ref_bound(
        input.build_verification_values.len(),
        MAX_PROVENANCE_REFS,
        "provenance build verification values",
    )?;
    let mut diagnostics = Vec::with_capacity(
        input
            .prior_diagnostics
            .len()
            .saturating_add(input.provenance_values.len())
            .saturating_add(input.build_verification_values.len())
            .saturating_add(4),
    );
    diagnostics.extend(input.prior_diagnostics.iter().cloned());
    let mut build_verifications = Vec::with_capacity(input.build_verification_values.len());
    let mut build_verification_refs = Vec::with_capacity(input.build_verification_values.len());
    for value in input.build_verification_values {
        match parse_provenance_build_verification_receipt(value) {
            Ok(receipt) => {
                build_verification_refs.push(receipt.receipt_ref.clone());
                build_verifications.push(receipt);
            }
            Err(error) => diagnostics.push(format!("malformed provenance build verification receipt: {error}")),
        }
    }
    let mut matched: Option<ProvenanceRecord> = None;
    let mut has_mismatched_record = false;
    for value in input.provenance_values {
        let record = match parse_provenance_record(value) {
            Ok(record) => record,
            Err(error) => {
                diagnostics.push(format!("malformed provenance record: {error}"));
                continue;
            }
        };
        if record.artifact_ref == input.artifact_ref {
            matched = Some(record);
            break;
        }
        has_mismatched_record = true;
    }
    if input.provenance_values.is_empty() {
        diagnostics.push(format!("missing provenance evidence for {}", input.artifact_ref));
    } else if matched.is_none() && has_mismatched_record {
        diagnostics.push(format!("no provenance record matches artifact {}", input.artifact_ref));
    }
    let matched_record_ref = matched.as_ref().map(|record| record.record_ref.clone());
    let trust_state = matched.as_ref().map(|record| record.trust_state.as_str()).unwrap_or(TRUST_STATE_UNKNOWN);
    let has_admitted_trust_state = is_trust_state_admitted(trust_state, input.profile);
    let trust_admission = if matched.is_some() && has_admitted_trust_state {
        TrustAdmission::Admitted
    } else if matched.is_some() {
        TrustAdmission::Denied
    } else {
        TrustAdmission::Missing
    };
    if trust_admission == TrustAdmission::Denied {
        diagnostics.push(format!("provenance trust state {trust_state} is not admitted for profile {}", input.profile));
    }
    if let Some(record) = matched.as_ref() {
        diagnostics.extend(stronger_provenance_diagnostics(record, input.operation, input.profile));
    }
    if let Some(record) = matched.as_ref().filter(|record| record.trust_state == TRUST_STATE_REPRODUCIBLE_VERIFIED) {
        diagnostics.extend(reproducible_build_binding_diagnostics(record, input.artifact_ref, &build_verifications));
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = provenance_receipt_value(&ProvenanceReceiptValueInput {
        decision,
        operation: input.operation,
        profile: input.profile,
        artifact_ref: input.artifact_ref,
        trust_state,
        record_ref: matched_record_ref.as_deref(),
        build_verification_refs: &build_verification_refs,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = canonical_hash(&receipt_value)?;
    Ok(ProvenanceEvaluation {
        decision: decision.to_string(),
        receipt_ref,
        receipt_value,
        matched_record_ref,
        diagnostics,
    })
}

pub fn parse_provenance_record(value: &IOValue) -> Result<ProvenanceRecord> {
    if let Some(fields) = value.collect_simple_record("provenance-record-v1", Some(12)) {
        require_schema(&fields[0], PROVENANCE_RECORD_SCHEMA, "provenance record")?;
        let trust_state = record_string(&fields[2], "trust-state")?;
        validate_trust_state(&trust_state)?;
        return Ok(ProvenanceRecord {
            record_ref: canonical_hash(value)?,
            artifact_ref: record_ref(&fields[1], "artifact")?,
            trust_state,
            source_refs: record_ref_sequence(&fields[3], "source")?,
            dependency_closure_ref: record_ref(&fields[4], "dependency-closure")?,
            toolchain_refs: record_ref_sequence(&fields[5], "toolchain")?,
            builder_ref: record_ref(&fields[6], "builder")?,
            review_refs: record_ref_sequence(&fields[7], "review")?,
            test_refs: record_ref_sequence(&fields[8], "tests")?,
            source_gate_refs: record_ref_sequence(&fields[9], "source-gates")?,
            policy_refs: record_ref_sequence(&fields[10], "policy")?,
            build_record_refs: record_ref_sequence(&fields[11], "build-records")?,
            value: value.clone(),
        });
    }
    let fields = value
        .collect_simple_record("provenance-record-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected <provenance-record-v1 ...>"))?;
    require_schema(&fields[0], PROVENANCE_RECORD_SCHEMA, "provenance record")?;
    let trust_state = record_string(&fields[2], "trust-state")?;
    validate_trust_state(&trust_state)?;
    Ok(ProvenanceRecord {
        record_ref: canonical_hash(value)?,
        artifact_ref: record_ref(&fields[1], "artifact")?,
        trust_state,
        source_refs: record_ref_sequence(&fields[3], "source")?,
        dependency_closure_ref: record_ref(&fields[4], "dependency-closure")?,
        toolchain_refs: record_ref_sequence(&fields[5], "toolchain")?,
        builder_ref: record_ref(&fields[6], "builder")?,
        review_refs: record_ref_sequence(&fields[7], "review")?,
        test_refs: record_ref_sequence(&fields[8], "tests")?,
        source_gate_refs: record_ref_sequence(&fields[9], "source-gates")?,
        policy_refs: record_ref_sequence(&fields[10], "policy")?,
        build_record_refs: Vec::new(),
        value: value.clone(),
    })
}

pub fn parse_provenance_build_record(value: &IOValue) -> Result<ProvenanceBuildRecord> {
    let fields = value
        .collect_simple_record("provenance-build-record-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <provenance-build-record-v1 ...>"))?;
    require_schema(&fields[0], PROVENANCE_BUILD_RECORD_SCHEMA, "provenance build record")?;
    let build_params = record_build_params_sequence(&fields[5], "build-params")?;
    Ok(ProvenanceBuildRecord {
        record_ref: canonical_hash(value)?,
        expected_artifact_ref: record_ref(&fields[1], "expected-artifact")?,
        source_refs: record_ref_sequence(&fields[2], "source")?,
        dependency_closure_ref: record_ref(&fields[3], "dependency-closure")?,
        toolchain_refs: record_ref_sequence(&fields[4], "toolchain")?,
        build_params,
        builder_ref: record_ref(&fields[6], "builder")?,
        nix_derivation_refs: record_ref_sequence(&fields[7], "nix-derivations")?,
        policy_refs: record_ref_sequence(&fields[8], "policy")?,
        evidence_refs: record_ref_sequence(&fields[9], "evidence")?,
        value: value.clone(),
    })
}

pub fn parse_provenance_build_verification_receipt(value: &IOValue) -> Result<ProvenanceBuildVerificationReceipt> {
    let fields = value
        .collect_simple_record("provenance-build-verify-receipt-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <provenance-build-verify-receipt-v1 ...>"))?;
    require_schema(&fields[0], PROVENANCE_BUILD_VERIFY_RECEIPT_SCHEMA, "provenance build verification receipt")?;
    let decision = record_string(&fields[1], "decision")?;
    if !matches!(decision.as_str(), "pass" | "deny") {
        return Err(MoltenError::invalid_harness(format!(
            "invalid provenance build verification decision `{decision}`"
        )));
    }
    Ok(ProvenanceBuildVerificationReceipt {
        decision,
        receipt_ref: canonical_hash(value)?,
        expected_artifact_ref: record_ref(&fields[2], "expected-artifact")?,
        actual_artifact_ref: record_ref(&fields[3], "actual-artifact")?,
        build_record_ref: record_ref(&fields[4], "build-record")?,
        diagnostics: record_string_sequence(&fields[5], "diagnostics")?,
        value: value.clone(),
    })
}

pub fn provenance_summary(value: &IOValue) -> Result<String> {
    if let Ok(record) = parse_provenance_record(value) {
        return Ok(format!(
            "provenance record artifact={} trust_state={} build_records={} record={}",
            record.artifact_ref,
            record.trust_state,
            record.build_record_refs.len(),
            record.record_ref
        ));
    }
    if let Ok(record) = parse_provenance_build_record(value) {
        return Ok(format!(
            "provenance build record expected_artifact={} sources={} toolchains={} params={} record={}",
            record.expected_artifact_ref,
            record.source_refs.len(),
            record.toolchain_refs.len(),
            record.build_params.len(),
            record.record_ref
        ));
    }
    if let Some(fields) = value.collect_simple_record("provenance-receipt-v1", Some(10)) {
        require_schema(&fields[0], PROVENANCE_RECEIPT_SCHEMA, "provenance receipt")?;
        return Ok(format!(
            "provenance receipt decision={} operation={} artifact={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "operation")?,
            record_ref(&fields[4], "artifact")?
        ));
    }
    if let Some(fields) = value.collect_simple_record("provenance-receipt-v1", Some(9)) {
        require_schema(&fields[0], PROVENANCE_RECEIPT_SCHEMA, "provenance receipt")?;
        return Ok(format!(
            "provenance receipt decision={} operation={} artifact={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "operation")?,
            record_ref(&fields[4], "artifact")?
        ));
    }
    if let Some(fields) = value.collect_simple_record("provenance-build-verify-receipt-v1", Some(8)) {
        require_schema(&fields[0], PROVENANCE_BUILD_VERIFY_RECEIPT_SCHEMA, "provenance build verify receipt")?;
        return Ok(format!(
            "provenance build verify receipt decision={} expected={} actual={}",
            record_string(&fields[1], "decision")?,
            record_ref(&fields[2], "expected-artifact")?,
            record_ref(&fields[3], "actual-artifact")?
        ));
    }
    Err(MoltenError::invalid_harness("unsupported provenance artifact"))
}

fn provenance_receipt_value(input: &ProvenanceReceiptValueInput<'_>) -> Result<IOValue> {
    let reproducible_check = if input.trust_state == TRUST_STATE_REPRODUCIBLE_VERIFIED {
        input.decision
    } else {
        "pass"
    };
    Ok(record("provenance-receipt-v1", vec![
        string(PROVENANCE_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("operation", vec![string(input.operation)]),
        record("profile", vec![string(input.profile)]),
        record("artifact", vec![string(input.artifact_ref)]),
        record("trust-state", vec![string(input.trust_state)]),
        record("provenance", vec![optional_ref_value(input.record_ref)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("hash-is-not-trust"), string("pass")]),
            record("check", vec![string("artifact-ref-bound"), string("pass")]),
            record("check", vec![string("trust-state-admitted"), string(input.decision)]),
            record("check", vec![string("reproducible-build-verification"), string(reproducible_check)]),
            record("check", vec![string("canonical-provenance-receipt"), string("pass")]),
        ])]),
        record("build-verifications", vec![refs_sequence(input.build_verification_refs)]),
    ]))
}

fn build_verify_receipt_value(input: &BuildVerifyReceiptValueInput<'_>) -> Result<IOValue> {
    Ok(record("provenance-build-verify-receipt-v1", vec![
        string(PROVENANCE_BUILD_VERIFY_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("expected-artifact", vec![string(input.expected_artifact_ref)]),
        record("actual-artifact", vec![string(input.actual_artifact_ref)]),
        record("build-record", vec![string(input.build_record_ref)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("build-record-bound"), string("pass")]),
            record("check", vec![string("expected-artifact-bound"), string("pass")]),
            record("check", vec![string("actual-artifact-bound"), string("pass")]),
            record("check", vec![string("artifact-match"), string(input.decision)]),
            record("check", vec![string("canonical-build-verify-receipt"), string("pass")]),
        ])]),
        record("boundary", vec![sequence(vec![
            record("does-not-grant", vec![string("authority")]),
            record("does-not-grant", vec![string("policy")]),
            record("does-not-grant", vec![string("resource")]),
            record("does-not-grant", vec![string("transport")]),
            record("does-not-grant", vec![string("execution")]),
            record("does-not-grant", vec![string("source-gate")]),
        ])]),
    ]))
}

fn reproducible_build_binding_diagnostics(
    record: &ProvenanceRecord,
    artifact_ref: &str,
    receipts: &[ProvenanceBuildVerificationReceipt],
) -> Vec<String> {
    if receipts.is_empty() {
        return vec![format!(
            "reproducible-verified provenance for {artifact_ref} requires a passing build verification receipt"
        )];
    }
    if record.build_record_refs.is_empty() {
        return vec![format!(
            "reproducible-verified provenance for {artifact_ref} must bind at least one build record ref"
        )];
    }
    let mut candidate_diagnostics = Vec::with_capacity(receipts.len().saturating_mul(3));
    for receipt in receipts {
        let mut receipt_diagnostics = Vec::with_capacity(3);
        if receipt.decision != "pass" {
            receipt_diagnostics
                .push(format!("build verification receipt {} decision is {}", receipt.receipt_ref, receipt.decision));
        }
        if receipt.expected_artifact_ref != artifact_ref || receipt.actual_artifact_ref != artifact_ref {
            receipt_diagnostics.push(format!(
                "build verification receipt {} does not match artifact {}: expected {} actual {}",
                receipt.receipt_ref, artifact_ref, receipt.expected_artifact_ref, receipt.actual_artifact_ref
            ));
        }
        if !record.build_record_refs.iter().any(|reference| reference == &receipt.build_record_ref) {
            receipt_diagnostics.push(format!(
                "build verification receipt {} build record {} is not bound by provenance record {}",
                receipt.receipt_ref, receipt.build_record_ref, record.record_ref
            ));
        }
        if receipt_diagnostics.is_empty() {
            return Vec::new();
        }
        candidate_diagnostics.extend(receipt_diagnostics);
    }
    candidate_diagnostics
}

fn is_trust_state_admitted(trust_state: &str, profile: &str) -> bool {
    matches!(trust_state, TRUST_STATE_REVIEWED | TRUST_STATE_REPRODUCIBLE_VERIFIED | TRUST_STATE_POLICY_TRUSTED)
        || (trust_state == TRUST_STATE_SANDBOX_ONLY && profile == PROFILE_LOCAL_TEST)
}

fn stronger_provenance_diagnostics(record: &ProvenanceRecord, operation: &str, profile: &str) -> Vec<String> {
    let has_strong_trust = is_strong_trust_state(&record.trust_state);
    if operation_requires_strong_provenance(operation) {
        if has_strong_trust {
            Vec::new()
        } else {
            vec![format!(
                "operation {operation} under profile {profile} requires stronger provenance than {} for artifact {}",
                record.trust_state, record.artifact_ref
            )]
        }
    } else {
        Vec::new()
    }
}

fn operation_requires_strong_provenance(operation: &str) -> bool {
    matches!(
        operation,
        "install-policy-artifact"
            | "install-migration-recipe"
            | "install-production-executable"
            | "remote-sync-execute"
    )
}

fn is_strong_trust_state(trust_state: &str) -> bool {
    matches!(trust_state, TRUST_STATE_REPRODUCIBLE_VERIFIED | TRUST_STATE_POLICY_TRUSTED)
}

fn validate_trust_state(trust_state: &str) -> Result<()> {
    if matches!(
        trust_state,
        TRUST_STATE_UNKNOWN
            | TRUST_STATE_SOURCE_KNOWN
            | TRUST_STATE_BUILDER_ATTESTED
            | TRUST_STATE_REVIEWED
            | TRUST_STATE_REPRODUCIBLE_VERIFIED
            | TRUST_STATE_SANDBOX_ONLY
            | TRUST_STATE_POLICY_TRUSTED
            | TRUST_STATE_DENIED
    ) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("invalid provenance trust state `{trust_state}`")))
    }
}

fn validate_profile(profile: &str) -> Result<()> {
    if matches!(profile, "node-control" | PROFILE_LOCAL_TEST) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("invalid provenance evaluation profile `{profile}`")))
    }
}

fn validate_build_params(params: &[BuildParam]) -> Result<()> {
    ensure_ref_bound(params.len(), MAX_BUILD_PARAMS, "provenance build params")?;
    for param in params {
        validate_build_param(param)?;
    }
    Ok(())
}

fn validate_build_param(param: &BuildParam) -> Result<()> {
    validate_build_param_token(&param.key, "provenance build param key")?;
    validate_build_param_token(&param.value, "provenance build param value")
}

fn validate_build_param_token(value: &str, context: &str) -> Result<()> {
    if value.is_empty() {
        return Err(MoltenError::invalid_harness(format!("{context} must not be empty")));
    }
    if value.len() > MAX_BUILD_PARAM_BYTES {
        return Err(MoltenError::invalid_harness(format!(
            "{context} is too long: {} > {MAX_BUILD_PARAM_BYTES}",
            value.len()
        )));
    }
    if value.contains('\n') || value.contains('\r') {
        return Err(MoltenError::invalid_harness(format!("{context} must not contain newlines")));
    }
    Ok(())
}

fn build_params_sequence(params: &[BuildParam]) -> IOValue {
    let mut sorted = params.to_vec();
    sorted.sort();
    sequence(
        sorted
            .iter()
            .map(|param| record("build-param", vec![string(&param.key), string(&param.value)]))
            .collect(),
    )
}

fn record_build_params_sequence(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<Vec<BuildParam>> {
    let record_value = value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{tag} sequence>")))?;
    let Some(items) = fields[0].collect_sequence() else {
        return Err(MoltenError::invalid_harness(format!("{tag} must contain a sequence")));
    };
    ensure_ref_bound(items.len(), MAX_BUILD_PARAMS, tag)?;
    let mut params = Vec::with_capacity(items.len());
    for item in items.iter() {
        params.push(required_build_param(item, tag)?);
    }
    validate_build_params(&params)?;
    Ok(params)
}

fn required_build_param(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<BuildParam> {
    let item_value = value_to_iovalue(value);
    let fields = item_value
        .collect_simple_record("build-param", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} item must be <build-param key value>")))?;
    let key = fields[0]
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} build param key must be a string")))?;
    let value = fields[1]
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} build param value must be a string")))?;
    Ok(BuildParam { key, value })
}

fn refs_sequence(refs: &[String]) -> IOValue {
    sequence(refs.iter().map(string).collect())
}

fn optional_ref_value(value: Option<&str>) -> IOValue {
    match value {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
}

fn record_string(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<String> {
    let record_value = value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{tag} string>")))?;
    fields[0]
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} must contain a string")))
}

fn record_ref(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<String> {
    let value = record_string(value, tag)?;
    validate_ref(&value, tag)?;
    Ok(value)
}

fn record_ref_sequence(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<Vec<String>> {
    let record_value = value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{tag} sequence>")))?;
    let Some(items) = fields[0].collect_sequence() else {
        return Err(MoltenError::invalid_harness(format!("{tag} must contain a sequence")));
    };
    ensure_ref_bound(items.len(), MAX_PROVENANCE_REFS, tag)?;
    items.iter().map(|item| required_ref(item, tag)).collect()
}

fn record_string_sequence(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<Vec<String>> {
    let record_value = value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{tag} sequence>")))?;
    let Some(items) = fields[0].collect_sequence() else {
        return Err(MoltenError::invalid_harness(format!("{tag} must contain a sequence")));
    };
    ensure_ref_bound(items.len(), MAX_PROVENANCE_REFS, tag)?;
    items
        .iter()
        .map(|item| {
            item.as_string()
                .map(|value| value.into_owned())
                .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} item must be a string")))
        })
        .collect()
}

fn required_ref(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<String> {
    let value = value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} ref must be a string")))?;
    validate_ref(&value, tag)?;
    Ok(value)
}

fn require_schema(value: &preserves::Value<preserves::IOValue>, expected: &str, context: &str) -> Result<()> {
    let actual = value
        .as_string()
        .ok_or_else(|| MoltenError::invalid_harness(format!("{context} schema must be a string")))?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "{context} schema mismatch: expected {expected}, got {actual}"
        )))
    }
}

fn validate_refs(refs: &[String], context: &str) -> Result<()> {
    ensure_ref_bound(refs.len(), MAX_PROVENANCE_REFS, context)?;
    for reference in refs {
        validate_ref(reference, context)?;
    }
    Ok(())
}

fn ensure_ref_bound(len: usize, max: usize, context: &str) -> Result<()> {
    if len <= max {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("too many {context}: {len} > {max}")))
    }
}

fn validate_ref(value: &str, context: &str) -> Result<()> {
    if value.starts_with("blake3:") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("invalid {context}: expected blake3 ref")))
    }
}

fn synthetic_ref(kind: &str, label: &str) -> Result<String> {
    canonical_hash(&record("provenance-synthetic-ref-v1", vec![string(kind), string(label)]))
}

#[cfg(test)]
mod tests {
    use hegel::TestCase;
    use hegel::generators;

    use super::*;

    #[test]
    fn reviewed_provenance_passes_node_control_and_wrong_artifact_denies() {
        let artifact_ref = synthetic_ref("artifact", "reviewed").expect("artifact ref");
        let record = synthetic_reviewed_provenance_record(&artifact_ref).expect("record");
        let pass = evaluate_provenance(&ProvenanceEvaluationInput {
            operation: "install",
            profile: "node-control",
            artifact_ref: &artifact_ref,
            provenance_values: std::slice::from_ref(&record),
            build_verification_values: &[],
            prior_diagnostics: &[],
        })
        .expect("evaluate pass");
        assert_eq!(pass.decision, "pass");
        assert_eq!(crate::ledger::artifact_kind(&pass.receipt_value), "provenance-receipt");
        let wrong_ref = synthetic_ref("artifact", "wrong").expect("wrong ref");
        let denied = evaluate_provenance(&ProvenanceEvaluationInput {
            operation: "install",
            profile: "node-control",
            artifact_ref: &wrong_ref,
            provenance_values: &[record],
            build_verification_values: &[],
            prior_diagnostics: &[],
        })
        .expect("evaluate deny");
        assert_eq!(denied.decision, "deny");
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("no provenance")));
    }

    #[test]
    fn sandbox_only_is_local_test_only_and_hash_identity_is_not_trust() {
        let artifact_ref = synthetic_ref("artifact", "sandbox").expect("artifact ref");
        let source_refs = vec![synthetic_ref("source", "sandbox").expect("source ref")];
        let toolchain_refs = vec![synthetic_ref("toolchain", "sandbox").expect("toolchain ref")];
        let dependency_ref = synthetic_ref("deps", "sandbox").expect("deps ref");
        let builder_ref = synthetic_ref("builder", "sandbox").expect("builder ref");
        let record = provenance_record_value(&ProvenanceRecordInput {
            artifact_ref: &artifact_ref,
            trust_state: TRUST_STATE_SANDBOX_ONLY,
            source_refs: &source_refs,
            dependency_closure_ref: &dependency_ref,
            toolchain_refs: &toolchain_refs,
            builder_ref: &builder_ref,
            review_refs: &[],
            test_refs: &[],
            source_gate_refs: &[],
            policy_refs: &[],
            build_record_refs: &[],
        })
        .expect("sandbox record");
        let node = evaluate_provenance(&ProvenanceEvaluationInput {
            operation: "run",
            profile: "node-control",
            artifact_ref: &artifact_ref,
            provenance_values: std::slice::from_ref(&record),
            build_verification_values: &[],
            prior_diagnostics: &[],
        })
        .expect("node deny");
        assert_eq!(node.decision, "deny");
        let local = evaluate_provenance(&ProvenanceEvaluationInput {
            operation: "run",
            profile: "local-test",
            artifact_ref: &artifact_ref,
            provenance_values: &[record],
            build_verification_values: &[],
            prior_diagnostics: &[],
        })
        .expect("local pass");
        assert_eq!(local.decision, "pass");
    }

    #[test]
    fn build_record_verification_passes_and_mismatch_denies() {
        let expected_ref = synthetic_ref("artifact", "expected-build").expect("expected ref");
        let actual_ref = synthetic_ref("artifact", "actual-build").expect("actual ref");
        let source_refs = vec![synthetic_ref("source", "build").expect("source ref")];
        let toolchain_refs = vec![synthetic_ref("toolchain", "rust").expect("toolchain ref")];
        let dependency_ref = synthetic_ref("deps", "build").expect("deps ref");
        let builder_ref = synthetic_ref("builder", "nix").expect("builder ref");
        let nix_refs = vec![synthetic_ref("nix-derivation", "build").expect("nix ref")];
        let policy_refs = vec![synthetic_ref("policy", "build").expect("policy ref")];
        let evidence_refs = vec![synthetic_ref("octet", "build").expect("evidence ref")];
        let params = vec![
            BuildParam {
                key: "target".to_string(),
                value: "x86_64-linux".to_string(),
            },
            BuildParam {
                key: "profile".to_string(),
                value: "release".to_string(),
            },
        ];
        let value = provenance_build_record_value(&ProvenanceBuildRecordInput {
            expected_artifact_ref: &expected_ref,
            source_refs: &source_refs,
            dependency_closure_ref: &dependency_ref,
            toolchain_refs: &toolchain_refs,
            build_params: &params,
            builder_ref: &builder_ref,
            nix_derivation_refs: &nix_refs,
            policy_refs: &policy_refs,
            evidence_refs: &evidence_refs,
        })
        .expect("build record");
        let record = parse_provenance_build_record(&value).expect("parse build record");
        assert_eq!(record.expected_artifact_ref, expected_ref);
        assert_eq!(record.build_params.len(), 2);
        assert_eq!(crate::ledger::artifact_kind(&value), "provenance-build-record");

        let pass = verify_provenance_build(&ProvenanceBuildVerificationInput {
            build_record_value: &value,
            actual_artifact_ref: &expected_ref,
            prior_diagnostics: &[],
        })
        .expect("verify pass");
        assert_eq!(pass.decision, "pass");
        assert_eq!(crate::ledger::artifact_kind(&pass.receipt_value), "provenance-build-verify-receipt");

        let deny = verify_provenance_build(&ProvenanceBuildVerificationInput {
            build_record_value: &value,
            actual_artifact_ref: &actual_ref,
            prior_diagnostics: &[],
        })
        .expect("verify deny");
        assert_eq!(deny.decision, "deny");
        assert!(deny.diagnostics.iter().any(|diagnostic| diagnostic.contains("build artifact mismatch")));
    }

    #[test]
    fn reproducible_verified_requires_matching_build_verification_evidence() {
        let artifact_ref = synthetic_ref("artifact", "reproducible").expect("artifact ref");
        let source_refs = vec![synthetic_ref("source", "reproducible").expect("source ref")];
        let toolchain_refs = vec![synthetic_ref("toolchain", "reproducible").expect("toolchain ref")];
        let dependency_ref = synthetic_ref("deps", "reproducible").expect("deps ref");
        let builder_ref = synthetic_ref("builder", "reproducible").expect("builder ref");
        let build_record = provenance_build_record_value(&ProvenanceBuildRecordInput {
            expected_artifact_ref: &artifact_ref,
            source_refs: &source_refs,
            dependency_closure_ref: &dependency_ref,
            toolchain_refs: &toolchain_refs,
            build_params: &[],
            builder_ref: &builder_ref,
            nix_derivation_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("build record");
        let build_record_ref = canonical_hash(&build_record).expect("build record ref");
        let build_record_refs = vec![build_record_ref.clone()];
        let provenance = provenance_record_value(&ProvenanceRecordInput {
            artifact_ref: &artifact_ref,
            trust_state: TRUST_STATE_REPRODUCIBLE_VERIFIED,
            source_refs: &source_refs,
            dependency_closure_ref: &dependency_ref,
            toolchain_refs: &toolchain_refs,
            builder_ref: &builder_ref,
            review_refs: &[],
            test_refs: &[],
            source_gate_refs: &[],
            policy_refs: &[],
            build_record_refs: &build_record_refs,
        })
        .expect("reproducible provenance");
        let missing_receipt = evaluate_provenance(&ProvenanceEvaluationInput {
            operation: "install",
            profile: "node-control",
            artifact_ref: &artifact_ref,
            provenance_values: std::slice::from_ref(&provenance),
            build_verification_values: &[],
            prior_diagnostics: &[],
        })
        .expect("missing build verification denies");
        assert_eq!(missing_receipt.decision, "deny");
        assert!(
            missing_receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("requires a passing build verification"))
        );

        let verification = verify_provenance_build(&ProvenanceBuildVerificationInput {
            build_record_value: &build_record,
            actual_artifact_ref: &artifact_ref,
            prior_diagnostics: &[],
        })
        .expect("verify reproducible build");
        let pass = evaluate_provenance(&ProvenanceEvaluationInput {
            operation: "install",
            profile: "node-control",
            artifact_ref: &artifact_ref,
            provenance_values: std::slice::from_ref(&provenance),
            build_verification_values: std::slice::from_ref(&verification.receipt_value),
            prior_diagnostics: &[],
        })
        .expect("matching build verification passes");
        assert_eq!(pass.decision, "pass");

        let wrong_record_refs = vec![synthetic_ref("build-record", "wrong").expect("wrong build record ref")];
        let wrong_binding = provenance_record_value(&ProvenanceRecordInput {
            artifact_ref: &artifact_ref,
            trust_state: TRUST_STATE_REPRODUCIBLE_VERIFIED,
            source_refs: &source_refs,
            dependency_closure_ref: &dependency_ref,
            toolchain_refs: &toolchain_refs,
            builder_ref: &builder_ref,
            review_refs: &[],
            test_refs: &[],
            source_gate_refs: &[],
            policy_refs: &[],
            build_record_refs: &wrong_record_refs,
        })
        .expect("wrong binding provenance");
        let wrong_binding_eval = evaluate_provenance(&ProvenanceEvaluationInput {
            operation: "install",
            profile: "node-control",
            artifact_ref: &artifact_ref,
            provenance_values: &[wrong_binding],
            build_verification_values: &[verification.receipt_value],
            prior_diagnostics: &[],
        })
        .expect("wrong binding denies");
        assert_eq!(wrong_binding_eval.decision, "deny");
        assert!(
            wrong_binding_eval
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("is not bound by provenance record"))
        );
    }

    #[test]
    fn sensitive_operations_require_stronger_provenance_than_reviewed() {
        let artifact_ref = synthetic_ref("artifact", "sensitive").expect("artifact ref");
        let reviewed = synthetic_reviewed_provenance_record(&artifact_ref).expect("reviewed record");
        let denied = evaluate_provenance(&ProvenanceEvaluationInput {
            operation: "install-policy-artifact",
            profile: "node-control",
            artifact_ref: &artifact_ref,
            provenance_values: std::slice::from_ref(&reviewed),
            build_verification_values: &[],
            prior_diagnostics: &[],
        })
        .expect("reviewed sensitive evaluation");
        assert_eq!(denied.decision, "deny");
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("requires stronger provenance")));

        let source_refs = vec![synthetic_ref("source", "policy-trusted").expect("source ref")];
        let toolchain_refs = vec![synthetic_ref("toolchain", "policy-trusted").expect("toolchain ref")];
        let dependency_ref = synthetic_ref("deps", "policy-trusted").expect("deps ref");
        let builder_ref = synthetic_ref("builder", "policy-trusted").expect("builder ref");
        let policy = provenance_record_value(&ProvenanceRecordInput {
            artifact_ref: &artifact_ref,
            trust_state: TRUST_STATE_POLICY_TRUSTED,
            source_refs: &source_refs,
            dependency_closure_ref: &dependency_ref,
            toolchain_refs: &toolchain_refs,
            builder_ref: &builder_ref,
            review_refs: &[],
            test_refs: &[],
            source_gate_refs: &[],
            policy_refs: &[],
            build_record_refs: &[],
        })
        .expect("policy trusted record");
        let admitted = evaluate_provenance(&ProvenanceEvaluationInput {
            operation: "install-policy-artifact",
            profile: "node-control",
            artifact_ref: &artifact_ref,
            provenance_values: &[policy],
            build_verification_values: &[],
            prior_diagnostics: &[],
        })
        .expect("policy trusted sensitive evaluation");
        assert_eq!(admitted.decision, "pass");
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_provenance_hash_only_denied_and_trust_monotonicity(tc: TestCase) {
        let salt = tc.draw(generators::integers::<u64>().min_value(1).max_value(1_000_000));
        let artifact_ref = synthetic_ref("artifact", &format!("hegel-{salt}")).expect("artifact ref");
        let hash_only = evaluate_provenance(&ProvenanceEvaluationInput {
            operation: "install",
            profile: "node-control",
            artifact_ref: &artifact_ref,
            provenance_values: &[],
            build_verification_values: &[],
            prior_diagnostics: &[],
        })
        .expect("hash-only evaluation");
        assert_eq!(hash_only.decision, "deny");
        assert!(hash_only.diagnostics.iter().any(|diagnostic| diagnostic.contains("missing provenance")));

        let reviewed = synthetic_reviewed_provenance_record(&artifact_ref).expect("reviewed record");
        let reviewed_eval = evaluate_provenance(&ProvenanceEvaluationInput {
            operation: "install",
            profile: "node-control",
            artifact_ref: &artifact_ref,
            provenance_values: std::slice::from_ref(&reviewed),
            build_verification_values: &[],
            prior_diagnostics: &[],
        })
        .expect("reviewed evaluation");
        assert_eq!(reviewed_eval.decision, "pass");

        let sensitive_eval = evaluate_provenance(&ProvenanceEvaluationInput {
            operation: "remote-sync-execute",
            profile: "node-control",
            artifact_ref: &artifact_ref,
            provenance_values: &[reviewed],
            build_verification_values: &[],
            prior_diagnostics: &[],
        })
        .expect("sensitive reviewed evaluation");
        assert_eq!(sensitive_eval.decision, "deny");
        assert!(
            sensitive_eval
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("requires stronger provenance"))
        );
    }
}
