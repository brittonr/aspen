//! Expected-binding diagnostics and live ticket scope evaluation.

pub(super) fn live_ticket_expected_diagnostics(
    input: &super::intake::LiveWorkflowBundleExpectedInput<'_>,
    ticket: &super::entitlements::ControlLiveTicket,
    admission: &super::entitlements::ControlLivePeerAdmission,
) -> Vec<String> {
    super::importing::live_ticket_import_diagnostics(
        &super::offers::ControlLiveTicketImportInput {
            state_root: std::path::Path::new("."),
            ticket_value: &ticket.value,
            peer_admission_value: Some(&admission.value),
            expected_node: input.expected_node,
            expected_topic: input.expected_topic,
            expected_endpoint: input.expected_endpoint,
            expected_peer: input.expected_peer,
            as_of_sequence: input.as_of_sequence,
        },
        ticket,
        Some(admission),
    )
}

const LIVE_TICKET_SCOPE_DIAGNOSTIC_CAPACITY: usize = 8;

pub(super) fn authority_grant_expected_diagnostics(
    input: &super::intake::LiveWorkflowBundleExpectedInput<'_>,
    authority: &super::entitlements::ControlAuthorityGrant,
) -> Vec<String> {
    super::importing::authority_grant_import_diagnostics(
        &super::offers::ControlAuthorityGrantImportInput {
            state_root: std::path::Path::new("."),
            grant_value: &authority.value,
            expected_peer: input.expected_peer,
            expected_node: input.expected_node,
            expected_operations: input.expected_operations,
            expected_target_scope: input.expected_target_scope,
            expected_resource_scope: input.expected_resource_scope,
            as_of_epoch: input.as_of_epoch,
        },
        authority,
    )
}

pub fn evaluate_live_ticket_scope(
    input: super::entitlements::LiveTicketScopeInput<'_>,
) -> super::entitlements::LiveTicketScopeDecision {
    let mut diagnostics = Vec::with_capacity(LIVE_TICKET_SCOPE_DIAGNOSTIC_CAPACITY);
    if let Some(expected) = input.expected_node
        && input.ticket.node_id != expected
    {
        diagnostics.push(format!(
            "node control live ticket import node {} does not match expected {expected}",
            input.ticket.node_id
        ));
    }
    if let Some(expected) = input.expected_topic
        && input.ticket.topic != expected
    {
        diagnostics.push(format!(
            "node control live ticket import topic {} does not match expected {expected}",
            input.ticket.topic
        ));
    }
    if let Some(expected) = input.expected_endpoint
        && input.ticket.live_endpoint_id != expected
    {
        diagnostics.push(format!(
            "node control live ticket import endpoint {} does not match expected {expected}",
            input.ticket.live_endpoint_id
        ));
    }
    for required_policy_ref in input.required_policy_refs {
        if !input.ticket.policy_refs.iter().any(|policy_ref| policy_ref == required_policy_ref) {
            diagnostics.push(format!("node control live ticket import missing required policy {required_policy_ref}"));
        }
    }
    if let Some(admission) = input.admission {
        diagnostics.extend(live_ticket_admission_scope_diagnostics(input, admission));
    } else if let Some(expected) = input.expected_peer {
        diagnostics
            .push(format!("node control live ticket import missing peer admission for expected peer {expected}"));
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    super::entitlements::LiveTicketScopeDecision {
        decision: decision.to_string(),
        diagnostics,
    }
}

fn live_ticket_admission_scope_diagnostics(
    input: super::entitlements::LiveTicketScopeInput<'_>,
    admission: &super::entitlements::ControlLivePeerAdmission,
) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(LIVE_TICKET_SCOPE_DIAGNOSTIC_CAPACITY);
    if admission.decision != "pass" {
        diagnostics.push(format!(
            "node control live peer admission {} decision {}",
            admission.admission_ref, admission.decision
        ));
    }
    if admission.ticket_ref != input.ticket.ticket_ref {
        diagnostics.push(format!(
            "node control live peer admission {} ticket {} does not match ticket {}",
            admission.admission_ref, admission.ticket_ref, input.ticket.ticket_ref
        ));
    }
    if admission.node_id != input.ticket.node_id {
        diagnostics.push(format!(
            "node control live peer admission {} node {} does not match ticket node {}",
            admission.admission_ref, admission.node_id, input.ticket.node_id
        ));
    }
    if admission.topic != input.ticket.topic {
        diagnostics.push(format!(
            "node control live peer admission {} topic {} does not match ticket topic {}",
            admission.admission_ref, admission.topic, input.ticket.topic
        ));
    }
    for required_policy_ref in input.required_policy_refs {
        if !admission.policy_refs.iter().any(|policy_ref| policy_ref == required_policy_ref) {
            diagnostics.push(format!(
                "node control live peer admission {} missing required policy {required_policy_ref}",
                admission.admission_ref
            ));
        }
    }
    if let Some(expected) = input.expected_peer
        && admission.peer_id != expected
    {
        diagnostics.push(format!(
            "node control live peer admission {} peer {} does not match expected {expected}",
            admission.admission_ref, admission.peer_id
        ));
    }
    if admission.sequence > input.as_of_sequence {
        diagnostics.push(format!(
            "node control live peer admission {} is not valid until sequence {}",
            admission.admission_ref, admission.sequence
        ));
    }
    if let Some(expires_at) = admission.expires_at
        && expires_at < input.as_of_sequence
    {
        diagnostics.push(format!(
            "node control live peer admission {} expired at sequence {expires_at}",
            admission.admission_ref
        ));
    }
    diagnostics
}
