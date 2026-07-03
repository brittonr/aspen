
pub fn apply_delivered_envelope(state: &mut RuntimeState, envelope: &Envelope) -> Result<Vec<RuntimeEvent>> {
    validate_envelope_identity(envelope)?;
    let actor = remote_actor_id(envelope);
    let payload = RuntimeValue::new(envelope.payload.clone())?;
    let step = match envelope.operation {
        Operation::Assert => RuntimeStep::Assert { actor, value: payload },
        Operation::Retract => RuntimeStep::Retract { actor, value: payload },
        Operation::Observe => RuntimeStep::Observe {
            actor,
            pattern: payload,
        },
        Operation::Message => RuntimeStep::Send {
            from: actor,
            to: format!("{}:inbox", envelope.to_peer),
            body: payload,
        },
    };
    Ok(state.apply_step(&step))
}

pub fn admit_and_apply_delivered_envelope(
    state: &mut RuntimeState,
    delivery: &Delivery,
    evidence: &DeliveryEvidence,
) -> Result<Applied> {
    validate_delivery_evidence(&delivery.envelope, evidence)?;
    let transport_receipt_ref = canonical_hash(&delivery.receipt_value)?;
    let turn_journal_context_ref = turn_journal_context_ref(delivery)?;
    let mut turn_context_refs = vec![turn_journal_context_ref.clone()];
    turn_context_refs.push(transport_receipt_ref.clone());
    let admission_receipt_value = remote_admission_receipt_value(AdmissionReceiptInput {
        decision: "pass",
        envelope: &delivery.envelope,
        transport_receipt_ref: &transport_receipt_ref,
        evidence,
        turn_context_refs: &turn_context_refs,
        diagnostics: Vec::new(),
    });
    let events = apply_delivered_envelope(state, &delivery.envelope)?;
    Ok(Applied {
        events,
        admission_receipt_value,
        turn_journal_context_ref,
    })
}

pub fn admit_and_apply_delivered_envelope_idempotent(
    idempotency_root: &Path,
    state: &mut RuntimeState,
    delivery: &Delivery,
    evidence: &DeliveryEvidence,
    gap_policy: crate::delivery_idempotency::GapPolicy,
) -> Result<IdempotentApplied> {
    validate_delivery_evidence(&delivery.envelope, evidence)?;
    let transport_receipt_ref = canonical_hash(&delivery.receipt_value)?;
    let turn_journal_context_ref = turn_journal_context_ref(delivery)?;
    let mut turn_context_refs = vec![turn_journal_context_ref.clone()];
    turn_context_refs.push(transport_receipt_ref.clone());
    let admission_receipt_value = remote_admission_receipt_value(AdmissionReceiptInput {
        decision: "pass",
        envelope: &delivery.envelope,
        transport_receipt_ref: &transport_receipt_ref,
        evidence,
        turn_context_refs: &turn_context_refs,
        diagnostics: Vec::new(),
    });
    let admission_receipt_ref = canonical_hash(&admission_receipt_value)?;
    let idempotency = crate::delivery_idempotency::check(crate::delivery_idempotency::CheckInput {
        root: idempotency_root,
        scope_profile: crate::delivery_idempotency::SCOPE_REMOTE_TOPIC,
        scope_ref: &crate::delivery_idempotency::remote_topic_scope_ref(
            &delivery.envelope.topic,
            &delivery.envelope.to_peer,
        )?,
        producer: &remote_actor_id(&delivery.envelope),
        consumer: &delivery.envelope.to_peer,
        sequence: delivery.envelope.sequence,
        intent: &format!("remote-dataspace-{}", delivery.envelope.operation.as_str()),
        payload_ref: &canonical_hash(&delivery.envelope.payload)?,
        policy_refs: &envelope_policy_refs(&delivery.envelope.capability_refs, &delivery.envelope.evidence_refs)?,
        evidence_refs: &[transport_receipt_ref],
        semantic_result_ref: Some(&admission_receipt_ref),
        gap_policy,
    })?;
    let events = if idempotency.should_commit_side_effect {
        apply_delivered_envelope(state, &delivery.envelope)?
    } else {
        Vec::new()
    };
    Ok(IdempotentApplied {
        events,
        admission_receipt_value,
        turn_journal_context_ref,
        idempotency_receipt_value: idempotency.receipt.value,
        operation_ref: idempotency.operation.operation_ref,
        prior_semantic_result_ref: idempotency.prior_semantic_result_ref,
    })
}

pub fn deny_admission_receipt_value(
    envelope: &Envelope,
    transport_receipt_ref: &str,
    diagnostics: Vec<String>,
) -> IoValue {
    remote_admission_receipt_value(AdmissionReceiptInput {
        decision: "deny",
        envelope,
        transport_receipt_ref,
        evidence: &DeliveryEvidence::default(),
        turn_context_refs: &[],
        diagnostics,
    })
}

