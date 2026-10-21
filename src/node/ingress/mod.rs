use super::*;
use crate::core_api::node_ingress::*;

pub(super) mod dispatch;
mod effects;
mod observations;
mod state;

// r[impl molten.audit_f03.recover_publication]
// r[impl molten.audit_f03.uncertainty]
pub(super) fn enqueue(
    root: &crate::node_state::DirectoryRoot,
    envelope: &ControlIngressEnvelope,
) -> Result<EnqueueOutcome> {
    let delivery_root = root.delivery_store()?;
    let mut store = match crate::delivery_idempotency::PublicationStore::open(&delivery_root) {
        Ok(store) => store,
        Err(error) => return Ok(state::denied(PublicationDecision::OutcomeUnknown, error.to_string())),
    };
    if store.quarantine_status() != transactional_reconciliation_core::QuarantineStatus::Clear {
        return Ok(state::denied(PublicationDecision::OutcomeUnknown, "publication storage is quarantined"));
    }
    match enqueue_locked(root, envelope, &mut store) {
        Ok(outcome) => Ok(outcome),
        Err(error) => Ok(state::denied(PublicationDecision::OutcomeUnknown, error.to_string())),
    }
}

fn enqueue_locked(
    root: &crate::node_state::DirectoryRoot,
    envelope: &ControlIngressEnvelope,
    store: &mut crate::delivery_idempotency::PublicationStore<'_>,
) -> Result<EnqueueOutcome> {
    let evidence = observations::Evidence::read(root, &envelope.request)?;
    let intent = store.read(&envelope.operation_ref, &envelope.request.request_ref)?;
    let preflight =
        state::decide(envelope, store, intent.as_ref(), &evidence, DedupObservation::Unobserved).or_else(|error| {
            store.quarantine()?;
            Err(error)
        })?;
    if preflight != PublicationDecision::CheckDedup {
        return Ok(state::denied(preflight, "publication preflight cannot admit dedup or queue effects"));
    }
    let delivery = reserve(store, envelope)?;
    match complete_enqueue(root, envelope, store, delivery, &evidence) {
        Ok(outcome) => Ok(outcome),
        Err(error) => {
            let mut outcome = state::denied(PublicationDecision::OutcomeUnknown, error.to_string());
            if store.quarantine_status() == transactional_reconciliation_core::QuarantineStatus::Clear
                && let Err(quarantine_error) = store.quarantine()
            {
                outcome.diagnostics.push(format!("quarantine persistence failed: {quarantine_error}"));
            }
            Ok(outcome)
        }
    }
}

fn complete_enqueue(
    root: &crate::node_state::DirectoryRoot,
    envelope: &ControlIngressEnvelope,
    store: &mut crate::delivery_idempotency::PublicationStore<'_>,
    delivery: crate::delivery_idempotency::Decision,
    evidence: &observations::Evidence,
) -> Result<EnqueueOutcome> {
    import_artifact(root, &delivery.receipt.value)?;
    let mut intent = store.read(&envelope.operation_ref, &envelope.request.request_ref)?;
    let dedup = match delivery.receipt.decision.as_str() {
        "first" => DedupObservation::First,
        "duplicate" => DedupObservation::Duplicate,
        _ => DedupObservation::Rejected,
    };
    let decision = state::decide(envelope, store, intent.as_ref(), evidence, dedup)?;
    let mut outcome = execute(
        ExecuteInput {
            root,
            envelope,
            evidence,
            decision,
        },
        store,
        &mut intent,
    )?;
    if decision == PublicationDecision::Rejected {
        outcome.diagnostics.extend(delivery.receipt.diagnostics);
        outcome
            .diagnostics
            .push(format!("node control ingress idempotency decision {}", delivery.receipt.decision));
    }
    outcome.idempotency_receipt_ref = Some(delivery.receipt.receipt_ref);
    Ok(outcome)
}

