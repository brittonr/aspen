type IoValue = preserves::IOValue;
type Result<T> = crate::error::Result<T>;

const PEER_HANDOFF_BUNDLE_SCHEMA: &str = "molten.peer-handoff-bundle.v1";
const PEER_HANDOFF_VERIFY_SCHEMA: &str = "molten.peer-handoff-verify-receipt.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerHandoffMember {
    pub name: String,
    pub member_ref: String,
    pub scope: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerHandoffBundle {
    pub peer_ref: String,
    pub node_ref: String,
    pub topic: String,
    pub scope: String,
    pub expires_at_tick: u64,
    pub members: Vec<PeerHandoffMember>,
    pub authority_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerHandoffVerifyInput {
    pub bundle: PeerHandoffBundle,
    pub expected_peer_ref: String,
    pub expected_scope: String,
    pub at_tick: u64,
    pub required_member_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerHandoffReceipt {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub imported_member_refs: Vec<String>,
    pub value: IoValue,
    pub receipt_ref: String,
}

pub fn peer_handoff_bundle_value(bundle: &PeerHandoffBundle) -> IoValue {
    crate::preserves_rail::record("peer-handoff-bundle-v1", vec![
        string(PEER_HANDOFF_BUNDLE_SCHEMA),
        field("peer-ref", &bundle.peer_ref),
        field("node-ref", &bundle.node_ref),
        field("topic", &bundle.topic),
        field("scope", &bundle.scope),
        crate::preserves_rail::record("expires-at-tick", vec![crate::preserves_rail::u64_value(
            bundle.expires_at_tick,
        )]),
        members_field(&bundle.members),
        list_field("authority-refs", &bundle.authority_refs),
        list_field("policy-refs", &bundle.policy_refs),
        list_field("resource-refs", &bundle.resource_refs),
    ])
}

pub fn verify_peer_handoff(input: &PeerHandoffVerifyInput) -> Result<PeerHandoffReceipt> {
    let mut diagnostics = Vec::new();
    if input.bundle.peer_ref != input.expected_peer_ref {
        diagnostics.push("wrong peer binding".to_string());
    }
    if input.bundle.scope != input.expected_scope {
        diagnostics.push("wrong scope binding".to_string());
    }
    if input.at_tick > input.bundle.expires_at_tick {
        diagnostics.push("stale handoff ticket".to_string());
    }
    let mut names = std::collections::BTreeSet::new();
    let mut imported_member_refs = Vec::new();
    for member in &input.bundle.members {
        crate::preserves_rail::validate_content_ref(&member.member_ref)?;
        if !names.insert(member.name.clone()) {
            diagnostics.push(format!("duplicate member {}", member.name));
        }
        if member.scope != input.bundle.scope {
            diagnostics.push(format!("member {} wrong scope", member.name));
        }
        imported_member_refs.push(member.member_ref.clone());
    }
    for required in &input.required_member_names {
        if !names.contains(required) {
            diagnostics.push(format!("missing bundle member {required}"));
        }
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = receipt_value(decision, &diagnostics, &imported_member_refs, &input.bundle);
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(PeerHandoffReceipt {
        decision: decision.to_string(),
        diagnostics,
        imported_member_refs,
        value,
        receipt_ref,
    })
}

pub fn handoff_as_authority_denial(bundle_ref: &str, operation: &str) -> Result<PeerHandoffReceipt> {
    crate::preserves_rail::validate_content_ref(bundle_ref)?;
    let diagnostics = vec![format!(
        "handoff bundle {bundle_ref} is not authority, provenance, source-gate, retention, execution, or resource trust for {operation}"
    )];
    let value = crate::preserves_rail::record("peer-handoff-verify-receipt-v1", vec![
        string(PEER_HANDOFF_VERIFY_SCHEMA),
        field("decision", "deny"),
        list_field("diagnostics", &diagnostics),
        list_field("imported-member-refs", &[]),
        field("evidence-only", "handoff-import-does-not-grant-operation-authority"),
    ]);
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(PeerHandoffReceipt {
        decision: "deny".to_string(),
        diagnostics,
        imported_member_refs: Vec::new(),
        value,
        receipt_ref,
    })
}

fn receipt_value(decision: &str, diagnostics: &[String], refs: &[String], bundle: &PeerHandoffBundle) -> IoValue {
    crate::preserves_rail::record("peer-handoff-verify-receipt-v1", vec![
        string(PEER_HANDOFF_VERIFY_SCHEMA),
        field("decision", decision),
        field("peer-ref", &bundle.peer_ref),
        field("scope", &bundle.scope),
        list_field("imported-member-refs", refs),
        list_field("diagnostics", diagnostics),
        field("dry-run-default", "pass"),
    ])
}

fn members_field(members: &[PeerHandoffMember]) -> IoValue {
    crate::preserves_rail::record("members", vec![crate::preserves_rail::sequence(
        members
            .iter()
            .map(|member| {
                crate::preserves_rail::record("member", vec![
                    string(&member.name),
                    string(&member.member_ref),
                    string(&member.scope),
                ])
            })
            .collect(),
    )])
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

    const CURRENT_TICK: u64 = 5;
    const STALE_TICK: u64 = 6;

    #[test]
    fn handoff_verifies_expected_members() {
        let input = verify_input(bundle(vec![member("ticket"), member("authority")]), CURRENT_TICK);
        let receipt = verify_peer_handoff(&input).expect("verify handoff");
        assert_eq!(receipt.decision, "pass");
        assert_eq!(receipt.imported_member_refs.len(), input.required_member_names.len());
    }

    #[test]
    fn handoff_denies_missing_duplicate_stale_and_wrong_scope() {
        let mut bundle = bundle(vec![member("ticket"), member("ticket")]);
        bundle.members[0].scope = "wrong".to_string();
        let receipt = verify_peer_handoff(&verify_input(bundle, STALE_TICK)).expect("verify handoff");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("duplicate")));
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("stale")));
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("wrong scope")));
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("missing bundle member authority")));
    }

    #[test]
    fn handoff_bundle_is_not_authority() {
        let receipt = handoff_as_authority_denial(&test_ref("bundle"), "retention-delete").expect("denial");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics[0].contains("not authority"));
    }

    fn verify_input(bundle: PeerHandoffBundle, at_tick: u64) -> PeerHandoffVerifyInput {
        PeerHandoffVerifyInput {
            bundle,
            expected_peer_ref: test_ref("peer"),
            expected_scope: "node-control".to_string(),
            at_tick,
            required_member_names: vec!["ticket".to_string(), "authority".to_string()],
        }
    }

    fn bundle(members: Vec<PeerHandoffMember>) -> PeerHandoffBundle {
        PeerHandoffBundle {
            peer_ref: test_ref("peer"),
            node_ref: test_ref("node"),
            topic: "node-control".to_string(),
            scope: "node-control".to_string(),
            expires_at_tick: CURRENT_TICK,
            members,
            authority_refs: vec![test_ref("authority")],
            policy_refs: vec![test_ref("policy")],
            resource_refs: vec![test_ref("resource")],
        }
    }

    fn member(name: &str) -> PeerHandoffMember {
        PeerHandoffMember {
            name: name.to_string(),
            member_ref: test_ref(name),
            scope: "node-control".to_string(),
        }
    }

    fn test_ref(label: &str) -> String {
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("handoff-test-ref", vec![string(label)]))
            .expect("test ref")
    }
}