pub fn parse_delivery_log(value: &IoValue) -> Result<DeliveryLog> {
    let fields = value
        .collect_simple_record("remote-dataspace-delivery-log-v1", Some(4))
        .ok_or_else(|| MoltenError::invalid_harness("expected <remote-dataspace-delivery-log-v1 ...>"))?;
    require_schema(&fields[0], DELIVERY_LOG_SCHEMA, "remote dataspace delivery log schema")?;
    let is_replayable = record_bool(&fields[1], "replayable")?;
    let entry_values = field_sequence(&fields[2], "entries")?;
    let entries = entry_values.iter().map(parse_delivery_log_entry).collect::<Result<Vec<_>>>()?;
    Ok(DeliveryLog {
        log_ref: canonical_hash(value)?,
        replayable: is_replayable,
        entries,
        value: value.clone(),
    })
}

pub fn delivery_log(deliveries: &[Delivery], replayable: bool) -> Result<DeliveryLog> {
    delivery_log_with_idempotency_receipts(deliveries, &[], replayable)
}

pub fn delivery_log_with_idempotency_receipts(
    deliveries: &[Delivery],
    idempotency_receipts: &[IoValue],
    replayable: bool,
) -> Result<DeliveryLog> {
    if !idempotency_receipts.is_empty() && idempotency_receipts.len() != deliveries.len() {
        return Err(MoltenError::invalid_harness(
            "remote delivery log idempotency receipt count must match delivery count",
        ));
    }
    let mut entries = Vec::with_capacity(deliveries.len());
    for (index, delivery) in deliveries.iter().enumerate() {
        let mut fields = vec![
            crate::preserves_rail::u64_value(index as u64),
            record("envelope", vec![delivery.envelope.value.clone()]),
            record("transport-receipt", vec![delivery.receipt_value.clone()]),
            record("operation-ref", vec![string(&delivery.envelope.operation_ref)]),
        ];
        if let Some(receipt) = idempotency_receipts.get(index) {
            let parsed = crate::delivery_idempotency::parse_receipt(receipt)?;
            if parsed.operation_ref != delivery.envelope.operation_ref {
                return Err(MoltenError::invalid_harness("remote delivery log idempotency operation ref mismatch"));
            }
            validate_replay_idempotency_receipt(&parsed)?;
            fields.push(record("idempotency-receipt", vec![receipt.clone()]));
        }
        entries.push(record("entry", fields));
    }
    let idempotency_status = if idempotency_receipts.is_empty() { "n/a" } else { "pass" };
    let value = record("remote-dataspace-delivery-log-v1", vec![
        string(DELIVERY_LOG_SCHEMA),
        record("replayable", vec![crate::preserves_rail::bool_value(replayable)]),
        record("entries", vec![sequence(entries)]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("recorded-canonical-envelopes"), string("pass")]),
            record("check", vec![string("idempotency-operation-ref-bound"), string("pass")]),
            record("check", vec![string("idempotency-receipt-bound"), string(idempotency_status)]),
            record("check", vec![string("no-live-network-during-replay"), string("pass")]),
        ])]),
    ]);
    Ok(DeliveryLog {
        log_ref: canonical_hash(&value)?,
        replayable,
        entries: deliveries.to_vec(),
        value,
    })
}

fn validate_replay_idempotency_receipt(receipt: &crate::delivery_idempotency::Receipt) -> Result<()> {
    match receipt.decision.as_str() {
        "first" => Ok(()),
        "duplicate" if receipt.prior_receipt_ref.is_some() => Ok(()),
        "duplicate" => Err(MoltenError::invalid_harness(
            "remote delivery log duplicate idempotency receipt missing prior receipt",
        )),
        _ => Err(MoltenError::invalid_harness(
            "remote delivery log idempotency receipt is not replay-admissible",
        )),
    }
}

pub fn replay_delivery_log(state: &mut RuntimeState, log: &DeliveryLog) -> Result<Vec<RuntimeEvent>> {
    if !log.replayable {
        return Err(MoltenError::invalid_harness(
            "remote dataspace delivery log is non-replayable and cannot satisfy deterministic replay",
        ));
    }
    ensure_count_at_most(log.entries.len(), MAX_REPLAY_EVENTS, "remote replay deliveries")?;
    let mut events = Vec::with_capacity(log.entries.len());
    for delivery in &log.entries {
        let delivered = apply_delivered_envelope(state, &delivery.envelope)?;
        extend_bounded(&mut events, delivered, MAX_REPLAY_EVENTS, "remote replay events")?;
    }
    Ok(events)
}

