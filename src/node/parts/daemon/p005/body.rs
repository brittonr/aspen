
pub fn parse_control_authority_grant(value: &IoValue) -> Result<ControlAuthorityGrant> {
    let fields = value
        .collect_simple_record("node-control-authority-grant-v1", Some(12))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-authority-grant-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_AUTHORITY_GRANT_SCHEMA,
        "node control authority grant",
    )?;
    let operations = record_strings(&fields[3], "operations")?;
    if operations.is_empty() {
        return Err(MoltenError::invalid_harness("node control authority grant operations missing"));
    }
    Ok(ControlAuthorityGrant {
        grant_ref: crate::preserves_rail::canonical_hash(value)?,
        peer_id: record_string(&fields[1], "peer")?,
        node_id: record_string(&fields[2], "node")?,
        operations,
        target_scope: record_string(&fields[4], "target-scope")?,
        resource_scope: record_string(&fields[5], "resource-scope")?,
        epoch: record_u64_string(&fields[6], "epoch")?,
        expires_at: record_optional_u64_string(&fields[7], "expires-at")?,
        policy_refs: record_ref_strings(&fields[8], "policy")?,
        revocation_refs: record_ref_strings(&fields[9], "revocations")?,
        evidence_refs: record_ref_strings(&fields[10], "evidence")?,
        value: value.clone(),
    })
}

pub fn import_control_authority_grant(state_root: &Path, grant_value: &IoValue) -> Result<ControlAuthorityGrant> {
    validate_state_root(state_root)?;
    let root = crate::node_state::NodeStateRoot::open(state_root)?;
    ensure_state_layout(&root)?;
    let grant = parse_control_authority_grant(grant_value)?;
    import_artifact(&root, grant_value)?;
    Ok(grant)
}

pub fn control_live_ticket_value(input: &ControlLiveTicketInput<'_>) -> Result<IoValue> {
    validate_node_id(input.node_id)?;
    validate_ingress_ref(input.identity_ref, "node control live ticket identity ref")?;
    validate_node_id(input.logical_endpoint_id)?;
    validate_node_id(input.live_endpoint_id)?;
    validate_node_id(input.topic)?;
    validate_ingress_refs(input.policy_refs, "node control live ticket policy ref")?;
    validate_ingress_refs(input.evidence_refs, "node control live ticket evidence ref")?;
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

pub fn parse_control_live_ticket(value: &IoValue) -> Result<ControlLiveTicket> {
    let fields = value
        .collect_simple_record("node-control-live-ticket-v1", Some(6))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-live-ticket-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::NODE_CONTROL_LIVE_TICKET_SCHEMA, "node control live ticket")?;
    let node = crate::preserves_rail::value_to_iovalue(&fields[1]);
    let node_fields = node
        .collect_simple_record("node", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("node control live ticket missing node"))?;
    let live = crate::preserves_rail::value_to_iovalue(&fields[2]);
    let live_fields = live
        .collect_simple_record("live", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("node control live ticket missing live endpoint"))?;
    Ok(ControlLiveTicket {
        ticket_ref: crate::preserves_rail::canonical_hash(value)?,
        node_id: record_string(&node_fields[0], "id")?,
        identity_ref: record_ref_string(&node_fields[1], "identity")?,
        logical_endpoint_id: record_string(&node_fields[2], "logical-endpoint")?,
        live_endpoint_id: record_string(&live_fields[0], "endpoint-id")?,
        topic: record_string(&live_fields[1], "topic")?,
        address_refs: record_strings(&live_fields[2], "addresses")?,
        policy_refs: record_ref_strings(&fields[3], "policy")?,
        evidence_refs: record_ref_strings(&fields[4], "evidence")?,
        value: value.clone(),
    })
}

pub fn export_control_live_ticket(input: &ControlLiveTicketExportInput<'_>) -> Result<ControlLiveTicket> {
    let state_root = crate::node_state::NodeStateRoot::open(input.state_root)?;
    validate_state_root(input.state_root)?;
    validate_node_id(input.topic)?;
    ensure_state_layout(&state_root)?;
    let identity = crate::node_identity::parse_identity(&read_preserves(
        &state_root,
        &crate::node_state::NodeStatePath::parse(IDENTITY_FILE)?,
    )?)?;
    let address_refs = Vec::new();
    let value = control_live_ticket_value(&ControlLiveTicketInput {
        node_id: &identity.node_id,
        identity_ref: &identity.identity_ref,
        logical_endpoint_id: &identity.endpoint_id,
        live_endpoint_id: &stable_live_endpoint_id(&identity),
        topic: input.topic,
        address_refs: &address_refs,
        policy_refs: input.policy_refs,
        evidence_refs: input.evidence_refs,
    })?;
    let ticket = parse_control_live_ticket(&value)?;
    import_artifact(&state_root, &value)?;
    Ok(ticket)
}

