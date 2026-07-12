
fn live_peer_admission_diagnostics(
    state_root: &crate::node_state::NodeStateRoot,
    envelope: &ControlIngressEnvelope,
    admission: &ControlLivePeerAdmission,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(8);
    if admission.decision != "pass" {
        diagnostics.push(format!(
            "node control live peer admission {} decision {}",
            admission.admission_ref, admission.decision
        ));
    }
    if admission.peer_id != envelope.from_peer {
        diagnostics.push(format!(
            "node control live peer admission {} peer {} does not match {}",
            admission.admission_ref, admission.peer_id, envelope.from_peer
        ));
    }
    if admission.node_id != envelope.to_node {
        diagnostics.push(format!(
            "node control live peer admission {} node {} does not match {}",
            admission.admission_ref, admission.node_id, envelope.to_node
        ));
    }
    if admission.topic != envelope.topic {
        diagnostics.push(format!(
            "node control live peer admission {} topic {} does not match {}",
            admission.admission_ref, admission.topic, envelope.topic
        ));
    }
    match read_ledger_artifact(state_root, &admission.ticket_ref) {
        Ok(value) => match parse_control_live_ticket(&value) {
            Ok(ticket) => {
                if ticket.node_id != admission.node_id || ticket.topic != admission.topic {
                    diagnostics.push(format!(
                        "node control live peer admission {} ticket binding mismatch",
                        admission.admission_ref
                    ));
                }
            }
            Err(error) => diagnostics.push(format!(
                "node control live peer admission {} ticket is not a live ticket: {error}",
                admission.admission_ref
            )),
        },
        Err(error) => diagnostics.push(format!(
            "node control live peer admission {} ticket {} not found: {error}",
            admission.admission_ref, admission.ticket_ref
        )),
    }
    if admission.sequence > envelope.sequence {
        diagnostics.push(format!(
            "node control live peer admission {} is not valid until sequence {}",
            admission.admission_ref, admission.sequence
        ));
    }
    if let Some(expires_at) = admission.expires_at
        && expires_at < envelope.sequence
    {
        diagnostics.push(format!(
            "node control live peer admission {} expired at sequence {expires_at}",
            admission.admission_ref
        ));
    }
    Ok(diagnostics)
}

