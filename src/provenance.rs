use preserves::IOValue;

use crate::error::MoltenError;
use crate::error::Result;
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

const _: () = assert!(MAX_PROVENANCE_REFS > 0);

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
}

#[derive(Debug, Clone, Copy)]
pub struct ProvenanceEvaluationInput<'a> {
    pub operation: &'a str,
    pub profile: &'a str,
    pub artifact_ref: &'a str,
    pub provenance_values: &'a [IOValue],
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
    })
}

pub fn evaluate_provenance(input: &ProvenanceEvaluationInput<'_>) -> Result<ProvenanceEvaluation> {
    validate_ref(input.artifact_ref, "provenance evaluation artifact ref")?;
    validate_profile(input.profile)?;
    ensure_ref_bound(input.provenance_values.len(), MAX_PROVENANCE_REFS, "provenance values")?;
    let mut diagnostics = Vec::with_capacity(
        input.prior_diagnostics.len().saturating_add(input.provenance_values.len()).saturating_add(3),
    );
    diagnostics.extend(input.prior_diagnostics.iter().cloned());
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
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = provenance_receipt_value(&ProvenanceReceiptValueInput {
        decision,
        operation: input.operation,
        profile: input.profile,
        artifact_ref: input.artifact_ref,
        trust_state,
        record_ref: matched_record_ref.as_deref(),
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
        value: value.clone(),
    })
}

pub fn provenance_summary(value: &IOValue) -> Result<String> {
    if let Ok(record) = parse_provenance_record(value) {
        return Ok(format!(
            "provenance record artifact={} trust_state={} record={}",
            record.artifact_ref, record.trust_state, record.record_ref
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
    Err(MoltenError::invalid_harness("unsupported provenance artifact"))
}

fn provenance_receipt_value(input: &ProvenanceReceiptValueInput<'_>) -> Result<IOValue> {
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
            record("check", vec![string("canonical-provenance-receipt"), string("pass")]),
        ])]),
    ]))
}

fn is_trust_state_admitted(trust_state: &str, profile: &str) -> bool {
    matches!(trust_state, TRUST_STATE_REVIEWED | TRUST_STATE_REPRODUCIBLE_VERIFIED | TRUST_STATE_POLICY_TRUSTED)
        || (trust_state == TRUST_STATE_SANDBOX_ONLY && profile == PROFILE_LOCAL_TEST)
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
        })
        .expect("sandbox record");
        let node = evaluate_provenance(&ProvenanceEvaluationInput {
            operation: "run",
            profile: "node-control",
            artifact_ref: &artifact_ref,
            provenance_values: std::slice::from_ref(&record),
            prior_diagnostics: &[],
        })
        .expect("node deny");
        assert_eq!(node.decision, "deny");
        let local = evaluate_provenance(&ProvenanceEvaluationInput {
            operation: "run",
            profile: "local-test",
            artifact_ref: &artifact_ref,
            provenance_values: &[record],
            prior_diagnostics: &[],
        })
        .expect("local pass");
        assert_eq!(local.decision, "pass");
    }
}
