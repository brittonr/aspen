pub fn step_value(step: &crate::harness::nucleus::CoreStep) -> preserves::IOValue {
    match step {
        crate::harness::nucleus::CoreStep::Send { from, to, body } => crate::preserves_rail::record("send", vec![
            crate::preserves_rail::string(from),
            crate::preserves_rail::string(to),
            body.as_iovalue().clone(),
        ]),
        crate::harness::nucleus::CoreStep::Observe { actor, pattern } => {
            crate::preserves_rail::record("observe", vec![
                crate::preserves_rail::string(actor),
                pattern.as_iovalue().clone(),
            ])
        }
        crate::harness::nucleus::CoreStep::Assert { actor, value } => crate::preserves_rail::record("assert", vec![
            crate::preserves_rail::string(actor),
            value.as_iovalue().clone(),
        ]),
        crate::harness::nucleus::CoreStep::Retract { actor, value } => crate::preserves_rail::record("retract", vec![
            crate::preserves_rail::string(actor),
            value.as_iovalue().clone(),
        ]),
        crate::harness::nucleus::CoreStep::Clock { actor } => {
            crate::preserves_rail::record("clock", vec![crate::preserves_rail::string(actor)])
        }
        crate::harness::nucleus::CoreStep::Random { actor, upper } => crate::preserves_rail::record("random", vec![
            crate::preserves_rail::string(actor),
            crate::preserves_rail::u64_value(*upper),
        ]),
    }
}

pub fn event_value(event: &crate::harness::nucleus::CoreEvent) -> preserves::IOValue {
    match event {
        crate::harness::nucleus::CoreEvent::MessageDelivered { from, to, body } => {
            texts_record("message-delivered", &[from, to], Some(body.as_iovalue()))
        }
        crate::harness::nucleus::CoreEvent::ObserveRegistered { actor, pattern } => {
            texts_record("observe-registered", &[actor], Some(pattern.as_iovalue()))
        }
        crate::harness::nucleus::CoreEvent::AssertionObserved { observer, owner, value } => {
            texts_record("assertion-observed", &[observer, owner], Some(value.as_iovalue()))
        }
        crate::harness::nucleus::CoreEvent::AssertionCommitted { actor, value } => {
            texts_record("assertion-committed", &[actor], Some(value.as_iovalue()))
        }
        crate::harness::nucleus::CoreEvent::AssertionRetracted { actor, value } => {
            texts_record("assertion-retracted", &[actor], Some(value.as_iovalue()))
        }
        crate::harness::nucleus::CoreEvent::AssertionRetractionObserved { observer, owner, value } => {
            texts_record("assertion-retraction-observed", &[observer, owner], Some(value.as_iovalue()))
        }
        crate::harness::nucleus::CoreEvent::FacetCreated {
            facet_id,
            actor,
            parent,
        } => texts_record("facet-created", &[facet_id, actor, parent.as_deref().unwrap_or("")], None),
        crate::harness::nucleus::CoreEvent::FacetStopped { facet_id } => {
            texts_record("facet-stopped", &[facet_id], None)
        }
        crate::harness::nucleus::CoreEvent::FacetStopHandlerRan { facet_id, handler_ref } => {
            texts_record("facet-stop-handler-ran", &[facet_id, handler_ref], None)
        }
        crate::harness::nucleus::CoreEvent::EffectRequest {
            effect,
            actor,
            sequence,
            upper,
        } => crate::preserves_rail::record("effect-request", effect_fields(effect, actor, *sequence, *upper)),
        crate::harness::nucleus::CoreEvent::EffectResponse {
            effect,
            actor,
            sequence,
            upper,
            value,
        } => {
            let mut fields = effect_fields(effect, actor, *sequence, *upper);
            fields.push(crate::preserves_rail::u64_value(*value));
            crate::preserves_rail::record("effect-response", fields)
        }
        crate::harness::nucleus::CoreEvent::AdmissionDecision { request, decision } => {
            admission_decision_event_value(request, decision)
        }
        crate::harness::nucleus::CoreEvent::TurnRolledBack { actor, reason } => {
            texts_record("turn-rolled-back", &[actor, reason], None)
        }
        crate::harness::nucleus::CoreEvent::TurnCommitted { actor, cause, .. } => {
            crate::preserves_rail::record("turn-committed", vec![
                crate::preserves_rail::string(actor),
                cause.to_value(),
            ])
        }
    }
}

