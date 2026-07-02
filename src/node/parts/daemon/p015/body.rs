
fn authority_grant_import_receipt_value(input: &AuthorityGrantImportReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    let binding_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("node-control-authority-grant-import-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_AUTHORITY_GRANT_IMPORT_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("state-root", vec![crate::preserves_rail::string(&state_root_profile_ref(
            input.state_root,
        )?)]),
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

fn live_workflow_bundle_value(input: &LiveWorkflowBundleValueInput<'_>) -> Result<IoValue> {
    let binding_status = if input.diagnostics.is_empty() { "pass" } else { "fail" };
    let receipt_refs = live_workflow_bundle_receipt_refs(input.receipt_values)?;
    Ok(crate::preserves_rail::record("node-control-live-workflow-bundle-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_SCHEMA),
        crate::preserves_rail::record("ticket", vec![(*input.ticket_value).clone()]),
        crate::preserves_rail::record("peer-admission", vec![(*input.admission_value).clone()]),
        crate::preserves_rail::record("authority-grant", vec![(*input.authority_value).clone()]),
        crate::preserves_rail::record("receipts", vec![crate::preserves_rail::sequence(
            input.receipt_values.iter().map(|value| (**value).clone()).collect(),
        )]),
        crate::preserves_rail::record("ticket-ref", vec![crate::preserves_rail::string(&input.ticket.ticket_ref)]),
        crate::preserves_rail::record("peer-admission-ref", vec![crate::preserves_rail::string(
            &input.admission.admission_ref,
        )]),
        crate::preserves_rail::record("authority-grant-ref", vec![crate::preserves_rail::string(
            &input.authority.grant_ref,
        )]),
        crate::preserves_rail::record("receipt-refs", vec![crate::preserves_rail::sequence(
            receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ticket-kind-version"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("peer-admission-kind-version"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("authority-grant-kind-version"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ticket-admission-bound"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("authority-grant-bound"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn live_workflow_bundle_export_receipt_value(input: &LiveWorkflowBundleExportReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    let binding_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("node-control-live-workflow-bundle-export-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_EXPORT_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("bundle", vec![crate::preserves_rail::string(&input.bundle.bundle_ref)]),
        crate::preserves_rail::record("ticket", vec![crate::preserves_rail::string(&input.bundle.ticket_ref)]),
        crate::preserves_rail::record("peer-admission", vec![crate::preserves_rail::string(
            &input.bundle.peer_admission_ref,
        )]),
        crate::preserves_rail::record("authority-grant", vec![crate::preserves_rail::string(
            &input.bundle.authority_grant_ref,
        )]),
        crate::preserves_rail::record("receipts", vec![crate::preserves_rail::sequence(
            input.bundle.receipt_refs.iter().map(crate::preserves_rail::string).collect(),
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
                crate::preserves_rail::string("bundle-member-bindings"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-receipt-kinds"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn live_workflow_bundle_verify_receipt_value(input: &LiveWorkflowBundleVerifyReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    let binding_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("node-control-live-workflow-bundle-verify-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_VERIFY_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("bundle", vec![crate::preserves_rail::string(input.bundle_ref)]),
        crate::preserves_rail::record("ticket", vec![optional_string(input.ticket_ref)]),
        crate::preserves_rail::record("peer-admission", vec![optional_string(input.peer_admission_ref)]),
        crate::preserves_rail::record("authority-grant", vec![optional_string(input.authority_grant_ref)]),
        crate::preserves_rail::record("receipts", vec![crate::preserves_rail::sequence(
            input.receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("expected", vec![live_workflow_bundle_expected_value(input.expected)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-kind-version"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-member-bindings"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-receipt-kinds"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("expected-bindings"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("verify-receipt-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn live_workflow_bundle_gate_receipt_value(input: &LiveWorkflowBundleGateReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    let gate_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("node-control-live-workflow-bundle-gate-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_GATE_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("bundle", vec![crate::preserves_rail::string(input.bundle_ref)]),
        crate::preserves_rail::record("verify-receipt", vec![optional_string(input.verify_receipt_ref)]),
        crate::preserves_rail::record("recomputed-verify", vec![crate::preserves_rail::string(
            input.recomputed_verify_receipt_ref,
        )]),
        crate::preserves_rail::record("ticket", vec![optional_string(input.ticket_ref)]),
        crate::preserves_rail::record("peer-admission", vec![optional_string(input.peer_admission_ref)]),
        crate::preserves_rail::record("authority-grant", vec![optional_string(input.authority_grant_ref)]),
        crate::preserves_rail::record("receipts", vec![crate::preserves_rail::sequence(
            input.receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("expected", vec![live_workflow_bundle_expected_value(input.expected)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-verification"),
                crate::preserves_rail::string(gate_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("verify-receipt-current"),
                crate::preserves_rail::string(gate_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("expected-bindings"),
                crate::preserves_rail::string(gate_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("gate-receipt-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-import-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}