pub(super) fn reserve(
    store: &mut crate::delivery_idempotency::PublicationStore<'_>,
    envelope: &ControlIngressEnvelope,
) -> Result<crate::delivery_idempotency::Decision> {
    let evidence = ingress_idempotency_evidence_refs(envelope);
    let scope_ref = crate::delivery_idempotency::remote_topic_scope_ref(&envelope.topic, &envelope.to_node)?;
    store.reserve(
        crate::delivery_idempotency::CheckRequest {
            scope_profile: crate::delivery_idempotency::SCOPE_REMOTE_TOPIC,
            scope_ref: &scope_ref,
            producer: &envelope.from_peer,
            consumer: &envelope.to_node,
            sequence: envelope.sequence,
            intent: "node-control-ingress",
            payload_ref: &envelope.request.request_ref,
            policy_refs: &envelope.policy_refs,
            evidence_refs: &evidence,
            semantic_result_ref: Some(&envelope.request.request_ref),
            gap_policy: crate::delivery_idempotency::GapPolicy::Deny,
        },
        &state::metadata(PublicationPhase::Reserved),
    )
}

struct ExecuteInput<'a> {
    root: &'a crate::node_state::DirectoryRoot,
    envelope: &'a ControlIngressEnvelope,
    evidence: &'a observations::Evidence,
    decision: PublicationDecision,
}

fn execute(
    input: ExecuteInput<'_>,
    store: &mut crate::delivery_idempotency::PublicationStore<'_>,
    intent: &mut Option<crate::delivery_idempotency::PublicationIntent>,
) -> Result<EnqueueOutcome> {
    let ExecuteInput {
        root,
        envelope,
        evidence,
        decision,
    } = input;
    let mut outcome = EnqueueOutcome::default();
    match decision {
        PublicationDecision::PublishFresh | PublicationDecision::RecoverPublication => {
            state::transition(store, intent, PublicationPhase::Publishing)?;
            let publication = effects::publish_request(root, &envelope.request, store)?;
            state::transition(store, intent, PublicationPhase::Published)?;
            outcome.queue_receipt_ref = Some(publication.submitted.queue_receipt_ref);
            outcome.has_enqueued = publication.has_enqueued;
            outcome.phase = if decision == PublicationDecision::PublishFresh {
                "deliver"
            } else {
                "recovered-publication"
            };
        }
        PublicationDecision::RepairQueueReceipt => {
            let submitted = effects::publish_queue(root, &envelope.request, "publication-observed")?;
            state::transition(store, intent, PublicationPhase::Published)?;
            outcome.queue_receipt_ref = Some(submitted.queue_receipt_ref);
            outcome.phase = "publication-observed";
        }
        PublicationDecision::ReusePublication => {
            let queue = evidence
                .queue
                .as_ref()
                .ok_or_else(|| MoltenError::invalid_harness("published queue evidence missing"))?;
            state::transition(store, intent, PublicationPhase::Published)?;
            outcome.queue_receipt_ref = Some(queue.receipt_ref.clone());
            outcome.phase = "duplicate-publication";
        }
        PublicationDecision::CompletedDispatch => {
            let control = evidence
                .dispatch
                .as_ref()
                .ok_or_else(|| MoltenError::invalid_harness("dispatch evidence missing"))?;
            outcome.queue_receipt_ref = Some(effects::observe_dispatch(root, &envelope.request, control)?);
            state::transition(store, intent, PublicationPhase::Dispatched)?;
            outcome.phase = "completed-dispatch";
        }
        PublicationDecision::CheckDedup
        | PublicationDecision::Conflict
        | PublicationDecision::OutcomeUnknown
        | PublicationDecision::Rejected => {
            return Ok(state::denied(decision, format!("publication reconciliation unresolved: {decision:?}")));
        }
    }
    Ok(outcome)
}

pub(super) fn submit(root: &crate::node_state::DirectoryRoot, request_value: &IoValue) -> Result<ControlSubmit> {
    let request = crate::node_runtime::parse_control_request(request_value)?;
    let delivery_root = root.delivery_store()?;
    let mut store = crate::delivery_idempotency::PublicationStore::open(&delivery_root)?;
    store.require_healthy()?;
    effects::publish_request(root, &request, &mut store).map(|publication| publication.submitted)
}

#[cfg(test)]
mod tests;
