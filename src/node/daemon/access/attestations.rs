//! Receipt values for live ticket and authority grant imports.

#[derive(Debug, Clone, Copy)]
pub(super) struct LiveTicketImportReceiptValueInput<'a> {
    pub(super) decision: &'a str,
    pub(super) ticket: &'a super::entitlements::ControlLiveTicket,
    pub(super) peer_admission_ref: Option<&'a str>,
    pub(super) peer_id: Option<&'a str>,
    pub(super) as_of_sequence: u64,
    pub(super) imported_refs: &'a [String],
    pub(super) diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub(super) struct AuthorityGrantImportReceiptValueInput<'a> {
    pub(super) decision: &'a str,
    pub(super) grant: &'a super::entitlements::ControlAuthorityGrant,
    pub(super) as_of_epoch: u64,
    pub(super) imported_refs: &'a [String],
    pub(super) diagnostics: &'a [String],
}

pub(super) fn live_ticket_import_receipt_value(
    input: &LiveTicketImportReceiptValueInput<'_>,
) -> crate::error::Result<preserves::IOValue> {
    super::guards::validate_decision(input.decision)?;
    let binding_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("node-control-live-ticket-import-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_TICKET_IMPORT_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("state-root", vec![crate::preserves_rail::string(
            &super::storage::state_root_profile_ref(&())?,
        )]),
        crate::preserves_rail::record("ticket", vec![crate::preserves_rail::string(&input.ticket.ticket_ref)]),
        crate::preserves_rail::record("node", vec![crate::preserves_rail::string(&input.ticket.node_id)]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(&input.ticket.topic)]),
        crate::preserves_rail::record("endpoint", vec![crate::preserves_rail::string(&input.ticket.live_endpoint_id)]),
        crate::preserves_rail::record("peer-admission", vec![super::fields::optional_string(input.peer_admission_ref)]),
        crate::preserves_rail::record("peer", vec![super::fields::optional_string(input.peer_id)]),
        crate::preserves_rail::record("as-of-sequence", vec![crate::preserves_rail::string(
            input.as_of_sequence.to_string(),
        )]),
        crate::preserves_rail::record("imported", vec![crate::preserves_rail::sequence(
            input.imported_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ticket-kind-version"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ticket-topic-endpoint-bound"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("peer-admission-kind-version"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("import-receipt-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
        super::tuning::live_profile_ref_records(None, None),
        super::tuning::live_effective_transport_optional_record(None, None),
    ]))
}

pub(super) fn authority_grant_import_receipt_value(
    input: &AuthorityGrantImportReceiptValueInput<'_>,
) -> crate::error::Result<preserves::IOValue> {
    super::guards::validate_decision(input.decision)?;
    let binding_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("node-control-authority-grant-import-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_AUTHORITY_GRANT_IMPORT_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("state-root", vec![crate::preserves_rail::string(
            &super::storage::state_root_profile_ref(&())?,
        )]),
        crate::preserves_rail::record("grant", vec![crate::preserves_rail::string(&input.grant.grant_ref)]),
        crate::preserves_rail::record("peer", vec![crate::preserves_rail::string(&input.grant.peer_id)]),
        crate::preserves_rail::record("node", vec![crate::preserves_rail::string(&input.grant.node_id)]),
        crate::preserves_rail::record("operations", vec![crate::preserves_rail::sequence(
            input.grant.operations.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("target-scope", vec![crate::preserves_rail::string(&input.grant.target_scope)]),
        crate::preserves_rail::record("resource-scope", vec![crate::preserves_rail::string(
            &input.grant.resource_scope,
        )]),
        crate::preserves_rail::record("as-of-epoch", vec![crate::preserves_rail::string(
            input.as_of_epoch.to_string(),
        )]),
        crate::preserves_rail::record("imported", vec![crate::preserves_rail::sequence(
            input.imported_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("grant-kind-version"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("peer-node-operation-scope-bound"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("grant-fresh-and-unrevoked"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("import-receipt-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}
