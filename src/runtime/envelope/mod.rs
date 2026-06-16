use serde::Deserialize;
use serde::Serialize;

use super::RuntimeValue;
use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::canonical_bytes;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::parse_text;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::to_text;
use crate::preserves_rail::u64_value;
use crate::preserves_rail::validate_content_ref;

const ENVELOPE_VERSION: u16 = 1;
const MAX_REF_LIST_ITEMS: usize = 256;
const MAX_ACTOR_ID_BYTES: usize = 256;
const MAX_CAPABILITY_BYTES: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActorId(String);

impl ActorId {
    pub fn parse(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        validate_nonempty_token("actor id", &value, MAX_ACTOR_ID_BYTES)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContentRef(String);

impl ContentRef {
    pub fn parse(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        validate_content_ref(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Capability(String);

impl Capability {
    pub fn parse(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        validate_nonempty_token("capability", &value, MAX_CAPABILITY_BYTES)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EvidenceRef(String);

impl EvidenceRef {
    pub fn parse(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        validate_content_ref(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvelopeDto {
    pub version: u16,
    pub sender: ActorId,
    pub subject_preserves: String,
    pub body_preserves: String,
    pub blob_refs: Vec<ContentRef>,
    pub capabilities: Vec<Capability>,
    pub evidence_refs: Vec<EvidenceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    pub version: u16,
    pub sender: ActorId,
    pub subject: RuntimeValue,
    pub body: RuntimeValue,
    pub blob_refs: Vec<ContentRef>,
    pub capabilities: Vec<Capability>,
    pub evidence_refs: Vec<EvidenceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvelopeBoundary {
    pub envelope_ref: String,
    pub subject_ref: String,
    pub body_ref: String,
    pub blob_refs: Vec<ContentRef>,
    pub evidence_refs: Vec<EvidenceRef>,
}

impl Envelope {
    pub fn new(input: EnvelopeInput) -> Result<Self> {
        validate_ref_list_len("blob refs", input.blob_refs.len())?;
        validate_ref_list_len("capabilities", input.capabilities.len())?;
        validate_ref_list_len("evidence refs", input.evidence_refs.len())?;
        Ok(Self {
            version: ENVELOPE_VERSION,
            sender: input.sender,
            subject: input.subject,
            body: input.body,
            blob_refs: input.blob_refs,
            capabilities: input.capabilities,
            evidence_refs: input.evidence_refs,
        })
    }

    pub fn from_dto(dto: EnvelopeDto) -> Result<Self> {
        if dto.version != ENVELOPE_VERSION {
            return Err(MoltenError::invalid_harness(format!(
                "unsupported envelope version {}, expected {ENVELOPE_VERSION}",
                dto.version
            )));
        }
        let subject = RuntimeValue::new(parse_text(&dto.subject_preserves)?)?;
        let body = RuntimeValue::new(parse_text(&dto.body_preserves)?)?;
        Self::new(EnvelopeInput {
            sender: dto.sender,
            subject,
            body,
            blob_refs: dto.blob_refs,
            capabilities: dto.capabilities,
            evidence_refs: dto.evidence_refs,
        })
    }

    pub fn to_dto(&self) -> Result<EnvelopeDto> {
        Ok(EnvelopeDto {
            version: self.version,
            sender: self.sender.clone(),
            subject_preserves: to_text(self.subject.as_iovalue())?,
            body_preserves: to_text(self.body.as_iovalue())?,
            blob_refs: self.blob_refs.clone(),
            capabilities: self.capabilities.clone(),
            evidence_refs: self.evidence_refs.clone(),
        })
    }

    pub fn to_value(&self) -> preserves::IOValue {
        record("runtime-envelope-v1", vec![
            u64_value(u64::from(self.version)),
            record("sender", vec![string(self.sender.as_str())]),
            record("subject", vec![self.subject.as_iovalue().clone()]),
            record("body", vec![self.body.as_iovalue().clone()]),
            ref_list_value("blob-refs", &self.blob_refs),
            capability_list_value(&self.capabilities),
            evidence_list_value(&self.evidence_refs),
        ])
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        canonical_bytes(&self.to_value())
    }

    pub fn canonical_hash(&self) -> Result<String> {
        canonical_hash(&self.to_value())
    }

    pub fn boundary(&self) -> Result<EnvelopeBoundary> {
        Ok(EnvelopeBoundary {
            envelope_ref: self.canonical_hash()?,
            subject_ref: self.subject.value_ref().to_string(),
            body_ref: self.body.value_ref().to_string(),
            blob_refs: self.blob_refs.clone(),
            evidence_refs: self.evidence_refs.clone(),
        })
    }

    pub fn validate_core(&self) -> Result<EnvelopeBoundary> {
        validate_ref_list_len("blob refs", self.blob_refs.len())?;
        validate_ref_list_len("capabilities", self.capabilities.len())?;
        validate_ref_list_len("evidence refs", self.evidence_refs.len())?;
        self.boundary()
    }
}

pub struct EnvelopeInput {
    pub sender: ActorId,
    pub subject: RuntimeValue,
    pub body: RuntimeValue,
    pub blob_refs: Vec<ContentRef>,
    pub capabilities: Vec<Capability>,
    pub evidence_refs: Vec<EvidenceRef>,
}

fn validate_nonempty_token(label: &str, value: &str, max_bytes: usize) -> Result<()> {
    if value.is_empty() {
        return Err(MoltenError::invalid_harness(format!("{label} must not be empty")));
    }
    if value.len() > max_bytes {
        return Err(MoltenError::invalid_harness(format!("{label} exceeds {max_bytes} bytes")));
    }
    Ok(())
}

fn validate_ref_list_len(label: &str, len: usize) -> Result<()> {
    if len > MAX_REF_LIST_ITEMS {
        return Err(MoltenError::invalid_harness(format!("{label} exceeds {MAX_REF_LIST_ITEMS} items")));
    }
    Ok(())
}

fn ref_list_value(label: &'static str, refs: &[ContentRef]) -> preserves::IOValue {
    let mut values = Vec::with_capacity(refs.len());
    for reference in refs {
        values.push(string(reference.as_str()));
    }
    record(label, vec![sequence(values)])
}

fn capability_list_value(capabilities: &[Capability]) -> preserves::IOValue {
    let mut values = Vec::with_capacity(capabilities.len());
    for capability in capabilities {
        values.push(string(capability.as_str()));
    }
    record("capabilities", vec![sequence(values)])
}

fn evidence_list_value(refs: &[EvidenceRef]) -> preserves::IOValue {
    let mut values = Vec::with_capacity(refs.len());
    for reference in refs {
        values.push(string(reference.as_str()));
    }
    record("evidence-refs", vec![sequence(values)])
}

#[cfg(test)]
mod tests {
    use super::ActorId;
    use super::Capability;
    use super::ContentRef;
    use super::Envelope;
    use super::EnvelopeDto;
    use super::EnvelopeInput;
    use super::EvidenceRef;
    use super::RuntimeValue;
    use crate::preserves_rail::content_ref_from_bytes;
    use crate::preserves_rail::parse_text;

    fn fixture_envelope() -> Envelope {
        let subject = RuntimeValue::string("molten.runtime.local.subject").expect("subject");
        let body = RuntimeValue::string("hello").expect("body");
        Envelope::new(EnvelopeInput {
            sender: ActorId::parse("actor:alice").expect("actor id"),
            subject,
            body,
            blob_refs: vec![ContentRef::parse(content_ref_from_bytes(b"blob-a")).expect("blob ref")],
            capabilities: vec![Capability::parse("send:molten.runtime.local.subject").expect("capability")],
            evidence_refs: vec![EvidenceRef::parse(content_ref_from_bytes(b"evidence-a")).expect("evidence ref")],
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
        let parsed_subject = RuntimeValue::new(parse_text("\"molten.runtime.local.subject\"").expect("subject text"))
            .expect("parsed subject");
        let parsed_body = RuntimeValue::new(parse_text("\"hello\"").expect("body text")).expect("parsed body");
        let parsed = Envelope::new(EnvelopeInput {
            sender: ActorId::parse("actor:alice").expect("actor id"),
            subject: parsed_subject,
            body: parsed_body,
            blob_refs: vec![ContentRef::parse(content_ref_from_bytes(b"blob-a")).expect("blob ref")],
            capabilities: vec![Capability::parse("send:molten.runtime.local.subject").expect("capability")],
            evidence_refs: vec![EvidenceRef::parse(content_ref_from_bytes(b"evidence-a")).expect("evidence ref")],
        })
        .expect("parsed envelope");

        let direct_bytes = direct.canonical_bytes().expect("direct canonical bytes");
        let parsed_bytes = parsed.canonical_bytes().expect("parsed canonical bytes");
        assert_eq!(direct_bytes, parsed_bytes);
        assert_eq!(direct.canonical_hash().expect("direct hash"), content_ref_from_bytes(&direct_bytes));
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
}
