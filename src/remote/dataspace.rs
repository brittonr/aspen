use std::fs;
use std::path::Path;
use std::path::PathBuf;

use preserves::IOValue;
use preserves::Value;

use crate::delivery_idempotency;
use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::REMOTE_DATASPACE_ADMISSION_RECEIPT_SCHEMA;
use crate::preserves_rail::REMOTE_DATASPACE_DELIVERY_LOG_SCHEMA;
use crate::preserves_rail::REMOTE_DATASPACE_ENVELOPE_SCHEMA;
use crate::preserves_rail::REMOTE_DATASPACE_GATE_RECEIPT_SCHEMA;
use crate::preserves_rail::REMOTE_DATASPACE_TRANSPORT_RECEIPT_SCHEMA;
use crate::preserves_rail::canonical_bytes;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::content_ref_from_bytes;
use crate::preserves_rail::content_ref_hex;
use crate::preserves_rail::parse_canonical_bytes;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::validate_content_ref;
use crate::preserves_rail::value_to_iovalue;
use crate::runtime::RuntimeEvent;
use crate::runtime::RuntimeState;
use crate::runtime::RuntimeStep;
use crate::runtime::RuntimeValue;

pub const LOCAL_GOSSIP_TRANSPORT: &str = "iroh-local-gossip";
pub const LIVE_GOSSIP_TRANSPORT: &str = "iroh-gossip";

