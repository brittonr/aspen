use molten_core::addressable_actor::*;
use molten_core::coordination_delivery::*;

use crate::addressable_actor::native::SigningPort;

pub(super) fn execute(
    session: &mut super::Session,
    action: &str,
    input: Option<&serde_json::Value>,
) -> crate::error::Result<serde_json::Value> {
    match action {
        "initialize" => {
            let tick = session.tick;
            let published = session.consumer()?.initialize(tick)?;
            Ok(
                serde_json::json!({"state_ref": published.state_ref, "revision": published.revision, "phase": published.state.phase.as_str(), "semantic_outcome": "progress"}),
            )
        }
        "fence" => {
            let tick = session.tick;
            Ok(fence_value(&session.consumer()?.fence(tick)?))
        }
        "result" => match input {
            Some(input) => completed_result(session, text(input, "payload")?),
            None => state(session, false),
        },
        "native" => native(session, required(input)?),
        "delivery" => {
            let request: DeliveryRequest = serde_json::from_value(required(input)?.clone()).map_err(super::problem)?;
            if request.logical_tick != session.tick || request.engine_epoch != session.engine_epoch {
                return Err(super::invalid("delivery request tick/engine differs from command"));
            }
            let preview = session.owner.delivery_preview(
                &session.configuration.delivery_member,
                &session.bridge,
                &request,
                None,
            )?;
            session.signing.prepare(&preview)?;
            let outcome =
                session
                    .owner
                    .delivery_apply(&session.configuration.delivery_member, &mut session.bridge, &request)?;
            session.retain(&outcome.receipt.value)?;
            Ok(
                serde_json::json!({"receipt_ref": outcome.receipt.receipt_ref, "receipt": crate::preserves_rail::to_text(&outcome.receipt.value)?,
                "transition": outcome.transition, "semantic_outcome": "progress"}),
            )
        }
        "apply" => {
            let input = required(input)?;
            match input.get("action").and_then(serde_json::Value::as_str) {
                Some("request") => request(session, text(input, "payload")?),
                Some("sleep") => sleep(session, text(input, "operation_id")?),
                _ => apply_input(session, input),
            }
        }
        _ => Err(super::invalid("unknown public actor action")),
    }
}

fn native(session: &mut super::Session, input: &serde_json::Value) -> crate::error::Result<serde_json::Value> {
    let member = session.member();
    let kind = text(input, "kind")?;
    if kind == "install" {
        let operation = crate::node_service_host::operation::Invocation::Install(text(input, "group")?.to_string());
        session.signing.prepare(&session.owner.preview(&operation, session.tick)?)?;
        let value = session.owner.execute(operation, session.tick)?;
        let reference = session.retain(&value)?;
        return Ok(
            serde_json::json!({"receipt_ref": reference, "receipt": crate::preserves_rail::to_text(&value)?, "semantic_outcome":"progress"}),
        );
    }
    let selection = match kind {
        "observe" => crate::node_service_hosting::native::actor::Input::Observe(&member),
        "checkpoint" => crate::node_service_hosting::native::actor::Input::Checkpoint(&member),
        "sleep" => crate::node_service_hosting::native::actor::Input::Sleep(&member),
        "drain" => crate::node_service_hosting::native::actor::Input::Drain(&member),
        "rewake" => crate::node_service_hosting::native::actor::Input::Rewake(&member),
        "state" => return state(session, false),
        "checkpoint-value" => return state(session, true),
        _ => return Err(super::invalid("unsupported native actor operation")),
    };
    session.signing.prepare(&session.owner.actor_preview(selection, session.tick)?)?;
    let record = match kind {
        "observe" => session.owner.actor_instance(&member, session.tick)?,
        "checkpoint" => session.owner.actor_checkpoint(&member, session.tick)?,
        "sleep" => session.owner.actor_sleep(&member, session.tick)?,
        "drain" => session.owner.actor_drain(&member, session.tick)?,
        "rewake" => session.owner.actor_rewake(&member, session.tick)?,
        _ => unreachable!(),
    };
    let canonical = crate::system_extension::canonical_instance_record(&record)?;
    session.retain(&canonical.value)?;
    Ok(serde_json::json!({"record_ref":canonical.record_ref, "phase":record.lifecycle.phase.as_str(),
        "generation":record.lifecycle.generation, "incarnation":record.lifecycle.incarnation,
        "callback_sequence":record.callback_sequence, "event_sequence":record.event_sequence,
        "state_ref":record.state_ref, "checkpoint_ref":record.checkpoint_ref, "unresolved":record.unresolved.len(), "semantic_outcome":"progress"}))
}

fn completed_result(session: &mut super::Session, payload_path: &str) -> crate::error::Result<serde_json::Value> {
    let bytes = session.root.read(&crate::node_state::RelativePath::parse(payload_path)?, 65_536)?;
    let input = crate::addressable_actor::counter::Add::decode(&bytes)?;
    let payload = input.materialize()?;
    let ingress_ref = identity("counter-ingress-request-v1", &[&input.request_id, &payload.value_ref])?;
    let tick = session.tick;
    let observed = {
        let mut consumer = session.consumer()?;
        consumer.result(&input, &ingress_ref, tick, None)?
    };
    match observed {
        Some(result) => counter_result_value(session, &result, crate::addressable_actor::PublicOutcome::Applied),
        None => Ok(serde_json::json!({"semantic_outcome":"unknown","reconciliation_required":true,
            "request_id":input.request_id,"message":"terminal actor and delivery completion not established; do not redispatch"})),
    }
}

