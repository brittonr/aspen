
pub(crate) fn deliver(input: super::super::command::control::IngressDeliver) -> molten::error::Result<()> {
    let super::super::command::control::IngressDeliver {
        state_root,
        topic,
        envelope_ref,
        receipt_out,
    } = input;
    let delivered = molten::node_daemon::deliver_control_ingress(&molten::node_daemon::ControlIngressDeliverInput {
        state_root: &state_root,
        topic: &topic,
        envelope_ref: &envelope_ref,
    })?;
    super::super::core::emit_named_receipt(
        receipt_out.as_ref(),
        "node control ingress receipt",
        &delivered.ingress_receipt_value,
    )?;
    println!(
        "node control ingress deliver envelope={} request={} receipt={} enqueued={}",
        delivered.envelope_ref,
        delivered.request_ref,
        delivered.ingress_receipt_ref,
        if delivered.has_enqueued { "yes" } else { "no" }
    );
    Ok(())
}
