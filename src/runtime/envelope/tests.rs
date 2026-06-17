use super::*;

fn ref_from_bytes(bytes: &[u8]) -> String {
    crate::preserves_rail::content_ref_from_bytes(bytes)
}

fn parse_value(text: &str) -> crate::error::Result<preserves::IOValue> {
    crate::preserves_rail::parse_text(text)
}

fn fixture_envelope() -> Envelope {
    let subject = RuntimeValue::string("molten.runtime.local.subject").expect("subject");
    let body = RuntimeValue::string("hello").expect("body");
    Envelope::new(EnvelopeInput {
        sender: ActorId::parse("actor:alice").expect("actor id"),
        subject,
        body,
        blob_refs: vec![ContentRef::parse(ref_from_bytes(b"blob-a")).expect("blob ref")],
        capabilities: vec![Capability::parse("send:molten.runtime.local.subject").expect("capability")],
        evidence_refs: vec![EvidenceRef::parse(ref_from_bytes(b"evidence-a")).expect("evidence ref")],
    })
    .expect("envelope")
}

#[test]
fn envelope_dto_round_trips_fields() {
    let envelope = fixture_envelope();
    let dto = envelope.to_dto().expect("dto");
    let json = serde_json::to_string(&dto).expect("json");
    let decoded: EnvelopeDto = serde_json::from_str(&json).expect("decoded dto");
    let round_trip = Envelope::from_dto(decoded).expect("round trip envelope");
    assert_eq!(round_trip.sender, envelope.sender);
    assert_eq!(round_trip.subject.value_ref(), envelope.subject.value_ref());
    assert_eq!(round_trip.body.value_ref(), envelope.body.value_ref());
    assert_eq!(round_trip.blob_refs, envelope.blob_refs);
    assert_eq!(round_trip.capabilities, envelope.capabilities);
    assert_eq!(round_trip.evidence_refs, envelope.evidence_refs);
}

#[test]
fn equivalent_envelopes_hash_identically_after_dto_boundary() {
    let envelope = fixture_envelope();
    let dto = envelope.to_dto().expect("dto");
    let from_dto = Envelope::from_dto(dto).expect("from dto");
    assert_eq!(from_dto.canonical_hash().expect("from dto hash"), envelope.canonical_hash().expect("hash"));
}

#[test]
fn equivalent_envelopes_canonicalize_to_same_bytes() {
    let direct = fixture_envelope();
    let parsed_subject = RuntimeValue::new(parse_value("\"molten.runtime.local.subject\"").expect("subject text"))
        .expect("parsed subject");
    let parsed_body = RuntimeValue::new(parse_value("\"hello\"").expect("body text")).expect("parsed body");
    let parsed = Envelope::new(EnvelopeInput {
        sender: ActorId::parse("actor:alice").expect("actor id"),
        subject: parsed_subject,
        body: parsed_body,
        blob_refs: vec![ContentRef::parse(ref_from_bytes(b"blob-a")).expect("blob ref")],
        capabilities: vec![Capability::parse("send:molten.runtime.local.subject").expect("capability")],
        evidence_refs: vec![EvidenceRef::parse(ref_from_bytes(b"evidence-a")).expect("evidence ref")],
    })
    .expect("parsed envelope");

    let direct_bytes = direct.canonical_bytes().expect("direct canonical bytes");
    let parsed_bytes = parsed.canonical_bytes().expect("parsed canonical bytes");
    assert_eq!(direct_bytes, parsed_bytes);
    assert_eq!(direct.canonical_hash().expect("direct hash"), ref_from_bytes(&direct_bytes));
}

#[test]
fn envelope_boundary_uses_preserves_refs_for_comms() {
    let envelope = fixture_envelope();
    let boundary = envelope.boundary().expect("boundary");
    assert_eq!(boundary.envelope_ref, envelope.canonical_hash().expect("envelope ref"));
    assert_eq!(boundary.subject_ref, envelope.subject.value_ref());
    assert_eq!(boundary.body_ref, envelope.body.value_ref());
    assert_eq!(boundary.blob_refs.as_slice(), envelope.blob_refs.as_slice());
    assert_eq!(boundary.evidence_refs.as_slice(), envelope.evidence_refs.as_slice());
    for reference in [boundary.envelope_ref, boundary.subject_ref, boundary.body_ref] {
        assert!(reference.starts_with("blake3:"));
    }
}

#[test]
fn envelope_core_validation_is_deterministic() {
    let envelope = fixture_envelope();
    let first = envelope.validate_core().expect("first validation");
    let second = envelope.validate_core().expect("second validation");
    assert_eq!(first, second);
}

#[test]
fn envelope_core_source_excludes_adapter_effects() {
    let source = include_str!("mod.rs");
    for (prefix, suffix) in [
        ("std", "::fs"),
        ("std", "::net"),
        ("std", "::process"),
        ("std", "::time"),
        ("tokio", "::"),
        ("async", " "),
        ("ir", "oh"),
        ("steel", "_core"),
        ("wasm", "time"),
        ("re", "db"),
    ] {
        let forbidden = format!("{prefix}{suffix}");
        assert!(!source.contains(&forbidden), "envelope core must not contain adapter effect token {forbidden}");
    }
}

#[test]
fn invalid_content_ref_is_rejected() {
    let error = ContentRef::parse("b3:not-canonical").expect_err("invalid ref");
    assert!(error.to_string().contains("content ref must start with blake3:"));
}
