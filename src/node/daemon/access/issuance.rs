//! Authority grant and live ticket encoding, parsing, import, and export.

pub fn control_authority_grant_value(
    input: &super::offers::ControlAuthorityGrantInput<'_>,
) -> crate::error::Result<preserves::IOValue> {
    super::storage::validate_node_id(input.peer_id)?;
    super::storage::validate_node_id(input.node_id)?;
    super::storage::validate_node_id(input.target_scope)?;
    super::storage::validate_node_id(input.resource_scope)?;
    if input.operations.is_empty() {
        return Err(crate::error::Failure::invalid_harness("node control authority grant operations missing"));
    }
    for operation in input.operations {
        super::storage::validate_node_id(operation)?;
    }
    super::guards::validate_ingress_refs(input.policy_refs, "node control authority grant policy ref")?;
    super::guards::validate_ingress_refs(input.revocation_refs, "node control authority grant revocation ref")?;
    super::guards::validate_ingress_refs(input.evidence_refs, "node control authority grant evidence ref")?;
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
        crate::preserves_rail::record("expires-at", vec![super::fields::optional_string(
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

pub fn parse_control_authority_grant(
    value: &preserves::IOValue,
) -> crate::error::Result<super::entitlements::ControlAuthorityGrant> {
    let fields = value
        .collect_simple_record("node-control-authority-grant-v1", Some(12))
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected <node-control-authority-grant-v1 ...>"))?;
    super::guards::require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_AUTHORITY_GRANT_SCHEMA,
        "node control authority grant",
    )?;
    let operations = super::fields::record_strings(&fields[3], "operations")?;
    if operations.is_empty() {
        return Err(crate::error::Failure::invalid_harness("node control authority grant operations missing"));
    }
    Ok(super::entitlements::ControlAuthorityGrant {
        grant_ref: crate::preserves_rail::canonical_hash(value)?,
        peer_id: super::fields::record_string(&fields[1], "peer")?,
        node_id: super::fields::record_string(&fields[2], "node")?,
        operations,
        target_scope: super::fields::record_string(&fields[4], "target-scope")?,
        resource_scope: super::fields::record_string(&fields[5], "resource-scope")?,
        epoch: super::fields::record_u64_string(&fields[6], "epoch")?,
        expires_at: super::fields::record_optional_u64_string(&fields[7], "expires-at")?,
        policy_refs: super::fields::record_ref_strings(&fields[8], "policy")?,
        revocation_refs: super::fields::record_ref_strings(&fields[9], "revocations")?,
        evidence_refs: super::fields::record_ref_strings(&fields[10], "evidence")?,
        value: value.clone(),
    })
}

pub fn import_control_authority_grant(
    state_root: &std::path::Path,
    grant_value: &preserves::IOValue,
) -> crate::error::Result<super::entitlements::ControlAuthorityGrant> {
    super::storage::validate_state_root(state_root)?;
    let root = crate::node_state::DirectoryRoot::open(state_root)?;
    super::storage::ensure_state_layout(&root)?;
    let grant = parse_control_authority_grant(grant_value)?;
    super::storage::import_artifact(&root, grant_value)?;
    Ok(grant)
}

pub fn control_live_ticket_value(
    input: &super::offers::ControlLiveTicketInput<'_>,
) -> crate::error::Result<preserves::IOValue> {
    super::storage::validate_node_id(input.node_id)?;
    super::guards::validate_ingress_ref(input.identity_ref, "node control live ticket identity ref")?;
    super::storage::validate_node_id(input.logical_endpoint_id)?;
    super::storage::validate_node_id(input.live_endpoint_id)?;
    super::storage::validate_node_id(input.topic)?;
    super::guards::validate_ingress_refs(input.policy_refs, "node control live ticket policy ref")?;
    super::guards::validate_ingress_refs(input.evidence_refs, "node control live ticket evidence ref")?;
    Ok(crate::preserves_rail::record("node-control-live-ticket-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_TICKET_SCHEMA),
        crate::preserves_rail::record("node", vec![
            crate::preserves_rail::record("id", vec![crate::preserves_rail::string(input.node_id)]),
            crate::preserves_rail::record("identity", vec![crate::preserves_rail::string(input.identity_ref)]),
            crate::preserves_rail::record("logical-endpoint", vec![crate::preserves_rail::string(
                input.logical_endpoint_id,
            )]),
        ]),
        crate::preserves_rail::record("live", vec![
            crate::preserves_rail::record("endpoint-id", vec![crate::preserves_rail::string(input.live_endpoint_id)]),
            crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(input.topic)]),
            crate::preserves_rail::record("addresses", vec![crate::preserves_rail::sequence(
                input.address_refs.iter().map(crate::preserves_rail::string).collect(),
            )]),
        ]),
        crate::preserves_rail::record("policy", vec![crate::preserves_rail::sequence(
            input.policy_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("evidence", vec![crate::preserves_rail::sequence(
            input.evidence_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("node-identity-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("live-endpoint-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ticket-is-bootstrap-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("authority-grant-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

pub fn parse_control_live_ticket(
    value: &preserves::IOValue,
) -> crate::error::Result<super::entitlements::ControlLiveTicket> {
    let fields = value
        .collect_simple_record("node-control-live-ticket-v1", Some(6))
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected <node-control-live-ticket-v1 ...>"))?;
    super::guards::require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_TICKET_SCHEMA,
        "node control live ticket",
    )?;
    let node = crate::preserves_rail::value_to_iovalue(&fields[1]);
    let node_fields = node
        .collect_simple_record("node", Some(3))
        .ok_or_else(|| crate::error::Failure::invalid_harness("node control live ticket missing node"))?;
    let live = crate::preserves_rail::value_to_iovalue(&fields[2]);
    let live_fields = live
        .collect_simple_record("live", Some(3))
        .ok_or_else(|| crate::error::Failure::invalid_harness("node control live ticket missing live endpoint"))?;
    Ok(super::entitlements::ControlLiveTicket {
        ticket_ref: crate::preserves_rail::canonical_hash(value)?,
        node_id: super::fields::record_string(&node_fields[0], "id")?,
        identity_ref: super::fields::record_ref_string(&node_fields[1], "identity")?,
        logical_endpoint_id: super::fields::record_string(&node_fields[2], "logical-endpoint")?,
        live_endpoint_id: super::fields::record_string(&live_fields[0], "endpoint-id")?,
        topic: super::fields::record_string(&live_fields[1], "topic")?,
        address_refs: super::fields::record_strings(&live_fields[2], "addresses")?,
        policy_refs: super::fields::record_ref_strings(&fields[3], "policy")?,
        evidence_refs: super::fields::record_ref_strings(&fields[4], "evidence")?,
        value: value.clone(),
    })
}

pub fn export_control_live_ticket(
    input: &super::offers::ControlLiveTicketExportInput<'_>,
) -> crate::error::Result<super::entitlements::ControlLiveTicket> {
    let state_root = crate::node_state::DirectoryRoot::open(input.state_root)?;
    super::storage::validate_state_root(input.state_root)?;
    super::storage::validate_node_id(input.topic)?;
    super::storage::ensure_state_layout(&state_root)?;
    let identity = crate::node_identity::parse_identity(&super::storage::read_preserves(
        &state_root,
        &crate::node_state::RelativePath::parse(super::settings::IDENTITY_FILE)?,
    )?)?;
    let address_refs = Vec::new();
    let value = control_live_ticket_value(&super::offers::ControlLiveTicketInput {
        node_id: &identity.node_id,
        identity_ref: &identity.identity_ref,
        logical_endpoint_id: &identity.endpoint_id,
        live_endpoint_id: &super::endpoints::stable_live_endpoint_id(&identity),
        topic: input.topic,
        address_refs: &address_refs,
        policy_refs: input.policy_refs,
        evidence_refs: input.evidence_refs,
    })?;
    let ticket = parse_control_live_ticket(&value)?;
    super::storage::import_artifact(&state_root, &value)?;
    Ok(ticket)
}
