use super::*;

pub(super) fn metadata(phase: PublicationPhase) -> IoValue {
    let phase = match phase {
        PublicationPhase::Reserved => "reserved",
        PublicationPhase::Publishing => "publishing",
        PublicationPhase::Published => "published",
        PublicationPhase::Dispatching => "dispatching",
        PublicationPhase::Dispatched => "dispatched",
    };
    crate::preserves_rail::record("node-ingress-publication-v1", vec![
        crate::preserves_rail::string("molten.node.ingress-publication.v1"),
        crate::preserves_rail::record("phase", vec![crate::preserves_rail::string(phase)]),
    ])
}

pub(super) fn phase(intent: &crate::delivery_idempotency::PublicationIntent) -> Result<PublicationPhase> {
    let fields = intent
        .metadata
        .collect_simple_record("node-ingress-publication-v1", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected node ingress publication state"))?;
    require_schema(&fields[0], "molten.node.ingress-publication.v1", "node ingress publication state")?;
    match record_string(&fields[1], "phase")?.as_str() {
        "reserved" => Ok(PublicationPhase::Reserved),
        "publishing" => Ok(PublicationPhase::Publishing),
        "published" => Ok(PublicationPhase::Published),
        "dispatching" => Ok(PublicationPhase::Dispatching),
        "dispatched" => Ok(PublicationPhase::Dispatched),
        _ => Err(MoltenError::invalid_harness("unsupported node ingress publication phase")),
    }
}

pub(super) fn transition(
    store: &mut crate::delivery_idempotency::PublicationStore<'_>,
    intent: &mut Option<crate::delivery_idempotency::PublicationIntent>,
    next: PublicationPhase,
) -> Result<()> {
    if let Some(intent) = intent
        && phase(intent)? != next
    {
        let value = metadata(next);
        store.update(intent, &value)?;
        intent.metadata = value;
    }
    Ok(())
}

pub(super) fn decide(
    envelope: &ControlIngressEnvelope,
    store: &crate::delivery_idempotency::PublicationStore<'_>,
    intent: Option<&crate::delivery_idempotency::PublicationIntent>,
    evidence: &observations::Evidence,
    dedup: DedupObservation,
) -> Result<PublicationDecision> {
    let intent = intent
        .map(|intent| {
            Ok::<_, MoltenError>(PublicationIntentObservation {
                operation_ref: &intent.operation_ref,
                request_ref: &intent.payload_ref,
                phase: phase(intent)?,
            })
        })
        .transpose()?;
    Ok(reconcile(&ReconcileInput {
        operation_ref: &envelope.operation_ref,
        request_ref: &envelope.request.request_ref,
        operation: &envelope.request.operation,
        dedup,
        intent,
        inbox: evidence.inbox_observation(),
        queue: evidence.queue_observation(),
        dispatch: evidence.dispatch_observation(&envelope.request),
        quarantine: store.quarantine_status(),
    }))
}

pub(super) fn denied(decision: PublicationDecision, diagnostic: impl Into<String>) -> EnqueueOutcome {
    EnqueueOutcome {
        phase: match decision {
            PublicationDecision::Conflict => "publication-conflict",
            PublicationDecision::OutcomeUnknown => "outcome-unknown",
            _ => "duplicate-or-deny",
        },
        diagnostics: vec![diagnostic.into()],
        ..EnqueueOutcome::default()
    }
}
