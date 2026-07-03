type IoValue = preserves::IOValue;
type Result<T> = crate::error::Result<T>;

const RAFT_MEMBERSHIP_PREFLIGHT_SCHEMA: &str = "molten.raft-membership-preflight-receipt.v1";
const RAFT_MEMBERSHIP_COMMIT_SCHEMA: &str = "molten.raft-membership-commit-receipt.v1";

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
pub struct RaftMembershipReceipt {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
    pub receipt_ref: String,
}

pub fn preflight_raft_membership(input: &RaftMembershipRequest) -> Result<RaftMembershipReceipt> {
    let mut diagnostics = Vec::new();
    if input.peer_session_scope != "raft-membership" {
        diagnostics.push("connected peer session is insufficient for membership admission".to_string());
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
    let value = membership_value(RAFT_MEMBERSHIP_PREFLIGHT_SCHEMA, decision, &diagnostics, input);
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(RaftMembershipReceipt {
        decision: decision.to_string(),
        diagnostics,
        value,
        receipt_ref,
    })
}

pub fn commit_raft_membership(
    input: &RaftMembershipRequest,
    preflight: &RaftMembershipReceipt,
) -> Result<RaftMembershipReceipt> {
    let diagnostics = if preflight.decision == "pass" {
        Vec::new()
    } else {
        vec!["membership commit requires passing preflight receipt".to_string()]
    };
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = membership_value(RAFT_MEMBERSHIP_COMMIT_SCHEMA, decision, &diagnostics, input);
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(RaftMembershipReceipt {
        decision: decision.to_string(),
        diagnostics,
        value,
        receipt_ref,
    })
}

fn require_non_empty(diagnostics: &mut Vec<String>, label: &str, refs: &[String]) {
    if refs.is_empty() {
        diagnostics.push(format!("missing {label} evidence"));
    }
}

fn membership_value(schema: &str, decision: &str, diagnostics: &[String], input: &RaftMembershipRequest) -> IoValue {
    crate::preserves_rail::record("raft-membership-receipt-v1", vec![
        string(schema),
        field("decision", decision),
        field("group-ref", &input.group_ref),
        field("target-peer-ref", &input.target_peer_ref),
        field("target-session-ref", &input.target_session_ref),
        field("requested-role", &input.requested_role),
        field("configuration-ref", &input.configuration_ref),
        list_field("quorum-safety-refs", &input.quorum_safety_refs),
        list_field("diagnostics", diagnostics),
        field("boundary", "peer-connectivity-is-not-membership"),
    ])
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
    fn membership_preflight_passes_with_quorum_and_source_evidence() {
        let request = request("raft-membership", "voter", true);
        let preflight = preflight_raft_membership(&request).expect("preflight");
        assert_eq!(preflight.decision, "pass");
        let commit = commit_raft_membership(&request, &preflight).expect("commit");
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
