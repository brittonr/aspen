type IoValue = preserves::IOValue;
type Result<T> = crate::error::Result<T>;

const RAFT_MEMBERSHIP_PREFLIGHT_SCHEMA: &str = "molten.raft-membership-preflight-receipt.v2";
const RAFT_MEMBERSHIP_COMMIT_SCHEMA: &str = "molten.raft-membership-commit-receipt.v2";
/// v2 receipts add `request-ref`; v1 receipts carry no request digest and never commit.
const RAFT_MEMBERSHIP_RECEIPT_LABEL: &str = "raft-membership-receipt-v2";
const RAFT_MEMBERSHIP_RECEIPT_V1_LABEL: &str = "raft-membership-receipt-v1";
const RAFT_MEMBERSHIP_RECEIPT_V1_FIELDS: usize = 10;
/// Upper bound on preflight diagnostics: the session scope, four canonical refs, ten evidence
/// categories and the requested role.
const MAX_PREFLIGHT_DIAGNOSTICS: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftMembershipRequest {
    pub group_ref: String,
    pub target_peer_ref: String,
    pub target_session_ref: String,
    pub requested_role: String,
    pub configuration_ref: String,
    pub peer_session_scope: String,
    pub authority_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub source_gate_refs: Vec<String>,
    pub provenance_refs: Vec<String>,
    pub compatibility_refs: Vec<String>,
    pub snapshot_refs: Vec<String>,
    pub replay_refs: Vec<String>,
    pub quorum_safety_refs: Vec<String>,
    pub operator_evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
    pub receipt_ref: String,
}

