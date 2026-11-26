//! Checked live ticket and authority grant imports with their diagnostics.

pub fn import_control_live_ticket(
    input: &super::offers::ControlLiveTicketImportInput<'_>,
) -> crate::error::Result<super::entitlements::ControlLiveTicketImport> {
    super::storage::validate_state_root(input.state_root)?;
    let state_root = crate::node_state::DirectoryRoot::open(input.state_root)?;
    import_control_live_ticket_with_root(&state_root, input)
}

pub(super) fn import_control_live_ticket_with_root(
    state_root: &crate::node_state::DirectoryRoot,
    input: &super::offers::ControlLiveTicketImportInput<'_>,
) -> crate::error::Result<super::entitlements::ControlLiveTicketImport> {
    super::storage::ensure_state_layout(state_root)?;
    if let Some(node) = input.expected_node {
        super::storage::validate_node_id(node)?;
    }
    if let Some(topic) = input.expected_topic {
        super::storage::validate_node_id(topic)?;
    }
    if let Some(endpoint) = input.expected_endpoint {
        super::storage::validate_node_id(endpoint)?;
    }
    if let Some(peer) = input.expected_peer {
        super::storage::validate_node_id(peer)?;
    }
    let ticket = super::issuance::parse_control_live_ticket(input.ticket_value)?;
    let admission = input.peer_admission_value.map(super::peering::parse_control_live_peer_admission).transpose()?;
    let mut diagnostics = live_ticket_import_diagnostics(input, &ticket, admission.as_ref());
    if input.peer_admission_value.is_some() && admission.is_none() {
        diagnostics.push("node control live ticket import admission was not parsed".to_string());
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let mut imported_refs = Vec::with_capacity(2);
    if diagnostics.is_empty() {
        imported_refs.push(super::storage::import_artifact(state_root, input.ticket_value)?);
        if let Some(value) = input.peer_admission_value {
            imported_refs.push(super::storage::import_artifact(state_root, value)?);
        }
    }
    let peer_id = admission.as_ref().map(|value| value.peer_id.as_str()).or(input.expected_peer);
    let receipt_value = super::attestations::live_ticket_import_receipt_value(
        &super::attestations::LiveTicketImportReceiptValueInput {
            decision,
            ticket: &ticket,
            peer_admission_ref: admission.as_ref().map(|value| value.admission_ref.as_str()),
            peer_id,
            as_of_sequence: input.as_of_sequence,
            imported_refs: &imported_refs,
            diagnostics: &diagnostics,
        },
    )?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    super::storage::import_artifact(state_root, &receipt_value)?;
    Ok(super::entitlements::ControlLiveTicketImport {
        decision: decision.to_string(),
        ticket_ref: ticket.ticket_ref,
        peer_admission_ref: admission.map(|value| value.admission_ref),
        imported_refs,
        diagnostics,
        receipt_ref,
        receipt_value,
    })
}

pub fn import_control_authority_grant_checked(
    input: &super::offers::ControlAuthorityGrantImportInput<'_>,
) -> crate::error::Result<super::entitlements::ControlAuthorityGrantImport> {
    super::storage::validate_state_root(input.state_root)?;
    let state_root = crate::node_state::DirectoryRoot::open(input.state_root)?;
    import_control_authority_grant_checked_with_root(&state_root, input)
}

pub(super) fn import_control_authority_grant_checked_with_root(
    state_root: &crate::node_state::DirectoryRoot,
    input: &super::offers::ControlAuthorityGrantImportInput<'_>,
) -> crate::error::Result<super::entitlements::ControlAuthorityGrantImport> {
    super::storage::ensure_state_layout(state_root)?;
    if let Some(peer) = input.expected_peer {
        super::storage::validate_node_id(peer)?;
    }
    if let Some(node) = input.expected_node {
        super::storage::validate_node_id(node)?;
    }
    for operation in input.expected_operations {
        super::storage::validate_node_id(operation)?;
    }
    if let Some(scope) = input.expected_target_scope {
        super::storage::validate_node_id(scope)?;
    }
    if let Some(scope) = input.expected_resource_scope {
        super::storage::validate_node_id(scope)?;
    }
    let grant = super::issuance::parse_control_authority_grant(input.grant_value)?;
    let diagnostics = authority_grant_import_diagnostics(input, &grant);
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let mut imported_refs = Vec::with_capacity(1);
    if diagnostics.is_empty() {
        imported_refs.push(super::storage::import_artifact(state_root, input.grant_value)?);
    }
    let receipt_value = super::attestations::authority_grant_import_receipt_value(
        &super::attestations::AuthorityGrantImportReceiptValueInput {
            decision,
            grant: &grant,
            as_of_epoch: input.as_of_epoch,
            imported_refs: &imported_refs,
            diagnostics: &diagnostics,
        },
    )?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    super::storage::import_artifact(state_root, &receipt_value)?;
    Ok(super::entitlements::ControlAuthorityGrantImport {
        decision: decision.to_string(),
        grant_ref: grant.grant_ref,
        imported_refs,
        diagnostics,
        receipt_ref,
        receipt_value,
    })
}

pub(super) fn live_ticket_import_diagnostics(
    input: &super::offers::ControlLiveTicketImportInput<'_>,
    ticket: &super::entitlements::ControlLiveTicket,
    admission: Option<&super::entitlements::ControlLivePeerAdmission>,
) -> Vec<String> {
    super::scoping::evaluate_live_ticket_scope(super::entitlements::LiveTicketScopeInput {
        ticket,
        admission,
        expected_node: input.expected_node,
        expected_topic: input.expected_topic,
        expected_endpoint: input.expected_endpoint,
        expected_peer: input.expected_peer,
        as_of_sequence: input.as_of_sequence,
        required_policy_refs: &[],
    })
    .diagnostics
}

pub(super) fn authority_grant_import_diagnostics(
    input: &super::offers::ControlAuthorityGrantImportInput<'_>,
    grant: &super::entitlements::ControlAuthorityGrant,
) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(8);
    if let Some(expected) = input.expected_peer
        && grant.peer_id != expected
    {
        diagnostics.push(format!(
            "node control authority grant import peer {} does not match expected {expected}",
            grant.peer_id
        ));
    }
    if let Some(expected) = input.expected_node
        && grant.node_id != expected
    {
        diagnostics.push(format!(
            "node control authority grant import node {} does not match expected {expected}",
            grant.node_id
        ));
    }
    for operation in input.expected_operations {
        if !grant.operations.iter().any(|candidate| candidate == "*" || candidate == operation) {
            diagnostics.push(format!("node control authority grant import does not allow operation {operation}"));
        }
    }
    if let Some(expected) = input.expected_target_scope
        && grant.target_scope != "*"
        && grant.target_scope != expected
    {
        diagnostics.push(format!(
            "node control authority grant import target scope {} does not cover expected {expected}",
            grant.target_scope
        ));
    }
    if let Some(expected) = input.expected_resource_scope
        && grant.resource_scope != "*"
        && grant.resource_scope != expected
    {
        diagnostics.push(format!(
            "node control authority grant import resource scope {} does not cover expected {expected}",
            grant.resource_scope
        ));
    }
    if grant.epoch > input.as_of_epoch {
        diagnostics.push(format!("node control authority grant import is not valid until epoch {}", grant.epoch));
    }
    if let Some(expires_at) = grant.expires_at
        && expires_at < input.as_of_epoch
    {
        diagnostics.push(format!("node control authority grant import expired at epoch {expires_at}"));
    }
    if !grant.revocation_refs.is_empty() {
        diagnostics.push("node control authority grant import has revocation refs".to_string());
    }
    diagnostics
}