const MAX_REMOTE_REPLAY_EVENTS: usize = 4_096;
const _: () = assert!(MAX_REMOTE_REPLAY_EVENTS > 0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteDataspaceOperation {
    Message,
    Assert,
    Retract,
    Observe,
}

impl RemoteDataspaceOperation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Message => "message",
            Self::Assert => "assert",
            Self::Retract => "retract",
            Self::Observe => "observe",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "message" => Ok(Self::Message),
            "assert" => Ok(Self::Assert),
            "retract" => Ok(Self::Retract),
            "observe" => Ok(Self::Observe),
            _ => Err(MoltenError::invalid_harness(format!("unsupported remote dataspace operation {value}"))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteDataspaceEnvelope {
    pub envelope_ref: String,
    pub from_peer: String,
    pub from_actor: String,
    pub to_peer: String,
    pub topic: String,
    pub operation: RemoteDataspaceOperation,
    pub payload: IOValue,
    pub content_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub sequence: u64,
    pub operation_ref: String,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteDataspaceExchange {
    pub envelope_ref: String,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteDataspaceDelivery {
    pub envelope: RemoteDataspaceEnvelope,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RemoteDeliveryEvidence {
    pub peer_bootstrap_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub authority_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteDataspaceApplied {
    pub events: Vec<RuntimeEvent>,
    pub admission_receipt_value: IOValue,
    pub turn_journal_context_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteDataspaceIdempotentApplied {
    pub events: Vec<RuntimeEvent>,
    pub admission_receipt_value: IOValue,
    pub turn_journal_context_ref: String,
    pub idempotency_receipt_value: IOValue,
    pub operation_ref: String,
    pub prior_semantic_result_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteDeliveryLog {
    pub log_ref: String,
    pub replayable: bool,
    pub entries: Vec<RemoteDataspaceDelivery>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteTwoPeerHarness {
    pub delivery_log: RemoteDeliveryLog,
    pub admission_receipt_value: IOValue,
    pub gate_receipt_value: IOValue,
    pub observed_events: Vec<RuntimeEvent>,
    pub replayed_events: Vec<RuntimeEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteDataspaceEnvelopeInput {
    pub from_peer: String,
    pub from_actor: String,
    pub to_peer: String,
    pub topic: String,
    pub operation: RemoteDataspaceOperation,
    pub payload: IOValue,
    pub content_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

pub struct AssertEnvelopeInput<'a> {
    pub from_peer: &'a str,
    pub from_actor: &'a str,
    pub to_peer: &'a str,
    pub topic: &'a str,
    pub payload: IOValue,
    pub capability_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

pub struct LocalTransportReceiptInput<'a> {
    pub operation: &'a str,
    pub decision: &'a str,
    pub node: &'a str,
    pub envelope: &'a RemoteDataspaceEnvelope,
    pub diagnostics: Vec<String>,
    pub checks: Vec<(String, String)>,
}

pub struct TransportReceiptInput<'a> {
    pub transport: &'a str,
    pub operation: &'a str,
    pub decision: &'a str,
    pub node: &'a str,
    pub envelope: &'a RemoteDataspaceEnvelope,
    pub diagnostics: Vec<String>,
    pub checks: Vec<(String, String)>,
}

struct AdmissionReceiptInput<'a> {
    decision: &'a str,
    envelope: &'a RemoteDataspaceEnvelope,
    transport_receipt_ref: &'a str,
    evidence: &'a RemoteDeliveryEvidence,
    turn_context_refs: &'a [String],
    diagnostics: Vec<String>,
}

pub fn build_envelope(input: RemoteDataspaceEnvelopeInput) -> Result<RemoteDataspaceEnvelope> {
    validate_name(&input.from_peer, "from peer")?;
    validate_name(&input.from_actor, "from actor")?;
    validate_name(&input.to_peer, "to peer")?;
    validate_name(&input.topic, "topic")?;
    validate_refs(&input.content_refs, "content ref")?;
    validate_refs(&input.capability_refs, "capability ref")?;
    validate_refs(&input.evidence_refs, "evidence ref")?;
    let value = envelope_value(&input)?;
    parse_envelope(&value)
}

pub fn assert_envelope(input: AssertEnvelopeInput<'_>) -> Result<RemoteDataspaceEnvelope> {
    build_envelope(RemoteDataspaceEnvelopeInput {
        from_peer: input.from_peer.to_owned(),
        from_actor: input.from_actor.to_owned(),
        to_peer: input.to_peer.to_owned(),
        topic: input.topic.to_owned(),
        operation: RemoteDataspaceOperation::Assert,
        payload: input.payload,
        content_refs: Vec::new(),
        capability_refs: input.capability_refs,
        evidence_refs: input.evidence_refs,
    })
}

pub fn parse_envelope(value: &IOValue) -> Result<RemoteDataspaceEnvelope> {
    let (fields, has_operation_ref) =
        if let Some(fields) = value.collect_simple_record("remote-dataspace-envelope-v1", Some(12)) {
            (fields, true)
        } else {
            (
                value
                    .collect_simple_record("remote-dataspace-envelope-v1", Some(10))
                    .ok_or_else(|| MoltenError::invalid_harness("expected <remote-dataspace-envelope-v1 ...>"))?,
                false,
            )
        };
    require_schema(&fields[0], REMOTE_DATASPACE_ENVELOPE_SCHEMA, "remote dataspace envelope schema")?;
    let from_peer = record_string(&fields[1], "from-peer")?;
    let from_actor = record_string(&fields[2], "from-actor")?;
    let to_peer = record_string(&fields[3], "to-peer")?;
    let topic = record_string(&fields[4], "topic")?;
    let operation = RemoteDataspaceOperation::parse(&record_string(&fields[5], "operation")?)?;
    let payload = record_iovalue(&fields[6], "payload")?;
    let content_refs = record_string_sequence(&fields[7], "content-refs")?;
    let capability_refs = record_string_sequence(&fields[8], "capability-refs")?;
    let evidence_refs = record_string_sequence(&fields[9], "evidence-refs")?;
    validate_name(&from_peer, "from peer")?;
    validate_name(&from_actor, "from actor")?;
    validate_name(&to_peer, "to peer")?;
    validate_name(&topic, "topic")?;
    validate_refs(&content_refs, "content ref")?;
    validate_refs(&capability_refs, "capability ref")?;
    validate_refs(&evidence_refs, "evidence ref")?;
    let sequence = if has_operation_ref {
        record_u64(&fields[10], "delivery-sequence")?
    } else {
        payload_delivery_sequence(&payload)?
    };
    let stored_ref = if has_operation_ref {
        Some(record_string(&fields[11], "operation-ref")?)
    } else {
        None
    };
    let operation_ref = parsed_ref(RefParts {
        stored_ref,
        sequence,
        from_peer: &from_peer,
        from_actor: &from_actor,
        to_peer: &to_peer,
        topic: &topic,
        operation,
        payload: &payload,
        capability_refs: &capability_refs,
        evidence_refs: &evidence_refs,
    })?;
    Ok(RemoteDataspaceEnvelope {
        envelope_ref: canonical_hash(value)?,
        from_peer,
        from_actor,
        to_peer,
        topic,
        operation,
        payload,
        content_refs,
        capability_refs,
        evidence_refs,
        sequence,
        operation_ref,
        value: value.clone(),
    })
}

struct RefParts<'a> {
    stored_ref: Option<String>,
    sequence: u64,
    from_peer: &'a str,
    from_actor: &'a str,
    to_peer: &'a str,
    topic: &'a str,
    operation: RemoteDataspaceOperation,
    payload: &'a IOValue,
    capability_refs: &'a [String],
    evidence_refs: &'a [String],
}

fn parsed_ref(input: RefParts<'_>) -> Result<String> {
    let operation_ref = if let Some(stored_ref) = input.stored_ref {
        stored_ref
    } else {
        envelope_operation_ref(EnvelopeOperationRefInput {
            from_peer: input.from_peer,
            from_actor: input.from_actor,
            to_peer: input.to_peer,
            topic: input.topic,
            operation: input.operation,
            payload: input.payload,
            capability_refs: input.capability_refs,
            evidence_refs: input.evidence_refs,
            sequence: input.sequence,
        })?
    };
    let expected_operation_ref = envelope_operation_ref(EnvelopeOperationRefInput {
        from_peer: input.from_peer,
        from_actor: input.from_actor,
        to_peer: input.to_peer,
        topic: input.topic,
        operation: input.operation,
        payload: input.payload,
        capability_refs: input.capability_refs,
        evidence_refs: input.evidence_refs,
        sequence: input.sequence,
    })?;
    if operation_ref != expected_operation_ref {
        return Err(MoltenError::invalid_harness(format!(
            "remote dataspace operation ref {operation_ref} does not match canonical ref {expected_operation_ref}"
        )));
    }
    Ok(operation_ref)
}

pub fn store_content_blob(root: &Path, bytes: &[u8]) -> Result<String> {
    fs::create_dir_all(root.join("blobs")).map_err(MoltenError::from)?;
    let content_ref = content_ref_from_bytes(bytes);
    fs::write(blob_path(root, &content_ref)?, bytes).map_err(MoltenError::from)?;
    Ok(content_ref)
}

pub fn publish_local_gossip(
    root: &Path,
    envelope: &RemoteDataspaceEnvelope,
    node: &str,
) -> Result<RemoteDataspaceExchange> {
    validate_name(node, "publisher node")?;
    validate_envelope_identity(envelope)?;
    validate_content_refs_available(root, &envelope.content_refs)?;
    let topic_dir = topic_dir(root, &envelope.topic);
    fs::create_dir_all(&topic_dir).map_err(MoltenError::from)?;
    fs::write(envelope_path(root, &envelope.topic, &envelope.envelope_ref)?, canonical_bytes(&envelope.value)?)
        .map_err(MoltenError::from)?;
    Ok(RemoteDataspaceExchange {
        envelope_ref: envelope.envelope_ref.clone(),
        receipt_value: transport_receipt_value_for_transport(TransportReceiptInput {
            transport: LOCAL_GOSSIP_TRANSPORT,
            operation: "publish",
            decision: "pass",
            node,
            envelope,
            diagnostics: Vec::new(),
            checks: vec![
                ("canonical-envelope-ref".to_owned(), "pass".to_owned()),
                ("content-refs-verified".to_owned(), "pass".to_owned()),
                ("transport-is-not-authority".to_owned(), "pass".to_owned()),
            ],
        }),
    })
}

pub async fn publish_live_gossip(
    sender: &iroh_gossip::api::GossipSender,
    envelope: &RemoteDataspaceEnvelope,
    node: &str,
) -> Result<RemoteDataspaceExchange> {
    validate_name(node, "publisher node")?;
    validate_envelope_identity(envelope)?;
    sender
        .broadcast(canonical_bytes(&envelope.value)?.into())
        .await
        .map_err(|error| MoltenError::invalid_harness(format!("live Iroh gossip publish failed: {error}")))?;
    Ok(RemoteDataspaceExchange {
        envelope_ref: envelope.envelope_ref.clone(),
        receipt_value: transport_receipt_value_for_transport(TransportReceiptInput {
            transport: LIVE_GOSSIP_TRANSPORT,
            operation: "publish",
            decision: "pass",
            node,
            envelope,
            diagnostics: Vec::new(),
            checks: vec![
                ("canonical-envelope-ref".to_owned(), "pass".to_owned()),
                ("live-iroh-gossip".to_owned(), "pass".to_owned()),
                ("transport-is-not-authority".to_owned(), "pass".to_owned()),
            ],
        }),
    })
}

pub fn deliver_live_gossip_event(
    root: &Path,
    event: &iroh_gossip::api::Event,
    topic: &str,
    receiver_peer: &str,
) -> Result<Option<RemoteDataspaceDelivery>> {
    match event {
        iroh_gossip::api::Event::Received(message) => deliver_live_gossip_bytes(
            root,
            message.content.as_ref(),
            topic,
            receiver_peer,
            &message.delivered_from.to_string(),
        )
        .map(Some),
        iroh_gossip::api::Event::NeighborUp(_)
        | iroh_gossip::api::Event::NeighborDown(_)
        | iroh_gossip::api::Event::Lagged => Ok(None),
    }
}

pub fn deliver_live_gossip_bytes(
    root: &Path,
    bytes: &[u8],
    topic: &str,
    receiver_peer: &str,
    delivered_from: &str,
) -> Result<RemoteDataspaceDelivery> {
    validate_name(topic, "topic")?;
    validate_name(receiver_peer, "receiver peer")?;
    validate_name(delivered_from, "delivered from")?;
    let value = parse_canonical_bytes(bytes)?;
    let envelope = parse_envelope(&value)?;
    if envelope.topic != topic {
        return Err(MoltenError::invalid_harness(format!(
            "live Iroh envelope topic {} does not match subscribed topic {topic}",
            envelope.topic
        )));
    }
    if envelope.to_peer != receiver_peer && envelope.to_peer != "*" {
        return Err(MoltenError::invalid_harness(format!(
            "live Iroh envelope target {} does not match receiver {receiver_peer}",
            envelope.to_peer
        )));
    }
    validate_content_refs_available(root, &envelope.content_refs)?;
    let receipt_value = transport_receipt_value_for_transport(TransportReceiptInput {
        transport: LIVE_GOSSIP_TRANSPORT,
        operation: "deliver",
        decision: "pass",
        node: receiver_peer,
        envelope: &envelope,
        diagnostics: Vec::new(),
        checks: vec![
            ("canonical-envelope-ref".to_owned(), "pass".to_owned()),
            ("topic-peer-binding".to_owned(), "pass".to_owned()),
            ("content-refs-verified".to_owned(), "pass".to_owned()),
            ("live-iroh-gossip".to_owned(), "pass".to_owned()),
            ("transport-is-not-authority".to_owned(), "pass".to_owned()),
        ],
    });
    Ok(RemoteDataspaceDelivery {
        envelope,
        receipt_value,
    })
}

pub fn deliver_local_gossip(
    root: &Path,
    topic: &str,
    envelope_ref: &str,
    receiver_peer: &str,
) -> Result<RemoteDataspaceDelivery> {
    validate_name(topic, "topic")?;
    validate_name(receiver_peer, "receiver peer")?;
    validate_ref(envelope_ref, "envelope ref")?;
    let bytes = fs::read(envelope_path(root, topic, envelope_ref)?).map_err(MoltenError::from)?;
    let value = parse_canonical_bytes(&bytes)?;
    let actual_ref = canonical_hash(&value)?;
    if actual_ref != envelope_ref {
        return Err(MoltenError::invalid_harness(format!(
            "remote dataspace envelope hashes to {actual_ref}, expected {envelope_ref}"
        )));
    }
    let envelope = parse_envelope(&value)?;
    if envelope.topic != topic {
        return Err(MoltenError::invalid_harness(format!(
            "remote dataspace envelope topic {} does not match requested topic {topic}",
            envelope.topic
        )));
    }
    if envelope.to_peer != receiver_peer && envelope.to_peer != "*" {
        return Err(MoltenError::invalid_harness(format!(
            "remote dataspace envelope target {} does not match receiver {receiver_peer}",
            envelope.to_peer
        )));
    }
    validate_content_refs_available(root, &envelope.content_refs)?;
    let receipt_value = transport_receipt_value_for_transport(TransportReceiptInput {
        transport: LOCAL_GOSSIP_TRANSPORT,
        operation: "deliver",
        decision: "pass",
        node: receiver_peer,
        envelope: &envelope,
        diagnostics: Vec::new(),
        checks: vec![
            ("canonical-envelope-ref".to_owned(), "pass".to_owned()),
            ("topic-peer-binding".to_owned(), "pass".to_owned()),
            ("content-refs-verified".to_owned(), "pass".to_owned()),
            ("transport-is-not-authority".to_owned(), "pass".to_owned()),
        ],
    });
    Ok(RemoteDataspaceDelivery {
        envelope,
        receipt_value,
    })
}

pub fn apply_delivered_envelope(
    state: &mut RuntimeState,
    envelope: &RemoteDataspaceEnvelope,
) -> Result<Vec<RuntimeEvent>> {
    validate_envelope_identity(envelope)?;
    let actor = remote_actor_id(envelope);
    let payload = RuntimeValue::new(envelope.payload.clone())?;
    let step = match envelope.operation {
        RemoteDataspaceOperation::Assert => RuntimeStep::Assert { actor, value: payload },
        RemoteDataspaceOperation::Retract => RuntimeStep::Retract { actor, value: payload },
        RemoteDataspaceOperation::Observe => RuntimeStep::Observe {
            actor,
            pattern: payload,
        },
        RemoteDataspaceOperation::Message => RuntimeStep::Send {
            from: actor,
            to: format!("{}:inbox", envelope.to_peer),
            body: payload,
        },
    };
    Ok(state.apply_step(&step))
}

pub fn admit_and_apply_delivered_envelope(
    state: &mut RuntimeState,
    delivery: &RemoteDataspaceDelivery,
    evidence: &RemoteDeliveryEvidence,
) -> Result<RemoteDataspaceApplied> {
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
    Ok(RemoteDataspaceApplied {
        events,
        admission_receipt_value,
        turn_journal_context_ref,
    })
}

pub fn admit_and_apply_delivered_envelope_idempotent(
    idempotency_root: &Path,
    state: &mut RuntimeState,
    delivery: &RemoteDataspaceDelivery,
    evidence: &RemoteDeliveryEvidence,
    gap_policy: delivery_idempotency::GapPolicy,
) -> Result<RemoteDataspaceIdempotentApplied> {
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
    let idempotency = delivery_idempotency::check_delivery(delivery_idempotency::DeliveryCheckInput {
        root: idempotency_root,
        scope_profile: delivery_idempotency::SCOPE_REMOTE_TOPIC,
        scope_ref: &delivery_idempotency::remote_topic_scope_ref(&delivery.envelope.topic, &delivery.envelope.to_peer)?,
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
    Ok(RemoteDataspaceIdempotentApplied {
        events,
        admission_receipt_value,
        turn_journal_context_ref,
        idempotency_receipt_value: idempotency.receipt.value,
        operation_ref: idempotency.operation.operation_ref,
        prior_semantic_result_ref: idempotency.prior_semantic_result_ref,
    })
}

pub fn deny_admission_receipt_value(
    envelope: &RemoteDataspaceEnvelope,
    transport_receipt_ref: &str,
    diagnostics: Vec<String>,
) -> IOValue {
    remote_admission_receipt_value(AdmissionReceiptInput {
        decision: "deny",
        envelope,
        transport_receipt_ref,
        evidence: &RemoteDeliveryEvidence::default(),
        turn_context_refs: &[],
        diagnostics,
    })
}

pub fn parse_delivery_log(value: &IOValue) -> Result<RemoteDeliveryLog> {
    let fields = value
        .collect_simple_record("remote-dataspace-delivery-log-v1", Some(4))
        .ok_or_else(|| MoltenError::invalid_harness("expected <remote-dataspace-delivery-log-v1 ...>"))?;
    require_schema(&fields[0], REMOTE_DATASPACE_DELIVERY_LOG_SCHEMA, "remote dataspace delivery log schema")?;
    let is_replayable = record_bool(&fields[1], "replayable")?;
    let entry_values = field_sequence(&fields[2], "entries")?;
    let entries = entry_values.iter().map(parse_delivery_log_entry).collect::<Result<Vec<_>>>()?;
    Ok(RemoteDeliveryLog {
        log_ref: canonical_hash(value)?,
        replayable: is_replayable,
        entries,
        value: value.clone(),
    })
}

pub fn delivery_log(deliveries: &[RemoteDataspaceDelivery], replayable: bool) -> Result<RemoteDeliveryLog> {
    delivery_log_with_idempotency_receipts(deliveries, &[], replayable)
}

pub fn delivery_log_with_idempotency_receipts(
    deliveries: &[RemoteDataspaceDelivery],
    idempotency_receipts: &[IOValue],
    replayable: bool,
) -> Result<RemoteDeliveryLog> {
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
            let parsed = delivery_idempotency::parse_idempotency_receipt(receipt)?;
            if parsed.operation_ref != delivery.envelope.operation_ref {
                return Err(MoltenError::invalid_harness("remote delivery log idempotency operation ref mismatch"));
            }
            fields.push(record("idempotency-receipt", vec![receipt.clone()]));
        }
        entries.push(record("entry", fields));
    }
    let idempotency_status = if idempotency_receipts.is_empty() { "n/a" } else { "pass" };
    let value = record("remote-dataspace-delivery-log-v1", vec![
        string(REMOTE_DATASPACE_DELIVERY_LOG_SCHEMA),
        record("replayable", vec![crate::preserves_rail::bool_value(replayable)]),
        record("entries", vec![sequence(entries)]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("recorded-canonical-envelopes"), string("pass")]),
            record("check", vec![string("idempotency-operation-ref-bound"), string("pass")]),
            record("check", vec![string("idempotency-receipt-bound"), string(idempotency_status)]),
            record("check", vec![string("no-live-network-during-replay"), string("pass")]),
        ])]),
    ]);
    Ok(RemoteDeliveryLog {
        log_ref: canonical_hash(&value)?,
        replayable,
        entries: deliveries.to_vec(),
        value,
    })
}

pub fn replay_delivery_log(state: &mut RuntimeState, log: &RemoteDeliveryLog) -> Result<Vec<RuntimeEvent>> {
    if !log.replayable {
        return Err(MoltenError::invalid_harness(
            "remote dataspace delivery log is non-replayable and cannot satisfy deterministic replay",
        ));
    }
    ensure_count_at_most(log.entries.len(), MAX_REMOTE_REPLAY_EVENTS, "remote replay deliveries")?;
    let mut events = Vec::with_capacity(log.entries.len());
    for delivery in &log.entries {
        let delivered = apply_delivered_envelope(state, &delivery.envelope)?;
        extend_bounded(&mut events, delivered, MAX_REMOTE_REPLAY_EVENTS, "remote replay events")?;
    }
    Ok(events)
}

pub fn remote_dataspace_gate_receipt_value(
    delivery_log: &RemoteDeliveryLog,
    admission_receipts: &[IOValue],
    turn_context_refs: &[String],
) -> Result<IOValue> {
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
        string(REMOTE_DATASPACE_GATE_RECEIPT_SCHEMA),
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

pub fn two_peer_service_ready_harness(root: &Path, evidence: RemoteDeliveryEvidence) -> Result<RemoteTwoPeerHarness> {
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
    let gate_receipt_value = remote_dataspace_gate_receipt_value(
        &delivery_log,
        std::slice::from_ref(&applied.admission_receipt_value),
        std::slice::from_ref(&applied.turn_journal_context_ref),
    )?;
    Ok(RemoteTwoPeerHarness {
        delivery_log,
        admission_receipt_value: applied.admission_receipt_value,
        gate_receipt_value,
        observed_events: applied.events,
        replayed_events,
    })
}

pub fn turn_journal_context_ref(delivery: &RemoteDataspaceDelivery) -> Result<String> {
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

pub fn remote_actor_id(envelope: &RemoteDataspaceEnvelope) -> String {
    remote_actor_id_parts(&envelope.from_peer, &envelope.from_actor)
}

fn remote_actor_id_parts(peer: &str, actor: &str) -> String {
    format!("{peer}/{actor}")
}

pub fn transport_receipt_value(input: LocalTransportReceiptInput<'_>) -> IOValue {
    transport_receipt_value_for_transport(TransportReceiptInput {
        transport: LOCAL_GOSSIP_TRANSPORT,
        operation: input.operation,
        decision: input.decision,
        node: input.node,
        envelope: input.envelope,
        diagnostics: input.diagnostics,
        checks: input.checks,
    })
}

pub fn transport_receipt_value_for_transport(input: TransportReceiptInput<'_>) -> IOValue {
    record("remote-dataspace-transport-receipt-v1", vec![
        string(REMOTE_DATASPACE_TRANSPORT_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("transport", vec![string(input.transport)]),
        record("envelope", vec![string(&input.envelope.envelope_ref)]),
        record("node", vec![string(input.node)]),
        record("from-peer", vec![string(&input.envelope.from_peer)]),
        record("to-peer", vec![string(&input.envelope.to_peer)]),
        record("topic", vec![string(&input.envelope.topic)]),
        record("delivery-sequence", vec![crate::preserves_rail::u64_value(input.envelope.sequence)]),
        record("operation-ref", vec![string(&input.envelope.operation_ref)]),
        record("content-refs", vec![sequence(input.envelope.content_refs.iter().map(string).collect())]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(
            input
                .checks
                .iter()
                .map(|(name, status)| record("check", vec![string(name), string(status)]))
                .collect(),
        )]),
    ])
}

fn remote_admission_receipt_value(input: AdmissionReceiptInput<'_>) -> IOValue {
    record("remote-dataspace-admission-receipt-v1", vec![
        string(REMOTE_DATASPACE_ADMISSION_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("envelope", vec![string(&input.envelope.envelope_ref)]),
        record("transport-receipt", vec![string(input.transport_receipt_ref)]),
        record("operation-ref", vec![string(&input.envelope.operation_ref)]),
        record("peer-bootstrap-refs", vec![sequence(
            input.evidence.peer_bootstrap_refs.iter().map(string).collect(),
        )]),
        record("capability-refs", vec![sequence(input.evidence.capability_refs.iter().map(string).collect())]),
        record("policy-refs", vec![sequence(input.evidence.policy_refs.iter().map(string).collect())]),
        record("resource-refs", vec![sequence(input.evidence.resource_refs.iter().map(string).collect())]),
        record("authority-refs", vec![sequence(input.evidence.authority_refs.iter().map(string).collect())]),
        record("turn-journal-context-refs", vec![sequence(input.turn_context_refs.iter().map(string).collect())]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("peer-bootstrap-binding"), string(input.decision)]),
            record("check", vec![string("capability-policy-binding"), string(input.decision)]),
            record("check", vec![string("resource-binding"), string(input.decision)]),
            record("check", vec![string("authority-binding"), string(input.decision)]),
            record("check", vec![string("transport-is-not-authority"), string("pass")]),
        ])]),
    ])
}

