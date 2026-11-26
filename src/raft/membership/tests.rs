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
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("membership-test-ref", vec![string(label)]))
        .expect("test ref")
}