pub fn preflight_raft_membership(input: &RaftMembershipRequest) -> Result<Receipt> {
    let mut diagnostics = Vec::with_capacity(MAX_PREFLIGHT_DIAGNOSTICS);
    if input.peer_session_scope != "raft-membership" {
        diagnostics.push("connected peer session is insufficient for membership admission".to_string());
    }
    for (label, reference) in [
        ("group", &input.group_ref),
        ("target peer", &input.target_peer_ref),
        ("target session", &input.target_session_ref),
        ("configuration", &input.configuration_ref),
    ] {
        if crate::preserves_rail::validate_content_ref(reference).is_err() {
            diagnostics.push(format!("membership {label} ref is not a canonical BLAKE3 content ref"));
        }
    }
    require_non_empty(&mut diagnostics, "authority", &input.authority_refs);
    require_non_empty(&mut diagnostics, "policy", &input.policy_refs);
    require_non_empty(&mut diagnostics, "resource", &input.resource_refs);
    require_non_empty(&mut diagnostics, "source-gate", &input.source_gate_refs);
    require_non_empty(&mut diagnostics, "provenance", &input.provenance_refs);
    require_non_empty(&mut diagnostics, "state-machine compatibility", &input.compatibility_refs);
    require_non_empty(&mut diagnostics, "snapshot readiness", &input.snapshot_refs);
    require_non_empty(&mut diagnostics, "replay readiness", &input.replay_refs);
    require_non_empty(&mut diagnostics, "quorum safety", &input.quorum_safety_refs);
    require_non_empty(&mut diagnostics, "operator evidence", &input.operator_evidence_refs);
    if !matches!(input.requested_role.as_str(), "voter" | "non-voter" | "learner") {
        diagnostics.push(format!("unsupported membership role {}", input.requested_role));
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = value(RAFT_MEMBERSHIP_PREFLIGHT_SCHEMA, decision, &diagnostics, input)?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(Receipt {
        decision: decision.to_string(),
        diagnostics,
        value,
        receipt_ref,
    })
}

/// Commits only with the preflight receipt of this exact request: the receipt digest must match its
/// value, and its schema, group, target peer and session, role, configuration and quorum evidence
/// must equal what a fresh preflight of `input` produces. A passing receipt for another request is
/// never reusable.
// r[impl molten.runtime_spine.admission_and_capability_hardening]
pub fn commit(input: &RaftMembershipRequest, preflight: &Receipt) -> Result<Receipt> {
    let mut diagnostics = Vec::new();
    if preflight.decision != "pass" {
        diagnostics.push("membership commit requires passing preflight receipt".to_string());
    }
    if crate::preserves_rail::canonical_hash(&preflight.value)? != preflight.receipt_ref {
        diagnostics.push("preflight receipt digest does not match its value".to_string());
    }
    let expected = preflight_raft_membership(input)?;
    if expected.decision != "pass" {
        diagnostics.push("membership request does not pass preflight".to_string());
    }
    push_binding_diagnostics(&mut diagnostics, &preflight.value, &expected.value);
    if preflight.value != expected.value || preflight.receipt_ref != expected.receipt_ref {
        diagnostics.push("preflight receipt does not bind this exact membership request".to_string());
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = value(RAFT_MEMBERSHIP_COMMIT_SCHEMA, decision, &diagnostics, input)?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(Receipt {
        decision: decision.to_string(),
        diagnostics,
        value,
        receipt_ref,
    })
}

fn require_non_empty(diagnostics: &mut impl crate::bounded::VecSink<String>, label: &str, refs: &[String]) {
    if refs.is_empty() {
        diagnostics.push_item(format!("missing {label} evidence"));
    }
}

/// Field-level diagnostics naming which bound request field a supplied preflight receipt disagrees
/// on.
fn push_binding_diagnostics(
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    supplied: &IoValue,
    expected: &IoValue,
) {
    const RECEIPT_FIELDS: usize = 11;
    const BOUND_FIELDS: [(usize, &str); 8] = [
        (0, "schema"),
        (2, "group"),
        (3, "target peer"),
        (4, "target session"),
        (5, "requested role"),
        (6, "configuration"),
        (7, "quorum safety evidence"),
        (8, "request digest"),
    ];
    if supplied
        .collect_simple_record(RAFT_MEMBERSHIP_RECEIPT_V1_LABEL, Some(RAFT_MEMBERSHIP_RECEIPT_V1_FIELDS))
        .is_some()
    {
        diagnostics.push_item("preflight receipt v1 lacks a request digest; re-run preflight".to_string());
        return;
    }
    let Some(supplied_fields) = supplied.collect_simple_record(RAFT_MEMBERSHIP_RECEIPT_LABEL, Some(RECEIPT_FIELDS))
    else {
        diagnostics.push_item("preflight receipt is not a raft membership receipt".to_string());
        return;
    };
    let Some(expected_fields) = expected.collect_simple_record(RAFT_MEMBERSHIP_RECEIPT_LABEL, Some(RECEIPT_FIELDS))
    else {
        diagnostics.push_item("expected preflight receipt is malformed".to_string());
        return;
    };
    for (index, label) in BOUND_FIELDS {
        if supplied_fields[index] != expected_fields[index] {
            diagnostics.push_item(format!("preflight receipt {label} does not match the membership request"));
        }
    }
}

/// Canonical digest of every request field, including all evidence categories, so a receipt binds
/// the exact request it evaluated rather than only its group, peer and role.
// r[impl molten.runtime_spine.admission_and_capability_hardening]
fn request_ref(input: &RaftMembershipRequest) -> Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("raft-membership-request-v1", vec![
        field("group-ref", &input.group_ref),
        field("target-peer-ref", &input.target_peer_ref),
        field("target-session-ref", &input.target_session_ref),
        field("requested-role", &input.requested_role),
        field("configuration-ref", &input.configuration_ref),
        field("peer-session-scope", &input.peer_session_scope),
        list_field("authority-refs", &input.authority_refs),
        list_field("policy-refs", &input.policy_refs),
        list_field("resource-refs", &input.resource_refs),
        list_field("source-gate-refs", &input.source_gate_refs),
        list_field("provenance-refs", &input.provenance_refs),
        list_field("compatibility-refs", &input.compatibility_refs),
        list_field("snapshot-refs", &input.snapshot_refs),
        list_field("replay-refs", &input.replay_refs),
        list_field("quorum-safety-refs", &input.quorum_safety_refs),
        list_field("operator-evidence-refs", &input.operator_evidence_refs),
    ]))
}

fn value(schema: &str, decision: &str, diagnostics: &[String], input: &RaftMembershipRequest) -> Result<IoValue> {
    Ok(crate::preserves_rail::record(RAFT_MEMBERSHIP_RECEIPT_LABEL, vec![
        string(schema),
        field("decision", decision),
        field("group-ref", &input.group_ref),
        field("target-peer-ref", &input.target_peer_ref),
        field("target-session-ref", &input.target_session_ref),
        field("requested-role", &input.requested_role),
        field("configuration-ref", &input.configuration_ref),
        list_field("quorum-safety-refs", &input.quorum_safety_refs),
        field("request-ref", &request_ref(input)?),
        list_field("diagnostics", diagnostics),
        field("boundary", "peer-connectivity-is-not-membership"),
    ]))
}

fn field(label: &'static str, value: &str) -> IoValue {
    crate::preserves_rail::record(label, vec![string(value)])
}

fn list_field(label: &'static str, values: &[String]) -> IoValue {
    crate::preserves_rail::record(label, vec![crate::preserves_rail::sequence(values.iter().map(string).collect())])
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

#[cfg(test)]
#[path = "membership/tests.rs"]
mod tests;
