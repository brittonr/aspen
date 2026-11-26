//! Live peer admission against an imported ticket.

#[derive(Debug, Clone, Copy)]
struct LivePeerAdmissionValueInput<'a> {
    decision: &'a str,
    peer_id: &'a str,
    ticket: &'a super::entitlements::ControlLiveTicket,
    admission_sequence: u64,
    expires_at: Option<u64>,
    policy_refs: &'a [String],
    evidence_refs: &'a [String],
    diagnostics: &'a [String],
}

pub fn admit_control_live_peer(
    input: &super::offers::ControlLivePeerAdmitInput<'_>,
) -> crate::error::Result<super::entitlements::ControlLivePeerAdmission> {
    let state_root = crate::node_state::DirectoryRoot::open(input.state_root)?;
    super::storage::validate_state_root(input.state_root)?;
    super::storage::validate_node_id(input.peer_id)?;
    super::guards::validate_ingress_refs(input.policy_refs, "node control live peer admission policy ref")?;
    super::guards::validate_ingress_refs(input.evidence_refs, "node control live peer admission evidence ref")?;
    super::storage::ensure_state_layout(&state_root)?;
    let ticket = super::issuance::parse_control_live_ticket(input.ticket_value)?;
    super::storage::import_artifact(&state_root, input.ticket_value)?;
    let identity = crate::node_identity::parse_identity(&super::storage::read_preserves(
        &state_root,
        &crate::node_state::RelativePath::parse(super::settings::IDENTITY_FILE)?,
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
    let expected_live_endpoint = super::endpoints::stable_live_endpoint_id(&identity);
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
    super::storage::import_artifact(&state_root, &value)?;
    Ok(admission)
}

fn control_live_peer_admission_value(
    input: &LivePeerAdmissionValueInput<'_>,
) -> crate::error::Result<preserves::IOValue> {
    super::guards::validate_decision(input.decision)?;
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
        crate::preserves_rail::record("expires-at", vec![super::fields::optional_string(
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

pub fn parse_control_live_peer_admission(
    value: &preserves::IOValue,
) -> crate::error::Result<super::entitlements::ControlLivePeerAdmission> {
    let fields = value
        .collect_simple_record("node-control-live-peer-admission-v1", Some(12))
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected <node-control-live-peer-admission-v1 ...>"))?;
    super::guards::require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_PEER_ADMISSION_SCHEMA,
        "node control live peer admission",
    )?;
    Ok(super::entitlements::ControlLivePeerAdmission {
        admission_ref: crate::preserves_rail::canonical_hash(value)?,
        decision: super::fields::record_string(&fields[1], "decision")?,
        peer_id: super::fields::record_string(&fields[2], "peer")?,
        ticket_ref: super::fields::record_ref_string(&fields[3], "ticket")?,
        node_id: super::fields::record_string(&fields[4], "node")?,
        topic: super::fields::record_string(&fields[5], "topic")?,
        sequence: super::fields::record_u64_string(&fields[6], "sequence")?,
        expires_at: super::fields::record_optional_u64_string(&fields[7], "expires-at")?,
        policy_refs: super::fields::record_ref_strings(&fields[8], "policy")?,
        evidence_refs: super::fields::record_ref_strings(&fields[9], "evidence")?,
        diagnostics: super::fields::record_strings(&fields[10], "diagnostics")?,
        value: value.clone(),
    })
}
