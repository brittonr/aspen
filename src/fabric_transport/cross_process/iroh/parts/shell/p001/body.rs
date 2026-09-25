
impl IrohCrossProcessListener {
    // r[impl molten.fabric_transport.cross_process_listener]
    // r[impl molten.fabric_transport.cross_process_session]
    pub async fn bind(input: IrohCrossProcessListenerInput) -> crate::error::Result<Self> {
        validate_listener_shell_input(&input)?;
        let alpn = input.protocol.alpn.as_bytes().to_vec();
        let endpoint = bind_explicit_endpoint(input.bind_addr, input.capability, &alpn).await?;
        let endpoint_addr = endpoint.addr();
        let locators = endpoint_locators(&endpoint_addr)?;
        let bindings = EndpointDescriptorBindings {
            public_endpoint_identity: format!("iroh:{}", endpoint_addr.id),
            listener_identity_ref: input.listener_identity_ref,
            expected_peer_context_ref: input.expected_peer_context_ref,
            locator_cohort_ref: input.locator_cohort_ref,
            locators,
            disclosure: input.disclosure,
            resources: EndpointResourceBounds {
                max_sessions: input.profile.profile.limits.max_sessions,
                max_frame_bytes: input.protocol.framing.max_frame_bytes,
                max_queued_bytes: input.profile.profile.limits.max_queued_bytes,
                max_inflight_bytes: input.profile.profile.limits.max_inflight_bytes,
            },
            validity: input.validity,
        };
        let endpoint_artifact = canonical_cross_process_endpoint(&input.profile.profile, &input.protocol, &bindings)?;
        let mut state =
            plan_cross_process_listener(&input.profile.profile, &input.protocol, &endpoint_artifact.descriptor, &[])
                .map_err(|issues| shell_validation_error("cross-process listener plan", &issues))?;
        state = apply_cross_process_listener_command(&state, &CrossProcessListenerCommand::Start)
            .map_err(|issues| shell_validation_error("cross-process listener start", &issues))?
            .next;
        state = apply_cross_process_listener_command(
            &state,
            &CrossProcessListenerCommand::MarkReady(ListenerReadinessObservation {
                endpoint_setup: true,
                exact_alpn_active: true,
                registration_owned: input.admission.registration_active,
                transport_capability_active: input.admission.transport_capability_active,
                protocol_capability_active: input.admission.protocol_capability_active,
                profile_active: input.admission.profile_active,
            }),
        )
        .map_err(|issues| shell_validation_error("cross-process listener readiness", &issues))?
        .next;
        let _export = plan_endpoint_export(
            &input.profile.profile,
            &input.protocol,
            &endpoint_artifact.descriptor,
            &state,
            input.admission,
            input.observed_tick,
        )
        .map_err(|issues| shell_validation_error("cross-process endpoint publication", &issues))?;
        let endpoint_status = canonical_endpoint_status(&endpoint_artifact.descriptor)?;
        Ok(Self {
            endpoint,
            profile: input.profile,
            protocol: input.protocol,
            admission: input.admission,
            endpoint_artifact,
            endpoint_status,
            state,
        })
    }

    pub fn profile(&self) -> &CanonicalTransportProfile {
        &self.profile
    }

    pub const fn admission(&self) -> EndpointAdmissionState {
        self.admission
    }

    pub fn handoff(&self) -> &CanonicalCrossProcessEndpoint {
        &self.endpoint_artifact
    }

    pub fn status(&self) -> &CanonicalEndpointStatus {
        &self.endpoint_status
    }

    pub fn state(&self) -> &CrossProcessListenerState {
        &self.state
    }

    // r[impl molten.fabric_transport.cross_process_listener]
    // r[impl molten.fabric_transport.cross_process_session]
    pub async fn accept_one(
        &mut self,
        session_ref: &str,
        request_ref: &str,
        timeout: std::time::Duration,
    ) -> crate::error::Result<CrossProcessFrameEvidence> {
        Ok(self.accept_one_frame(session_ref, request_ref, timeout).await?.evidence)
    }

    // r[impl molten.fabric_consistency.live_service_ports]
    pub async fn accept_one_frame(
        &mut self,
        session_ref: &str,
        request_ref: &str,
        timeout: std::time::Duration,
    ) -> crate::error::Result<CrossProcessReceivedFrame> {
        validate_exchange_refs(session_ref, request_ref)?;
        self.accept_one_derived_frame(session_ref, timeout, |_| Ok(request_ref.to_string())).await
    }

