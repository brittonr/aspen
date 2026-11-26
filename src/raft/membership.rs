type IoValue = preserves::IOValue;
type Result<T> = crate::error::Result<T>;

const RAFT_MEMBERSHIP_PREFLIGHT_SCHEMA: &str = "molten.raft-membership-preflight-receipt.v2";
const RAFT_MEMBERSHIP_COMMIT_SCHEMA: &str = "molten.raft-membership-commit-receipt.v2";
/// v2 receipts add `request-ref`; v1 receipts carry no request digest and never commit.
const RAFT_MEMBERSHIP_RECEIPT_LABEL: &str = "raft-membership-receipt-v2";
const RAFT_MEMBERSHIP_RECEIPT_V1_LABEL: &str = "raft-membership-receipt-v1";
const RAFT_MEMBERSHIP_RECEIPT_V1_FIELDS: usize = 10;

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
    let mut diagnostics = Vec::new();
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
fn push_binding_diagnostics(diagnostics: &mut Vec<String>, supplied: &IoValue, expected: &IoValue) {
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
        diagnostics.push("preflight receipt v1 lacks a request digest; re-run preflight".to_string());
        return;
    }
    let Some(supplied_fields) = supplied.collect_simple_record(RAFT_MEMBERSHIP_RECEIPT_LABEL, Some(RECEIPT_FIELDS))
    else {
        diagnostics.push("preflight receipt is not a raft membership receipt".to_string());
        return;
    };
    let Some(expected_fields) = expected.collect_simple_record(RAFT_MEMBERSHIP_RECEIPT_LABEL, Some(RECEIPT_FIELDS))
    else {
        diagnostics.push("expected preflight receipt is malformed".to_string());
        return;
    };
    for (index, label) in BOUND_FIELDS {
        if supplied_fields[index] != expected_fields[index] {
            diagnostics.push(format!("preflight receipt {label} does not match the membership request"));
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
mod tests {
    use super::*;

    #[test]
    fn preflight_passes_with_quorum_and_source_evidence() {
        let request = request("raft-membership", "voter", true);
        let preflight = preflight_raft_membership(&request).expect("preflight");
        assert_eq!(preflight.decision, "pass");
        let commit = commit(&request, &preflight).expect("commit");
        assert_eq!(commit.decision, "pass");
    }

    #[test]
    fn connected_peer_only_and_missing_source_gate_deny() {
        let request = request("node-control", "voter", false);
        let preflight = preflight_raft_membership(&request).expect("preflight");
        assert_eq!(preflight.decision, "deny");
        assert!(preflight.diagnostics.iter().any(|diagnostic| diagnostic.contains("insufficient")));
        assert!(preflight.diagnostics.iter().any(|diagnostic| diagnostic.contains("source-gate")));
        assert!(preflight.diagnostics.iter().any(|diagnostic| diagnostic.contains("quorum")));
    }

    // r[verify molten.runtime_spine.admission_and_capability_hardening]
    #[test]
    fn commit_rejects_passing_preflight_for_another_request() {
        let request = request("raft-membership", "voter", true);
        let mut other_peer = request.clone();
        other_peer.target_peer_ref = test_ref("other-peer");
        let other_preflight = preflight_raft_membership(&other_peer).expect("other preflight");
        assert_eq!(other_preflight.decision, "pass");
        let reused = commit(&request, &other_preflight).expect("commit");
        assert_eq!(reused.decision, "deny");
        assert!(reused.diagnostics.iter().any(|diagnostic| diagnostic.contains("target peer does not match")));

        let preflight = preflight_raft_membership(&request).expect("preflight");
        let mutations: [fn(&mut RaftMembershipRequest); 4] = [
            |request| request.group_ref = test_ref("other-group"),
            |request| request.requested_role = "learner".to_string(),
            |request| request.configuration_ref = test_ref("other-configuration"),
            |request| request.target_session_ref = test_ref("other-session"),
        ];
        for mutate in mutations {
            let mut changed = request.clone();
            mutate(&mut changed);
            assert_eq!(commit(&changed, &preflight).expect("commit").decision, "deny");
        }
    }

    // r[verify molten.runtime_spine.admission_and_capability_hardening]
    #[test]
    fn commit_rejects_forged_preflight_digest_and_non_canonical_refs() {
        let request = request("raft-membership", "voter", true);
        let mut forged = preflight_raft_membership(&request).expect("preflight");
        forged.receipt_ref = test_ref("forged-receipt");
        let denied = commit(&request, &forged).expect("commit");
        assert_eq!(denied.decision, "deny");
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("digest")));

        let mut non_canonical = request.clone();
        non_canonical.group_ref = "group-a".to_string();
        let preflight = preflight_raft_membership(&non_canonical).expect("preflight");
        assert_eq!(preflight.decision, "deny");
        assert!(preflight.diagnostics.iter().any(|diagnostic| diagnostic.contains("BLAKE3")));
    }

    // r[verify molten.runtime_spine.admission_and_capability_hardening]
    #[test]
    fn commit_binds_every_evidence_category_of_the_request() {
        let request = request("raft-membership", "voter", true);
        let preflight = preflight_raft_membership(&request).expect("preflight");
        let other = vec![test_ref("other-evidence")];
        let mutations: [fn(&mut RaftMembershipRequest, Vec<String>); 10] = [
            |request, refs| request.authority_refs = refs,
            |request, refs| request.policy_refs = refs,
            |request, refs| request.resource_refs = refs,
            |request, refs| request.source_gate_refs = refs,
            |request, refs| request.provenance_refs = refs,
            |request, refs| request.compatibility_refs = refs,
            |request, refs| request.snapshot_refs = refs,
            |request, refs| request.replay_refs = refs,
            |request, refs| request.quorum_safety_refs = refs,
            |request, refs| request.operator_evidence_refs = refs,
        ];
        for mutate in mutations {
            let mut changed = request.clone();
            mutate(&mut changed, other.clone());
            assert_eq!(preflight_raft_membership(&changed).expect("changed preflight").decision, "pass");
            let committed = commit(&changed, &preflight).expect("commit");
            assert_eq!(committed.decision, "deny", "a preflight for other evidence must not commit");
            assert!(committed.diagnostics.iter().any(|diagnostic| diagnostic.contains("request digest")));
        }
    }

    // r[verify molten.runtime_spine.admission_and_capability_hardening]
    #[test]
    fn commit_rejects_a_v1_preflight_without_request_digest() {
        let request = request("raft-membership", "voter", true);
        let v1_value = crate::preserves_rail::record(RAFT_MEMBERSHIP_RECEIPT_V1_LABEL, vec![
            string("molten.raft-membership-preflight-receipt.v1"),
            field("decision", "pass"),
            field("group-ref", &request.group_ref),
            field("target-peer-ref", &request.target_peer_ref),
            field("target-session-ref", &request.target_session_ref),
            field("requested-role", &request.requested_role),
            field("configuration-ref", &request.configuration_ref),
            list_field("quorum-safety-refs", &request.quorum_safety_refs),
            list_field("diagnostics", &[]),
            field("boundary", "peer-connectivity-is-not-membership"),
        ]);
        let v1 = Receipt {
            decision: "pass".to_string(),
            diagnostics: Vec::new(),
            receipt_ref: crate::preserves_rail::canonical_hash(&v1_value).expect("v1 ref"),
            value: v1_value,
        };
        let denied = commit(&request, &v1).expect("commit");
        assert_eq!(denied.decision, "deny");
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("v1 lacks a request digest")));
        let v2 = preflight_raft_membership(&request).expect("v2 preflight");
        assert!(crate::preserves_rail::to_text(&v2.value).expect("text").contains("raft-membership-receipt-v2"));
        assert_eq!(commit(&request, &v2).expect("v2 commit").decision, "pass");
    }

    fn request(scope: &str, role: &str, complete: bool) -> RaftMembershipRequest {
        let present = if complete {
            vec![test_ref("evidence")]
        } else {
            Vec::new()
        };
        RaftMembershipRequest {
            group_ref: test_ref("group"),
            target_peer_ref: test_ref("peer"),
            target_session_ref: test_ref("session"),
            requested_role: role.to_string(),
            configuration_ref: test_ref("configuration"),
            peer_session_scope: scope.to_string(),
            authority_refs: present.clone(),
            policy_refs: present.clone(),
            resource_refs: present.clone(),
            source_gate_refs: present.clone(),
            provenance_refs: present.clone(),
            compatibility_refs: present.clone(),
            snapshot_refs: present.clone(),
            replay_refs: present.clone(),
            quorum_safety_refs: present.clone(),
            operator_evidence_refs: present,
        }
    }

    fn test_ref(label: &str) -> String {
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("membership-test-ref", vec![string(
            label,
        )]))
        .expect("test ref")
    }
}
