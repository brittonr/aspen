use super::*;

struct ExecutionPorts<'a> {
    content: &'a mut dyn ContentPort,
    transport: &'a mut dyn TransportPort,
    durable: &'a mut dyn DurablePort,
    retention: &'a mut dyn RetentionPort,
    observations: &'a mut dyn ObservationPort,
    receipts: &'a mut dyn ReceiptPort,
}

pub(super) struct ExecutionResult<'a> {
    pub(super) operations: Vec<PriorOperation>,
    pub(super) durable: &'a mut dyn DurablePort,
    pub(super) observations: &'a mut dyn ObservationPort,
    pub(super) receipts: &'a mut dyn ReceiptPort,
}

pub(super) fn execute_actions<'a>(
    manifest: &Manifest,
    plan: &Plan,
    history: &[PriorOperation],
    evidence_refs: &mut Vec<String>,
    ports: ReconcilePorts<'a>,
) -> Result<ExecutionResult<'a>> {
    let execution_ports = ExecutionPorts {
        content: ports.content,
        transport: ports.transport,
        durable: ports.durable,
        retention: ports.retention,
        observations: ports.observations,
        receipts: ports.receipts,
    };
    let mut operations = history.to_vec();
    for action in &plan.actions {
        if action.kind == ActionKind::Reuse {
            if let Some(result_ref) = history
                .iter()
                .find(|prior| prior.operation_id == action.operation_id)
                .and_then(|prior| prior.result_ref.clone())
            {
                evidence_refs.push(result_ref);
            }
            continue;
        }
        if let Some(operation) = execute_action(
            manifest,
            action,
            history,
            evidence_refs,
            &mut *execution_ports.content,
            &mut *execution_ports.transport,
            &mut *execution_ports.retention,
        )? {
            let canonical = canonical_operation(&operation)?;
            let durable_ref = execution_ports.durable.store_operation(&operation)?;
            validate_ref(&durable_ref, "replication durable operation")?;
            execution_ports.observations.publish_operation(&canonical)?;
            evidence_refs.push(durable_ref);
            evidence_refs.push(canonical.record_ref);
            operations.push(operation);
        }
    }
    Ok(ExecutionResult {
        operations,
        durable: execution_ports.durable,
        observations: execution_ports.observations,
        receipts: execution_ports.receipts,
    })
}

fn execute_action(
    manifest: &Manifest,
    action: &Action,
    history: &[PriorOperation],
    evidence_refs: &mut Vec<String>,
    content: &mut dyn ContentPort,
    transport: &mut dyn TransportPort,
    retention: &mut dyn RetentionPort,
) -> Result<Option<PriorOperation>> {
    match action.kind {
        ActionKind::Defer => Ok(None),
        ActionKind::Reuse => Ok(history.iter().find(|prior| prior.operation_id == action.operation_id).cloned()),
        ActionKind::Cleanup => execute_cleanup(manifest, action, content, retention, evidence_refs).map(Some),
        ActionKind::Transfer | ActionKind::Repair | ActionKind::Handoff => {
            execute_transfer(manifest, action, content, transport, retention, evidence_refs).map(Some)
        }
    }
}

fn execute_transfer(
    manifest: &Manifest,
    action: &Action,
    content: &mut dyn ContentPort,
    transport: &mut dyn TransportPort,
    retention: &mut dyn RetentionPort,
    evidence_refs: &mut Vec<String>,
) -> Result<PriorOperation> {
    let pin = retention.acquire_pin(action)?;
    validate_pin(manifest, action, &pin)?;
    evidence_refs.push(pin.pin_ref);
    let outcome = transport.fetch(action)?;
    let (outcome, result_ref) = match outcome {
        TransferOutcome::Received(envelope) => {
            validate_envelope(manifest, action, &envelope)?;
            evidence_refs.push(envelope.transfer_ref.clone());
            evidence_refs.push(envelope.transport_verification_ref.clone());
            let verification = content.verify(action, &envelope)?;
            validate_verification(manifest, action, &verification)?;
            evidence_refs.push(verification.verification_ref.clone());
            (OperationOutcome::Verified, Some(verification.verification_ref))
        }
        TransferOutcome::Cancelled(reference) => (OperationOutcome::Cancelled, Some(reference)),
        TransferOutcome::Uncertain(reference) => (OperationOutcome::Uncertain, Some(reference)),
        TransferOutcome::Unavailable(reference) | TransferOutcome::TimedOut(reference) => {
            (OperationOutcome::Failed, Some(reference))
        }
    };
    operation_from_action(manifest, action, outcome, result_ref)
}

fn execute_cleanup(
    manifest: &Manifest,
    action: &Action,
    content: &mut dyn ContentPort,
    retention: &mut dyn RetentionPort,
    evidence_refs: &mut Vec<String>,
) -> Result<PriorOperation> {
    let admission = retention.authorize_cleanup(action)?;
    validate_cleanup(manifest, action, &admission)?;
    let cleanup_ref = content.cleanup(action, &admission)?;
    validate_ref(&cleanup_ref, "replication cleanup observation")?;
    evidence_refs.push(admission.cleanup_ref);
    evidence_refs.push(cleanup_ref.clone());
    operation_from_action(manifest, action, OperationOutcome::Verified, Some(cleanup_ref))
}

fn operation_from_action(
    manifest: &Manifest,
    action: &Action,
    outcome: OperationOutcome,
    result_ref: Option<String>,
) -> Result<PriorOperation> {
    Ok(PriorOperation {
        operation_id: action.operation_id.clone(),
        content_ref: action.content_ref.clone(),
        source_peer: action.source_peer.clone(),
        target_peer: action.target_peer.clone(),
        generation: manifest.generation,
        membership_epoch: manifest.membership_epoch,
        placement_epoch: manifest.placement_epoch,
        attempt: action.attempt,
        outcome,
        result_ref,
    })
}

pub(super) fn execution_receipt(
    instance: &ServiceInstance,
    plan: &Plan,
    status: &Status,
    canonical_status: &CanonicalReplicationRecord,
    operations: Vec<PriorOperation>,
    evidence_refs: Vec<String>,
) -> ExecutionReceipt {
    let decision = if plan.decision == Decision::Denied {
        ReceiptDecision::Denied
    } else if status.under_replicated.is_empty() && status.active_operations.is_empty() && status.failures.is_empty() {
        ReceiptDecision::Complete
    } else {
        ReceiptDecision::Partial
    };
    ExecutionReceipt {
        decision,
        service_id: instance.manifest.service_id.clone(),
        generation: instance.manifest.generation,
        plan_ref: plan.plan_ref.clone(),
        status_ref: canonical_status.record_ref.clone(),
        operations,
        evidence_refs,
        issues: plan.issues.clone(),
        non_claims: NON_CLAIMS.iter().map(ToString::to_string).collect(),
    }
}