    // The derivation runs after the bounded read and before acknowledgement, so
    // malformed protocol payloads cannot obtain delivered-frame evidence.
    // r[impl molten.fabric_consistency.live_service_ports]
    pub async fn accept_one_derived_frame<F>(
        &mut self,
        session_ref: &str,
        timeout: std::time::Duration,
        derive_request_ref: F,
    ) -> crate::error::Result<CrossProcessReceivedFrame>
    where
        F: FnOnce(&[u8]) -> crate::error::Result<String>,
    {
        crate::preserves_rail::validate_content_ref(session_ref)?;
        if !self.state.is_ready() {
            return Err(crate::error::MoltenError::invalid_harness("cross-process listener is not ready"));
        }
        let incoming = tokio::time::timeout(timeout, self.endpoint.accept())
            .await
            .map_err(|_| crate::error::MoltenError::invalid_harness("cross-process listener accept timed out"))?
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("cross-process listener closed before accept"))?;
        let connection = tokio::time::timeout(timeout, incoming)
            .await
            .map_err(|_| crate::error::MoltenError::invalid_harness("cross-process listener handshake timed out"))?
            .map_err(iroh_error)?;
        let remote_transport_identity_ref = blake3_ref(connection.remote_id().to_string().as_bytes());
        self.state = apply_cross_process_listener_command(&self.state, &CrossProcessListenerCommand::AcceptSession {
            callback_generation: self.protocol.generation,
        })
        .map_err(|issues| shell_validation_error("cross-process listener accept", &issues))?
        .next;

        let dial_plan = dial_plan_from_descriptor(&self.endpoint_artifact.descriptor);
        let mut session = plan_cross_process_session(&dial_plan, session_ref, EndpointParticipantRole::Listener)
            .map_err(|issues| shell_validation_error("cross-process inbound session plan", &issues))?;
        session = apply_cross_process_session_command(&session, &CrossProcessSessionCommand::BeginAccept {
            observed_descriptor_ref: self.endpoint_artifact.descriptor_ref.clone(),
            callback_generation: self.protocol.generation,
        })
        .map_err(|issues| shell_validation_error("cross-process inbound accept", &issues))?
        .next;
        session = apply_cross_process_session_command(&session, &CrossProcessSessionCommand::Established {
            observed_peer_context_ref: self.endpoint_artifact.descriptor.expected_peer_context_ref.clone(),
            callback_generation: self.protocol.generation,
        })
        .map_err(|issues| shell_validation_error("cross-process inbound establishment", &issues))?
        .next;

        let exchange =
            run_server_exchange(&connection, &mut session, derive_request_ref, self.protocol.generation, timeout).await;
        let received = match exchange {
            Ok(exchange) => {
                let evidence = finalize_successful_session(SessionCloseInput {
                    session,
                    role: EndpointParticipantRole::Listener,
                    descriptor_ref: &self.endpoint_artifact.descriptor_ref,
                    session_ref,
                    request_ref: &exchange.request_ref,
                    remote_transport_identity_ref: &remote_transport_identity_ref,
                    frame: exchange.frame,
                })?;
                CrossProcessReceivedFrame {
                    payload: exchange.payload,
                    evidence,
                }
            }
            Err(error) => {
                let _failed = finalize_failed_session(session, SessionTerminalClass::AdapterFailure)?;
                self.finish_listener_session()?;
                return Err(error);
            }
        };
        self.finish_listener_session()?;
        Ok(received)
    }

    // r[impl molten.fabric_transport.cross_process_listener]
    pub async fn drain_and_close(
        mut self,
        reason: ListenerDrainReason,
    ) -> crate::error::Result<CrossProcessListenerCleanup> {
        self.state =
            apply_cross_process_listener_command(&self.state, &CrossProcessListenerCommand::BeginDrain { reason })
                .map_err(|issues| shell_validation_error("cross-process listener drain", &issues))?
                .next;
        if self.state.active_sessions != 0 {
            return Err(crate::error::MoltenError::invalid_harness(
                "cross-process listener drain requires all sessions to be terminal",
            ));
        }
        self.state = apply_cross_process_listener_command(&self.state, &CrossProcessListenerCommand::Close)
            .map_err(|issues| shell_validation_error("cross-process listener close", &issues))?
            .next;
        self.endpoint.close().await;
        self.state = apply_cross_process_listener_command(&self.state, &CrossProcessListenerCommand::BeginCleanup)
            .map_err(|issues| shell_validation_error("cross-process listener cleanup", &issues))?
            .next;
        let cleanup_evidence_ref = cleanup_ref(
            &self.state.identity.listener_identity_ref,
            &self.endpoint_artifact.descriptor_ref,
            self.protocol.generation,
        );
        self.state = apply_cross_process_listener_command(&self.state, &CrossProcessListenerCommand::CompleteCleanup {
            cleanup_evidence_ref: cleanup_evidence_ref.clone(),
        })
        .map_err(|issues| shell_validation_error("cross-process listener cleanup completion", &issues))?
        .next;
        Ok(CrossProcessListenerCleanup {
            listener_identity_ref: self.state.identity.listener_identity_ref.clone(),
            descriptor_ref: self.endpoint_artifact.descriptor_ref.clone(),
            generation: self.protocol.generation,
            drain_reason: reason,
            terminal_class: self.state.terminal_class.unwrap_or(ListenerTerminalClass::Clean),
            cleanup_evidence_ref,
        })
    }

    fn finish_listener_session(&mut self) -> crate::error::Result<()> {
        self.state = apply_cross_process_listener_command(&self.state, &CrossProcessListenerCommand::SessionTerminal {
            callback_generation: self.protocol.generation,
        })
        .map_err(|issues| shell_validation_error("cross-process listener session terminal", &issues))?
        .next;
        Ok(())
    }
}
