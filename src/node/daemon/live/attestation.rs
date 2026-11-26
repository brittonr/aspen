//! Live workflow flow receipts and the receipt refs they bind.

struct FlowChecks<'a> {
    ticket: &'a super::entitlements::ControlLiveTicket,
    admission: &'a super::entitlements::ControlLivePeerAdmission,
    authority: &'a super::entitlements::ControlAuthorityGrant,
    send: &'a super::transits::ControlLiveSendReceipt,
    service_receipt_ref: &'a str,
}

struct FlowRefs {
    receive_receipt_refs: Vec<String>,
    listener_receipt_ref: Option<String>,
}

impl FlowChecks<'_> {
    fn note_bindings(&self, diagnostics: &mut impl crate::bounded::VecSink<String>) {
        if self.admission.ticket_ref != self.ticket.ticket_ref {
            diagnostics.push_item("node control live workflow admission does not bind receiver ticket".to_string());
        }
        if self.admission.decision != "pass" {
            diagnostics.push_item(format!("node control live workflow admission decision {}", self.admission.decision));
        }
        if self.authority.peer_id != self.admission.peer_id {
            diagnostics
                .push_item("node control live workflow authority grant peer does not match admission".to_string());
        }
        if self.authority.node_id != self.ticket.node_id {
            diagnostics.push_item("node control live workflow authority grant node does not match ticket".to_string());
        }
        if self.send.receiver_ticket_ref != self.ticket.ticket_ref {
            diagnostics.push_item("node control live workflow send receipt does not bind receiver ticket".to_string());
        }
        if self.send.from_peer != self.admission.peer_id {
            diagnostics.push_item("node control live workflow send peer does not match admission".to_string());
        }
        if self.send.to_node != self.ticket.node_id || self.send.topic != self.ticket.topic {
            diagnostics.push_item("node control live workflow send destination does not match ticket".to_string());
        }
        if self.send.decision != "pass" {
            diagnostics.push_item(format!("node control live workflow send decision {}", self.send.decision));
        }
    }

    fn collect_refs(
        &self,
        input: &super::bundling::ControlLiveWorkflowInput<'_>,
        diagnostics: &mut impl crate::bounded::VecSink<String>,
    ) -> crate::error::Result<FlowRefs> {
        let mut receive_receipt_refs = Vec::with_capacity(input.receive_receipt_values.len());
        for receive_value in input.receive_receipt_values {
            let (receipt_ref, operation, envelope_ref) = live_transport_receipt_ref(receive_value)?;
            if operation != "receive" {
                diagnostics.push_item(format!(
                    "node control live workflow transport receipt operation {operation} is not receive"
                ));
            }
            if envelope_ref != self.send.envelope_ref {
                diagnostics
                    .push_item("node control live workflow receive envelope does not match send envelope".to_string());
            }
            receive_receipt_refs.push(receipt_ref);
        }
        if receive_receipt_refs.is_empty() {
            diagnostics.push_item("node control live workflow missing receive receipt".to_string());
        }
        let listener_receipt_ref = if let Some(listener_value) = input.listener_receipt_value {
            let (listener_ref, listener_transport_refs, listener_service_ref) =
                live_listener_receipt_refs(listener_value)?;
            for receive_ref in &receive_receipt_refs {
                if !listener_transport_refs.iter().any(|reference| reference == receive_ref) {
                    diagnostics
                        .push_item("node control live workflow listener does not bind receive receipt".to_string());
                }
            }
            if listener_service_ref != self.service_receipt_ref {
                diagnostics.push_item(
                    "node control live workflow listener service run does not match service receipt".to_string(),
                );
            }
            Some(listener_ref)
        } else {
            None
        };
        Ok(FlowRefs {
            receive_receipt_refs,
            listener_receipt_ref,
        })
    }
}

fn import_flow_values(
    state_root: &crate::node_state::DirectoryRoot,
    input: &super::bundling::ControlLiveWorkflowInput<'_>,
    receipt_ref: &str,
    receipt_value: &preserves::IOValue,
) -> crate::error::Result<()> {
    super::storage::import_artifact(state_root, input.receiver_ticket_value)?;
    super::storage::import_artifact(state_root, input.peer_admission_value)?;
    super::storage::import_artifact(state_root, input.authority_grant_value)?;
    super::storage::import_artifact(state_root, input.send_receipt_value)?;
    for receive_value in input.receive_receipt_values {
        super::storage::import_artifact(state_root, receive_value)?;
    }
    if let Some(listener_value) = input.listener_receipt_value {
        super::storage::import_artifact(state_root, listener_value)?;
    }
    super::storage::import_artifact(state_root, input.service_receipt_value)?;
    super::storage::write_preserves(
        state_root,
        &super::paths::control_live_workflow_receipt_path(receipt_ref)?,
        receipt_value,
    )?;
    super::storage::import_artifact(state_root, receipt_value)?;
    Ok(())
}

