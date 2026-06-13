use serde::Deserialize;
use serde::Serialize;

use super::Envelope;
use super::EnvelopeBoundary;
use super::RuntimeBoundaryError;
use crate::error::Result;
use crate::preserves_rail::content_ref_from_bytes;
use crate::preserves_rail::validate_content_ref;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrohEnvelopeBridgeRecord {
    pub topic: String,
    pub envelope_ref: String,
    pub subject_ref: String,
    pub blob_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobReferenceRecord {
    pub blob_ref: String,
    pub byte_len: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocsMutationEvidence {
    pub namespace: String,
    pub mutation_ref: String,
    pub envelope_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteEnvelopeAdmission {
    pub envelope_ref: String,
    pub admitted_blob_refs: Vec<String>,
}

pub fn iroh_envelope_bridge_record(topic: impl Into<String>, envelope: &Envelope) -> Result<IrohEnvelopeBridgeRecord> {
    let boundary = envelope.boundary()?;
    Ok(bridge_record_from_boundary(topic.into(), &boundary))
}

pub fn verify_blob_reference(
    bytes: &[u8],
    declared_ref: &str,
) -> std::result::Result<BlobReferenceRecord, RuntimeBoundaryError> {
    validate_content_ref(declared_ref)
        .map_err(|error| RuntimeBoundaryError::invalid_input("iroh-blob", error.to_string()))?;
    let actual_ref = content_ref_from_bytes(bytes);
    if actual_ref != declared_ref {
        return Err(RuntimeBoundaryError::denied_operation(
            "iroh-blob",
            format!("blob ref mismatch declared={declared_ref} actual={actual_ref}"),
        ));
    }
    Ok(BlobReferenceRecord {
        blob_ref: actual_ref,
        byte_len: bytes.len() as u64,
    })
}

pub fn docs_mutation_evidence(
    namespace: impl Into<String>,
    mutation_bytes: &[u8],
    envelope: &Envelope,
) -> Result<DocsMutationEvidence> {
    Ok(DocsMutationEvidence {
        namespace: namespace.into(),
        mutation_ref: content_ref_from_bytes(mutation_bytes),
        envelope_ref: envelope.canonical_hash()?,
    })
}

pub fn admit_remote_envelope(
    envelope: &Envelope,
    declared_envelope_ref: &str,
) -> std::result::Result<RemoteEnvelopeAdmission, RuntimeBoundaryError> {
    validate_content_ref(declared_envelope_ref)
        .map_err(|error| RuntimeBoundaryError::invalid_input("remote-envelope", error.to_string()))?;
    let actual_ref = envelope
        .canonical_hash()
        .map_err(|error| RuntimeBoundaryError::invalid_input("remote-envelope", error.to_string()))?;
    if actual_ref != declared_envelope_ref {
        return Err(RuntimeBoundaryError::denied_operation(
            "remote-envelope",
            format!("envelope ref mismatch declared={declared_envelope_ref} actual={actual_ref}"),
        ));
    }
    let mut admitted_blob_refs = Vec::with_capacity(envelope.blob_refs.len());
    for reference in &envelope.blob_refs {
        admitted_blob_refs.push(reference.as_str().to_string());
    }
    Ok(RemoteEnvelopeAdmission {
        envelope_ref: actual_ref,
        admitted_blob_refs,
    })
}

fn bridge_record_from_boundary(topic: String, boundary: &EnvelopeBoundary) -> IrohEnvelopeBridgeRecord {
    let mut blob_refs = Vec::with_capacity(boundary.blob_refs.len());
    for reference in &boundary.blob_refs {
        blob_refs.push(reference.as_str().to_string());
    }
    IrohEnvelopeBridgeRecord {
        topic,
        envelope_ref: boundary.envelope_ref.clone(),
        subject_ref: boundary.subject_ref.clone(),
        blob_refs,
    }
}

#[cfg(test)]
mod tests {
    use super::admit_remote_envelope;
    use super::docs_mutation_evidence;
    use super::iroh_envelope_bridge_record;
    use super::verify_blob_reference;
    use crate::preserves_rail::content_ref_from_bytes;
    use crate::runtime::ActorId;
    use crate::runtime::Capability;
    use crate::runtime::ContentRef;
    use crate::runtime::Envelope;
    use crate::runtime::EnvelopeInput;
    use crate::runtime::EvidenceRef;
    use crate::runtime::RuntimeErrorCategory;
    use crate::runtime::RuntimeValue;

    fn envelope() -> Envelope {
        Envelope::new(EnvelopeInput {
            sender: ActorId::parse("actor:remote").expect("sender"),
            subject: RuntimeValue::string("protocol.ready").expect("subject"),
            body: RuntimeValue::string("payload metadata").expect("body"),
            blob_refs: vec![ContentRef::parse(content_ref_from_bytes(b"blob payload")).expect("blob")],
            capabilities: vec![Capability::parse("send:protocol.ready").expect("capability")],
            evidence_refs: vec![EvidenceRef::parse(content_ref_from_bytes(b"transport evidence")).expect("evidence")],
        })
        .expect("envelope")
    }

    #[test]
    fn iroh_bridge_record_binds_envelope_and_blob_refs() {
        let envelope = envelope();
        let record = iroh_envelope_bridge_record("topic-a", &envelope).expect("bridge record");
        assert_eq!(record.topic, "topic-a");
        assert_eq!(record.envelope_ref, envelope.canonical_hash().expect("envelope ref"));
        assert_eq!(record.blob_refs, vec![content_ref_from_bytes(b"blob payload")]);
    }

    #[test]
    fn blob_reference_verification_rejects_tampering() {
        let declared = content_ref_from_bytes(b"blob payload");
        let verified = verify_blob_reference(b"blob payload", &declared).expect("verified blob");
        assert_eq!(verified.blob_ref, declared);

        let error = verify_blob_reference(b"tampered", &declared).expect_err("tampered blob denied");
        assert_eq!(error.category(), RuntimeErrorCategory::DeniedOperation);
    }

    #[test]
    fn docs_mutation_evidence_binds_namespace_mutation_and_envelope() {
        let envelope = envelope();
        let evidence = docs_mutation_evidence("docs:runtime", b"set service.ready", &envelope).expect("docs evidence");
        assert_eq!(evidence.namespace, "docs:runtime");
        assert_eq!(evidence.mutation_ref, content_ref_from_bytes(b"set service.ready"));
        assert_eq!(evidence.envelope_ref, envelope.canonical_hash().expect("envelope ref"));
    }

    #[test]
    fn remote_admission_rejects_tampered_envelope_ref() {
        let envelope = envelope();
        let envelope_ref = envelope.canonical_hash().expect("envelope ref");
        let admitted = admit_remote_envelope(&envelope, &envelope_ref).expect("admitted");
        assert_eq!(admitted.envelope_ref, envelope_ref);

        let wrong = content_ref_from_bytes(b"wrong envelope");
        let error = admit_remote_envelope(&envelope, &wrong).expect_err("stale ref denied");
        assert_eq!(error.category(), RuntimeErrorCategory::DeniedOperation);
    }
}
