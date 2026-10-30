//! Canonical, closed-class observations from actual rejection sites; never raw errors or payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    RecordRejected,
    SignatureRejected,
    CorrelationRejected,
    ContentIdentityRejected,
}
impl FailureClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RecordRejected => "record-rejected",
            Self::SignatureRejected => "signature-rejected",
            Self::CorrelationRejected => "correlation-rejected",
            Self::ContentIdentityRejected => "content-identity-rejected",
        }
    }
}
pub struct Observation {
    pub class: FailureClass,
    pub session_ref: String,
    pub generation: u64,
    pub wire_ref: String,
    pub process_id: u64,
}
impl Observation {
    pub fn value(&self) -> preserves::IOValue {
        crate::preserves_rail::record("federation-boundary-failure-v1", vec![
            crate::preserves_rail::string(self.class.as_str()),
            crate::preserves_rail::string(&self.session_ref),
            crate::preserves_rail::u64_value(self.generation),
            crate::preserves_rail::string(&self.wire_ref),
            crate::preserves_rail::u64_value(self.process_id),
        ])
    }
}

// r[impl molten.federated_pull_runtime.status_evidence]
pub fn observe(
    scope: &crate::node_state::Scope,
    class: FailureClass,
    session_ref: &str,
    generation: u64,
    wire_ref: &str,
) -> crate::error::Result<Observation> {
    crate::preserves_rail::validate_content_ref(session_ref)?;
    crate::preserves_rail::validate_content_ref(wire_ref)?;
    if generation == 0 {
        return Err(invalid());
    }
    let observation = Observation {
        class,
        session_ref: session_ref.to_string(),
        generation,
        wire_ref: wire_ref.to_string(),
        process_id: u64::from(std::process::id()),
    };
    scope.write(&path()?, &crate::preserves_rail::canonical_bytes(&observation.value())?)?;
    Ok(observation)
}

pub fn read(scope: &crate::node_state::Scope) -> crate::error::Result<Observation> {
    let value = crate::preserves_rail::parse_canonical_bytes(&scope.read(&path()?, 1024)?)?;
    let fields = value.collect_simple_record("federation-boundary-failure-v1", Some(5)).ok_or_else(invalid)?;
    let class = match fields[0].as_string().as_deref() {
        Some("record-rejected") => FailureClass::RecordRejected,
        Some("signature-rejected") => FailureClass::SignatureRejected,
        Some("correlation-rejected") => FailureClass::CorrelationRejected,
        Some("content-identity-rejected") => FailureClass::ContentIdentityRejected,
        _ => return Err(invalid()),
    };
    let session_ref = fields[1].as_string().ok_or_else(invalid)?.into_owned();
    let wire_ref = fields[3].as_string().ok_or_else(invalid)?.into_owned();
    crate::preserves_rail::validate_content_ref(&session_ref)?;
    crate::preserves_rail::validate_content_ref(&wire_ref)?;
    let generation = number(&fields[2])?;
    let process_id = number(&fields[4])?;
    if generation == 0 || process_id == 0 {
        return Err(invalid());
    }
    Ok(Observation {
        class,
        session_ref,
        generation,
        wire_ref,
        process_id,
    })
}
fn number(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<u64> {
    value.as_u64().ok_or_else(invalid)?.map_err(|_| invalid())
}
fn path() -> crate::error::Result<crate::node_state::RelativePath> {
    crate::node_state::RelativePath::parse("federation-boundary-failure.preserves")
}
fn invalid() -> crate::error::Failure {
    crate::error::Failure::invalid_harness("invalid federation boundary observation")
}
