//! Live workflow bundle import and its receipt value.

#[derive(Debug)]
struct ImportParts {
    imported_refs: Vec<String>,
    ticket_import_ref: Option<String>,
    authority_import_ref: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct LiveWorkflowBundleExpectedInput<'a> {
    pub(super) expected_node: Option<&'a str>,
    pub(super) expected_topic: Option<&'a str>,
    pub(super) expected_endpoint: Option<&'a str>,
    pub(super) expected_peer: Option<&'a str>,
    pub(super) expected_operations: &'a [String],
    pub(super) expected_target_scope: Option<&'a str>,
    pub(super) expected_resource_scope: Option<&'a str>,
    pub(super) as_of_sequence: u64,
    pub(super) as_of_epoch: u64,
}

#[derive(Debug, Clone, Copy)]
struct LiveWorkflowBundleImportReceiptValueInput<'a> {
    decision: &'a str,
    bundle: &'a super::artifacts::ControlLiveWorkflowBundle,
    ticket_import_ref: Option<&'a str>,
    authority_import_ref: Option<&'a str>,
    imported_refs: &'a [String],
    diagnostics: &'a [String],
}

pub fn import_control_live_workflow_bundle(
    input: &super::bundling::ControlLiveWorkflowBundleImportInput<'_>,
) -> crate::error::Result<super::artifacts::ControlLiveWorkflowBundleImport> {
    super::storage::validate_state_root(input.state_root)?;
    let state_root = crate::node_state::DirectoryRoot::open(input.state_root)?;
    import_control_live_workflow_bundle_with_root(&state_root, input)
}

