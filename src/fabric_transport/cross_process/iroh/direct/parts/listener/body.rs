impl Acceptor {
    // r[impl molten.fabric_transport.direct_lan]
    // r[impl molten.fabric_transport.receive_gate]
    pub async fn bind(config: Config) -> crate::fabric::FabricPortResult<Self> {
        validate_direct_stream_bounds(&config.listener.profile.profile, &config.listener.protocol, &config.bounds)
            .map_err(|issue| crate::fabric::FabricPortError::malformed(issue.code()))?;
        validate_bind(config.listener.bind_addr)?;
        if config.peer.remote_role != EndpointParticipantRole::Client {
            return Err(crate::fabric::FabricPortError::malformed("direct listener requires the client peer role"));
        }
        if config.peer.peer_context_ref != config.listener.expected_peer_context_ref {
            return Err(crate::fabric::FabricPortError::malformed(DirectStreamIssue::WrongContext.code()));
        }
        config.peer.endpoint_id.parse::<iroh::EndpointId>().map_err(port_error)?;
        let gate = ReceiveGate::new(config.receive_budget).map_err(|deny| crate::fabric::FabricPortError::malformed(deny.code()))?;
        let initial_tick = config.listener.observed_tick;
        let inner = Listener::bind_admitted(config.listener, Some(&config.bounds)).await.map_err(port_error)?;
        Ok(Self {
            inner,
            peer: config.peer,
            bounds: config.bounds,
            gate,
            clock_origin: (config.clock)(),
            clock: config.clock,
            initial_tick,
        })
    }

    pub fn handoff(&self) -> &Endpoint {
        self.inner.handoff()
    }

    pub fn begin_drain(&mut self) -> crate::fabric::FabricPortResult<()> {
        self.begin_drain_with_reason(ListenerDrainReason::OperatorRequest)
    }

    pub fn begin_drain_with_reason(&mut self, reason: ListenerDrainReason) -> crate::fabric::FabricPortResult<()> {
        self.inner.state =
            apply_cross_process_listener_command(&self.inner.state, &CrossProcessListenerCommand::BeginDrain {
                reason,
            })
            .map_err(|issues| crate::fabric::FabricPortError::malformed(format!("direct drain denied: {issues:?}")))?
            .next;
        self.inner.endpoint.set_alpns(Vec::new());
        Ok(())
    }

    /// No hidden task or pending transfer queue: the mutable listener reserves
    /// its one admitted connection until terminal is_cleanup. Other services retain
    /// their own independent executor/control-plane budgets.
    pub async fn accept_proxy<S: ProtectedSocketService>(
        &mut self,
        request: DirectStreamRequest,
        service: &mut S,
        mut cancel: tokio::sync::watch::Receiver<bool>,
    ) -> crate::fabric::FabricPortResult<Evidence> {
        validate_request(&request, &self.inner.protocol)?;
        let started = (self.clock)();
        let contract = contract_ref(&self.inner.profile, &self.inner.protocol, &self.bounds)?;
        let mut progress = Progress::default();
        let mut session = None;
        let mut connection = None;
        let mut actual_peer = None;
        let mut path = DirectStreamPath::Unavailable;
        let mut is_reserved = false;
        let total_timeout = std::time::Duration::from_millis(self.bounds.total_timeout_ms);
        let runtime = async {
            if !self.inner.state.is_ready() {
                return Err(DirectStreamTerminal::Denied);
            }
            let incoming = self.inner.endpoint.accept().await.ok_or(DirectStreamTerminal::Disconnected)?;
            let source = match incoming.remote_addr() {
                iroh::endpoint::IncomingAddr::Ip(address) => ReceiveSource::from(address.ip()),
                _ => {
                    incoming.refuse();
                    return Err(DirectStreamTerminal::Denied);
                }
            };
            self.inner.state =
                apply_cross_process_listener_command(&self.inner.state, &CrossProcessListenerCommand::AcceptSession {
                    callback_generation: self.inner.protocol.generation,
                })
                .map_err(|_| DirectStreamTerminal::Denied)?
                .next;
            is_reserved = true;
            // Full handshake only. Never into_0rtt or a replayable early stream.
            let accepted = incoming.await.map_err(|_| DirectStreamTerminal::Disconnected)?;
            actual_peer = Some(accepted.remote_id().to_string());
            connection = Some(accepted);
            path = observe_path(connection.as_ref().ok_or(DirectStreamTerminal::Disconnected)?)?;
            let connection = connection.as_ref().ok_or(DirectStreamTerminal::Disconnected)?;
            let elapsed = elapsed_ms(self.clock_origin, (self.clock)())
                .map_err(|_| DirectStreamTerminal::OverBound)?;
            let now = self.initial_tick.checked_add(elapsed).ok_or(DirectStreamTerminal::Denied)?;
            let gated = self
                .gate
                .admit_inbound(
                    source,
                    observation(connection, &self.peer, &self.inner.protocol, EndpointParticipantRole::Client),
                    now,
                )
                .map_err(|deny| {
                    progress.deny_code = Some(deny.code());
                    DirectStreamTerminal::Denied
                })?;
            validate_direct_peer(&gated, &self.peer, &self.inner.protocol, true).map_err(|issue| {
                progress.deny_code = Some(issue.code());
                DirectStreamTerminal::Denied
            })?;
            let (mut send, mut receive) =
                connection.accept_bi().await.map_err(connection_error)?;
            let operation = exchange(Exchange {
                send: &mut send, receive: &mut receive, gated, peer: &self.peer,
                protocol: &self.inner.protocol, bounds: &self.bounds, request: &request,
                contract: &contract, descriptor: &self.inner.endpoint_artifact.descriptor,
            }, service, &mut progress, &mut session);
            tokio::select! {
                result = operation => result,
                _ = refuse_excess(&self.inner.endpoint) => Err(DirectStreamTerminal::Disconnected),
            }
        };
        let terminal = tokio::select! {
            biased;
            _ = cancellation(&mut cancel) => DirectStreamTerminal::Cancelled,
            result = tokio::time::timeout(total_timeout, runtime) => {
                match result { Ok(Ok(())) => DirectStreamTerminal::Clean, Ok(Err(terminal)) => terminal,
                    Err(_) => DirectStreamTerminal::Timeout }
            }
        };
        let is_reset = terminal != DirectStreamTerminal::Clean && connection.is_some();
        let grace = std::time::Duration::from_millis(self.bounds.cleanup_timeout_ms);
        let mut is_cleanup = true;
        if let Some(connection) = &connection {
            connection.close(if is_reset { 1_u8 } else { 0_u8 }.into(), b"direct-stream-terminal");
            is_cleanup = tokio::time::timeout(grace, connection.closed()).await.is_ok();
        } else if is_reserved {
            // A cancelled handshake has no authenticated connection handle to
            // observe. Close the owning endpoint, rather than inventing cleanup
            // from merely dropping a handshake future or silently rebinding it.
            is_cleanup = tokio::time::timeout(grace, self.inner.endpoint.close()).await.is_ok();
            self.begin_drain_with_reason(ListenerDrainReason::AdapterFailure)?;
        }
        if !is_cleanup && self.inner.state.is_ready() {
            self.begin_drain_with_reason(ListenerDrainReason::AdapterFailure)?;
        }
        if is_cleanup && is_reserved {
            self.inner.finish_listener_session().map_err(port_error)?;
        }
        if is_cleanup {
            finish_session(&mut session, terminal, &request, &contract)?;
        }
        evidence::build(evidence::Input {
            descriptor_ref: &self.inner.endpoint_artifact.descriptor_ref,
            request: &request,
            peer: &self.peer,
            actual_peer: actual_peer.as_deref(),
            contract_ref: &contract,
            progress: &progress,
            terminal,
            path,
            duration_ms: elapsed_ms(started, (self.clock)())?,
            is_reset,
            is_cleanup,
        })
    }

    pub async fn drain_and_close(self, reason: ListenerDrainReason) -> crate::fabric::FabricPortResult<ListenerCleanup> {
        let grace = std::time::Duration::from_millis(self.bounds.cleanup_timeout_ms);
        tokio::time::timeout(grace, async move {
            if self.inner.state.phase == CrossProcessListenerPhase::Draining {
                self.inner.close_draining(reason).await.map_err(port_error)
            } else {
                self.inner.drain_and_close(reason).await.map_err(port_error)
            }
        })
        .await
        .map_err(|_| crate::fabric::FabricPortError::Timeout {
            message: "direct listener cleanup deadline elapsed".to_owned(),
        })?
    }
}
