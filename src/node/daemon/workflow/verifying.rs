//! Live workflow bundle verify and gate with their input checks.

pub fn verify_control_live_workflow_bundle(
    input: &super::bundling::ControlLiveWorkflowBundleVerifyInput<'_>,
) -> crate::error::Result<super::artifacts::ControlLiveWorkflowBundleVerify> {
    validate_live_workflow_bundle_verify_input(input)?;
    let bundle_ref = crate::preserves_rail::canonical_hash(input.bundle_value)?;
    let expected = live_workflow_bundle_expected_input_from_verify(input);
    let parsed = super::manifests::parse_control_live_workflow_bundle(input.bundle_value);
    let (ticket_ref, peer_admission_ref, authority_grant_ref, receipt_refs, diagnostics) = match parsed {
        Ok(bundle) => {
            let ticket = super::issuance::parse_control_live_ticket(&bundle.ticket_value)?;
            let admission = super::peering::parse_control_live_peer_admission(&bundle.peer_admission_value)?;
            let authority = super::issuance::parse_control_authority_grant(&bundle.authority_grant_value)?;
            let receipt_value_refs = bundle.receipt_values.iter().collect::<Vec<_>>();
            let mut diagnostics = live_workflow_bundle_expected_diagnostics(&expected, &ticket, &admission, &authority);
            diagnostics.extend(super::exporting::live_workflow_bundle_receipt_diagnostics(&receipt_value_refs));
            (
                Some(bundle.ticket_ref),
                Some(bundle.peer_admission_ref),
                Some(bundle.authority_grant_ref),
                bundle.receipt_refs,
                diagnostics,
            )
        }
        Err(error) => (None, None, None, Vec::new(), vec![format!(
            "node control live workflow bundle parse failed: {error}"
        )]),
    };
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = super::exports::live_workflow_bundle_verify_receipt_value(
        &super::exports::LiveWorkflowBundleVerifyReceiptValueInput {
            decision,
            bundle_ref: &bundle_ref,
            ticket_ref: ticket_ref.as_deref(),
            peer_admission_ref: peer_admission_ref.as_deref(),
            authority_grant_ref: authority_grant_ref.as_deref(),
            receipt_refs: &receipt_refs,
            expected: &expected,
            diagnostics: &diagnostics,
        },
    )?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    Ok(super::artifacts::ControlLiveWorkflowBundleVerify {
        bundle_ref,
        ticket_ref,
        peer_admission_ref,
        authority_grant_ref,
        receipt_refs,
        diagnostics,
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
    })
}

pub fn gate_control_live_workflow_bundle(
    input: &super::bundling::ControlLiveWorkflowBundleGateInput<'_>,
) -> crate::error::Result<super::artifacts::ControlLiveWorkflowBundleGate> {
    let verify_input = live_workflow_bundle_verify_input_from_gate(input);
    let verified = verify_control_live_workflow_bundle(&verify_input)?;
    let expected = live_workflow_bundle_expected_input_from_verify(&verify_input);
    let mut diagnostics = verified.diagnostics.clone();
    let verify_receipt_ref = match input.verify_receipt_value {
        Some(value) => match super::manifests::parse_control_live_workflow_bundle_verify_receipt(value) {
            Ok(receipt) => {
                if receipt.receipt_ref != verified.receipt_ref {
                    diagnostics.push(format!(
                        "node control live workflow bundle gate verify receipt {} does not match recomputed {}",
                        receipt.receipt_ref, verified.receipt_ref
                    ));
                }
                Some(receipt.receipt_ref)
            }
            Err(error) => {
                let receipt_ref = crate::preserves_rail::canonical_hash(value)?;
                diagnostics
                    .push(format!("node control live workflow bundle gate verify receipt parse failed: {error}"));
                Some(receipt_ref)
            }
        },
        None => {
            if input.require_verify_receipt {
                diagnostics
                    .push("node control live workflow bundle gate requires a current verify receipt".to_string());
            }
            None
        }
    };
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = super::gates::live_workflow_bundle_gate_receipt_value(
        &super::gates::LiveWorkflowBundleGateReceiptValueInput {
            decision,
            bundle_ref: &verified.bundle_ref,
            verify_receipt_ref: verify_receipt_ref.as_deref(),
            recomputed_verify_receipt_ref: &verified.receipt_ref,
            ticket_ref: verified.ticket_ref.as_deref(),
            peer_admission_ref: verified.peer_admission_ref.as_deref(),
            authority_grant_ref: verified.authority_grant_ref.as_deref(),
            receipt_refs: &verified.receipt_refs,
            expected: &expected,
            diagnostics: &diagnostics,
        },
    )?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    Ok(super::artifacts::ControlLiveWorkflowBundleGate {
        bundle_ref: verified.bundle_ref,
        verify_receipt_ref,
        recomputed_verify_receipt_ref: verified.receipt_ref,
        ticket_ref: verified.ticket_ref,
        peer_admission_ref: verified.peer_admission_ref,
        authority_grant_ref: verified.authority_grant_ref,
        receipt_refs: verified.receipt_refs,
        diagnostics,
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
    })
}

pub(super) fn validate_live_workflow_bundle_verify_input(
    input: &super::bundling::ControlLiveWorkflowBundleVerifyInput<'_>,
) -> crate::error::Result<()> {
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
    Ok(())
}

pub(super) fn live_workflow_bundle_expected_input_from_verify<'a>(
    input: &'a super::bundling::ControlLiveWorkflowBundleVerifyInput<'a>,
) -> super::intake::LiveWorkflowBundleExpectedInput<'a> {
    super::intake::LiveWorkflowBundleExpectedInput {
        expected_node: input.expected_node,
        expected_topic: input.expected_topic,
        expected_endpoint: input.expected_endpoint,
        expected_peer: input.expected_peer,
        expected_operations: input.expected_operations,
        expected_target_scope: input.expected_target_scope,
        expected_resource_scope: input.expected_resource_scope,
        as_of_sequence: input.as_of_sequence,
        as_of_epoch: input.as_of_epoch,
    }
}

fn live_workflow_bundle_verify_input_from_gate<'a>(
    input: &'a super::bundling::ControlLiveWorkflowBundleGateInput<'a>,
) -> super::bundling::ControlLiveWorkflowBundleVerifyInput<'a> {
    super::bundling::ControlLiveWorkflowBundleVerifyInput {
        bundle_value: input.bundle_value,
        expected_node: input.expected_node,
        expected_topic: input.expected_topic,
        expected_endpoint: input.expected_endpoint,
        expected_peer: input.expected_peer,
        expected_operations: input.expected_operations,
        expected_target_scope: input.expected_target_scope,
        expected_resource_scope: input.expected_resource_scope,
        as_of_sequence: input.as_of_sequence,
        as_of_epoch: input.as_of_epoch,
    }
}

pub(super) fn live_workflow_bundle_expected_diagnostics(
    input: &super::intake::LiveWorkflowBundleExpectedInput<'_>,
    ticket: &super::entitlements::ControlLiveTicket,
    admission: &super::entitlements::ControlLivePeerAdmission,
    authority: &super::entitlements::ControlAuthorityGrant,
) -> Vec<String> {
    let mut diagnostics = super::exporting::live_workflow_bundle_binding_diagnostics(ticket, admission, authority);
    diagnostics.extend(super::scoping::live_ticket_expected_diagnostics(input, ticket, admission));
    diagnostics.extend(super::scoping::authority_grant_expected_diagnostics(input, authority));
    diagnostics
}
