type IoValue = preserves::IOValue;
type Result<T> = crate::error::Result<T>;

const PEER_PROMOTION_RECEIPT_SCHEMA: &str = "molten.peer-promotion-receipt.v1";
const PEER_DEMOTION_RECEIPT_SCHEMA: &str = "molten.peer-demotion-receipt.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerPromotionRequest {
    pub issuer_ref: String,
    pub target_peer_ref: String,
    pub target_session_ref: String,
    pub current_roles: Vec<String>,
    pub requested_roles: Vec<String>,
    pub scope: String,
    pub promotion_authority_refs: Vec<String>,
    pub approval_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub revocation_refs: Vec<String>,
    pub expires_at_tick: u64,
    pub at_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerPromotionReceipt {
    pub decision: String,
    pub admitted_roles: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
    pub receipt_ref: String,
}

pub fn preflight_peer_promotion(input: &PeerPromotionRequest) -> Result<PeerPromotionReceipt> {
    let mut diagnostics = Vec::new();
    if input.issuer_ref == input.target_peer_ref {
        diagnostics.push("self-promotion denied".to_string());
    }
    if input.promotion_authority_refs.is_empty() {
        diagnostics.push("missing promotion authority".to_string());
    }
    if input.policy_refs.is_empty() {
        diagnostics.push("missing promotion policy".to_string());
    }
    if input.resource_refs.is_empty() {
        diagnostics.push("missing promotion resource".to_string());
    }
    if input.at_tick > input.expires_at_tick {
        diagnostics.push("stale promotion grant".to_string());
    }
    if input.revocation_refs.iter().any(|reference| reference == &input.issuer_ref) {
        diagnostics.push("revoked promotion issuer".to_string());
    }
    for role in &input.requested_roles {
        if is_raft_role(role) {
            diagnostics.push(format!("Raft role {role} requires separate membership admission"));
        }
        if role == "publisher" && input.current_roles.iter().any(|current| current == "subscriber") {
            require_approval(&mut diagnostics, "subscriber-write-upgrade", &input.approval_refs);
        }
        if !role.starts_with(&input.scope) && input.scope != "global" {
            diagnostics.push(format!("requested role {role} outside scope {}", input.scope));
        }
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let admitted_roles = if decision == "pass" {
        input.requested_roles.clone()
    } else {
        Vec::new()
    };
    let value = promotion_value(PEER_PROMOTION_RECEIPT_SCHEMA, decision, &admitted_roles, &diagnostics, input);
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(PeerPromotionReceipt {
        decision: decision.to_string(),
        admitted_roles,
        diagnostics,
        value,
        receipt_ref,
    })
}

pub fn demote_peer(input: &PeerPromotionRequest, retained_roles: Vec<String>) -> Result<PeerPromotionReceipt> {
    let diagnostics =
        if retained_roles.iter().any(|role| input.requested_roles.iter().any(|requested| requested == role)) {
            vec!["demotion retained a requested revoked role".to_string()]
        } else {
            Vec::new()
        };
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = promotion_value(PEER_DEMOTION_RECEIPT_SCHEMA, decision, &retained_roles, &diagnostics, input);
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(PeerPromotionReceipt {
        decision: decision.to_string(),
        admitted_roles: retained_roles,
        diagnostics,
        value,
        receipt_ref,
    })
}

fn require_approval(diagnostics: &mut Vec<String>, required: &str, approvals: &[String]) {
    if !approvals.iter().any(|approval| approval == required) {
        diagnostics.push(format!("missing approval {required}"));
    }
}

fn is_raft_role(role: &str) -> bool {
    matches!(role, "raft-voter" | "raft-non-voter" | "raft-learner" | "linearizable-reader")
}

fn promotion_value(
    schema: &str,
    decision: &str,
    roles: &[String],
    diagnostics: &[String],
    input: &PeerPromotionRequest,
) -> IoValue {
    crate::preserves_rail::record("peer-promotion-receipt-v1", vec![
        string(schema),
        field("decision", decision),
        field("target-peer-ref", &input.target_peer_ref),
        field("target-session-ref", &input.target_session_ref),
        field("scope", &input.scope),
        list_field("admitted-roles", roles),
        list_field("diagnostics", diagnostics),
        field("side-effect-boundary", "session-role-state-only"),
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

    const CURRENT_TICK: u64 = 4;
    const STALE_TICK: u64 = 5;

    #[test]
    fn scoped_promotion_passes_with_authority_and_approval() {
        let mut request = request(vec!["node.publisher".to_string()], CURRENT_TICK);
        request.approval_refs = vec!["subscriber-write-upgrade".to_string()];
        let receipt = preflight_peer_promotion(&request).expect("promotion");
        assert_eq!(receipt.decision, "pass");
        assert_eq!(receipt.admitted_roles, request.requested_roles);
    }

    #[test]
    fn self_promotion_raft_and_stale_grants_deny() {
        let mut request = request(vec!["raft-voter".to_string(), "publisher".to_string()], STALE_TICK);
        request.issuer_ref = request.target_peer_ref.clone();
        request.promotion_authority_refs.clear();
        let receipt = preflight_peer_promotion(&request).expect("promotion");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("self-promotion")));
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("Raft role")));
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("missing promotion authority")));
    }

    #[test]
    fn demotion_removes_requested_roles() {
        let receipt =
            demote_peer(&request(vec!["node.publisher".to_string()], CURRENT_TICK), vec!["subscriber".to_string()])
                .expect("demotion");
        assert_eq!(receipt.decision, "pass");
        assert_eq!(receipt.admitted_roles, vec!["subscriber".to_string()]);
    }

    fn request(requested_roles: Vec<String>, at_tick: u64) -> PeerPromotionRequest {
        PeerPromotionRequest {
            issuer_ref: test_ref("issuer"),
            target_peer_ref: test_ref("peer"),
            target_session_ref: test_ref("session"),
            current_roles: vec!["subscriber".to_string()],
            requested_roles,
            scope: "node".to_string(),
            promotion_authority_refs: vec![test_ref("authority")],
            approval_refs: Vec::new(),
            policy_refs: vec![test_ref("policy")],
            resource_refs: vec![test_ref("resource")],
            revocation_refs: Vec::new(),
            expires_at_tick: CURRENT_TICK,
            at_tick,
        }
    }

    fn test_ref(label: &str) -> String {
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("promotion-test-ref", vec![string(label)]))
            .expect("test ref")
    }
}
