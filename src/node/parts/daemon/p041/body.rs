
pub fn control_authority_grant_value(input: &ControlAuthorityGrantInput<'_>) -> Result<IoValue> {
    validate_node_id(input.peer_id)?;
    validate_node_id(input.node_id)?;
    validate_node_id(input.target_scope)?;
    validate_node_id(input.resource_scope)?;
    if input.operations.is_empty() {
        return Err(MoltenError::invalid_harness("node control authority grant operations missing"));
    }
    for operation in input.operations {
        validate_node_id(operation)?;
    }
    validate_ingress_refs(input.policy_refs, "node control authority grant policy ref")?;
    validate_ingress_refs(input.revocation_refs, "node control authority grant revocation ref")?;
    validate_ingress_refs(input.evidence_refs, "node control authority grant evidence ref")?;
    Ok(crate::preserves_rail::record("node-control-authority-grant-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_AUTHORITY_GRANT_SCHEMA),
        crate::preserves_rail::record("peer", vec![crate::preserves_rail::string(input.peer_id)]),
        crate::preserves_rail::record("node", vec![crate::preserves_rail::string(input.node_id)]),
        crate::preserves_rail::record("operations", vec![crate::preserves_rail::sequence(
            input.operations.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("target-scope", vec![crate::preserves_rail::string(input.target_scope)]),
        crate::preserves_rail::record("resource-scope", vec![crate::preserves_rail::string(input.resource_scope)]),
        crate::preserves_rail::record("epoch", vec![crate::preserves_rail::string(input.epoch.to_string())]),
        crate::preserves_rail::record("expires-at", vec![optional_string(
            input.expires_at.map(|value| value.to_string()).as_deref(),
        )]),
        crate::preserves_rail::record("policy", vec![crate::preserves_rail::sequence(
            input.policy_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("revocations", vec![crate::preserves_rail::sequence(
            input.revocation_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("evidence", vec![crate::preserves_rail::sequence(
            input.evidence_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("peer-node-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("operation-scope-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("revocation-checked-at-ingress"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("transport-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}