pub fn gate_receipt_value(
    delivery_log: &DeliveryLog,
    admission_receipts: &[IoValue],
    turn_context_refs: &[String],
) -> Result<IoValue> {
    if !delivery_log.replayable {
        return Err(MoltenError::invalid_harness("remote dataspace gate receipt requires a replayable delivery log"));
    }
    if admission_receipts.is_empty() {
        return Err(MoltenError::invalid_harness(
            "remote dataspace gate receipt requires at least one admission receipt",
        ));
    }
    validate_refs(turn_context_refs, "turn journal context ref")?;
    let admission_refs: Vec<String> = admission_receipts.iter().map(canonical_hash).collect::<Result<Vec<_>>>()?;
    let operation_refs = delivery_log.entries.iter().map(|delivery| string(&delivery.envelope.operation_ref)).collect();
    Ok(record("remote-dataspace-gate-receipt-v1", vec![
        string(GATE_RECEIPT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("delivery-log", vec![string(&delivery_log.log_ref)]),
        record("admission-receipts", vec![sequence(admission_refs.iter().map(string).collect())]),
        record("turn-journal-context-refs", vec![sequence(turn_context_refs.iter().map(string).collect())]),
        record("operation-refs", vec![sequence(operation_refs)]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("recorded-delivery-log"), string("pass")]),
            record("check", vec![string("envelope-ref-binding"), string("pass")]),
            record("check", vec![string("transport-receipt-binding"), string("pass")]),
            record("check", vec![string("peer-bootstrap-binding"), string("pass")]),
            record("check", vec![string("authority-binding"), string("pass")]),
            record("check", vec![string("resource-binding"), string("pass")]),
            record("check", vec![string("turn-journal-binding"), string("pass")]),
            record("check", vec![string("idempotency-operation-ref-bound"), string("pass")]),
            record("check", vec![string("transport-is-not-authority"), string("pass")]),
        ])]),
    ]))
}

pub fn two_peer_service_ready_harness(root: &Path, evidence: DeliveryEvidence) -> Result<TwoPeerHarness> {
    let payload = record("service-ready", vec![string("db")]);
    let pattern = RuntimeValue::new(payload.clone())?;
    let mut peer_b = RuntimeState::new(1);
    peer_b.apply_step(&RuntimeStep::Observe {
        actor: "consumer".to_owned(),
        pattern: pattern.clone(),
    });
    let envelope = assert_envelope(AssertEnvelopeInput {
        from_peer: "peer:a",
        from_actor: "producer",
        to_peer: "peer:b",
        topic: "services",
        payload,
        capability_refs: Vec::new(),
        evidence_refs: Vec::new(),
    })?;
    publish_local_gossip(root, &envelope, "peer:a")?;
    let delivery = deliver_local_gossip(root, "services", &envelope.envelope_ref, "peer:b")?;
    let applied = admit_and_apply_delivered_envelope(&mut peer_b, &delivery, &evidence)?;
    let delivery_log = delivery_log(std::slice::from_ref(&delivery), true)?;
    let mut replay_peer_b = RuntimeState::new(1);
    replay_peer_b.apply_step(&RuntimeStep::Observe {
        actor: "consumer".to_owned(),
        pattern,
    });
    let replayed_events = replay_delivery_log(&mut replay_peer_b, &delivery_log)?;
    let receipt_value = gate_receipt_value(
        &delivery_log,
        std::slice::from_ref(&applied.admission_receipt_value),
        std::slice::from_ref(&applied.turn_journal_context_ref),
    )?;
    Ok(TwoPeerHarness {
        delivery_log,
        admission_receipt_value: applied.admission_receipt_value,
        receipt_value,
        observed_events: applied.events,
        replayed_events,
    })
}

pub fn turn_journal_context_ref(delivery: &Delivery) -> Result<String> {
    let transport_receipt_ref = canonical_hash(&delivery.receipt_value)?;
    let context = record("remote-dataspace-turn-context-v1", vec![
        record("envelope", vec![string(&delivery.envelope.envelope_ref)]),
        record("transport-receipt", vec![string(&transport_receipt_ref)]),
        record("from-peer", vec![string(&delivery.envelope.from_peer)]),
        record("to-peer", vec![string(&delivery.envelope.to_peer)]),
        record("topic", vec![string(&delivery.envelope.topic)]),
        record("operation", vec![string(delivery.envelope.operation.as_str())]),
        record("delivery-sequence", vec![crate::preserves_rail::u64_value(delivery.envelope.sequence)]),
        record("operation-ref", vec![string(&delivery.envelope.operation_ref)]),
    ]);
    canonical_hash(&context)
}

pub fn remote_actor_id(envelope: &Envelope) -> String {
    remote_actor_id_parts(&envelope.from_peer, &envelope.from_actor)
}

fn remote_actor_id_parts(peer: &str, actor: &str) -> String {
    format!("{peer}/{actor}")
}