fn evaluate_live_authority_delegation(
    root: &crate::node_state::NodeStateRoot,
    envelope: &ControlIngressEnvelope,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(envelope.authority_refs.len().saturating_add(2));
    let mut admitted_grant_ref = None;
    let candidate_authority_refs = envelope
        .authority_refs
        .iter()
        .filter(|authority_ref| envelope.request.authority_refs.contains(*authority_ref))
        .collect::<Vec<_>>();
    if candidate_authority_refs.is_empty() {
        diagnostics.push("node control live authority refs are not bound to the request".to_string());
    }
    for authority_ref in candidate_authority_refs {
        match read_ledger_artifact(root, authority_ref) {
            Ok(value) => match parse_control_authority_grant(&value) {
                Ok(grant) => {
                    let grant_diagnostics = authority_grant_diagnostics(envelope, &grant);
                    if grant_diagnostics.is_empty() {
                        admitted_grant_ref = Some(grant.grant_ref);
                        break;
                    }
                    diagnostics.extend(grant_diagnostics);
                }
                Err(error) => {
                    if let Some(diagnostic) = transport_evidence_not_authority_diagnostic(
                        &value,
                        authority_ref,
                        "node control authority ref",
                        "authority",
                    ) {
                        diagnostics.push(diagnostic);
                    } else {
                        diagnostics.push(format!("node control authority ref {authority_ref} is not a grant: {error}"));
                    }
                }
            },
            Err(error) => diagnostics.push(format!("node control authority grant {authority_ref} not found: {error}")),
        }
    }
    if admitted_grant_ref.is_none() {
        diagnostics.push("node control live authority delegation missing admitted grant".to_string());
    }
    let decision = if admitted_grant_ref.is_some() { "pass" } else { "deny" };
    let receipt_value = receipt_value(&AuthorityReceiptValueInput {
        decision,
        envelope,
        grant_ref: admitted_grant_ref.as_deref(),
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    write_preserves(root, &control_authority_receipt_path(&envelope.envelope_ref)?, &receipt_value)?;
    import_artifact(root, &receipt_value)?;
    if decision == "deny" {
        diagnostics.push(format!("node control authority receipt {receipt_ref} denied"));
    }
    Ok(diagnostics)
}

fn authority_grant_diagnostics(envelope: &ControlIngressEnvelope, grant: &ControlAuthorityGrant) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(8);
    if grant.peer_id != envelope.from_peer {
        diagnostics.push(format!(
            "node control authority grant {} peer {} does not match {}",
            grant.grant_ref, grant.peer_id, envelope.from_peer
        ));
    }
    if grant.node_id != envelope.to_node {
        diagnostics.push(format!(
            "node control authority grant {} node {} does not match {}",
            grant.grant_ref, grant.node_id, envelope.to_node
        ));
    }
    if !grant
        .operations
        .iter()
        .any(|operation| operation == "*" || operation == &envelope.request.operation)
    {
        diagnostics.push(format!(
            "node control authority grant {} does not allow operation {}",
            grant.grant_ref, envelope.request.operation
        ));
    }
    if grant.epoch > envelope.sequence {
        diagnostics
            .push(format!("node control authority grant {} is not valid until epoch {}", grant.grant_ref, grant.epoch));
    }
    if let Some(expires_at) = grant.expires_at
        && expires_at < envelope.sequence
    {
        diagnostics.push(format!("node control authority grant {} expired at epoch {expires_at}", grant.grant_ref));
    }
    if !grant.revocation_refs.is_empty() {
        diagnostics.push(format!("node control authority grant {} has revocation refs", grant.grant_ref));
    }
    if !scope_matches_request(
        &grant.target_scope,
        envelope.request.target_ref.as_deref(),
        envelope.request.payload_ref.as_deref(),
    ) {
        diagnostics.push(format!(
            "node control authority grant {} target scope {} does not match request",
            grant.grant_ref, grant.target_scope
        ));
    }
    if !scope_matches_refs(&grant.resource_scope, &envelope.resource_refs, &envelope.request.resource_refs) {
        diagnostics.push(format!(
            "node control authority grant {} resource scope {} does not match request",
            grant.grant_ref, grant.resource_scope
        ));
    }
    diagnostics
}

fn scope_matches_request(scope: &str, target_ref: Option<&str>, payload_ref: Option<&str>) -> bool {
    scope == "*" || target_ref == Some(scope) || payload_ref == Some(scope)
}

fn scope_matches_refs(scope: &str, envelope_refs: &[String], request_refs: &[String]) -> bool {
    scope == "*"
        || envelope_refs.iter().any(|reference| reference == scope)
        || request_refs.iter().any(|reference| reference == scope)
}

fn transport_evidence_not_authority_diagnostic(
    value: &IoValue,
    reference: &str,
    reference_label: &str,
    authority_label: &str,
) -> Option<String> {
    let kind = crate::ledger::artifact_kind(value);
    is_transport_observation_kind(kind)
        .then(|| format!("{reference_label} {reference} is transport evidence, not {authority_label}"))
}

fn is_transport_observation_kind(kind: &str) -> bool {
    matches!(
        kind,
        "node-control-live-transport-receipt"
            | "node-control-live-listener-receipt"
            | "node-control-live-send-receipt"
            | "node-control-live-send-retry-receipt"
            | "node-control-live-send-duplicate-receipt"
    )
}

fn ingress_idempotency_evidence_refs(envelope: &ControlIngressEnvelope) -> Vec<String> {
    let mut refs = Vec::with_capacity(
        envelope.peer_bootstrap_refs.len()
            + envelope.authority_refs.len()
            + envelope.resource_refs.len()
            + envelope.evidence_refs.len(),
    );
    refs.extend(envelope.peer_bootstrap_refs.iter().cloned());
    refs.extend(envelope.authority_refs.iter().cloned());
    refs.extend(envelope.resource_refs.iter().cloned());
    refs.extend(envelope.evidence_refs.iter().cloned());
    refs.sort();
    refs.dedup();
    refs
}

fn prior_queue_receipt_ref(root: &crate::node_state::NodeStateRoot, request_ref: &str) -> Result<String> {
    let receipt = read_preserves(root, &queue_receipt_path(request_ref)?)?;
    crate::preserves_rail::canonical_hash(&receipt)
}

fn prior_dispatch_for_request(
    root: &crate::node_state::NodeStateRoot,
    request: &crate::node_runtime::ControlRequest,
) -> Result<Option<ControlDispatch>> {
    let receipt_path = control_outbox_receipt_path(&request.request_ref)?;
    if !root.try_exists(&receipt_path)? {
        return Ok(None);
    }
    let archived_path = control_outbox_request_path(&request.request_ref)?;
    if root.try_exists(&archived_path)? {
        let archived_value = read_preserves(root, &archived_path)?;
        let archived_ref = crate::preserves_rail::canonical_hash(&archived_value)?;
        if archived_ref != request.request_ref {
            return Err(MoltenError::invalid_harness(
                "node control duplicate request conflicts with archived request evidence",
            ));
        }
    }
    let control_receipt_value = read_preserves(root, &receipt_path)?;
    let control = crate::node_runtime::parse_control_receipt(&control_receipt_value)?;
    if control.request_ref != request.request_ref {
        return Err(MoltenError::invalid_harness("node control duplicate receipt conflicts with request ref"));
    }
    Ok(Some(ControlDispatch {
        operation: request.operation.clone(),
        request_ref: request.request_ref.clone(),
        control_receipt_ref: control.receipt_ref,
        control_receipt_value: control.value,
        subreceipt_refs: control.subreceipt_refs,
    }))
}

fn write_dispatch_queue_receipt(
    root: &crate::node_state::NodeStateRoot,
    request: &crate::node_runtime::ControlRequest,
    phase: &str,
) -> Result<String> {
    let location_ref = local_ref(
        "node-control-outbox-path",
        &control_outbox_receipt_path(&request.request_ref)?.display(),
    )?;
    let diagnostics = Vec::new();
    let queue_receipt = queue_receipt_value(&QueueReceiptValueInput {
        decision: "pass",
        phase,
        operation: &request.operation,
        request_ref: &request.request_ref,
        location_ref: &location_ref,
        diagnostics: &diagnostics,
    })?;
    let queue_receipt_ref = crate::preserves_rail::canonical_hash(&queue_receipt)?;
    write_preserves(root, &dispatch_receipt_path(&request.request_ref)?, &queue_receipt)?;
    import_artifact(root, &queue_receipt)?;
    Ok(queue_receipt_ref)
}

fn dispatch_status_request(
    root: &crate::node_state::NodeStateRoot,
    request: &crate::node_runtime::ControlRequest,
) -> Result<ControlDispatch> {
    let status = status_local_node_with_request(root, request)?;
    write_preserves(
        root,
        &control_outbox_receipt_path(&request.request_ref)?,
        &status.control_receipt_value,
    )?;
    Ok(ControlDispatch {
        operation: request.operation.clone(),
        request_ref: request.request_ref.clone(),
        control_receipt_ref: status.control_receipt_ref,
        control_receipt_value: status.control_receipt_value,
        subreceipt_refs: vec![status.health_ref],
    })
}

fn dispatch_shutdown_request(
    root: &crate::node_state::NodeStateRoot,
    request: &crate::node_runtime::ControlRequest,
) -> Result<ControlDispatch> {
    let stop = stop_local_node_with_request(root, request)?;
    write_preserves(
        root,
        &control_outbox_receipt_path(&request.request_ref)?,
        &stop.control_receipt_value,
    )?;
    Ok(ControlDispatch {
        operation: request.operation.clone(),
        request_ref: request.request_ref.clone(),
        control_receipt_ref: stop.control_receipt_ref,
        control_receipt_value: stop.control_receipt_value,
        subreceipt_refs: vec![stop.shutdown_ref],
    })
}

#[derive(Debug, Clone, Copy)]
struct ControlProvenanceInput<'a> {
    state_root: &'a crate::node_state::NodeStateRoot,
    request: &'a crate::node_runtime::ControlRequest,
    artifact_ref: &'a str,
    operation: &'a str,
    subreceipt_kind: &'a str,
}