pub fn control_live_workflow_receipt(
    input: &super::bundling::ControlLiveWorkflowInput<'_>,
) -> crate::error::Result<super::transits::ControlLiveWorkflowReceipt> {
    let state_root = input.state_root.map(crate::node_state::DirectoryRoot::open).transpose()?;
    if let Some(path) = input.state_root {
        super::storage::validate_state_root(path)?;
    }
    if let Some(state_root) = state_root.as_ref() {
        super::storage::ensure_state_layout(state_root)?;
    }
    let ticket = super::issuance::parse_control_live_ticket(input.receiver_ticket_value)?;
    let admission = super::peering::parse_control_live_peer_admission(input.peer_admission_value)?;
    let authority = super::issuance::parse_control_authority_grant(input.authority_grant_value)?;
    let send = super::receipts::parse_control_live_send_receipt(input.send_receipt_value)?;
    let service_receipt_ref = service_run_receipt_ref(input.service_receipt_value)?;
    let checks = FlowChecks {
        ticket: &ticket,
        admission: &admission,
        authority: &authority,
        send: &send,
        service_receipt_ref: &service_receipt_ref,
    };
    let mut diagnostics = Vec::with_capacity(input.receive_receipt_values.len().saturating_add(8));
    checks.note_bindings(&mut diagnostics);
    let refs = checks.collect_refs(input, &mut diagnostics)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = super::stamps::live_workflow_receipt_value(&super::stamps::LiveWorkflowReceiptValueInput {
        decision,
        ticket: &ticket,
        admission: &admission,
        authority: &authority,
        send: &send,
        receive_receipt_refs: &refs.receive_receipt_refs,
        listener_receipt_ref: refs.listener_receipt_ref.as_deref(),
        service_receipt_ref: &service_receipt_ref,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    if let Some(state_root) = state_root.as_ref() {
        import_flow_values(state_root, input, &receipt_ref, &receipt_value)?;
    }
    Ok(super::transits::ControlLiveWorkflowReceipt {
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
        diagnostics,
    })
}

fn service_run_receipt_ref(value: &preserves::IOValue) -> crate::error::Result<String> {
    if let Some(fields) = value.collect_simple_record("node-control-service-run-receipt-v1", Some(17)) {
        super::guards::require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_SERVICE_RUN_RECEIPT_SCHEMA,
            "node control service run receipt",
        )?;
        return crate::preserves_rail::canonical_hash(value);
    }
    if let Some(fields) = value.collect_simple_record("node-control-service-run-receipt-v1", Some(15)) {
        super::guards::require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_SERVICE_RUN_RECEIPT_SCHEMA,
            "node control service run receipt",
        )?;
        return crate::preserves_rail::canonical_hash(value);
    }
    Err(crate::error::Failure::invalid_harness("expected <node-control-service-run-receipt-v1 ...>"))
}

fn live_transport_receipt_ref(value: &preserves::IOValue) -> crate::error::Result<(String, String, String)> {
    let fields = value
        .collect_simple_record("node-control-live-transport-receipt-v1", Some(13))
        .or_else(|| value.collect_simple_record("node-control-live-transport-receipt-v1", Some(11)))
        .ok_or_else(|| {
            crate::error::Failure::invalid_harness("expected <node-control-live-transport-receipt-v1 ...>")
        })?;
    super::guards::require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_TRANSPORT_RECEIPT_SCHEMA,
        "node control live transport receipt",
    )?;
    Ok((
        crate::preserves_rail::canonical_hash(value)?,
        super::fields::record_string(&fields[1], "operation")?,
        super::fields::record_ref_string(&fields[7], "envelope")?,
    ))
}

fn live_listener_receipt_refs(value: &preserves::IOValue) -> crate::error::Result<(String, Vec<String>, String)> {
    let fields = value
        .collect_simple_record("node-control-live-listener-receipt-v1", Some(16))
        .or_else(|| value.collect_simple_record("node-control-live-listener-receipt-v1", Some(14)))
        .ok_or_else(|| {
            crate::error::Failure::invalid_harness("expected <node-control-live-listener-receipt-v1 ...>")
        })?;
    super::guards::require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_LISTENER_RECEIPT_SCHEMA,
        "node control live listener receipt",
    )?;
    Ok((
        crate::preserves_rail::canonical_hash(value)?,
        super::fields::record_ref_strings(&fields[9], "transport-receipts")?,
        super::fields::record_ref_string(&fields[11], "service-run")?,
    ))
}
