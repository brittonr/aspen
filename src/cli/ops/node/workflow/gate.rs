pub(crate) fn run(input: super::super::command::live::Gate) -> molten::error::Result<()> {
    let super::super::command::live::Gate {
        bundle,
        verify_receipt,
        require_verify_receipt,
        expected_node,
        expected_topic,
        expected_endpoint,
        expected_peer,
        operations,
        target_scope,
        resource_scope,
        as_of_sequence,
        as_of_epoch,
        receipt_out,
    } = input;
    let bundle_value = super::super::core::read_preserves_file(&bundle)?;
    let verify_receipt_value =
        verify_receipt.as_ref().map(|path| super::super::core::read_preserves_file(path)).transpose()?;
    let gated = molten::node_daemon::gate_node_control_live_workflow_bundle(
        &molten::node_daemon::NodeControlLiveWorkflowBundleGateInput {
            bundle_value: &bundle_value,
            verify_receipt_value: verify_receipt_value.as_ref(),
            require_verify_receipt,
            expected_node: expected_node.as_deref(),
            expected_topic: expected_topic.as_deref(),
            expected_endpoint: expected_endpoint.as_deref(),
            expected_peer: expected_peer.as_deref(),
            expected_operations: &operations,
            expected_target_scope: target_scope.as_deref(),
            expected_resource_scope: resource_scope.as_deref(),
            as_of_sequence,
            as_of_epoch,
        },
    )?;
    super::super::core::emit_named_receipt(
        receipt_out.as_ref(),
        "node control live workflow bundle gate receipt",
        &gated.receipt_value,
    )?;
    println!(
        "node live workflow bundle gate decision={} bundle={} verify={} recomputed-verify={} diagnostics={}",
        gated.decision,
        gated.bundle_ref,
        gated.verify_receipt_ref.as_deref().unwrap_or("none"),
        gated.recomputed_verify_receipt_ref,
        gated.diagnostics.len()
    );
    print_next_step(&gated);
    Ok(())
}

fn print_next_step(gated: &molten::node_daemon::NodeControlLiveWorkflowBundleGate) {
    if gated.decision == "pass" {
        println!("next-step=import-bundle command=\"molten node live-workflow-bundle-import ...\"");
        return;
    }
    let has_malformed_bundle = gated.diagnostics.iter().any(|diagnostic| {
        diagnostic.contains("bundle parse failed") || diagnostic.contains("unsupported receipt kind")
    });
    if has_malformed_bundle {
        println!("next-step=fix-malformed-bundle rerun=\"molten node live-workflow-bundle-verify ...\"");
        return;
    }
    let has_verify_receipt_problem = gated.diagnostics.iter().any(|diagnostic| {
        diagnostic.contains("verify receipt")
            || diagnostic.contains("requires a current verify receipt")
            || diagnostic.contains("does not match recomputed")
    });
    if has_verify_receipt_problem {
        println!(
            "next-step=rerun-verify-receipt command=\"molten node live-workflow-bundle-verify ... --receipt-out ...\""
        );
        return;
    }
    println!(
        "next-step=import-missing-ticket-or-grant command=\"molten node live-ticket-import ...; molten node authority-grant-import ...\""
    );
}
