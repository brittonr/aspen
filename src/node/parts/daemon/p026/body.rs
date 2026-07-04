
fn live_receive_diagnostics(
    input: &ControlLiveIngressReceiveBytesInput<'_>,
    envelope: &ControlIngressEnvelope,
) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if envelope.transport != LIVE_CONTROL_INGRESS_TRANSPORT {
        diagnostics.push(format!(
            "node control live receive requires transport {LIVE_CONTROL_INGRESS_TRANSPORT}, got {}",
            envelope.transport
        ));
    }
    if envelope.topic != input.topic {
        diagnostics.push(format!(
            "node control live receive topic {} does not match subscribed topic {}",
            envelope.topic, input.topic
        ));
    }
    if envelope.to_node != input.receiver_node {
        diagnostics.push(format!(
            "node control live receive target {} does not match receiver {}",
            envelope.to_node, input.receiver_node
        ));
    }
    if envelope.peer_bootstrap_refs.is_empty() {
        diagnostics.push("node control live receive peer bootstrap refs missing".to_string());
    }
    diagnostics
}

pub fn parse_control_ingress_envelope(value: &IoValue) -> Result<ControlIngressEnvelope> {
    crate::preserves_rail::validate_boundary_schema(
        value,
        &crate::preserves_rail::NODE_CONTROL_INGRESS_BOUNDARY_SCHEMA,
    )?;
    let fields = value
        .collect_simple_record("node-control-ingress-envelope-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-ingress-envelope-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_INGRESS_ENVELOPE_SCHEMA,
        "node control ingress envelope",
    )?;
    let transport = record_string(&fields[1], "transport")?;
    let topic = record_string(&fields[2], "topic")?;
    let from_peer = record_string(&fields[3], "from-peer")?;
    let to_node = record_string(&fields[4], "to-node")?;
    let sequence = record_u64_string(&fields[5], "sequence")?;
    let operation_ref = record_ref_string(&fields[6], "operation")?;
    let request_ref = record_ref_string(&fields[7], "request-ref")?;
    let request_value = record_value(&fields[8], "request")?;
    let request = crate::node_runtime::parse_control_request(&request_value)?;
    if request.request_ref != request_ref {
        return Err(MoltenError::invalid_harness("node control ingress embedded request ref mismatch"));
    }
    let peer_bootstrap_refs = record_ref_strings(&fields[9], "peer-bootstrap")?;
    let authority_refs = record_ref_strings(&fields[10], "authority")?;
    let policy_refs = record_ref_strings(&fields[11], "policy")?;
    let resource_refs = record_ref_strings(&fields[12], "resource")?;
    let evidence_refs = record_ref_strings(&fields[13], "evidence")?;
    let expected_scope = crate::delivery_idempotency::remote_topic_scope_ref(&topic, &to_node)?;
    let expected_operation =
        crate::delivery_idempotency::derive_operation_id(crate::delivery_idempotency::OperationIdInput {
            scope_ref: expected_scope,
            producer: from_peer.clone(),
            consumer: to_node.clone(),
            sequence,
            intent: "node-control-ingress".to_string(),
            payload_ref: request.request_ref.clone(),
            policy_refs: policy_refs.clone(),
        })?;
    if expected_operation.operation_ref != operation_ref {
        return Err(MoltenError::invalid_harness("node control ingress operation ref mismatch"));
    }
    Ok(ControlIngressEnvelope {
        envelope_ref: crate::preserves_rail::canonical_hash(value)?,
        transport,
        topic,
        from_peer,
        to_node,
        sequence,
        operation_ref,
        request,
        peer_bootstrap_refs,
        authority_refs,
        policy_refs,
        resource_refs,
        evidence_refs,
        value: value.clone(),
    })
}

