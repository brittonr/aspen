/// Network reception and cryptographic verification grant no import authority.
#[derive(Clone)]
pub struct VerifiedRecord {
    record: super::signatures::SignedRecord,
    verification: crate::fabric_crypto_identity::VerificationOutcome,
    transport: crate::fabric_transport::cross_process::FrameEvidence,
}
impl VerifiedRecord {
    pub fn record(&self) -> &super::signatures::SignedRecord {
        &self.record
    }
    pub fn verification(&self) -> &crate::fabric_crypto_identity::VerificationOutcome {
        &self.verification
    }
    pub fn transport(&self) -> &crate::fabric_transport::cross_process::FrameEvidence {
        &self.transport
    }
}

pub struct ReceiveInput<'a> {
    pub session_ref: &'a str,
    pub verification: super::signatures::VerificationInput<'a>,
    pub maximum_bytes: u64,
    pub timeout: std::time::Duration,
    pub diagnostics: &'a crate::node_state::Scope,
}

// r[impl molten.federated_pull_runtime.crypto]
// r[impl molten.federated_pull_runtime.discovery_hints]
pub async fn receive_record(
    listener: &mut crate::fabric_transport::cross_process::Listener,
    adapter: &crate::fabric_crypto_identity::IrohEd25519<'_>,
    input: ReceiveInput<'_>,
) -> crate::error::Result<VerifiedRecord> {
    let mut verified = None;
    let generation = listener.handoff().descriptor.generation;
    let frame = listener
        .accept_one_derived_frame(input.session_ref, input.timeout, |bytes| {
            let request_ref = crate::preserves_rail::content_ref_from_bytes(bytes);
            let record = match super::wire::decode(bytes, &input.verification, input.maximum_bytes) {
                Ok(record) => record,
                Err(error) => {
                    super::boundary::observe(
                        input.diagnostics,
                        super::boundary::FailureClass::RecordRejected,
                        input.session_ref,
                        generation,
                        &request_ref,
                    )?;
                    return Err(error);
                }
            };
            let verification = match super::signatures::verify_record(adapter, &record, &input.verification) {
                Ok(verification) => verification,
                Err(error) => {
                    super::boundary::observe(
                        input.diagnostics,
                        super::boundary::FailureClass::SignatureRejected,
                        input.session_ref,
                        generation,
                        &request_ref,
                    )?;
                    return Err(error);
                }
            };
            verified = Some((record, verification));
            Ok(request_ref)
        })
        .await?;
    let (record, verification) = verified.ok_or_else(|| {
        crate::error::Failure::invalid_harness("federation transport completed without verified record")
    })?;
    Ok(VerifiedRecord {
        record,
        verification,
        transport: frame.evidence,
    })
}

// r[impl molten.federated_pull_runtime.crypto]
pub async fn send_record(
    mut client: crate::fabric_transport::cross_process::ClientInput,
    record: &super::signatures::SignedRecord,
    maximum_bytes: u64,
    timeout: std::time::Duration,
) -> crate::error::Result<crate::fabric_transport::cross_process::FrameEvidence> {
    let bytes = super::wire::encode(record, maximum_bytes)?;
    client.request_ref = crate::preserves_rail::content_ref_from_bytes(&bytes);
    crate::fabric_transport::cross_process::exchange_frame(client, &bytes, timeout).await
}