/// Record of string fields in order, optionally followed by one embedded value.
fn texts_record(label: &'static str, texts: &[&str], tail: Option<&preserves::IOValue>) -> preserves::IOValue {
    // `chain` reports an exact size hint, so `collect` allocates the field vector once.
    let fields = texts.iter().map(crate::preserves_rail::string).chain(tail.cloned()).collect();
    crate::preserves_rail::record(label, fields)
}

fn effect_fields(
    effect: &crate::harness::nucleus::CoreEffect,
    actor: &str,
    sequence: u64,
    upper: Option<u64>,
) -> Vec<preserves::IOValue> {
    let mut fields = vec![
        crate::preserves_rail::string(effect_name(effect)),
        crate::preserves_rail::string(actor),
        crate::preserves_rail::u64_value(sequence),
    ];
    fields.extend(upper.map(crate::preserves_rail::u64_value));
    fields
}

fn admission_decision_event_value(
    request: &crate::harness::nucleus::AdmissionRequest,
    decision: &crate::runtime::AdmissionDecision,
) -> preserves::IOValue {
    crate::preserves_rail::record("admission-decision-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RUNTIME_ADMISSION_DECISION_SCHEMA),
        admission_request_value(request),
        crate::preserves_rail::record("decision", vec![
            crate::preserves_rail::string(decision.status()),
            crate::preserves_rail::string(decision.reason()),
        ]),
    ])
}

pub fn admission_decision_event_value_with_authority(
    request: &crate::harness::nucleus::AdmissionRequest,
    authority: &super::observed::AdmissionAuthorityEvidence,
    decision: &crate::runtime::AdmissionDecision,
) -> preserves::IOValue {
    crate::preserves_rail::record("admission-decision-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RUNTIME_ADMISSION_DECISION_SCHEMA),
        admission_request_value(request),
        admission_authority_value(authority),
        crate::preserves_rail::record("decision", vec![
            crate::preserves_rail::string(decision.status()),
            crate::preserves_rail::string(decision.reason()),
        ]),
    ])
}

pub(super) fn admission_authority_value(authority: &super::observed::AdmissionAuthorityEvidence) -> preserves::IOValue {
    crate::preserves_rail::record("authority", vec![
        crate::preserves_rail::record("source", vec![crate::preserves_rail::string(&authority.source)]),
        crate::preserves_rail::record("capability-ref", vec![crate::preserves_rail::string(&authority.capability_ref)]),
        crate::preserves_rail::record("authorized", vec![crate::preserves_rail::bool_value(authority.authorized)]),
        optional_string_value(authority.grant_ref.as_deref()),
        crate::preserves_rail::record("request-ref", vec![crate::preserves_rail::string(&authority.request_ref)]),
        crate::preserves_rail::record("ucan-proofset-ref", vec![crate::preserves_rail::string(
            &authority.proofset_ref,
        )]),
        authority_ref_sequence("ucan-verification-receipt-refs", &authority.ucan_verification_receipt_refs),
        authority_ref_sequence("derived-grant-refs", &authority.derived_grant_refs),
        crate::preserves_rail::record("basalt-enforcement-receipt-ref", vec![crate::preserves_rail::string(
            &authority.basalt_enforcement_receipt_ref,
        )]),
        crate::preserves_rail::record("basalt-enforcement-receipt", vec![
            authority.basalt_enforcement_receipt_value.clone(),
        ]),
    ])
}

fn authority_ref_sequence(label: &'static str, refs: &[String]) -> preserves::IOValue {
    crate::preserves_rail::record(label, vec![crate::preserves_rail::sequence(
        refs.iter().map(crate::preserves_rail::string).collect(),
    )])
}