pub fn publish_control_ingress(input: &ControlIngressPublishInput<'_>) -> Result<ControlIngressPublish> {
    validate_state_root(input.state_root)?;
    ensure_state_layout(input.state_root)?;
    let envelope = parse_control_ingress_envelope(input.envelope_value)?;
    let envelope_path = control_ingress_envelope_path(input.state_root, &envelope.topic, &envelope.envelope_ref);
    write_ingress_envelope_and_verify(input.state_root, &envelope.topic, &envelope)?;
    import_artifact(input.state_root, &envelope.value)?;
    let diagnostics = Vec::new();
    let receipt_value = ingress_receipt_value(&IngressReceiptValueInput {
        decision: "pass",
        phase: "publish",
        transport: &envelope.transport,
        envelope: &envelope,
        idempotency_receipt_ref: None,
        queue_receipt_ref: None,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    write_preserves(
        &control_ingress_receipt_path(input.state_root, &envelope.envelope_ref, "publish"),
        &receipt_value,
    )?;
    import_artifact(input.state_root, &receipt_value)?;
    Ok(ControlIngressPublish {
        envelope_ref: envelope.envelope_ref,
        envelope_path,
        receipt_ref,
        receipt_value,
    })
}

#[derive(Debug, Default)]
struct EnqueueOutcome {
    idempotency_receipt_ref: Option<String>,
    queue_receipt_ref: Option<String>,
    has_enqueued: bool,
    diagnostics: Vec<String>,
}

fn apply_ingress_enqueue(state_root: &Path, envelope: &ControlIngressEnvelope) -> Result<EnqueueOutcome> {
    let idempotency_evidence_refs = ingress_idempotency_evidence_refs(envelope);
    let scope_ref = crate::delivery_idempotency::remote_topic_scope_ref(&envelope.topic, &envelope.to_node)?;
    let delivery = crate::delivery_idempotency::check(crate::delivery_idempotency::CheckInput {
        root: &state_root.join(CONTROL_IDEMPOTENCY_DIR),
        scope_profile: crate::delivery_idempotency::SCOPE_REMOTE_TOPIC,
        scope_ref: &scope_ref,
        producer: &envelope.from_peer,
        consumer: &envelope.to_node,
        sequence: envelope.sequence,
        intent: "node-control-ingress",
        payload_ref: &envelope.request.request_ref,
        policy_refs: &envelope.policy_refs,
        evidence_refs: &idempotency_evidence_refs,
        semantic_result_ref: Some(&envelope.request.request_ref),
        gap_policy: crate::delivery_idempotency::GapPolicy::Deny,
    })?;
    let idempotency_receipt_ref = Some(delivery.receipt.receipt_ref.clone());
    import_artifact(state_root, &delivery.receipt.value)?;
    if delivery.should_commit_side_effect {
        let submitted = submit_control_request(&ControlSubmitInput {
            state_root,
            request_value: &envelope.request.value,
        })?;
        return Ok(EnqueueOutcome {
            idempotency_receipt_ref,
            queue_receipt_ref: Some(submitted.queue_receipt_ref),
            has_enqueued: true,
            diagnostics: Vec::new(),
        });
    }
    if delivery.receipt.decision == "duplicate" {
        return Ok(EnqueueOutcome {
            idempotency_receipt_ref,
            queue_receipt_ref: prior_queue_receipt_ref(state_root, &envelope.request.request_ref).ok(),
            has_enqueued: false,
            diagnostics: Vec::new(),
        });
    }
    let mut diagnostics = delivery.receipt.diagnostics.clone();
    diagnostics.push(format!("node control ingress idempotency decision {}", delivery.receipt.decision));
    Ok(EnqueueOutcome {
        idempotency_receipt_ref,
        queue_receipt_ref: None,
        has_enqueued: false,
        diagnostics,
    })
}

pub fn deliver_control_ingress(input: &ControlIngressDeliverInput<'_>) -> Result<ControlIngressDeliver> {
    validate_state_root(input.state_root)?;
    validate_node_id(input.topic)?;
    validate_ingress_ref(input.envelope_ref, "node control ingress envelope ref")?;
    ensure_state_layout(input.state_root)?;
    let envelope_value =
        read_preserves(&control_ingress_envelope_path(input.state_root, input.topic, input.envelope_ref))?;
    let envelope = parse_control_ingress_envelope(&envelope_value)?;
    if envelope.envelope_ref != input.envelope_ref {
        return Err(MoltenError::invalid_harness(format!(
            "node control ingress materialized envelope ref {} does not match requested {}",
            envelope.envelope_ref, input.envelope_ref
        )));
    }
    let mut diagnostics = ingress_pre_enqueue_diagnostics(input.state_root, input.topic, &envelope)?;
    let mut enqueue = EnqueueOutcome::default();
    if diagnostics.is_empty() {
        enqueue = apply_ingress_enqueue(input.state_root, &envelope)?;
        diagnostics.append(&mut enqueue.diagnostics);
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = ingress_receipt_value(&IngressReceiptValueInput {
        decision,
        phase: if enqueue.has_enqueued {
            "deliver"
        } else {
            "duplicate-or-deny"
        },
        transport: &envelope.transport,
        envelope: &envelope,
        idempotency_receipt_ref: enqueue.idempotency_receipt_ref.as_deref(),
        queue_receipt_ref: enqueue.queue_receipt_ref.as_deref(),
        diagnostics: &diagnostics,
    })?;
    let ingress_receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    write_preserves(
        &control_ingress_receipt_path(input.state_root, &envelope.envelope_ref, "deliver"),
        &receipt_value,
    )?;
    import_artifact(input.state_root, &receipt_value)?;
    Ok(ControlIngressDeliver {
        envelope_ref: envelope.envelope_ref,
        request_ref: envelope.request.request_ref,
        ingress_receipt_ref,
        ingress_receipt_value: receipt_value,
        idempotency_receipt_ref: enqueue.idempotency_receipt_ref,
        queue_receipt_ref: enqueue.queue_receipt_ref,
        has_enqueued: enqueue.has_enqueued,
    })
}

fn ingress_pre_enqueue_diagnostics(
    state_root: &Path,
    topic: &str,
    envelope: &ControlIngressEnvelope,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if !matches!(envelope.transport.as_str(), LOCAL_CONTROL_INGRESS_TRANSPORT | LIVE_CONTROL_INGRESS_TRANSPORT) {
        diagnostics.push(format!("unsupported node control ingress transport {}", envelope.transport));
    }
    if envelope.topic != topic {
        diagnostics.push(format!("node control ingress topic {} does not match requested {topic}", envelope.topic));
    }
    let identity = crate::node_identity::parse_identity(&read_preserves(&state_root.join(IDENTITY_FILE))?)?;
    if envelope.to_node != identity.node_id {
        diagnostics
            .push(format!("node control ingress target {} does not match node {}", envelope.to_node, identity.node_id));
    }
    if envelope.peer_bootstrap_refs.is_empty() {
        diagnostics.push("node control ingress peer bootstrap refs missing".to_string());
    }
    if envelope.authority_refs.is_empty() || envelope.request.authority_refs.is_empty() {
        diagnostics.push("node control ingress authority refs missing".to_string());
    }
    if envelope.policy_refs.is_empty() || envelope.request.policy_refs.is_empty() {
        diagnostics.push("node control ingress policy refs missing".to_string());
    }
    if envelope.resource_refs.is_empty() || envelope.request.resource_refs.is_empty() {
        diagnostics.push("node control ingress resource refs missing".to_string());
    }
    if diagnostics.is_empty() && envelope.transport == LIVE_CONTROL_INGRESS_TRANSPORT {
        diagnostics.extend(evaluate_live_peer_bootstrap(state_root, envelope)?);
    }
    if diagnostics.is_empty() && envelope.transport == LIVE_CONTROL_INGRESS_TRANSPORT {
        diagnostics.extend(evaluate_live_authority_delegation(state_root, envelope)?);
    }
    Ok(diagnostics)
}

fn evaluate_live_peer_bootstrap(state_root: &Path, envelope: &ControlIngressEnvelope) -> Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(envelope.peer_bootstrap_refs.len().saturating_add(1));
    let mut admitted_peer_ref = None;
    for peer_ref in envelope.peer_bootstrap_refs.iter() {
        match read_ledger_artifact(state_root, peer_ref) {
            Ok(value) => match parse_control_live_peer_admission(&value) {
                Ok(admission) => {
                    let admission_diagnostics = live_peer_admission_diagnostics(state_root, envelope, &admission)?;
                    if admission_diagnostics.is_empty() {
                        admitted_peer_ref = Some(admission.admission_ref);
                        break;
                    }
                    diagnostics.extend(admission_diagnostics);
                }
                Err(error) => {
                    if let Some(diagnostic) = transport_evidence_not_authority_diagnostic(
                        &value,
                        peer_ref,
                        "node control live peer bootstrap ref",
                        "bootstrap authority",
                    ) {
                        diagnostics.push(diagnostic);
                    } else {
                        diagnostics.push(format!(
                            "node control live peer bootstrap ref {peer_ref} is not an admission: {error}"
                        ));
                    }
                }
            },
            Err(error) => diagnostics.push(format!("node control live peer bootstrap {peer_ref} not found: {error}")),
        }
    }
    if admitted_peer_ref.is_none() {
        diagnostics.push("node control live peer bootstrap missing admitted ticket".to_string());
    }
    Ok(diagnostics)
}