pub fn admit_control_live_peer(input: &ControlLivePeerAdmitInput<'_>) -> Result<ControlLivePeerAdmission> {
    let state_root = crate::node_state::NodeStateRoot::open(input.state_root)?;
    validate_state_root(input.state_root)?;
    validate_node_id(input.peer_id)?;
    validate_ingress_refs(input.policy_refs, "node control live peer admission policy ref")?;
    validate_ingress_refs(input.evidence_refs, "node control live peer admission evidence ref")?;
    ensure_state_layout(&state_root)?;
    let ticket = parse_control_live_ticket(input.ticket_value)?;
    import_artifact(&state_root, input.ticket_value)?;
    let identity = crate::node_identity::parse_identity(&read_preserves(
        &state_root,
        &crate::node_state::NodeStatePath::parse(IDENTITY_FILE)?,
    )?)?;
    let mut diagnostics = Vec::new();
    if ticket.node_id != identity.node_id {
        diagnostics.push(format!(
            "node control live ticket node {} does not match local node {}",
            ticket.node_id, identity.node_id
        ));
    }
    if ticket.identity_ref != identity.identity_ref {
        diagnostics.push("node control live ticket identity ref does not match local identity".to_string());
    }
    let expected_live_endpoint = stable_live_endpoint_id(&identity);
    if ticket.live_endpoint_id != expected_live_endpoint {
        diagnostics.push(format!(
            "node control live ticket endpoint {} does not match local endpoint {}",
            ticket.live_endpoint_id, expected_live_endpoint
        ));
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = control_live_peer_admission_value(&LivePeerAdmissionValueInput {
        decision,
        peer_id: input.peer_id,
        ticket: &ticket,
        admission_sequence: input.sequence,
        expires_at: input.expires_at,
        policy_refs: input.policy_refs,
        evidence_refs: input.evidence_refs,
        diagnostics: &diagnostics,
    })?;
    let admission = parse_control_live_peer_admission(&value)?;
    import_artifact(&state_root, &value)?;
    Ok(admission)
}

fn control_live_peer_admission_value(input: &LivePeerAdmissionValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    Ok(crate::preserves_rail::record("node-control-live-peer-admission-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_PEER_ADMISSION_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("peer", vec![crate::preserves_rail::string(input.peer_id)]),
        crate::preserves_rail::record("ticket", vec![crate::preserves_rail::string(&input.ticket.ticket_ref)]),
        crate::preserves_rail::record("node", vec![crate::preserves_rail::string(&input.ticket.node_id)]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(&input.ticket.topic)]),
        crate::preserves_rail::record("sequence", vec![crate::preserves_rail::string(
            input.admission_sequence.to_string(),
        )]),
        crate::preserves_rail::record("expires-at", vec![optional_string(
            input.expires_at.map(|value| value.to_string()).as_deref(),
        )]),
        crate::preserves_rail::record("policy", vec![crate::preserves_rail::sequence(
            input.policy_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("evidence", vec![crate::preserves_rail::sequence(
            input.evidence_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ticket-bound"),
                crate::preserves_rail::string(if input.decision == "pass" { "pass" } else { "fail" }),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("peer-topic-bound"),
                crate::preserves_rail::string(if input.decision == "pass" { "pass" } else { "fail" }),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bootstrap-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("authority-grant-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

pub fn parse_control_live_peer_admission(value: &IoValue) -> Result<ControlLivePeerAdmission> {
    let fields = value
        .collect_simple_record("node-control-live-peer-admission-v1", Some(12))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-live-peer-admission-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_PEER_ADMISSION_SCHEMA,
        "node control live peer admission",
    )?;
    Ok(ControlLivePeerAdmission {
        admission_ref: crate::preserves_rail::canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        peer_id: record_string(&fields[2], "peer")?,
        ticket_ref: record_ref_string(&fields[3], "ticket")?,
        node_id: record_string(&fields[4], "node")?,
        topic: record_string(&fields[5], "topic")?,
        sequence: record_u64_string(&fields[6], "sequence")?,
        expires_at: record_optional_u64_string(&fields[7], "expires-at")?,
        policy_refs: record_ref_strings(&fields[8], "policy")?,
        evidence_refs: record_ref_strings(&fields[9], "evidence")?,
        diagnostics: record_strings(&fields[10], "diagnostics")?,
        value: value.clone(),
    })
}