fn state(session: &mut super::Session, checkpoint: bool) -> crate::error::Result<serde_json::Value> {
    let member = session.member();
    let input = if checkpoint {
        crate::node_service_hosting::native::actor::Input::CheckpointValue(&member)
    } else {
        crate::node_service_hosting::native::actor::Input::State(&member)
    };
    session.signing.prepare(&session.owner.actor_preview(input, session.tick)?)?;
    let value = if checkpoint {
        session.owner.actor_checkpoint_value(&member, session.tick)?
    } else {
        session.owner.actor_state(&member, session.tick)?
    };
    let decoded = crate::addressable_actor::counter::State::decode(&value.bytes)?;
    if decoded.actor_key_ref != member {
        return Err(super::invalid("counter result belongs to another actor"));
    }
    Ok(
        serde_json::json!({"state_ref": value.value_ref, "value": decoded.value, "actor_key_ref":decoded.actor_key_ref,
        "completed":decoded.completed.iter().map(|done| serde_json::json!({"request_id":done.request_id,"increment":done.increment,"result":done.result})).collect::<Vec<_>>(),
        "semantic_outcome":"observation-not-delivery-completion"}),
    )
}

fn apply_input(session: &mut super::Session, input: &serde_json::Value) -> crate::error::Result<serde_json::Value> {
    let operation: ActorOperation = serde_json::from_value(
        input.get("operation").cloned().ok_or_else(|| super::invalid("apply requires exact operation"))?,
    )
    .map_err(super::problem)?;
    let is_completion = matches!(&operation, ActorOperation::CompleteDelivery { .. });
    let operation_id = text(input, "operation_id")?;
    let work = input
        .get("work")
        .and_then(serde_json::Value::as_str)
        .map(|path| {
            let bytes = session.root.read(&crate::node_state::RelativePath::parse(path)?, 1_048_576)?;
            match crate::node_service_host::operation::decode(&bytes, "deliver", session.tick)? {
                crate::node_service_host::operation::Invocation::Deliver(work) => Ok(*work),
                crate::node_service_host::operation::Invocation::Install(_)
                | crate::node_service_host::operation::Invocation::Status
                | crate::node_service_host::operation::Invocation::Supervise { .. }
                | crate::node_service_host::operation::Invocation::Submit { .. }
                | crate::node_service_host::operation::Invocation::Readmit { .. }
                | crate::node_service_host::operation::Invocation::Migrate { .. }
                | crate::node_service_host::operation::Invocation::Upgrade { .. }
                | crate::node_service_host::operation::Invocation::Reconcile(_) => {
                    Err(super::invalid("apply work must be an exact retained native delivery fixture"))
                }
            }
        })
        .transpose()?;
    let acknowledgement = input
        .get("acknowledgement")
        .map(|value| serde_json::from_value::<DeliveryRequest>(value.clone()).map_err(super::problem))
        .transpose()?;
    let tick = session.tick;
    let mut consumer = session.consumer()?;
    let fence = input.get("fence").map(parse_fence).transpose()?.map_or_else(|| consumer.fence(tick), Ok)?;
    let outcome = consumer.apply(crate::addressable_actor::native::Apply { fence: &fence, operation_id, operation, logical_tick: tick, work: work.as_ref(), acknowledgement: acknowledgement.as_ref() })?;
    if is_completion && outcome.transition.decision == ActorDecision::DuplicateReplay {
        let work = work.as_ref().ok_or_else(|| super::invalid("completion replay lacks original native work"))?;
        let logical_request = crate::addressable_actor::counter::Add::decode(&work.ingress.payload.bytes)?;
        if let Some(result) =
            consumer.result(&logical_request, &work.ingress.request_ref, tick, acknowledgement.as_ref())?
        {
            drop(consumer);
            return counter_result_value(session, &result, crate::addressable_actor::PublicOutcome::DuplicateReplay);
        }
    }
    drop(consumer);
    outcome_value(session, &outcome)
}

fn sleep(session: &mut super::Session, operation_id: &str) -> crate::error::Result<serde_json::Value> {
    // The exact current semantic bytes are checkpoint intent; the actual checkpoint callback must
    // return them.
    let observation = state(session, false)?;
    let checkpoint_ref = text(&observation, "state_ref")?.to_string();
    let tick = session.tick;
    let mut consumer = session.consumer()?;
    let fence = consumer.fence(tick)?;
    let outcome = consumer.apply(crate::addressable_actor::native::Apply { fence: &fence, operation_id, operation: ActorOperation::IdleSleep {
        checkpoint_ref,
        pending_mailbox_items: 0,
        unresolved_effects: 0,
    }, logical_tick: tick, work: None, acknowledgement: None })?;
    drop(consumer);
    outcome_value(session, &outcome)
}

