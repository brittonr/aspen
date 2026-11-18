//! Canonical bridge evidence. Replay uses recorded values and never opens transport.

#[derive(Clone, Debug)]
pub struct Call {
    pub scope: super::bridge::Scope,
    pub authority: String,
    pub request: crate::fabric_transport::FrameEvidence,
    pub response: crate::fabric_transport::FrameEvidence,
    pub result_wire: Vec<u8>,
    pub events: Vec<String>,
    pub reference: String,
}

// r[impl molten.component_rpc.transcript]
impl Call {
    pub fn complete(
        lease: &super::authority::Lease,
        request: &crate::fabric_transport::FrameEvidence,
        response: &crate::fabric_transport::FrameEvidence,
        wire: &[u8],
    ) -> super::Result<Self> {
        let mut transcript = Self {
            scope: lease.scope().clone(),
            authority: lease.receipt().to_string(),
            request: request.clone(),
            response: response.clone(),
            result_wire: wire.to_vec(),
            events: Vec::new(),
            reference: String::new(),
        };
        transcript.events = transcript.expected_events()?;
        transcript.reference = super::hash(&transcript.value()?)?;
        transcript.validate()?;
        Ok(transcript)
    }
    fn expected_events(&self) -> super::Result<Vec<String>> {
        let invocation = super::bridge::invocation(&self.scope)?;
        let result = super::bridge::result(&invocation, &self.result_wire)?;
        let authorized = super::hash(&crate::codec::record("component-rpc-begin-v1", vec![
            self.scope.value(),
            crate::codec::string(&self.authority),
            crate::codec::string(&invocation),
        ]))?;
        let delivered = super::hash(&crate::codec::record("component-rpc-delivery-v1", vec![
            crate::codec::string(&authorized),
            evidence(&self.request),
        ]))?;
        let terminal = super::hash(&crate::codec::record("component-rpc-terminal-v1", vec![
            crate::codec::string(&delivered),
            result,
            evidence(&self.response),
            crate::codec::string("returned"),
            crate::codec::string("experimental-only"),
        ]))?;
        Ok(vec![authorized, delivered, terminal])
    }
    pub fn value(&self) -> super::Result<preserves::IOValue> {
        Ok(crate::codec::record("component-rpc-transcript-v1", vec![
            self.scope.value(),
            crate::codec::string(&self.authority),
            evidence(&self.request),
            evidence(&self.response),
            super::bridge::request(&self.scope, &[])?,
            super::bridge::result(&super::bridge::invocation(&self.scope)?, &self.result_wire)?,
            crate::codec::record("ordered-events", self.events.iter().map(crate::codec::string).collect()),
        ]))
    }
    pub fn validate(&self) -> super::Result<()> {
        self.scope.validate(0)?;
        crate::codec::validate_content_ref(&self.authority).map_err(super::failure)?;
        let invocation = super::bridge::invocation(&self.scope)?;
        let result = super::bridge::result(&invocation, &self.result_wire)?;
        let result_ref = super::hash(&result)?;
        check_evidence(&self.request, &invocation)?;
        check_evidence(&self.response, &result_ref)?;
        let request_bytes =
            crate::codec::canonical_bytes(&super::bridge::request(&self.scope, &[])?).map_err(super::failure)?;
        let result_bytes = crate::codec::canonical_bytes(&result).map_err(super::failure)?;
        let (request_peer, result_peer) = match self.request.role {
            crate::fabric_transport::EndpointParticipantRole::Client => (&self.scope.peer, &self.scope.peer),
            crate::fabric_transport::EndpointParticipantRole::Listener => {
                (&self.scope.caller_peer, &self.scope.caller_peer)
            }
        };
        if self.request.session_ref != self.scope.session
            || self.response.session_ref != super::profile::response_session(&self.scope)?
            || self.request.remote_transport_identity_ref != *request_peer
            || self.response.remote_transport_identity_ref != *result_peer
            || self.request.role == self.response.role
            || self.request.payload_bytes != request_bytes.len() as u64
            || self.response.payload_bytes != result_bytes.len() as u64
            || self.request.payload_ref != crate::fabric_transport::frame_ref(&invocation, &request_bytes)
            || self.response.payload_ref != crate::fabric_transport::frame_ref(&result_ref, &result_bytes)
            || self.events != self.expected_events()?
            || super::hash(&self.value()?)? != self.reference
        {
            return Err("transcript identity, order, parent, wire or session mismatch".into());
        }
        Ok(())
    }
}
fn check_evidence(evidence: &crate::fabric_transport::FrameEvidence, request: &str) -> super::Result<()> {
    if evidence.request_ref != request
        || evidence.delivery != crate::fabric_transport::DeliveryOutcome::Delivered
        || evidence.terminal_class != crate::fabric_transport::SessionTerminalClass::Clean
        || evidence.automatic_retry_count != 0
        || evidence.retry != crate::fabric_transport::RetryDisposition::NotApplicable
    {
        return Err("RPC delivery is incomplete/uncertain; automatic retries are never safe".into());
    }
    for reference in [
        &evidence.descriptor_ref,
        &evidence.session_ref,
        &evidence.payload_ref,
        &evidence.acknowledgement_ref,
        &evidence.remote_transport_identity_ref,
        &evidence.cleanup_evidence_ref,
    ] {
        crate::codec::validate_content_ref(reference).map_err(super::failure)?;
    }
    Ok(())
}
fn evidence(evidence: &crate::fabric_transport::FrameEvidence) -> preserves::IOValue {
    crate::codec::record("component-rpc-transport-evidence-v1", vec![
        crate::codec::string(evidence.role.as_str()),
        crate::codec::string(evidence.terminal_class.as_str()),
        crate::codec::string(&evidence.descriptor_ref),
        crate::codec::string(&evidence.session_ref),
        crate::codec::string(&evidence.request_ref),
        crate::codec::string(&evidence.payload_ref),
        crate::codec::string(&evidence.acknowledgement_ref),
        crate::codec::string(&evidence.remote_transport_identity_ref),
        crate::codec::string(evidence.payload_bytes.to_string()),
        crate::codec::string(evidence.delivery.as_str()),
        crate::codec::string(evidence.retry.as_str()),
        crate::codec::string(evidence.automatic_retry_count.to_string()),
        crate::codec::string(&evidence.cleanup_evidence_ref),
    ])
}

pub fn replay(scope: &super::bridge::Scope, recorded: &Call) -> super::Result<String> {
    if scope != &recorded.scope {
        return Err("replay scope differs from recorded invocation".into());
    }
    recorded.validate()?;
    super::bridge::decode_result(&recorded.result_wire)
}