fn validate_delivery_evidence(envelope: &RemoteDataspaceEnvelope, evidence: &RemoteDeliveryEvidence) -> Result<()> {
    require_non_empty_refs(&evidence.peer_bootstrap_refs, "peer bootstrap ref")?;
    require_non_empty_refs(&evidence.capability_refs, "capability ref")?;
    require_non_empty_refs(&evidence.policy_refs, "policy ref")?;
    require_non_empty_refs(&evidence.resource_refs, "resource ref")?;
    require_non_empty_refs(&evidence.authority_refs, "authority ref")?;
    for capability_ref in &envelope.capability_refs {
        if !evidence.capability_refs.contains(capability_ref) {
            return Err(MoltenError::invalid_harness(format!(
                "remote dataspace capability evidence missing declared capability {capability_ref}"
            )));
        }
    }
    let evidence_refs: Vec<&String> = evidence
        .peer_bootstrap_refs
        .iter()
        .chain(evidence.policy_refs.iter())
        .chain(evidence.resource_refs.iter())
        .chain(evidence.authority_refs.iter())
        .collect();
    for evidence_ref in &envelope.evidence_refs {
        if !evidence_refs.contains(&evidence_ref) {
            return Err(MoltenError::invalid_harness(format!(
                "remote dataspace admission evidence missing declared evidence ref {evidence_ref}"
            )));
        }
    }
    Ok(())
}