pub(super) fn import_control_live_workflow_bundle_with_root(
    state_root: &crate::node_state::DirectoryRoot,
    input: &super::bundling::ControlLiveWorkflowBundleImportInput<'_>,
) -> crate::error::Result<super::artifacts::ControlLiveWorkflowBundleImport> {
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
    for operation in input.expected_operations {
        super::storage::validate_node_id(operation)?;
    }
    if let Some(scope) = input.expected_target_scope {
        super::storage::validate_node_id(scope)?;
    }
    if let Some(scope) = input.expected_resource_scope {
        super::storage::validate_node_id(scope)?;
    }
    let bundle = super::manifests::parse_control_live_workflow_bundle(input.bundle_value)?;
    let ticket = super::issuance::parse_control_live_ticket(&bundle.ticket_value)?;
    let admission = super::peering::parse_control_live_peer_admission(&bundle.peer_admission_value)?;
    let authority = super::issuance::parse_control_authority_grant(&bundle.authority_grant_value)?;
    let mut diagnostics =
        super::applying::live_workflow_bundle_import_diagnostics(input, &ticket, &admission, &authority);
    let mut parts = ImportParts {
        imported_refs: Vec::with_capacity(bundle.receipt_values.len().saturating_add(5)),
        ticket_import_ref: None,
        authority_import_ref: None,
    };
    if diagnostics.is_empty() {
        let (imported, import_diagnostics) = import_parts(state_root, input, &bundle)?;
        parts = imported;
        diagnostics.extend(import_diagnostics);
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = live_workflow_bundle_import_receipt_value(&LiveWorkflowBundleImportReceiptValueInput {
        decision,
        bundle: &bundle,
        ticket_import_ref: parts.ticket_import_ref.as_deref(),
        authority_import_ref: parts.authority_import_ref.as_deref(),
        imported_refs: &parts.imported_refs,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    super::storage::import_artifact(state_root, &receipt_value)?;
    Ok(super::artifacts::ControlLiveWorkflowBundleImport {
        bundle_ref: bundle.bundle_ref,
        ticket_import_ref: parts.ticket_import_ref,
        authority_import_ref: parts.authority_import_ref,
        imported_refs: parts.imported_refs,
        diagnostics,
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
    })
}

fn import_parts(
    state_root: &crate::node_state::DirectoryRoot,
    input: &super::bundling::ControlLiveWorkflowBundleImportInput<'_>,
    bundle: &super::artifacts::ControlLiveWorkflowBundle,
) -> crate::error::Result<(ImportParts, Vec<String>)> {
    let mut diagnostics = Vec::new();
    let mut parts = ImportParts {
        imported_refs: Vec::with_capacity(bundle.receipt_values.len().saturating_add(5)),
        ticket_import_ref: None,
        authority_import_ref: None,
    };
    let ticket_import_input = super::offers::ControlLiveTicketImportInput {
        state_root: input.state_root,
        ticket_value: &bundle.ticket_value,
        peer_admission_value: Some(&bundle.peer_admission_value),
        expected_node: input.expected_node,
        expected_topic: input.expected_topic,
        expected_endpoint: input.expected_endpoint,
        expected_peer: input.expected_peer,
        as_of_sequence: input.as_of_sequence,
    };
    let ticket_import = super::importing::import_control_live_ticket_with_root(state_root, &ticket_import_input)?;
    let authority_import_input = super::offers::ControlAuthorityGrantImportInput {
        state_root: input.state_root,
        grant_value: &bundle.authority_grant_value,
        expected_peer: input.expected_peer,
        expected_node: input.expected_node,
        expected_operations: input.expected_operations,
        expected_target_scope: input.expected_target_scope,
        expected_resource_scope: input.expected_resource_scope,
        as_of_epoch: input.as_of_epoch,
    };
    let authority_import =
        super::importing::import_control_authority_grant_checked_with_root(state_root, &authority_import_input)?;
    parts.ticket_import_ref = Some(ticket_import.receipt_ref.clone());
    parts.authority_import_ref = Some(authority_import.receipt_ref.clone());
    if ticket_import.decision != "pass" {
        diagnostics.extend(ticket_import.diagnostics.iter().cloned());
    }
    if authority_import.decision != "pass" {
        diagnostics.extend(authority_import.diagnostics.iter().cloned());
    }
    if diagnostics.is_empty() {
        parts.imported_refs.extend(ticket_import.imported_refs);
        parts.imported_refs.extend(authority_import.imported_refs);
        parts.imported_refs.push(super::storage::import_artifact(state_root, input.bundle_value)?);
        for receipt_value in &bundle.receipt_values {
            parts.imported_refs.push(super::storage::import_artifact(state_root, receipt_value)?);
        }
    }
    Ok((parts, diagnostics))
}

fn live_workflow_bundle_import_receipt_value(
    input: &LiveWorkflowBundleImportReceiptValueInput<'_>,
) -> crate::error::Result<preserves::IOValue> {
    super::guards::validate_decision(input.decision)?;
    let binding_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("node-control-live-workflow-bundle-import-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_IMPORT_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("state-root", vec![crate::preserves_rail::string(
            &super::storage::state_root_profile_ref(&())?,
        )]),
        crate::preserves_rail::record("bundle", vec![crate::preserves_rail::string(&input.bundle.bundle_ref)]),
        crate::preserves_rail::record("ticket", vec![crate::preserves_rail::string(&input.bundle.ticket_ref)]),
        crate::preserves_rail::record("peer-admission", vec![crate::preserves_rail::string(
            &input.bundle.peer_admission_ref,
        )]),
        crate::preserves_rail::record("authority-grant", vec![crate::preserves_rail::string(
            &input.bundle.authority_grant_ref,
        )]),
        crate::preserves_rail::record("ticket-import", vec![super::fields::optional_string(input.ticket_import_ref)]),
        crate::preserves_rail::record("authority-import", vec![super::fields::optional_string(
            input.authority_import_ref,
        )]),
        crate::preserves_rail::record("imported", vec![crate::preserves_rail::sequence(
            input.imported_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-kind-version"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ticket-admission-imported"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("authority-grant-imported"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-receipt-imported"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-import-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}
