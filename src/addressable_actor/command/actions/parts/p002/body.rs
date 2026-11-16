fn delivery_request(
    session: &super::Session,
    id: &str,
    step: &str,
    operation: DeliveryOperation,
    evidence_refs: Vec<String>,
) -> DeliveryRequest {
    DeliveryRequest {
        schema: DELIVERY_REQUEST_SCHEMA.to_string(),
        queue_id: session.configuration.queue_id.clone(),
        operation_id: crate::preserves_rail::content_ref_from_bytes(format!("{id}:{step}:{}", session.tick).as_bytes()),
        actor_id: session.configuration.node_id.clone(),
        service_generation: session.manifest.service_generation,
        consistency_epoch: 1,
        engine_epoch: session.engine_epoch,
        time_profile_ref: session.manifest.time_profile_ref.clone(),
        logical_tick: session.tick,
        currentness: DeliveryCurrentness::Linearizable,
        authority_refs: vec![session.configuration.profile.authority_profile_ref.clone()],
        policy_refs: vec![session.manifest.policy_ref.clone()],
        resource_refs: vec![session.configuration.profile.resource_profile_ref.clone()],
        evidence_refs: if evidence_refs.is_empty() {
            vec![session.configuration.profile.evidence_profile_ref.clone()]
        } else {
            evidence_refs
        },
        operation,
    }
}
fn apply_delivery(session: &mut super::Session, request: &DeliveryRequest) -> crate::error::Result<()> {
    session.signing.prepare(&session.owner.delivery_preview(
        &session.configuration.delivery_member,
        &session.bridge,
        request,
        None,
    )?)?;
    let outcome = session.owner.delivery_apply(&session.configuration.delivery_member, &mut session.bridge, request)?;
    session.retain(&outcome.receipt.value)?;
    if !outcome.receipt.status.commit_confirmed() {
        return Err(super::invalid("durable coordination delivery is not confirmed"));
    }
    Ok(())
}
fn counter_result_value(
    session: &super::Session,
    result: &crate::addressable_actor::native::CounterResult,
    outcome: crate::addressable_actor::PublicOutcome,
) -> crate::error::Result<serde_json::Value> {
    let reference = session.retain(result.value())?;
    if reference != result.receipt_ref {
        return Err(super::invalid("native result receipt projection differs from exact canonical bytes"));
    }
    Ok(serde_json::json!({"semantic_outcome":outcome.as_str(),"result":result,"receipt_ref":reference,
        "receipt":crate::preserves_rail::to_text(result.value())?}))
}

fn outcome_value(
    session: &super::Session,
    outcome: &crate::addressable_actor::Outcome,
) -> crate::error::Result<serde_json::Value> {
    session.retain(&outcome.receipt.value)?;
    let public = crate::addressable_actor::public_outcome(outcome);
    Ok(serde_json::json!({"semantic_outcome":public.as_str(),
        "reconciliation_required":public == crate::addressable_actor::PublicOutcome::Unknown,
        "receipt_ref":outcome.receipt.receipt_ref, "receipt":crate::preserves_rail::to_text(&outcome.receipt.value)?,
        "state_ref":outcome.final_state.state_ref, "revision":outcome.final_state.revision,
        "phase":outcome.final_state.state.phase.as_str(), "generation":outcome.final_state.state.extension_generation}))
}
fn fence_value(f: &crate::addressable_actor::native::Fence) -> serde_json::Value {
    serde_json::json!({"actor_key_ref":f.actor_key_ref,"placement_ref":f.placement_ref,"generation":f.generation,"incarnation":f.incarnation,
        "lifecycle_sequence":f.lifecycle_sequence,"state_ref":f.expected.state_ref,"revision":f.expected.revision})
}
fn parse_fence(value: &serde_json::Value) -> crate::error::Result<crate::addressable_actor::native::Fence> {
    let number = |key| {
        value
            .get(key)
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| super::invalid("fence numeric field missing"))
    };
    Ok(crate::addressable_actor::native::Fence {
        actor_key_ref: text(value, "actor_key_ref")?.to_string(),
        placement_ref: text(value, "placement_ref")?.to_string(),
        generation: number("generation")?,
        incarnation: number("incarnation")?,
        lifecycle_sequence: number("lifecycle_sequence")?,
        expected: crate::addressable_actor::ExpectedState {
            state_ref: Some(text(value, "state_ref")?.to_string()),
            revision: number("revision")?,
        },
    })
}
fn identity(tag: &'static str, fields: &[&str]) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
        tag,
        fields.iter().map(crate::preserves_rail::string).collect(),
    ))
}
fn required(input: Option<&serde_json::Value>) -> crate::error::Result<&serde_json::Value> {
    input.ok_or_else(|| super::invalid("action requires --input"))
}
fn text<'a>(input: &'a serde_json::Value, key: &str) -> crate::error::Result<&'a str> {
    input
        .get(key)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| super::invalid(format!("missing {key}")))
}

fn retain_work(session: &super::Session, work: &crate::node_service_hosting::DeliveryInput) -> crate::error::Result<String> {
    let payload = crate::preserves_rail::record("node-service-host-value-v1", vec![
        crate::preserves_rail::string(&work.ingress.payload.value_ref),
        crate::preserves_rail::sequence(work.ingress.payload.bytes.iter().map(|byte| crate::preserves_rail::u64_value(u64::from(*byte))).collect()),
    ]);
    let i = &work.ingress;
    let ingress = crate::preserves_rail::record("node-service-host-ingress-v1", vec![
        crate::preserves_rail::string(&i.request_ref),
        crate::preserves_rail::string(&i.endpoint_ref),
        crate::preserves_rail::string(&i.peer_ref),
        crate::preserves_rail::string(&i.service_id),
        crate::preserves_rail::string(&i.manifest_ref),
        crate::preserves_rail::u64_value(i.generation.get()),
        crate::preserves_rail::string(&i.authority_ref),
        crate::preserves_rail::string(&i.policy_ref),
        crate::preserves_rail::string(&i.resource_ref),
        crate::preserves_rail::string(&i.transport_profile_ref),
        payload,
        crate::preserves_rail::u64_value(i.accounted_bytes),
        crate::preserves_rail::string("materialized-native-v3"),
    ]);
    let value = crate::preserves_rail::record("node-service-host-deliver-v1", vec![
        crate::preserves_rail::string(&work.member),
        crate::preserves_rail::record("node-service-host-work-v1", vec![
            crate::preserves_rail::string(&work.presentation.work_ref),
            crate::preserves_rail::string("durable"),
            crate::preserves_rail::u64_value(work.presentation.generation.get()),
            crate::preserves_rail::u64_value(work.presentation.incarnation.get()),
        ]),
        crate::preserves_rail::record("node-service-host-delivery-v1", vec![
            crate::preserves_rail::sequence(work.claim_ref.iter().map(crate::preserves_rail::string).collect()),
            ingress,
        ]),
    ]);
    let reference = session.retain(&value)?;
    Ok(format!("evidence/{}.preserves", &reference[7..]))
}