fn require_non_empty_refs(refs: &[String], label: &str) -> Result<()> {
    if refs.is_empty() {
        return Err(MoltenError::invalid_harness(format!("missing remote dataspace {label}")));
    }
    validate_refs(refs, label)
}

struct EnvelopeOperationRefInput<'a> {
    from_peer: &'a str,
    from_actor: &'a str,
    to_peer: &'a str,
    topic: &'a str,
    operation: RemoteDataspaceOperation,
    payload: &'a IOValue,
    capability_refs: &'a [String],
    evidence_refs: &'a [String],
    sequence: u64,
}

fn envelope_operation_ref(input: EnvelopeOperationRefInput<'_>) -> Result<String> {
    let scope_ref = delivery_idempotency::remote_topic_scope_ref(input.topic, input.to_peer)?;
    let operation = delivery_idempotency::derive_operation_id(delivery_idempotency::OperationIdInput {
        scope_ref,
        producer: remote_actor_id_parts(input.from_peer, input.from_actor),
        consumer: input.to_peer.to_owned(),
        sequence: input.sequence,
        intent: format!("remote-dataspace-{}", input.operation.as_str()),
        payload_ref: canonical_hash(input.payload)?,
        policy_refs: envelope_policy_refs(input.capability_refs, input.evidence_refs)?,
    })?;
    Ok(operation.operation_ref)
}

