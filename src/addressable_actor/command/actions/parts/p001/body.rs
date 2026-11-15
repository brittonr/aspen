fn request(session: &mut super::Session, payload_path: &str) -> crate::error::Result<serde_json::Value> {
    let bytes = session.root.read(&crate::node_state::RelativePath::parse(payload_path)?, 65_536)?;
    let input = crate::addressable_actor::counter::Add::decode(&bytes)?;
    if input.actor_key_ref != session.member() {
        return Err(super::invalid("counter request targets a different configured key"));
    }
    let payload = input.materialize()?;
    let ingress_ref = identity("counter-ingress-request-v1", &[&input.request_id, &payload.value_ref])?;
    let tick = session.tick;
    let prior = state(session, false)?;
    if let Some(completed) = prior["completed"]
        .as_array()
        .and_then(|items| items.iter().find(|done| done["request_id"] == input.request_id))
    {
        if completed["increment"].as_u64() != Some(input.increment) {
            return Err(super::invalid("logical counter request reused with different increment"));
        }
        let Some(result) = session.consumer()?.result(&input, &ingress_ref, tick, None)? else {
            return Ok(serde_json::json!({"semantic_outcome":"unknown", "reconciliation_required":true,
                "request_id":input.request_id, "message":"native semantic publication exists without terminal actor/delivery completion; do not redispatch"}));
        };
        return counter_result_value(session, &result, crate::addressable_actor::PublicOutcome::DuplicateReplay);
    }
    let Some((ingress, token)) = prepare_work(session, &input, payload, ingress_ref)? else {
        return Ok(serde_json::json!({"semantic_outcome":"progress", "queued":true,
            "request_id":input.request_id, "message":"request queued; no exact claim for this request was committed"}));
    };
    let reason = WakeReason::Message {
        delivery_item_ref: ingress.ingress.payload.value_ref.clone(),
        delivery_token_ref: token.token_ref.clone(),
    };
    let ack = delivery_request(session, &input.request_id, "acknowledge",
        DeliveryOperation::Acknowledge { token: token.clone() }, Vec::new());
    wake_request(session, &input.request_id, &ingress, &ack, reason)?;
    complete_request(session, &input.request_id, ingress, ack, token)
}

fn prepare_work(
    session: &mut super::Session,
    input: &crate::addressable_actor::counter::Add,
    payload: crate::system_extension::NativeCallbackValue,
    ingress_ref: String,
) -> crate::error::Result<Option<(crate::node_service_hosting::DeliveryInput, DeliveryToken)>> {
    let member = session.member();
    let native = session.observe()?;
    let service = session
        .bundle
        .services
        .iter()
        .find(|service| service.manifest.service_id == member)
        .ok_or_else(|| super::invalid("actor is absent from exact admission bundle"))?;
    let authority_ref = service.executable.authority_ref.clone();
    let policy_ref = service.executable.policy_ref.clone();
    let resource_ref = service.executable.resource_ref.clone();
    let transport_profile_ref = service.host_profile.transport_profile_ref.clone();
    let manifest_ref = service.executable.manifest_ref.clone();
    let presentation = crate::system_extension::WorkPresentation {
        work_ref: payload.value_ref.clone(),
        class: crate::system_extension::WorkClass::Durable,
        generation: native.lifecycle.generation,
        incarnation: native.lifecycle.incarnation,
    };
    submit_work(session, &member, &presentation, &payload, native.lifecycle.phase)?;
    let Some(token) = claim_request(session, input, &payload)? else {
        return Ok(None);
    };
    let endpoint_ref = identity("counter-local-ingress-endpoint-v1", &[&member, &transport_profile_ref])?;
    let ingress = crate::node_service_hosting::DeliveryInput {
        member: member.clone(),
        presentation,
        claim_ref: None,
        logical_tick: session.tick,
        ingress: crate::system_extension::NativeIngressEnvelope {
            schema: crate::system_extension::NATIVE_INGRESS_SCHEMA.to_string(),
            request_ref: ingress_ref,
            endpoint_ref,
            peer_ref: session.configuration.profile.authority_profile_ref.clone(),
            service_id: member,
            manifest_ref,
            generation: native.lifecycle.generation,
            authority_ref,
            policy_ref,
            resource_ref,
            transport_profile_ref,
            alpn: crate::system_extension::NATIVE_ALPN.to_string(),
            framing: crate::system_extension::NATIVE_FRAMING.to_string(),
            accounted_bytes: payload.bytes.len() as u64,
            payload,
        },
    };
    Ok(Some((ingress, token)))
}

