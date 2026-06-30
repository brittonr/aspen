#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IrohEnvelopeBridgeRecord {
    pub topic: String,
    pub envelope_ref: String,
    pub subject_ref: String,
    pub blob_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BlobReferenceRecord {
    pub blob_ref: String,
    pub byte_len: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

pub fn iroh_envelope_bridge_record(
    topic: impl Into<String>,
    envelope: &super::Envelope,
) -> crate::error::Result<IrohEnvelopeBridgeRecord> {
    let boundary = envelope.boundary()?;
    Ok(bridge_record_from_boundary(topic.into(), &boundary))
}

pub fn verify_blob_reference(
    bytes: &[u8],
    declared_ref: &str,
) -> std::result::Result<BlobReferenceRecord, super::RuntimeBoundaryError> {
    crate::preserves_rail::validate_content_ref(declared_ref)
        .map_err(|error| super::RuntimeBoundaryError::invalid_input("iroh-blob", error.to_string()))?;
    let actual_ref = crate::preserves_rail::content_ref_from_bytes(bytes);
    if actual_ref != declared_ref {
        return Err(super::RuntimeBoundaryError::denied_operation(
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
    envelope: &super::Envelope,
) -> crate::error::Result<DocsMutationEvidence> {
    Ok(DocsMutationEvidence {
        namespace: namespace.into(),
        mutation_ref: crate::preserves_rail::content_ref_from_bytes(mutation_bytes),
        envelope_ref: envelope.canonical_hash()?,
    })
}

pub fn admit_remote_envelope(
    envelope: &super::Envelope,
    declared_envelope_ref: &str,
) -> std::result::Result<RemoteEnvelopeAdmission, super::RuntimeBoundaryError> {
    crate::preserves_rail::validate_content_ref(declared_envelope_ref)
        .map_err(|error| super::RuntimeBoundaryError::invalid_input("remote-envelope", error.to_string()))?;
    let actual_ref = envelope
        .canonical_hash()
        .map_err(|error| super::RuntimeBoundaryError::invalid_input("remote-envelope", error.to_string()))?;
    if actual_ref != declared_envelope_ref {
        return Err(super::RuntimeBoundaryError::denied_operation(
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

fn bridge_record_from_boundary(topic: String, boundary: &super::EnvelopeBoundary) -> IrohEnvelopeBridgeRecord {
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
    fn content_ref(bytes: &[u8]) -> String {
        crate::preserves_rail::content_ref_from_bytes(bytes)
    }

    fn envelope() -> crate::runtime::Envelope {
        crate::runtime::Envelope::new(crate::runtime::EnvelopeInput {
            sender: crate::runtime::ActorId::parse("actor:remote").expect("sender"),
            subject: crate::runtime::RuntimeValue::string("protocol.ready").expect("subject"),
            body: crate::runtime::RuntimeValue::string("payload metadata").expect("body"),
            blob_refs: vec![crate::runtime::ContentRef::parse(content_ref(b"blob payload")).expect("blob")],
            capabilities: vec![crate::runtime::Capability::parse("send:protocol.ready").expect("capability")],
            evidence_refs: vec![
                crate::runtime::EvidenceRef::parse(content_ref(b"transport evidence")).expect("evidence"),
            ],
        })
        .expect("envelope")
    }

    #[test]
    fn iroh_bridge_record_binds_envelope_and_blob_refs() {
        let envelope = envelope();
        let record = super::iroh_envelope_bridge_record("topic-a", &envelope).expect("bridge record");
        assert_eq!(record.topic, "topic-a");
        assert_eq!(record.envelope_ref, envelope.canonical_hash().expect("envelope ref"));
        assert_eq!(record.blob_refs, vec![content_ref(b"blob payload")]);
    }

    #[test]
    fn blob_reference_verification_rejects_tampering() {
        let declared = content_ref(b"blob payload");
        let verified = super::verify_blob_reference(b"blob payload", &declared).expect("verified blob");
        assert_eq!(verified.blob_ref, declared);

        let error = super::verify_blob_reference(b"tampered", &declared).expect_err("tampered blob denied");
        assert_eq!(error.category(), crate::runtime::RuntimeErrorCategory::DeniedOperation);
    }

    #[test]
    fn docs_mutation_evidence_binds_namespace_mutation_and_envelope() {
        let envelope = envelope();
        let evidence =
            super::docs_mutation_evidence("docs:runtime", b"set service.ready", &envelope).expect("docs evidence");
        assert_eq!(evidence.namespace, "docs:runtime");
        assert_eq!(evidence.mutation_ref, content_ref(b"set service.ready"));
        assert_eq!(evidence.envelope_ref, envelope.canonical_hash().expect("envelope ref"));
    }

    #[test]
    fn remote_admission_rejects_tampered_envelope_ref() {
        let envelope = envelope();
        let envelope_ref = envelope.canonical_hash().expect("envelope ref");
        let admitted = super::admit_remote_envelope(&envelope, &envelope_ref).expect("admitted");
        assert_eq!(admitted.envelope_ref, envelope_ref);

        let wrong = content_ref(b"wrong envelope");
        let error = super::admit_remote_envelope(&envelope, &wrong).expect_err("stale ref denied");
        assert_eq!(error.category(), crate::runtime::RuntimeErrorCategory::DeniedOperation);
    }
}
