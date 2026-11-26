
impl Listener {
    // r[impl molten.fabric_transport.cross_process_listener]
    // r[impl molten.fabric_transport.cross_process_session]
    pub async fn bind(input: ListenerInput) -> crate::error::Result<Self> {
        validate_listener_input(&input)?;
        Self::bind_admitted(input, None).await
    }

    async fn bind_admitted(input: ListenerInput, direct: Option<&DirectStreamBounds>) -> crate::error::Result<Self> {
        let alpn = input.protocol.alpn.as_bytes().to_vec();
        let endpoint = match direct {
            Some(bounds) => direct::bind_endpoint(input.bind_addr, input.capability, &alpn, bounds).await?,
            None => bind_explicit_endpoint(input.bind_addr, input.capability, &alpn).await?,
        };
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
        let endpoint_artifact = crate::fabric_transport::cross_process::endpoint(&input.profile.profile, &input.protocol, &bindings)?;
        let mut state =
            plan_cross_process_listener(&input.profile.profile, &input.protocol, &endpoint_artifact.descriptor, &[])
                .map_err(|issues| validation_error("cross-process listener plan", &issues))?;
        state = apply_cross_process_listener_command(&state, &CrossProcessListenerCommand::Start)
            .map_err(|issues| validation_error("cross-process listener start", &issues))?
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
        .map_err(|issues| validation_error("cross-process listener readiness", &issues))?
        .next;
        let _export = plan_endpoint_export(
            &input.profile.profile,
            &input.protocol,
            &endpoint_artifact.descriptor,
            &state,
            input.admission,
            input.observed_tick,
        )
        .map_err(|issues| validation_error("cross-process endpoint publication", &issues))?;
        let endpoint_status = endpoint_status(&endpoint_artifact.descriptor)?;
        Ok(Self {
            endpoint,
            profile: input.profile,
            protocol: input.protocol,
            admission: input.admission,
            endpoint_artifact,
            endpoint_status,
            state,
            admitted_peers: input.admitted_peers,
        })
    }

    pub fn profile(&self) -> &Profile {
        &self.profile
    }

    pub const fn admission(&self) -> EndpointAdmissionState {
        self.admission
    }

    pub fn handoff(&self) -> &Endpoint {
        &self.endpoint_artifact
    }

    pub fn status(&self) -> &EndpointStatus {
        &self.endpoint_status
    }

    pub fn state(&self) -> &CrossProcessListenerState {
        &self.state
    }

    /// The remote endpoints this listener admits, keyed by peer id.
    pub fn admitted_peers(&self) -> &std::collections::BTreeMap<String, AdmittedPeer> {
        &self.admitted_peers
    }

    /// Admit one more remote endpoint, for peers whose identity is learned after binding. The
    /// table stays bounded with distinct identities; a denied admission leaves it unchanged.
    pub fn admit_peer(&mut self, peer_id: String, peer: AdmittedPeer) -> crate::error::Result<()> {
        let mut admitted = self.admitted_peers.clone();
        if admitted.insert(peer_id, peer).is_some() {
            return Err(crate::error::Failure::invalid_harness("cross-process listener peer id is already admitted"));
        }
        validate_admitted_peers(&admitted)?;
        self.admitted_peers = admitted;
        Ok(())
    }

    // r[impl molten.fabric_transport.cross_process_listener]
    // r[impl molten.fabric_transport.cross_process_session]
    pub async fn accept_one(
        &mut self,
        session_ref: &str,
        request_ref: &str,
        timeout: std::time::Duration,
    ) -> crate::error::Result<FrameEvidence> {
        Ok(self.accept_one_frame(session_ref, request_ref, timeout).await?.evidence)
    }

    // r[impl molten.fabric_consistency.live_service_ports]
    pub async fn accept_one_frame(
        &mut self,
        session_ref: &str,
        request_ref: &str,
        timeout: std::time::Duration,
    ) -> crate::error::Result<ReceivedFrame> {
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
    ) -> crate::error::Result<ReceivedFrame>
    where
        F: FnOnce(&[u8]) -> crate::error::Result<String>,
    {
        let derive = |_peer_id: &str, payload: &[u8]| derive_request_ref(payload);
        match self.poll_one_derived_frame(session_ref, timeout, derive).await? {
            ListenerPoll::Received(received) => Ok(received),
            ListenerPoll::Rejected(rejection) => Err(rejection.failure),
            ListenerPoll::Idle => Err(crate::error::Failure::invalid_harness("cross-process listener accept timed out")),
        }
    }

