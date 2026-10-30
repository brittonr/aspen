/// A receiver-issued transfer is staged outside trusted local registries.
pub struct StagedObject {
    pub(super) object_ref: molten_core::dag_sync::DagObjectRef,
    pub(super) origin: crate::mesh::envelope::network::VerifiedRecord,
    pub(super) transport_ref: String,
    pub(super) encoded_bytes: u64,
}

pub struct NetworkDagPort<'a> {
    pub runtime: &'a tokio::runtime::Runtime,
    pub client: crate::fabric_transport::cross_process::ClientInput,
    pub listener: &'a mut crate::fabric_transport::cross_process::Listener,
    pub adapter: &'a crate::fabric_crypto_identity::IrohEd25519<'a>,
    pub signing: crate::mesh::envelope::signatures::SigningInput<'a>,
    pub verification: crate::mesh::envelope::signatures::VerificationInput<'a>,
    pub session_ref: &'a str,
    pub peer: molten_core::dag_sync::DagPeerId,
    pub maximum_bytes: u64,
    pub remaining_bytes: u64,
    pub timeout: std::time::Duration,
    pub deadline: crate::fabric_time::SupervisionDeadline,
    pub staged: &'a std::cell::RefCell<Option<StagedObject>>,
    pub requests: &'a std::cell::RefCell<Vec<crate::mesh::envelope::signatures::SignedRecord>>,
    pub request_transport: &'a std::cell::RefCell<Vec<crate::fabric_transport::cross_process::FrameEvidence>>,
    pub diagnostics: &'a crate::node_state::Scope,
}

// r[impl molten.federated_pull_runtime.receiver_pull]
impl crate::dag_sync::DagTransportPort for NetworkDagPort<'_> {
    fn request(
        &mut self,
        request: &molten_core::dag_sync::DagFetchRequest,
    ) -> crate::error::Result<crate::dag_sync::DagTransferOutcome> {
        if self.staged.borrow().is_some() || request.assigned_peer.as_ref() != Some(&self.peer) {
            return Err(invalid("federation transfer has pending bytes or unassigned peer"));
        }
        if self.requests.borrow().len() >= molten_core::federation::MAX_FEDERATION_ENTRIES
            || self.request_transport.borrow().len() >= molten_core::federation::MAX_FEDERATION_ENTRIES
        {
            return Err(invalid("federation signed request evidence bound exhausted"));
        }
        let maximum_bytes = self.maximum_bytes.min(self.remaining_bytes);
        if maximum_bytes == 0 {
            return Err(invalid("federation session byte budget exhausted"));
        }
        if self.signing.purpose != molten_core::federation::RecordPurpose::Request
            || self.verification.purpose != molten_core::federation::RecordPurpose::Response
        {
            return Err(invalid("federation DAG transfer requires request/response signature domains"));
        }
        let payload = crate::preserves_rail::record("federation-fetch-v1", vec![
            crate::preserves_rail::string(self.session_ref),
            crate::preserves_rail::string(request.object_ref.as_str()),
            crate::preserves_rail::string(request.object_ref.kind()),
            crate::preserves_rail::string(request.sequence.to_string()),
        ]);
        let record = crate::mesh::envelope::signatures::sign_record(
            self.adapter,
            &crate::mesh::envelope::signatures::SigningInput {
                profile: self.signing.profile,
                handle: self.signing.handle,
                context_ref: self.signing.context_ref,
                policy_ref: self.signing.policy_ref,
                purpose: self.signing.purpose,
                issued_at: self.signing.issued_at,
                expires_at: self.signing.expires_at,
                payload: &payload,
            },
        )?;
        let timeout = bounded_timeout(&mut self.deadline, self.timeout)?;
        let sent = self.runtime.block_on(crate::mesh::envelope::network::send_record(
            self.client.clone(),
            &record,
            maximum_bytes,
            timeout,
        ))?;
        self.requests.borrow_mut().push(record);
        let request_ref = sent.request_ref.clone();
        self.request_transport.borrow_mut().push(sent);
        let timeout = bounded_timeout(&mut self.deadline, self.timeout)?;
        let response = self.runtime.block_on(crate::mesh::envelope::network::receive_record(
            self.listener,
            self.adapter,
            crate::mesh::envelope::network::ReceiveInput {
                session_ref: self.session_ref,
                verification: crate::mesh::envelope::signatures::VerificationInput {
                    profile: self.verification.profile,
                    public_key: self.verification.public_key,
                    signer_ref: self.verification.signer_ref,
                    generation: self.verification.generation,
                    currentness: self.verification.currentness,
                    context_ref: self.verification.context_ref,
                    policy_ref: self.verification.policy_ref,
                    purpose: self.verification.purpose,
                    now: self.verification.now,
                    max_age: self.verification.max_age,
                },
                maximum_bytes,
                timeout,
                diagnostics: self.diagnostics,
            },
        ))?;
        let fields = response
            .record()
            .payload
            .collect_simple_record("federation-object-v1", Some(4))
            .ok_or_else(|| invalid("malformed signed federation object response"))?;
        if fields[0].as_string().as_deref() != Some(request_ref.as_str())
            || fields[1].as_string().as_deref() != Some(request.object_ref.as_str())
        {
            crate::mesh::envelope::boundary::observe(
                self.diagnostics,
                crate::mesh::envelope::boundary::FailureClass::CorrelationRejected,
                self.session_ref,
                self.client.protocol.generation,
                &response.transport().request_ref,
            )?;
            return Err(invalid("unsolicited or miscorrelated federation object response"));
        }
        let encoded_bytes = response.transport().payload_bytes;
        self.remaining_bytes = self
            .remaining_bytes
            .checked_sub(encoded_bytes)
            .ok_or_else(|| invalid("federation cumulative byte budget exhausted"))?;
        let transport_ref = response.transport().acknowledgement_ref.clone();
        *self.staged.borrow_mut() = Some(StagedObject {
            object_ref: request.object_ref.clone(),
            origin: response,
            transport_ref: transport_ref.clone(),
            encoded_bytes,
        });
        Ok(crate::dag_sync::DagTransferOutcome::Received(crate::dag_sync::DagTransportEnvelope {
            object_ref: request.object_ref.clone(),
            assigned_peer: request.assigned_peer.clone(),
            encoded_bytes,
            transport_observation_ref: transport_ref,
        }))
    }
}
fn bounded_timeout(
    deadline: &mut crate::fabric_time::SupervisionDeadline,
    maximum: std::time::Duration,
) -> crate::error::Result<std::time::Duration> {
    if deadline.is_expired()? {
        return Err(invalid("federation live session deadline exhausted"));
    }
    let timeout = maximum.min(deadline.remaining()?);
    if timeout.is_zero() {
        return Err(invalid("federation live timeout is zero"));
    }
    Ok(timeout)
}
fn invalid(message: &str) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}
