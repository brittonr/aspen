pub(crate) fn run(input: super::super::command::live::Apply) -> molten::error::Result<()> {
    let loaded = Loaded::read(&input)?;
    let applied = execute(&input, &loaded)?;
    write_outputs(&input, &applied)?;
    print_next_step(&applied, loaded.request_value.is_some(), input.send);
    Ok(())
}

struct Loaded {
    bundle_value: preserves::IOValue,
    gate_receipt_value: Option<preserves::IOValue>,
    request_value: Option<preserves::IOValue>,
}

impl Loaded {
    fn read(input: &super::super::command::live::Apply) -> molten::error::Result<Self> {
        let bundle_value = super::super::core::read_preserves_file(&input.bundle)?;
        let gate_receipt_value =
            input.gate_receipt.as_ref().map(|path| super::super::core::read_preserves_file(path)).transpose()?;
        let request_value =
            input.request.as_ref().map(|path| super::super::core::read_preserves_file(path)).transpose()?;
        Ok(Self {
            bundle_value,
            gate_receipt_value,
            request_value,
        })
    }
}

fn execute(
    input: &super::super::command::live::Apply,
    loaded: &Loaded,
) -> molten::error::Result<molten::node_daemon::ControlLiveWorkflowBundleApply> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(molten::error::MoltenError::from)?;
    runtime.block_on(molten::node_daemon::apply_node_control_live_workflow_bundle(
        &molten::node_daemon::ControlLiveWorkflowBundleApplyInput {
            state_root: &input.state_root,
            bundle_value: &loaded.bundle_value,
            gate_receipt_value: loaded.gate_receipt_value.as_ref(),
            is_gate_receipt_required: input.require_gate_receipt,
            request_value: loaded.request_value.as_ref(),
            should_send: input.send,
            from_peer: input.from_peer.as_deref(),
            sequence: input.sequence,
            expected_operation_ref: input.operation_id.as_deref(),
            expected_node: input.expected_node.as_deref(),
            expected_topic: input.expected_topic.as_deref(),
            expected_endpoint: input.expected_endpoint.as_deref(),
            expected_peer: input.expected_peer.as_deref(),
            expected_operations: &input.operations,
            expected_target_scope: input.target_scope.as_deref(),
            expected_resource_scope: input.resource_scope.as_deref(),
            as_of_sequence: input.as_of_sequence,
            as_of_epoch: input.as_of_epoch,
            peer_bootstrap_refs: &input.peer_bootstrap_refs,
            authority_refs: &input.authority_refs,
            policy_refs: &input.policy_refs,
            resource_refs: &input.resource_refs,
            evidence_refs: &input.evidence_refs,
            max_attempts: input.max_attempts,
            join_timeout_ms: input.join_timeout_ms,
        },
    ))
}

fn write_outputs(
    input: &super::super::command::live::Apply,
    applied: &molten::node_daemon::ControlLiveWorkflowBundleApply,
) -> molten::error::Result<()> {
    if let (Some(path), Some(value)) = (input.send_receipt_out.as_ref(), applied.send_receipt_value.as_ref()) {
        super::super::core::write_file(path, &molten::preserves_rail::to_text(value)?)?;
    }
    super::super::core::emit_named_receipt(
        input.receipt_out.as_ref(),
        "node control live workflow bundle apply receipt",
        &applied.receipt_value,
    )?;
    println!(
        "node live workflow bundle apply decision={} bundle={} gate={} import={} imported={} send={} diagnostics={}",
        applied.decision,
        applied.bundle_ref,
        applied.gate_receipt_ref.as_deref().unwrap_or("none"),
        applied.import_receipt_ref.as_deref().unwrap_or("none"),
        applied.imported_refs.len(),
        applied.send_receipt_ref.as_deref().unwrap_or("none"),
        applied.diagnostics.len()
    );
    Ok(())
}

fn print_next_step(
    applied: &molten::node_daemon::ControlLiveWorkflowBundleApply,
    has_request: bool,
    was_send_requested: bool,
) {
    if applied.decision == "pass" {
        if was_send_requested {
            println!("next-step=inspect-live-send-receipt command=\"molten node show <send-receipt>\"");
        } else if has_request {
            println!(
                "next-step=send-live-workflow-bundle command=\"molten node live-workflow-bundle-apply ... --send\""
            );
        } else {
            println!(
                "next-step=dry-run-or-send-request command=\"molten node live-workflow-bundle-apply ... --request <request> [--send]\""
            );
        }
        return;
    }
    let has_gate_problem = applied.diagnostics.iter().any(|diagnostic| {
        diagnostic.contains("gate receipt")
            || diagnostic.contains("requires a current gate receipt")
            || diagnostic.contains("recomputed verify")
    });
    if has_gate_problem {
        println!("next-step=rerun-gate command=\"molten node live-workflow-bundle-gate ... --receipt-out ...\"");
        return;
    }
    let has_address_problem = applied
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.contains("no endpoint addresses") || diagnostic.contains("unsupported address"));
    if has_address_problem {
        println!("next-step=refresh-bound-live-ticket command=\"molten node serve --live-iroh --live-ticket-out ...\"");
        return;
    }
    println!("next-step=inspect-apply-diagnostics command=\"molten node show <apply-receipt>\"");
}
