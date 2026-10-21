use super::*;

// r[impl molten.audit_f03.no_blind_replay]
// r[impl molten.audit_f03.uncertainty]
pub(in crate::daemon_core) fn run(
    root: &crate::node_state::DirectoryRoot,
    request: &crate::node_runtime::ControlRequest,
    store: &mut crate::delivery_idempotency::PublicationStore<'_>,
    action: impl FnOnce() -> Result<ObservedControlDispatch>,
) -> Result<ObservedControlDispatch> {
    store.require_healthy()?;
    let mut intents = prepare(root, request, store)?;
    let result = (|| {
        let observed = action()?;
        complete_intents(root, request, store, &mut intents)?;
        Ok(observed)
    })();
    match result {
        Ok(observed) => Ok(observed),
        Err(error) if intents.is_empty() => Err(error),
        Err(error) => match store.quarantine() {
            Ok(()) => Err(error),
            Err(quarantine_error) => Err(MoltenError::invalid_harness(format!(
                "ingress dispatch outcome unknown: {error}; quarantine persistence failed: {quarantine_error}"
            ))),
        },
    }
}

fn prepare(
    root: &crate::node_state::DirectoryRoot,
    request: &crate::node_runtime::ControlRequest,
    store: &mut crate::delivery_idempotency::PublicationStore<'_>,
) -> Result<Vec<crate::delivery_idempotency::PublicationIntent>> {
    let mut intents = store.for_payload(&request.request_ref)?;
    if intents.is_empty() {
        return Ok(intents);
    }
    let evidence = observations::Evidence::read(root, request)?;
    let mut needs_queue_observation = false;
    for intent in &intents {
        match classify(request, store, intent, &evidence)? {
            PublicationDecision::ReusePublication => {}
            PublicationDecision::RepairQueueReceipt => needs_queue_observation = true,
            _ => return Err(MoltenError::invalid_harness("ingress publication is unresolved; dispatch is blocked")),
        }
    }
    if needs_queue_observation {
        effects::publish_queue(root, request, "publication-observed")?;
    }
    for intent in &mut intents {
        transition(store, intent, PublicationPhase::Dispatching)?;
    }
    Ok(intents)
}

pub(in crate::daemon_core) fn complete(
    root: &crate::node_state::DirectoryRoot,
    request: &crate::node_runtime::ControlRequest,
    store: &mut crate::delivery_idempotency::PublicationStore<'_>,
) -> Result<()> {
    store.require_healthy()?;
    let mut intents = store.for_payload(&request.request_ref)?;
    complete_intents(root, request, store, &mut intents)
}

fn complete_intents(
    root: &crate::node_state::DirectoryRoot,
    request: &crate::node_runtime::ControlRequest,
    store: &mut crate::delivery_idempotency::PublicationStore<'_>,
    intents: &mut [crate::delivery_idempotency::PublicationIntent],
) -> Result<()> {
    if intents.is_empty() {
        return Ok(());
    }
    let evidence = observations::Evidence::read(root, request)?;
    for intent in intents.iter() {
        if classify(request, store, intent, &evidence)? != PublicationDecision::CompletedDispatch {
            return Err(MoltenError::invalid_harness(
                "completed ingress dispatch lacks exact durable result observations",
            ));
        }
    }
    for intent in intents {
        transition(store, intent, PublicationPhase::Dispatched)?;
    }
    Ok(())
}

fn classify(
    request: &crate::node_runtime::ControlRequest,
    store: &crate::delivery_idempotency::PublicationStore<'_>,
    intent: &crate::delivery_idempotency::PublicationIntent,
    evidence: &observations::Evidence,
) -> Result<PublicationDecision> {
    Ok(reconcile(&ReconcileInput {
        operation_ref: &intent.operation_ref,
        request_ref: &request.request_ref,
        operation: &request.operation,
        dedup: DedupObservation::Duplicate,
        intent: Some(PublicationIntentObservation {
            operation_ref: &intent.operation_ref,
            request_ref: &intent.payload_ref,
            phase: state::phase(intent)?,
        }),
        inbox: evidence.inbox_observation(),
        queue: evidence.queue_observation(),
        dispatch: evidence.dispatch_observation(request),
        quarantine: store.quarantine_status(),
    }))
}

fn transition(
    store: &mut crate::delivery_idempotency::PublicationStore<'_>,
    intent: &mut crate::delivery_idempotency::PublicationIntent,
    next: PublicationPhase,
) -> Result<()> {
    if state::phase(intent)? != next {
        let value = state::metadata(next);
        store.update(intent, &value)?;
        intent.metadata = value;
    }
    Ok(())
}