    /// An idle accept deadline creates no session or delivery. A failed or stalled handshake, an
    /// unadmitted remote endpoint, or a failed session rejects only that connection; an `Err` is a
    /// listener failure (closed endpoint, not ready, or a listener state-machine denial). The
    /// derivation receives the admitted peer id of the authenticated remote endpoint and runs
    /// before acknowledgement, so a payload it denies never obtains delivered-frame evidence.
    // r[impl molten.fabric_transport.raft_peer_auth_and_isolation]
    pub async fn poll_one_derived_frame<F>(
        &mut self,
        session_ref: &str,
        timeout: std::time::Duration,
        derive_request_ref: F,
    ) -> crate::error::Result<ListenerPoll>
    where
        F: FnOnce(&str, &[u8]) -> crate::error::Result<String>,
    {
        crate::preserves_rail::validate_content_ref(session_ref)?;
        if !self.state.is_ready() {
            return Err(crate::error::Failure::invalid_harness("cross-process listener is not ready"));
        }
        let incoming = match tokio::time::timeout(timeout, self.endpoint.accept()).await {
            Err(_) => return Ok(ListenerPoll::Idle),
            Ok(incoming) => incoming
                .ok_or_else(|| crate::error::Failure::invalid_harness("cross-process listener closed before accept"))?,
        };
        let connection = match tokio::time::timeout(timeout, incoming).await {
            Err(_) => {
                return Ok(rejected(
                    ConnectionRejectionKind::Handshake,
                    crate::error::Failure::invalid_harness("cross-process listener handshake timed out"),
                ));
            }
            Ok(Err(failure)) => return Ok(rejected(ConnectionRejectionKind::Handshake, error(failure))),
            Ok(Ok(connection)) => connection,
        };
        let remote_endpoint_identity = format!("iroh:{}", connection.remote_id());
        let Some((peer_id, observed_peer_context_ref)) = self
            .admitted_peers
            .iter()
            .find(|(_, peer)| peer.public_endpoint_identity == remote_endpoint_identity)
            .map(|(peer_id, peer)| (peer_id.clone(), peer.peer_context_ref.clone()))
        else {
            connection.close(IROH_REFUSED_CLOSE_CODE.into(), UNADMITTED_PEER_CLOSE_REASON);
            return Ok(rejected(
                ConnectionRejectionKind::UnadmittedPeer,
                crate::error::Failure::invalid_harness("cross-process listener refused an unadmitted remote endpoint"),
            ));
        };
        let remote_transport_identity_ref = blake3_ref(connection.remote_id().to_string().as_bytes());
        self.state = apply_cross_process_listener_command(&self.state, &CrossProcessListenerCommand::AcceptSession {
            callback_generation: self.protocol.generation,
        })
        .map_err(|issues| validation_error("cross-process listener accept", &issues))?
        .next;

        let dial_plan = dial_plan_from_descriptor(&self.endpoint_artifact.descriptor);
        let mut session = plan_cross_process_session(&dial_plan, session_ref, EndpointParticipantRole::Listener)
            .map_err(|issues| validation_error("cross-process inbound session plan", &issues))?;
        session = apply_cross_process_session_command(&session, &CrossProcessSessionCommand::BeginAccept {
            observed_descriptor_ref: self.endpoint_artifact.descriptor_ref.clone(),
            callback_generation: self.protocol.generation,
        })
        .map_err(|issues| validation_error("cross-process inbound accept", &issues))?
        .next;
        let established = apply_cross_process_session_command(&session, &CrossProcessSessionCommand::Established {
            observed_peer_context_ref,
            callback_generation: self.protocol.generation,
        });
        session = match established {
            Ok(transition) => transition.next,
            Err(issues) => {
                connection.close(IROH_REFUSED_CLOSE_CODE.into(), UNADMITTED_PEER_CLOSE_REASON);
                let _failed = finalize_failed_session(session, SessionTerminalClass::AdapterFailure)?;
                self.finish_listener_session()?;
                return Ok(rejected(
                    ConnectionRejectionKind::UnadmittedPeer,
                    validation_error("cross-process inbound establishment", &issues),
                ));
            }
        };

        let derive = |payload: &[u8]| derive_request_ref(&peer_id, payload);
        let exchange = run_server_exchange(&connection, &mut session, derive, self.protocol.generation, timeout).await;
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
                ReceivedFrame {
                    payload: exchange.payload,
                    evidence,
                    peer_id,
                }
            }
            Err(failure) => {
                let _failed = finalize_failed_session(session, SessionTerminalClass::AdapterFailure)?;
                self.finish_listener_session()?;
                return Ok(rejected(ConnectionRejectionKind::Session, failure));
            }
        };
        self.finish_listener_session()?;
        Ok(ListenerPoll::Received(received))
    }

    // r[impl molten.fabric_transport.cross_process_listener]
    pub async fn drain_and_close(
        mut self,
        reason: ListenerDrainReason,
    ) -> crate::error::Result<ListenerCleanup> {
        self.state =
            apply_cross_process_listener_command(&self.state, &CrossProcessListenerCommand::BeginDrain { reason })
                .map_err(|issues| validation_error("cross-process listener drain", &issues))?
                .next;
        self.close_draining(reason).await
    }

    async fn close_draining(mut self, reason: ListenerDrainReason) -> crate::error::Result<ListenerCleanup> {
        if self.state.active_sessions != 0 {
            return Err(crate::error::Failure::invalid_harness(
                "cross-process listener drain requires all sessions to be terminal",
            ));
        }
        self.state = apply_cross_process_listener_command(&self.state, &CrossProcessListenerCommand::Close)
            .map_err(|issues| validation_error("cross-process listener close", &issues))?
            .next;
        self.endpoint.close().await;
        self.state = apply_cross_process_listener_command(&self.state, &CrossProcessListenerCommand::BeginCleanup)
            .map_err(|issues| validation_error("cross-process listener cleanup", &issues))?
            .next;
        let cleanup_evidence_ref = cleanup_ref(
            &self.state.identity.listener_identity_ref,
            &self.endpoint_artifact.descriptor_ref,
            self.protocol.generation,
        );
        self.state = apply_cross_process_listener_command(&self.state, &CrossProcessListenerCommand::CompleteCleanup {
            cleanup_evidence_ref: cleanup_evidence_ref.clone(),
        })
        .map_err(|issues| validation_error("cross-process listener cleanup completion", &issues))?
        .next;
        Ok(ListenerCleanup {
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
        .map_err(|issues| validation_error("cross-process listener session terminal", &issues))?
        .next;
        Ok(())
    }
}
