use crate::error;
use crate::preserves_rail;

const ENVELOPE_VERSION: u16 = 1;
const MAX_REF_LIST_ITEMS: usize = 256;
const MAX_ACTOR_ID_BYTES: usize = 256;
const MAX_CAPABILITY_BYTES: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct ActorId(String);

impl ActorId {
    pub fn parse(value: impl Into<String>) -> error::Result<Self> {
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct ContentRef(String);

impl ContentRef {
    pub fn parse(value: impl Into<String>) -> error::Result<Self> {
        let value = value.into();
        preserves_rail::validate_content_ref(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Capability(String);

impl Capability {
    pub fn parse(value: impl Into<String>) -> error::Result<Self> {
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct EvidenceRef(String);

impl EvidenceRef {
    pub fn parse(value: impl Into<String>) -> error::Result<Self> {
        let value = value.into();
        preserves_rail::validate_content_ref(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    pub subject: super::RuntimeValue,
    pub body: super::RuntimeValue,
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
    pub fn new(input: EnvelopeInput) -> error::Result<Self> {
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

    pub fn from_dto(dto: EnvelopeDto) -> error::Result<Self> {
        if dto.version != ENVELOPE_VERSION {
            return Err(error::MoltenError::invalid_harness(format!(
                "unsupported envelope version {}, expected {ENVELOPE_VERSION}",
                dto.version
            )));
        }
        let subject = super::RuntimeValue::new(preserves_rail::parse_text(&dto.subject_preserves)?)?;
        let body = super::RuntimeValue::new(preserves_rail::parse_text(&dto.body_preserves)?)?;
        Self::new(EnvelopeInput {
            sender: dto.sender,
            subject,
            body,
            blob_refs: dto.blob_refs,
            capabilities: dto.capabilities,
            evidence_refs: dto.evidence_refs,
        })
    }

    pub fn to_dto(&self) -> error::Result<EnvelopeDto> {
        Ok(EnvelopeDto {
            version: self.version,
            sender: self.sender.clone(),
            subject_preserves: preserves_rail::to_text(self.subject.as_iovalue())?,
            body_preserves: preserves_rail::to_text(self.body.as_iovalue())?,
            blob_refs: self.blob_refs.clone(),
            capabilities: self.capabilities.clone(),
            evidence_refs: self.evidence_refs.clone(),
        })
    }

    pub fn to_value(&self) -> preserves::IOValue {
        preserves_rail::record("runtime-envelope-v1", vec![
            preserves_rail::u64_value(u64::from(self.version)),
            preserves_rail::record("sender", vec![preserves_rail::string(self.sender.as_str())]),
            preserves_rail::record("subject", vec![self.subject.as_iovalue().clone()]),
            preserves_rail::record("body", vec![self.body.as_iovalue().clone()]),
            ref_list_value("blob-refs", &self.blob_refs),
            capability_list_value(&self.capabilities),
            evidence_list_value(&self.evidence_refs),
        ])
    }

    pub fn canonical_bytes(&self) -> error::Result<Vec<u8>> {
        preserves_rail::canonical_bytes(&self.to_value())
    }

    pub fn canonical_hash(&self) -> error::Result<String> {
        preserves_rail::canonical_hash(&self.to_value())
    }

    pub fn boundary(&self) -> error::Result<EnvelopeBoundary> {
        Ok(EnvelopeBoundary {
            envelope_ref: self.canonical_hash()?,
            subject_ref: self.subject.value_ref().to_string(),
            body_ref: self.body.value_ref().to_string(),
            blob_refs: self.blob_refs.clone(),
            evidence_refs: self.evidence_refs.clone(),
        })
    }

    pub fn validate_core(&self) -> error::Result<EnvelopeBoundary> {
        validate_ref_list_len("blob refs", self.blob_refs.len())?;
        validate_ref_list_len("capabilities", self.capabilities.len())?;
        validate_ref_list_len("evidence refs", self.evidence_refs.len())?;
        self.boundary()
    }
}

pub struct EnvelopeInput {
    pub sender: ActorId,
    pub subject: super::RuntimeValue,
    pub body: super::RuntimeValue,
    pub blob_refs: Vec<ContentRef>,
    pub capabilities: Vec<Capability>,
    pub evidence_refs: Vec<EvidenceRef>,
}

fn validate_nonempty_token(label: &str, value: &str, max_bytes: usize) -> error::Result<()> {
    if value.is_empty() {
        return Err(error::MoltenError::invalid_harness(format!("{label} must not be empty")));
    }
    if value.len() > max_bytes {
        return Err(error::MoltenError::invalid_harness(format!("{label} exceeds {max_bytes} bytes")));
    }
    Ok(())
}

fn validate_ref_list_len(label: &str, len: usize) -> error::Result<()> {
    if len > MAX_REF_LIST_ITEMS {
        return Err(error::MoltenError::invalid_harness(format!("{label} exceeds {MAX_REF_LIST_ITEMS} items")));
    }
    Ok(())
}

fn ref_list_value(label: &'static str, refs: &[ContentRef]) -> preserves::IOValue {
    let mut values = Vec::with_capacity(refs.len());
    for reference in refs {
        values.push(preserves_rail::string(reference.as_str()));
    }
    preserves_rail::record(label, vec![preserves_rail::sequence(values)])
}

fn capability_list_value(capabilities: &[Capability]) -> preserves::IOValue {
    let mut values = Vec::with_capacity(capabilities.len());
    for capability in capabilities {
        values.push(preserves_rail::string(capability.as_str()));
    }
    preserves_rail::record("capabilities", vec![preserves_rail::sequence(values)])
}

fn evidence_list_value(refs: &[EvidenceRef]) -> preserves::IOValue {
    let mut values = Vec::with_capacity(refs.len());
    for reference in refs {
        values.push(preserves_rail::string(reference.as_str()));
    }
    preserves_rail::record("evidence-refs", vec![preserves_rail::sequence(values)])
}

#[cfg(test)]
mod tests;