fn envelope_policy_refs(capability_refs: &[String], evidence_refs: &[String]) -> Result<Vec<String>> {
    let total = capability_refs
        .len()
        .checked_add(evidence_refs.len())
        .ok_or_else(|| MoltenError::invalid_harness("remote dataspace policy ref count overflow"))?;
    ensure_count_at_most(total, MAX_REMOTE_REPLAY_EVENTS, "remote dataspace operation policy refs")?;
    let mut refs = Vec::with_capacity(total);
    refs.extend(capability_refs.iter().cloned());
    refs.extend(evidence_refs.iter().cloned());
    refs.sort();
    refs.dedup();
    validate_refs(&refs, "operation policy ref")?;
    Ok(refs)
}

fn payload_delivery_sequence(payload: &IOValue) -> Result<u64> {
    if let Some(fields) = payload.collect_simple_record("protocol-message-v1", Some(11)) {
        return record_u64(&fields[8], "sequence");
    }
    Ok(1)
}

fn envelope_value(input: &RemoteDataspaceEnvelopeInput) -> Result<IOValue> {
    let delivery_sequence = payload_delivery_sequence(&input.payload)?;
    let operation_ref = envelope_operation_ref(EnvelopeOperationRefInput {
        from_peer: &input.from_peer,
        from_actor: &input.from_actor,
        to_peer: &input.to_peer,
        topic: &input.topic,
        operation: input.operation,
        payload: &input.payload,
        capability_refs: &input.capability_refs,
        evidence_refs: &input.evidence_refs,
        sequence: delivery_sequence,
    })?;
    Ok(record("remote-dataspace-envelope-v1", vec![
        string(REMOTE_DATASPACE_ENVELOPE_SCHEMA),
        record("from-peer", vec![string(&input.from_peer)]),
        record("from-actor", vec![string(&input.from_actor)]),
        record("to-peer", vec![string(&input.to_peer)]),
        record("topic", vec![string(&input.topic)]),
        record("operation", vec![string(input.operation.as_str())]),
        record("payload", vec![input.payload.clone()]),
        record("content-refs", vec![sequence(input.content_refs.iter().map(string).collect())]),
        record("capability-refs", vec![sequence(input.capability_refs.iter().map(string).collect())]),
        record("evidence-refs", vec![sequence(input.evidence_refs.iter().map(string).collect())]),
        record("delivery-sequence", vec![crate::preserves_rail::u64_value(delivery_sequence)]),
        record("operation-ref", vec![string(&operation_ref)]),
    ]))
}

fn validate_envelope_identity(envelope: &RemoteDataspaceEnvelope) -> Result<()> {
    let actual_ref = canonical_hash(&envelope.value)?;
    if actual_ref != envelope.envelope_ref {
        return Err(MoltenError::invalid_harness(format!(
            "remote dataspace envelope ref {} does not match canonical ref {actual_ref}",
            envelope.envelope_ref
        )));
    }
    Ok(())
}