pub(super) fn admission_request_value(request: &crate::harness::nucleus::AdmissionRequest) -> preserves::IOValue {
    crate::preserves_rail::record("request", vec![
        crate::preserves_rail::string(&request.actor),
        crate::preserves_rail::string(request.action.as_str()),
        optional_string_value(request.target.as_deref()),
        optional_runtime_value(request.value.as_ref()),
        optional_u64_value(request.upper),
    ])
}

pub(super) fn optional_string_value(value: Option<&str>) -> preserves::IOValue {
    value.map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |value| crate::preserves_rail::record("some", vec![crate::preserves_rail::string(value)]),
    )
}

fn optional_runtime_value(value: Option<&crate::harness::nucleus::RuntimeValue>) -> preserves::IOValue {
    value.map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |value| crate::preserves_rail::record("some", vec![value.as_iovalue().clone()]),
    )
}

fn optional_u64_value(value: Option<u64>) -> preserves::IOValue {
    value.map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |value| crate::preserves_rail::record("some", vec![crate::preserves_rail::u64_value(value)]),
    )
}

pub fn snapshot_value(snapshot: &crate::harness::nucleus::RuntimeSnapshot) -> preserves::IOValue {
    crate::preserves_rail::record("runtime-state-v1", vec![
        crate::preserves_rail::u64_value(snapshot.logical_time),
        crate::preserves_rail::u64_value(snapshot.rng_state),
        crate::preserves_rail::u64_value(snapshot.effect_sequence),
        tuple_set("messages", &snapshot.messages, |message| {
            crate::preserves_rail::record("message", vec![
                crate::preserves_rail::string(&message.from),
                crate::preserves_rail::string(&message.to),
                message.body.as_iovalue().clone(),
            ])
        }),
        tuple_set("assertions", &snapshot.assertions, |assertion| {
            crate::preserves_rail::record("assertion", vec![
                crate::preserves_rail::string(&assertion.actor),
                assertion.value.as_iovalue().clone(),
            ])
        }),
        tuple_set("observers", &snapshot.observers, |observer| {
            crate::preserves_rail::record("observer", vec![
                crate::preserves_rail::string(&observer.actor),
                observer.pattern.as_iovalue().clone(),
            ])
        }),
    ])
}

pub fn observation_value(
    index: u64,
    step_ref: String,
    before_state_hash: String,
    after_state_hash: String,
    events: Vec<preserves::IOValue>,
) -> crate::error::Result<preserves::IOValue> {
    let mut event_refs = Vec::with_capacity(events.len());
    for event in &events {
        event_refs.push(crate::preserves_rail::canonical_hash(event)?);
    }
    let mut event_ref_values: Vec<preserves::IOValue> = Vec::with_capacity(event_refs.len());
    for reference in event_refs {
        event_ref_values.push(crate::preserves_rail::string(reference));
    }
    Ok(crate::preserves_rail::record("turn-observation-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_OBSERVATION_SCHEMA),
        crate::preserves_rail::u64_value(index),
        crate::preserves_rail::string(step_ref),
        crate::preserves_rail::string(before_state_hash),
        crate::preserves_rail::string(after_state_hash),
        crate::preserves_rail::record("event-refs", vec![crate::preserves_rail::sequence(event_ref_values)]),
        crate::preserves_rail::sequence(events),
    ]))
}

fn tuple_set<T, F>(label: &'static str, values: &std::collections::BTreeSet<T>, mut render: F) -> preserves::IOValue
where F: FnMut(&T) -> preserves::IOValue {
    crate::preserves_rail::record(label, vec![crate::preserves_rail::sequence(
        values.iter().map(&mut render).collect(),
    )])
}

fn effect_name(effect: &crate::harness::nucleus::CoreEffect) -> &'static str {
    match effect {
        crate::harness::nucleus::CoreEffect::Clock => "clock",
        crate::harness::nucleus::CoreEffect::Random => "random",
    }
}