fn submit_work(
    session: &mut super::Session,
    member: &str,
    presentation: &crate::system_extension::WorkPresentation,
    payload: &crate::system_extension::NativeCallbackValue,
    phase: crate::system_extension::LifecyclePhase,
) -> crate::error::Result<()> {
    if phase == crate::system_extension::LifecyclePhase::Drained {
        session.signing.prepare(&session.owner.actor_submit_dormant_preview(
            member,
            presentation,
            payload,
            session.tick,
        )?)?;
        session.owner.actor_submit_dormant(member, presentation.clone(), payload.clone(), session.tick)?;
    } else {
        let work = crate::node_service_host::operation::Invocation::Submit {
            member: member.to_string(),
            work: presentation.clone(),
            payload: payload.clone(),
        };
        session.signing.prepare(&session.owner.preview(&work, session.tick)?)?;
        let submitted = session.owner.execute(work, session.tick)?;
        session.retain(&submitted)?;
    }
    Ok(())
}

fn claim_request(
    session: &mut super::Session,
    input: &crate::addressable_actor::counter::Add,
    payload: &crate::system_extension::NativeCallbackValue,
) -> crate::error::Result<Option<DeliveryToken>> {
    let item_ref = &payload.value_ref;
    let enqueue = delivery_request(
        session,
        &input.request_id,
        "enqueue",
        DeliveryOperation::Enqueue {
            item_ref: item_ref.clone(),
            content_ref: payload.value_ref.clone(),
            metadata_ref: payload.value_ref.clone(),
            metadata_bytes: 0,
        },
        Vec::new(),
    );
    if !session.bridge.retained_enqueue(&enqueue)? {
        apply_delivery(session, &enqueue)?;
    }
    let claim = delivery_request(session, &input.request_id, "claim", DeliveryOperation::Claim, Vec::new());
    if session.bridge.claimable_item(&claim)?.as_deref() != Some(item_ref.as_str()) {
        return Ok(None);
    }
    apply_delivery(session, &claim)?;
    let token = session.bridge.active_token(&session.configuration.queue_id, item_ref)?.ok_or_else(|| {
        super::invalid(
            "counter request has no exact current durable coordination claim; use retained completion for replay",
        )
    })?;
    Ok(Some(token))
}

fn wake_request(
    session: &mut super::Session,
    request_id: &str,
    ingress: &crate::node_service_hosting::DeliveryInput,
    ack: &DeliveryRequest,
    reason: WakeReason,
) -> crate::error::Result<()> {
    let wake_ref = identify_actor_wake(&reason);
    let tick = session.tick;
    let mut consumer = session.consumer()?;
    let fence = consumer.fence(tick)?;
    let wake = consumer.apply(crate::addressable_actor::native::Apply { fence: &fence, operation_id: &format!("wake-{}", &request_id[7..]), operation: ActorOperation::Wake { reason }, logical_tick: tick, work: Some(ingress), acknowledgement: Some(ack) })?;
    if !matches!(crate::addressable_actor::public_outcome(&wake), crate::addressable_actor::PublicOutcome::Progress) {
        return Err(super::invalid("actor wake did not durably complete its actual native effects"));
    }
    if wake.final_state.state.phase == ActorPhase::Starting {
        let fence = consumer.fence(tick)?;
        let started = consumer.apply(crate::addressable_actor::native::Apply { fence: &fence, operation_id: &format!("start-{}", &request_id[7..]), operation: ActorOperation::StartSucceeded { wake_ref }, logical_tick: tick, work: None, acknowledgement: None })?;
        if crate::addressable_actor::public_outcome(&started) != crate::addressable_actor::PublicOutcome::Progress {
            return Err(super::invalid("actor startup remains unknown"));
        }
    }
    Ok(())
}

fn complete_request(
    session: &mut super::Session,
    request_id: &str,
    ingress: crate::node_service_hosting::DeliveryInput,
    ack: DeliveryRequest,
    token: DeliveryToken,
) -> crate::error::Result<serde_json::Value> {
    let tick = session.tick;
    session
        .signing
        .prepare(&session.owner.delivery_native_commit_preview(&ingress.member, &ingress.ingress.request_ref, tick)?)?;
    let proof = session.owner.delivery_native_commit(&ingress.member, &ingress.ingress.request_ref, tick)?;
    let mut ack = ack;
    ack.evidence_refs = vec![
        proof.event_ref().to_string(),
        proof.commit_ref().to_string(),
        proof.state().value_ref.clone(),
    ];
    let operation = ActorOperation::CompleteDelivery {
        delivery_item_ref: ingress.ingress.payload.value_ref.clone(),
        delivery_token_ref: token.token_ref,
        semantic_event_ref: proof.event_ref().to_string(),
        semantic_commit_ref: proof.commit_ref().to_string(),
    };
    let mut consumer = session.consumer()?;
    let fence = consumer.fence(tick)?;
    let outcome = consumer.apply(crate::addressable_actor::native::Apply { fence: &fence, operation_id: &format!("complete-{}", &request_id[7..]), operation: operation.clone(), logical_tick: tick, work: Some(&ingress), acknowledgement: Some(&ack) })?;
    drop(consumer);
    let path = retain_work(session, &ingress)?;
    let mut result = outcome_value(session, &outcome)?;
    result["replay"] = serde_json::json!({"operation_id":format!("replay-{}", &request_id[7..]), "operation":operation,
        "work":path, "acknowledgement":ack});
    result["counter"] = state(session, false)?;
    Ok(result)
}