fn validate_content_refs_available(root: &Path, refs: &[String]) -> Result<()> {
    for reference in refs {
        validate_ref(reference, "content ref")?;
        let bytes = fs::read(blob_path(root, reference)?).map_err(MoltenError::from)?;
        let actual_ref = content_ref_from_bytes(&bytes);
        if actual_ref != *reference {
            return Err(MoltenError::invalid_harness(format!(
                "remote dataspace content ref {reference} hashes to {actual_ref}"
            )));
        }
    }
    Ok(())
}

fn ensure_count_at_most(actual: usize, maximum: usize, label: &str) -> Result<()> {
    if actual <= maximum {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("{label} count {actual} exceeds bound {maximum}")))
}

fn extend_bounded<T>(
    values: &mut impl crate::bounded::VecSink<T>,
    incoming: Vec<T>,
    maximum: usize,
    label: &str,
) -> Result<()> {
    let total = values
        .item_count()
        .checked_add(incoming.len())
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    ensure_count_at_most(total, maximum, label)?;
    for value in incoming {
        values.push_item(value);
    }
    Ok(())
}

fn validate_refs(refs: &[String], label: &str) -> Result<()> {
    for reference in refs {
        validate_ref(reference, label)?;
    }
    Ok(())
}

fn validate_ref(reference: &str, label: &str) -> Result<()> {
    validate_content_ref(reference).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "unsupported {label} {reference}; expected canonical content ref: {error}"
        ))
    })
}

fn validate_name(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() || value.contains('\0') || value.contains('/') {
        return Err(MoltenError::invalid_harness(format!("invalid remote dataspace {field} {value:?}")));
    }
    Ok(())
}

fn blob_path(root: &Path, reference: &str) -> Result<PathBuf> {
    Ok(root.join("blobs").join(filename_for_ref(reference)?))
}

fn envelope_path(root: &Path, topic: &str, envelope_ref: &str) -> Result<PathBuf> {
    Ok(topic_dir(root, topic).join(filename_for_ref(envelope_ref)?))
}

fn topic_dir(root: &Path, topic: &str) -> PathBuf {
    let topic_hash = blake3::hash(topic.as_bytes()).to_hex().to_string();
    root.join("gossip").join(format!("topic_{topic_hash}"))
}

fn filename_for_ref(reference: &str) -> Result<String> {
    validate_ref(reference, "local materialized ref")?;
    let hex = content_ref_hex(reference)?;
    Ok(format!("blake3_{hex}.bin"))
}

fn parse_delivery_log_entry(value: &Value<IOValue>) -> Result<RemoteDataspaceDelivery> {
    let value = value_to_iovalue(value);
    let (fields, has_operation_ref, has_idempotency_receipt) =
        if let Some(fields) = value.collect_simple_record("entry", Some(5)) {
            (fields, true, true)
        } else if let Some(fields) = value.collect_simple_record("entry", Some(4)) {
            (fields, true, false)
        } else {
            (
                value
                    .collect_simple_record("entry", Some(3))
                    .ok_or_else(|| MoltenError::invalid_harness("expected remote dataspace delivery log entry"))?,
                false,
                false,
            )
        };
    let _index = fields[0]
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness("expected u64 delivery log entry index"))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for delivery log entry: {error}")))?;
    let envelope_value = record_iovalue(&fields[1], "envelope")?;
    let receipt_value = record_iovalue(&fields[2], "transport-receipt")?;
    let envelope = parse_envelope(&envelope_value)?;
    if has_operation_ref {
        let operation_ref = record_string(&fields[3], "operation-ref")?;
        if operation_ref != envelope.operation_ref {
            return Err(MoltenError::invalid_harness("remote delivery log operation ref mismatch"));
        }
    }
    if has_idempotency_receipt {
        let receipt_value = record_iovalue(&fields[4], "idempotency-receipt")?;
        let receipt = delivery_idempotency::parse_idempotency_receipt(&receipt_value)?;
        if receipt.operation_ref != envelope.operation_ref {
            return Err(MoltenError::invalid_harness("remote delivery log idempotency receipt mismatch"));
        }
    }
    Ok(RemoteDataspaceDelivery {
        envelope,
        receipt_value,
    })
}

fn record_iovalue(value: &Value<IOValue>, label: &str) -> Result<IOValue> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    Ok(value_to_iovalue(&fields[0]))
}

fn record_bool(value: &Value<IOValue>, label: &str) -> Result<bool> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    fields[0]
        .as_boolean()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected bool for {label}")))
}

fn record_u64(value: &Value<IOValue>, label: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    fields[0]
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {label}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {label}: {error}")))
}

fn record_string(value: &Value<IOValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    required_string(&fields[0], label)
}

fn record_string_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    field_sequence(value, label)?.iter().map(|value| required_string(value, label)).collect()
}

fn field_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<Value<IOValue>>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    let values = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    Ok(values.iter().cloned().collect())
}

fn require_schema(value: &Value<IOValue>, expected: &str, field: &str) -> Result<()> {
    let actual = required_string(value, field)?;
    if actual != expected {
        return Err(MoltenError::invalid_harness(format!("expected {field} {expected}, got {actual}")));
    }
    Ok(())
}

