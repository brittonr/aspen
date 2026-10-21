use super::*;

pub(super) struct Evidence {
    pub(super) inbox: Option<crate::node_runtime::ControlRequest>,
    pub(super) queue: Option<ControlQueueReceipt>,
    pub(super) dispatch: Option<crate::node_runtime::ControlReceipt>,
    pub(super) has_dispatch_gap: bool,
    pub(super) has_conflict: bool,
}

impl Evidence {
    pub(super) fn read(
        root: &crate::node_state::DirectoryRoot,
        request: &crate::node_runtime::ControlRequest,
    ) -> Result<Self> {
        let inbox = read_optional(root, &control_inbox_path(&request.request_ref)?)?
            .map(|value| crate::node_runtime::parse_control_request(&value))
            .transpose()?;
        let queue = read_optional(root, &queue_receipt_path(&request.request_ref)?)?
            .map(|value| parse_control_queue_receipt(&value))
            .transpose()?;
        let dispatch = read_optional(root, &control_outbox_receipt_path(&request.request_ref)?)?
            .map(|value| crate::node_runtime::parse_control_receipt(&value))
            .transpose()?;
        let archived = read_optional(root, &control_outbox_request_path(&request.request_ref)?)?
            .map(|value| crate::node_runtime::parse_control_request(&value))
            .transpose()?;
        let dispatch_queue = read_optional(root, &dispatch_receipt_path(&request.request_ref)?)?
            .map(|value| parse_control_queue_receipt(&value))
            .transpose()?;
        let operation_value = read_optional(root, &control_operation_receipt_path(&request.request_ref)?)?;
        let has_conflict = archived
            .as_ref()
            .is_some_and(|value| value.request_ref != request.request_ref || value.operation != request.operation)
            || dispatch_queue.as_ref().is_some_and(|value| {
                value.decision != "pass"
                    || value.request_ref != request.request_ref
                    || value.operation != request.operation
            });
        let has_dispatch_gap =
            dispatch.is_none() && (archived.is_some() || dispatch_queue.is_some() || operation_value.is_some());
        let evidence = Self {
            inbox,
            queue,
            dispatch,
            has_dispatch_gap,
            has_conflict,
        };
        evidence.validate_queue_location(root, request)?;
        Ok(evidence)
    }

    pub(super) fn inbox_observation(&self) -> PublicationObservation<'_> {
        self.inbox.as_ref().map_or(PublicationObservation::Absent, |request| PublicationObservation::Bound {
            operation: &request.operation,
            request_ref: &request.request_ref,
        })
    }

    pub(super) fn queue_observation(&self) -> PublicationObservation<'_> {
        match &self.queue {
            Some(queue) if queue.decision == "pass" => PublicationObservation::Bound {
                operation: &queue.operation,
                request_ref: &queue.request_ref,
            },
            Some(_) => PublicationObservation::Conflict,
            None => PublicationObservation::Absent,
        }
    }

    pub(super) fn dispatch_observation<'a>(
        &'a self,
        request: &'a crate::node_runtime::ControlRequest,
    ) -> PublicationObservation<'a> {
        if self.has_conflict {
            return PublicationObservation::Conflict;
        }
        if self.has_dispatch_gap {
            return PublicationObservation::Unavailable;
        }
        self.dispatch
            .as_ref()
            .map_or(PublicationObservation::Absent, |control| PublicationObservation::Bound {
                // The canonical request reference binds the operation; control receipts do not repeat it.
                operation: &request.operation,
                request_ref: &control.request_ref,
            })
    }

    fn validate_queue_location(
        &self,
        _root: &crate::node_state::DirectoryRoot,
        request: &crate::node_runtime::ControlRequest,
    ) -> Result<()> {
        let Some(queue) = &self.queue else {
            return Ok(());
        };
        let expected = match queue.phase.as_str() {
            "enqueue" | "publication-observed" => {
                local_ref("node-control-inbox-path", &control_inbox_path(&request.request_ref)?.display())?
            }
            "dispatch-observed" => {
                local_ref("node-control-outbox-path", &control_outbox_receipt_path(&request.request_ref)?.display())?
            }
            _ => return Err(MoltenError::invalid_harness("unsupported publication queue observation phase")),
        };
        if queue.location_ref != expected {
            return Err(MoltenError::invalid_harness("queue observation location conflicts with request"));
        }
        Ok(())
    }
}

pub(super) fn read_optional(
    root: &crate::node_state::DirectoryRoot,
    path: &crate::node_state::RelativePath,
) -> Result<Option<IoValue>> {
    match root.entry_kind(path)? {
        None => Ok(None),
        Some(crate::node_state::EntryKind::RegularFile) => read_preserves(root, path).map(Some),
        Some(_) => Err(MoltenError::invalid_harness("publication observation is not a regular file")),
    }
}
