#[derive(Debug, Clone)]
pub struct RegisteredStreamRequest {
    pub peer: DirectPeerBinding,
    pub bounds: DirectStreamBounds,
    pub operation: DirectStreamRequest,
    /// Monotonic clock supplied by the imperative consumer boundary.
    pub clock: fn() -> std::time::Instant,
}

impl Registered {
    /// Route a request-bound stream through the existing extension/port/profile
    /// admission boundary. Payloads remain in the protected socket; they never
    /// enter the frame-registration queue or a whole-image allocation.
    // r[impl molten.fabric_transport.direct_lan]
    pub async fn proxy_stream<S: ProtectedSocketService>(
        &mut self,
        binding: &crate::fabric::CanonicalPortBinding,
        effect: &crate::system_extension::TypedEffectRequest,
        request: RegisteredStreamRequest,
        service: &mut S,
        cancel: tokio::sync::watch::Receiver<bool>,
    ) -> crate::fabric::FabricPortResult<super::direct::Evidence> {
        if effect.request_ref != request.operation.request_ref || self.routed_requests.contains(&effect.request_ref) {
            return Err(crate::fabric::FabricPortError::malformed("direct stream request mismatch or replay"));
        }
        if self.routed_requests.len() as u64 >= self.profile.profile.limits.max_queued_events {
            return Err(crate::fabric::FabricPortError::capability("direct effect request lifetime budget exhausted"));
        }
        validate_direct_stream_bounds(&self.profile.profile, &self.protocol, &request.bounds)
            .map_err(|issue| crate::fabric::FabricPortError::malformed(issue.code()))?;
        let TransportCommand::OpenStream { operation_id, direction, .. } =
            self.control.registered_command(&effect.request_ref)?
        else {
            return Err(crate::fabric::FabricPortError::malformed("direct stream effect requires registered OpenStream"));
        };
        if operation_id != &request.operation.operation_ref {
            return Err(crate::fabric::FabricPortError::malformed("direct stream operation mismatch"));
        }
        if *direction != StreamDirection::Bidirectional {
            return Err(crate::fabric::FabricPortError::malformed("direct stream opening was not admitted"));
        }
        let (command, transition) = self.control.execute_effect(binding, effect)?;
        let TransportCommand::OpenStream { operation_id, session_id, stream_id, .. } = command else {
            return Err(crate::fabric::FabricPortError::malformed("direct stream effect requires registered OpenStream"));
        };
        if transition.decision != TransportTransitionDecision::Applied {
            return Err(crate::fabric::FabricPortError::malformed("direct stream opening was not admitted"));
        }
        // Consume before any socket effects. A lost result cannot authorize a
        // second attempt over bytes the application may already have consumed.
        self.routed_requests.insert(effect.request_ref.clone());
        let input = ClientInput {
            profile: self.profile.clone(), protocol: self.protocol.clone(),
            capability: self.client.capability.clone(), bind_addr: self.client.bind_addr,
            endpoint: self.client.endpoint.clone(), expected: self.client.expected.clone(),
            admission: self.client.admission, session_ref: session_id.opaque_ref.clone(),
            request_ref: effect.request_ref.clone(),
        };
        let result = super::direct::proxy(super::direct::Outbound {
            input, peer: request.peer, bounds: request.bounds, request: request.operation, clock: request.clock,
        }, service, cancel).await;
        // This is only closure of the admitted control handle. Actual delivery
        // and uncertainty are owned by the returned direct-stream evidence; no
        // ordinary SendFrame acknowledgement or application success is invented.
        self.control.adapter_mut().execute_command(&TransportCommand::CloseStream {
            operation_id, session_id, stream_id,
        })?;
        result
    }
}