fn required_string(value: &Value<IOValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;

    use super::*;
    use crate::preserves_rail::record;
    use crate::preserves_rail::string;
    use crate::runtime::RuntimeEvent;
    use crate::runtime::RuntimeStep;

    #[test]
    fn local_gossip_roundtrip_preserves_envelope_identity() {
        let root = temp_dir("remote-dataspace-roundtrip");
        let payload = record("service-ready", vec![string("db")]);
        let envelope = assert_envelope(AssertEnvelopeInput {
            from_peer: "peer:a",
            from_actor: "producer",
            to_peer: "peer:b",
            topic: "services",
            payload,
            capability_refs: Vec::new(),
            evidence_refs: Vec::new(),
        })
        .expect("envelope");
        let published = publish_local_gossip(&root, &envelope, "peer:a").expect("publish");
        assert_eq!(published.envelope_ref, envelope.envelope_ref);
        let delivered = deliver_local_gossip(&root, "services", &envelope.envelope_ref, "peer:b").expect("deliver");
        assert_eq!(delivered.envelope.envelope_ref, envelope.envelope_ref);
        assert_eq!(delivered.envelope.topic, "services");
        let receipt_ref = crate::preserves_rail::canonical_hash(&delivered.receipt_value).expect("receipt ref");
        crate::preserves_rail::validate_content_ref(&receipt_ref).expect("receipt ref is canonical");
    }

    #[test]
    fn remote_assertion_applies_through_local_observer_semantics() {
        let root = temp_dir("remote-dataspace-observe");
        let payload_value = record("service-ready", vec![string("db")]);
        let pattern = RuntimeValue::new(payload_value.clone()).expect("runtime value");
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
            payload: payload_value,
            capability_refs: Vec::new(),
            evidence_refs: Vec::new(),
        })
        .expect("envelope");
        publish_local_gossip(&root, &envelope, "peer:a").expect("publish");
        let delivered = deliver_local_gossip(&root, "services", &envelope.envelope_ref, "peer:b").expect("deliver");
        let events = apply_delivered_envelope(&mut peer_b, &delivered.envelope).expect("apply delivered envelope");
        assert!(events.iter().any(|event| matches!(event, RuntimeEvent::AssertionCommitted { actor, value }
            if actor == "peer:a/producer" && value == &pattern)));
        assert!(events.iter().any(|event| matches!(event, RuntimeEvent::AssertionObserved { observer, owner, value }
            if observer == "consumer" && owner == "peer:a/producer" && value == &pattern)));
    }

    #[test]
    fn missing_or_tampered_content_ref_is_rejected_before_delivery() {
        let root = temp_dir("remote-dataspace-content-ref");
        let content_ref = store_content_blob(&root, b"large payload").expect("store content");
        let envelope = build_envelope(RemoteDataspaceEnvelopeInput {
            from_peer: "peer:a".to_owned(),
            from_actor: "producer".to_owned(),
            to_peer: "peer:b".to_owned(),
            topic: "services".to_owned(),
            operation: RemoteDataspaceOperation::Assert,
            payload: record("content-ref", vec![string(&content_ref)]),
            content_refs: vec![content_ref.clone()],
            capability_refs: Vec::new(),
            evidence_refs: Vec::new(),
        })
        .expect("envelope");
        publish_local_gossip(&root, &envelope, "peer:a").expect("publish with valid content");
        fs::write(blob_path(&root, &content_ref).expect("blob path"), b"tampered").expect("tamper blob");
        let error = deliver_local_gossip(&root, "services", &envelope.envelope_ref, "peer:b")
            .expect_err("tampered content rejects delivery");
        assert!(error.to_string().contains("content ref"));
    }

    #[test]
    fn remote_dataspace_refs_reject_malformed_content_refs() {
        for reference in [
            "blake3:short",
            "blake3:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "blake3:zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
        ] {
            let error = validate_ref(reference, "remote regression ref").expect_err("malformed ref must fail closed");
            assert!(error.to_string().contains("canonical content ref"));
        }
    }

    #[test]
    fn admitted_remote_delivery_binds_bootstrap_capability_resource_policy_and_turn_context() {
        let root = temp_dir("remote-dataspace-admission");
        let payload = record("service-ready", vec![string("db")]);
        let envelope = assert_envelope(AssertEnvelopeInput {
            from_peer: "peer:a",
            from_actor: "producer",
            to_peer: "peer:b",
            topic: "services",
            payload,
            capability_refs: Vec::new(),
            evidence_refs: Vec::new(),
        })
        .expect("envelope");
        publish_local_gossip(&root, &envelope, "peer:a").expect("publish");
        let delivery = deliver_local_gossip(&root, "services", &envelope.envelope_ref, "peer:b").expect("deliver");
        let evidence = evidence_fixture();
        let mut state = RuntimeState::new(1);
        let applied = admit_and_apply_delivered_envelope(&mut state, &delivery, &evidence).expect("admit and apply");
        assert!(!applied.events.is_empty());
        crate::preserves_rail::validate_content_ref(&applied.turn_journal_context_ref)
            .expect("turn journal context ref is canonical");
        assert_eq!(
            crate::ledger::artifact_kind(&applied.admission_receipt_value),
            "remote-dataspace-admission-receipt"
        );
        let missing = admit_and_apply_delivered_envelope(
            &mut RuntimeState::new(1),
            &delivery,
            &RemoteDeliveryEvidence::default(),
        )
        .expect_err("missing evidence denies before applying");
        assert!(missing.to_string().contains("peer bootstrap"));
    }

    #[test]
    fn idempotent_remote_delivery_suppresses_duplicate_and_denies_conflict_before_commit() {
        let root = temp_dir("remote-dataspace-idempotency");
        let payload = record("service-ready", vec![string("db")]);
        let envelope = assert_envelope(AssertEnvelopeInput {
            from_peer: "peer:a",
            from_actor: "producer",
            to_peer: "peer:b",
            topic: "services",
            payload,
            capability_refs: Vec::new(),
            evidence_refs: Vec::new(),
        })
        .expect("envelope");
        publish_local_gossip(&root, &envelope, "peer:a").expect("publish");
        let delivery = deliver_local_gossip(&root, "services", &envelope.envelope_ref, "peer:b").expect("deliver");
        let evidence = evidence_fixture();
        let mut state = RuntimeState::new(1);
        let first = admit_and_apply_delivered_envelope_idempotent(
            &root,
            &mut state,
            &delivery,
            &evidence,
            delivery_idempotency::GapPolicy::Deny,
        )
        .expect("first idempotent apply");
        assert!(!first.events.is_empty());
        assert_eq!(crate::ledger::artifact_kind(&first.idempotency_receipt_value), "delivery-idempotency-receipt");
        let duplicate = admit_and_apply_delivered_envelope_idempotent(
            &root,
            &mut state,
            &delivery,
            &evidence,
            delivery_idempotency::GapPolicy::Deny,
        )
        .expect("duplicate idempotent apply");
        assert!(duplicate.events.is_empty());
        let first_admission_ref = canonical_hash(&first.admission_receipt_value).expect("first admission ref");
        assert_eq!(duplicate.prior_semantic_result_ref.as_deref(), Some(first_admission_ref.as_str()));
        let log = delivery_log_with_idempotency_receipts(
            std::slice::from_ref(&delivery),
            std::slice::from_ref(&first.idempotency_receipt_value),
            true,
        )
        .expect("idempotent delivery log");
        assert!(crate::preserves_rail::to_text(&log.value).expect("log text").contains("idempotency-receipt"));
        let changed = assert_envelope(AssertEnvelopeInput {
            from_peer: "peer:a",
            from_actor: "producer",
            to_peer: "peer:b",
            topic: "services",
            payload: record("service-ready", vec![string("api")]),
            capability_refs: Vec::new(),
            evidence_refs: Vec::new(),
        })
        .expect("changed envelope");
        publish_local_gossip(&root, &changed, "peer:a").expect("publish changed");
        let changed_delivery =
            deliver_local_gossip(&root, "services", &changed.envelope_ref, "peer:b").expect("deliver changed");
        let conflict = admit_and_apply_delivered_envelope_idempotent(
            &root,
            &mut state,
            &changed_delivery,
            &evidence,
            delivery_idempotency::GapPolicy::Deny,
        )
        .expect("conflict receipt");
        assert!(conflict.events.is_empty());
        assert!(
            crate::preserves_rail::to_text(&conflict.idempotency_receipt_value)
                .expect("conflict text")
                .contains("conflict")
        );
    }

    #[test]
    fn recorded_delivery_log_replays_without_live_transport() {
        let root = temp_dir("remote-dataspace-delivery-log");
        let payload = record("service-ready", vec![string("db")]);
        let envelope = assert_envelope(AssertEnvelopeInput {
            from_peer: "peer:a",
            from_actor: "producer",
            to_peer: "peer:b",
            topic: "services",
            payload,
            capability_refs: Vec::new(),
            evidence_refs: Vec::new(),
        })
        .expect("envelope");
        publish_local_gossip(&root, &envelope, "peer:a").expect("publish");
        let delivery = deliver_local_gossip(&root, "services", &envelope.envelope_ref, "peer:b").expect("deliver");
        let log = delivery_log(std::slice::from_ref(&delivery), true).expect("delivery log");
        assert_eq!(crate::ledger::artifact_kind(&log.value), "remote-dataspace-delivery-log");
        fs::remove_dir_all(root.join("gossip")).expect("remove live transport bytes");
        let mut state = RuntimeState::new(1);
        let events = replay_delivery_log(&mut state, &log).expect("replay from recorded log");
        assert!(events.iter().any(|event| matches!(event, RuntimeEvent::AssertionCommitted { .. })));
        let non_replayable = delivery_log(&[delivery], false).expect("non replayable log");
        let error = replay_delivery_log(&mut RuntimeState::new(1), &non_replayable)
            .expect_err("non replayable live run excluded");
        assert!(error.to_string().contains("non-replayable"));
    }

    #[test]
    fn live_gossip_bytes_use_same_receipt_boundary() {
        let root = temp_dir("remote-dataspace-live-bytes");
        let envelope = assert_envelope(AssertEnvelopeInput {
            from_peer: "peer:a",
            from_actor: "producer",
            to_peer: "peer:b",
            topic: "services",
            payload: record("service-ready", vec![string("db")]),
            capability_refs: Vec::new(),
            evidence_refs: Vec::new(),
        })
        .expect("envelope");
        let bytes = canonical_bytes(&envelope.value).expect("envelope bytes");
        let delivered =
            deliver_live_gossip_bytes(&root, &bytes, "services", "peer:b", "endpoint:a").expect("deliver live bytes");
        assert_eq!(delivered.envelope.envelope_ref, envelope.envelope_ref);
        assert_eq!(crate::ledger::artifact_kind(&delivered.receipt_value), "remote-dataspace-transport-receipt");
    }

    #[test]
    fn two_peer_harness_records_replay_and_gate_receipt() {
        let root = temp_dir("remote-dataspace-two-peer-harness");
        let harness = two_peer_service_ready_harness(&root, evidence_fixture()).expect("two peer harness");
        assert!(harness.observed_events.iter().any(|event| matches!(event, RuntimeEvent::AssertionObserved { .. })));
        assert!(harness.replayed_events.iter().any(|event| matches!(event, RuntimeEvent::AssertionObserved { .. })));
        assert_eq!(crate::ledger::artifact_kind(&harness.gate_receipt_value), "remote-dataspace-gate-receipt");
    }

    #[test]
    fn wrong_topic_wrong_peer_and_tampered_envelope_are_rejected() {
        let root = temp_dir("remote-dataspace-negative-routing");
        let envelope = assert_envelope(AssertEnvelopeInput {
            from_peer: "peer:a",
            from_actor: "producer",
            to_peer: "peer:b",
            topic: "services",
            payload: record("service-ready", vec![string("db")]),
            capability_refs: Vec::new(),
            evidence_refs: Vec::new(),
        })
        .expect("envelope");
        publish_local_gossip(&root, &envelope, "peer:a").expect("publish");
        let wrong_topic = deliver_local_gossip(&root, "other", &envelope.envelope_ref, "peer:b")
            .expect_err("wrong topic has no stored envelope");
        assert!(wrong_topic.to_string().contains("io error"));
        let wrong_peer =
            deliver_local_gossip(&root, "services", &envelope.envelope_ref, "peer:c").expect_err("wrong peer rejects");
        assert!(wrong_peer.to_string().contains("target"));
        fs::write(envelope_path(&root, "services", &envelope.envelope_ref).expect("envelope path"), b"not-preserves")
            .expect("tamper envelope bytes");
        let tampered = deliver_local_gossip(&root, "services", &envelope.envelope_ref, "peer:b")
            .expect_err("tampered envelope rejects");
        assert!(tampered.to_string().contains("preserves"));
    }

    #[test]
    fn stale_bootstrap_or_missing_capability_evidence_denies_before_side_effects() {
        let root = temp_dir("remote-dataspace-negative-admission");
        let capability_ref = fake_ref("capability-required");
        let bootstrap_ref = fake_ref("bootstrap-required");
        let envelope = build_envelope(RemoteDataspaceEnvelopeInput {
            from_peer: "peer:a".to_owned(),
            from_actor: "producer".to_owned(),
            to_peer: "peer:b".to_owned(),
            topic: "services".to_owned(),
            operation: RemoteDataspaceOperation::Assert,
            payload: record("service-ready", vec![string("db")]),
            content_refs: Vec::new(),
            capability_refs: vec![capability_ref.clone()],
            evidence_refs: vec![bootstrap_ref.clone()],
        })
        .expect("envelope");
        publish_local_gossip(&root, &envelope, "peer:a").expect("publish");
        let delivery = deliver_local_gossip(&root, "services", &envelope.envelope_ref, "peer:b").expect("deliver");
        let mut stale_bootstrap = evidence_fixture();
        stale_bootstrap.capability_refs = vec![capability_ref.clone()];
        let stale = admit_and_apply_delivered_envelope(&mut RuntimeState::new(1), &delivery, &stale_bootstrap)
            .expect_err("missing declared bootstrap evidence denies");
        assert!(stale.to_string().contains("evidence ref"));
        let mut missing_capability = evidence_fixture();
        missing_capability.peer_bootstrap_refs = vec![bootstrap_ref];
        let denied = admit_and_apply_delivered_envelope(&mut RuntimeState::new(1), &delivery, &missing_capability)
            .expect_err("missing declared capability evidence denies");
        assert!(denied.to_string().contains("capability evidence"));
    }

    fn evidence_fixture() -> RemoteDeliveryEvidence {
        RemoteDeliveryEvidence {
            peer_bootstrap_refs: vec![fake_ref("bootstrap")],
            capability_refs: vec![fake_ref("capability")],
            policy_refs: vec![fake_ref("policy")],
            resource_refs: vec![fake_ref("resource")],
            authority_refs: vec![fake_ref("authority")],
        }
    }

    fn fake_ref(label: &str) -> String {
        let value = record("fake-ref", vec![string(label)]);
        crate::preserves_rail::canonical_hash(&value).expect("fake ref")
    }

    fn temp_dir(name: &str) -> PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
