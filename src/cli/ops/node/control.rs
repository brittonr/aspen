#[path = "control/ingress.rs"]
pub(crate) mod ingress;

const CONTROL_INBOX_RELATIVE: &str = "control/inbox";

pub(crate) fn submit(input: super::command::control::Submit) -> molten::error::Result<()> {
    let super::command::control::Submit {
        state_root,
        request,
        receipt_out,
    } = input;
    let request_value = super::core::read_preserves_file(&request)?;
    let submitted = molten::node_daemon::submit_control_request(&molten::node_daemon::ControlSubmitInput {
        state_root: &state_root,
        request_value: &request_value,
    })?;
    super::core::emit_named_receipt(
        receipt_out.as_ref(),
        "node control queue receipt",
        &submitted.queue_receipt_value,
    )?;
    let inbox_path = state_root.join(CONTROL_INBOX_RELATIVE).join(&submitted.inbox_entry);
    println!(
        "node control submit request={} queue_receipt={} inbox={}",
        submitted.request_ref,
        submitted.queue_receipt_ref,
        inbox_path.display()
    );
    Ok(())
}

pub(crate) fn dispatch(input: super::command::control::Dispatch) -> molten::error::Result<()> {
    let super::command::control::Dispatch {
        state_root,
        request,
        receipt_out,
    } = input;
    let dispatched = molten::node_daemon::dispatch_control_request(&molten::node_daemon::ControlDispatchInput {
        state_root: &state_root,
        request_path: request.as_deref(),
    })?;
    super::core::emit_named_receipt(receipt_out.as_ref(), "node control receipt", &dispatched.control_receipt_value)?;
    println!(
        "node control dispatch operation={} request={} control_receipt={} subreceipts={}",
        dispatched.operation,
        dispatched.request_ref,
        dispatched.control_receipt_ref,
        dispatched.subreceipt_refs.len()
    );
    Ok(())
}

pub(crate) fn deny(input: super::command::control::Deny) -> molten::error::Result<()> {
    let super::command::control::Deny {
        request,
        startup,
        diagnostic,
        receipt_out,
    } = input;
    let request_value = super::core::read_preserves_file(&request)?;
    let request = molten::node_runtime::parse_control_request(&request_value)?;
    let receipt = molten::node_runtime::control_deny_receipt_value(&request, &startup, &diagnostic)?;
    super::core::emit_named_receipt(receipt_out.as_ref(), "node control receipt", &receipt)?;
    println!("node control deny receipt={}", molten::preserves_rail::canonical_hash(&receipt)?);
    Ok(())
}
