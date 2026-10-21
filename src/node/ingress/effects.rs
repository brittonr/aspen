use super::*;

pub(super) struct Publication {
    pub(super) submitted: ControlSubmit,
    pub(super) has_enqueued: bool,
}

struct QueuePublication {
    path: crate::node_state::RelativePath,
    location_ref: String,
    prior: Option<IoValue>,
}

pub(super) fn write_new(
    root: &crate::node_state::DirectoryRoot,
    path: &crate::node_state::RelativePath,
    value: &IoValue,
) -> Result<()> {
    let text = crate::preserves_rail::to_text(value)?;
    root.write_new(path, text.as_bytes())
}

fn observe_inbox(
    root: &crate::node_state::DirectoryRoot,
    request: &crate::node_runtime::ControlRequest,
    path: &crate::node_state::RelativePath,
) -> Result<bool> {
    if let Some(value) = observations::read_optional(root, path)? {
        let prior = crate::node_runtime::parse_control_request(&value)?;
        if prior.request_ref != request.request_ref || prior.operation != request.operation {
            return Err(MoltenError::invalid_harness("existing inbox conflicts with exact control request"));
        }
        return Ok(true);
    }
    Ok(false)
}

fn observe_queue(
    root: &crate::node_state::DirectoryRoot,
    request: &crate::node_runtime::ControlRequest,
) -> Result<QueuePublication> {
    let path = queue_receipt_path(&request.request_ref)?;
    let location_ref = local_ref("node-control-inbox-path", &control_inbox_path(&request.request_ref)?.display())?;
    let prior = observations::read_optional(root, &path)?;
    if let Some(value) = &prior {
        let receipt = parse_control_queue_receipt(value)?;
        if receipt.decision != "pass"
            || receipt.request_ref != request.request_ref
            || receipt.operation != request.operation
            || receipt.location_ref != location_ref
        {
            return Err(MoltenError::invalid_harness("existing queue evidence conflicts with exact publication"));
        }
    }
    Ok(QueuePublication {
        path,
        location_ref,
        prior,
    })
}

pub(super) fn publish_queue(
    root: &crate::node_state::DirectoryRoot,
    request: &crate::node_runtime::ControlRequest,
    phase: &'static str,
) -> Result<ControlSubmit> {
    let publication = observe_queue(root, request)?;
    write_queue(root, request, phase, publication)
}

fn write_queue(
    root: &crate::node_state::DirectoryRoot,
    request: &crate::node_runtime::ControlRequest,
    phase: &'static str,
    publication: QueuePublication,
) -> Result<ControlSubmit> {
    let receipt_value = match publication.prior {
        Some(value) => value,
        None => {
            let value = queue_receipt_value(&QueueReceiptValueInput {
                decision: "pass",
                phase,
                operation: &request.operation,
                request_ref: &request.request_ref,
                location_ref: &publication.location_ref,
                diagnostics: &[],
            })?;
            write_new(root, &publication.path, &value)?;
            value
        }
    };
    let queue_receipt_ref = import_artifact(root, &receipt_value)?;
    Ok(ControlSubmit {
        request_ref: request.request_ref.clone(),
        inbox_entry: control_inbox_entry_name(&request.request_ref),
        queue_receipt_ref,
        queue_receipt_value: receipt_value,
    })
}

pub(super) fn publish_request(
    root: &crate::node_state::DirectoryRoot,
    request: &crate::node_runtime::ControlRequest,
    store: &mut crate::delivery_idempotency::PublicationStore<'_>,
) -> Result<Publication> {
    let inbox_path = control_inbox_path(&request.request_ref)?;
    let has_inbox = observe_inbox(root, request, &inbox_path)?;
    let queue = observe_queue(root, request)?;
    let result: Result<Publication> = (|| {
        import_artifact(root, &request.value)?;
        if !has_inbox {
            write_new(root, &inbox_path, &request.value)?;
        }
        let submitted = write_queue(root, request, "enqueue", queue)?;
        Ok(Publication {
            submitted,
            has_enqueued: !has_inbox,
        })
    })();
    result.or_else(|error| {
        store.quarantine()?;
        Err(error)
    })
}

pub(super) fn observe_dispatch(
    root: &crate::node_state::DirectoryRoot,
    request: &crate::node_runtime::ControlRequest,
    control: &crate::node_runtime::ControlReceipt,
) -> Result<String> {
    if control.request_ref != request.request_ref {
        return Err(MoltenError::invalid_harness("dispatch observation conflicts with exact request"));
    }
    let value = queue_receipt_value(&QueueReceiptValueInput {
        decision: "pass",
        phase: "dispatch-observed",
        operation: &request.operation,
        request_ref: &request.request_ref,
        location_ref: &control.receipt_ref,
        diagnostics: &[],
    })?;
    // A new observation references the immutable result. It is not an original enqueue receipt.
    import_artifact(root, &value)
}
