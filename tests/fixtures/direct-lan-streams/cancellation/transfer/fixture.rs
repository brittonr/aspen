use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

/// More than every check and open event one fixture operation can emit.
const EVENT_CAPACITY: usize = 64;

pub(super) enum Event {
    Checked,
    Opened,
}

pub(super) type Events = tokio::sync::mpsc::Receiver<Event>;

pub(super) struct Service {
    authority: crate::service::SocketService,
    socket: Option<tokio::net::UnixStream>,
    events: tokio::sync::mpsc::Sender<Event>,
}

impl molten::fabric_transport::ProtectedSocketService for Service {
    fn authorize(
        &mut self,
        inbound: molten::fabric_transport::GatedInbound,
        request: &molten::fabric_transport::DirectStreamRequest,
    ) -> molten::fabric::FabricPortResult<String> {
        self.authority.authorize(inbound, request)
    }

    fn check_current(
        &mut self,
        admitted: &molten::fabric_transport::direct::AdmittedOperation,
    ) -> molten::fabric::FabricPortResult<()> {
        self.authority.check_current(admitted)?;
        self.notify(Event::Checked)
    }

    async fn open(
        &mut self,
        admitted: &molten::fabric_transport::direct::AdmittedOperation,
    ) -> molten::fabric::FabricPortResult<tokio::net::UnixStream> {
        self.check_current(admitted)?;
        let socket = self
            .socket
            .take()
            .ok_or_else(|| molten::fabric::FabricPortError::malformed("duplicate protected open"))?;
        self.authority.opens += 1;
        self.notify(Event::Opened)?;
        Ok(socket)
    }
}

impl Service {
    /// A full observer buffer fails the operation instead of dropping a barrier.
    fn notify(&self, event: Event) -> molten::fabric::FabricPortResult<()> {
        match self.events.try_send(event) {
            Ok(()) | Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => Ok(()),
            Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                Err(molten::fabric::FabricPortError::malformed("fixture event observer is full"))
            }
        }
    }

    pub(super) async fn fill_protected_send_buffer(&self) -> crate::TestResult<Vec<u8>> {
        let socket = self.socket.as_ref().ok_or("protected socket already moved")?;
        let limit = usize::try_from(crate::config::SATURATION_BYTES)?;
        socket.writable().await?;
        let block = [0x5a_u8; 1];
        let mut queued = Vec::new();
        loop {
            if queued.len() >= limit {
                return Err("protected socket did not reach finite backpressure bound".into());
            }
            match socket.try_write(&block) {
                Ok(0) => return Err("protected socket closed during buffer preparation".into()),
                Ok(count) => queued.extend_from_slice(&block[..count]),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return Ok(queued),
                Err(error) => return Err(error.into()),
            }
        }
    }
}

fn service(is_client: bool) -> crate::TestResult<(Service, tokio::net::UnixStream, Events)> {
    let (socket, peer) = tokio::net::UnixStream::pair()?;
    let (events, receiver) = tokio::sync::mpsc::channel(EVENT_CAPACITY);
    Ok((
        Service {
            authority: crate::service::SocketService {
                path: std::path::PathBuf::new(),
                expected: crate::config::request(1)?,
                peer: crate::config::peer(is_client, 1)?,
                revoked: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                opens: 0,
            },
            socket: Some(socket),
            events,
        },
        peer,
        receiver,
    ))
}

fn role() -> crate::config::RoleConfig {
    crate::config::RoleConfig {
        role: "listener".to_owned(),
        case: "positive".to_owned(),
        root: std::path::PathBuf::new(),
        generation: 1,
        client_generation: 1,
        bind_ip: std::net::Ipv4Addr::LOCALHOST.into(),
        bind_port: 0,
    }
}

pub(super) struct Fixture {
    pub(super) listener: molten::fabric_transport::direct::Acceptor,
    pub(super) service: Service,
    pub(super) peer: tokio::net::UnixStream,
    pub(super) events: Events,
}

impl Fixture {
    pub(super) async fn new() -> crate::TestResult<Self> {
        let listener = molten::fabric_transport::direct::Acceptor::bind(molten::fabric_transport::direct::Config {
            listener: crate::config::listener_input(&role())?,
            peer: crate::config::peer(false, 1)?,
            bounds: crate::config::bounds(),
            receive_budget: molten::fabric_transport::ReceiveBudget {
                burst: 1,
                refill_ticks: 100,
                max_sources: 1,
            },
            clock: std::time::Instant::now,
        })
        .await?;
        let (service, peer, events) = service(false)?;
        Ok(Self {
            listener,
            service,
            peer,
            events,
        })
    }
}

pub(super) async fn opened(events: &mut Events) -> crate::TestResult {
    while let Some(event) = events.recv().await {
        if matches!(event, Event::Opened) {
            return Ok(());
        }
    }
    Err("service event owner disappeared before open".into())
}

pub(super) async fn checked(events: &mut Events) -> crate::TestResult {
    match events.recv().await {
        Some(Event::Checked) => Ok(()),
        Some(Event::Opened) | None => Err("missing current-authority pump barrier".into()),
    }
}

pub(super) fn data_header(count: usize) -> [u8; 9] {
    let mut header = [0; 9];
    header[0] = 1;
    header[1..].copy_from_slice(&(count as u64).to_be_bytes());
    header
}

pub(super) struct Raw {
    endpoint: iroh::Endpoint,
    connection: Option<iroh::endpoint::Connection>,
    send: Option<iroh::endpoint::SendStream>,
    receive: Option<iroh::endpoint::RecvStream>,
}

impl Raw {
    pub(super) async fn new() -> crate::TestResult<Self> {
        let endpoint = endpoint(true).await?;
        Ok(Self {
            endpoint,
            connection: None,
            send: None,
            receive: None,
        })
    }

    pub(super) async fn connect(&mut self, handoff: &molten::fabric_transport::Endpoint) -> crate::TestResult {
        let protocol = crate::config::protocol(1, "positive")?;
        let connection = self.endpoint.connect(crate::raw::endpoint_addr(handoff)?, protocol.alpn.as_bytes()).await?;
        let (mut send, mut receive) = connection.open_bi().await?;
        let digest = crate::raw::operation_digest(1, "positive")?;
        send.write_all(&digest).await?;
        let mut observed = [0; 32];
        receive.read_exact(&mut observed).await?;
        assert_eq!(observed, digest, "operation/generation contract mismatch");
        self.connection = Some(connection);
        self.send = Some(send);
        self.receive = Some(receive);
        Ok(())
    }

    pub(super) fn send_mut(&mut self) -> crate::TestResult<&mut iroh::endpoint::SendStream> {
        self.send.as_mut().ok_or_else(|| "raw stream not connected".into())
    }

    pub(super) fn receive_mut(&mut self) -> crate::TestResult<&mut iroh::endpoint::RecvStream> {
        self.receive.as_mut().ok_or_else(|| "raw stream not connected".into())
    }

    pub(super) fn connection(&self) -> crate::TestResult<&iroh::endpoint::Connection> {
        self.connection.as_ref().ok_or_else(|| "raw transport not connected".into())
    }

    pub(super) async fn data(&mut self, bytes: &[u8]) -> crate::TestResult {
        crate::raw::header(self.send_mut()?, 1, bytes.len() as u64).await?;
        self.send_mut()?.write_all(blake3::hash(bytes).as_bytes()).await?;
        self.send_mut()?.write_all(bytes).await?;
        Ok(())
    }

    pub(super) async fn data_received(&mut self) -> crate::TestResult<Vec<u8>> {
        let mut header = [0; 9];
        self.receive_mut()?.read_exact(&mut header).await?;
        assert_eq!(header[0], 1);
        let count = u64::from_be_bytes(header[1..].try_into()?);
        assert!(count <= crate::config::FRAME_BYTES);
        let mut digest = [0; 32];
        self.receive_mut()?.read_exact(&mut digest).await?;
        let mut bytes = vec![0; usize::try_from(count)?];
        self.receive_mut()?.read_exact(&mut bytes).await?;
        assert_eq!(blake3::hash(&bytes).as_bytes(), &digest);
        Ok(bytes)
    }

    pub(super) async fn ack(&mut self) -> crate::TestResult<(u64, [u8; 32])> {
        let (kind, sequence) = crate::raw::discard_frame(self.receive_mut()?).await?;
        assert_eq!(kind, 2);
        let mut digest = [0; 32];
        self.receive_mut()?.read_exact(&mut digest).await?;
        Ok((sequence, digest))
    }

    pub(super) async fn send_ack(&mut self, sequence: u64, digest: blake3::Hash) -> crate::TestResult {
        crate::raw::header(self.send_mut()?, 2, sequence).await?;
        self.send_mut()?.write_all(digest.as_bytes()).await?;
        Ok(())
    }

    pub(super) async fn close(self) -> crate::TestResult {
        if let Some(connection) = self.connection {
            connection.close(0_u8.into(), b"fixture-observations-complete");
        }
        tokio::time::timeout(crate::CHILD_BUDGET, self.endpoint.close()).await?;
        Ok(())
    }
}

async fn endpoint(is_client: bool) -> crate::TestResult<iroh::Endpoint> {
    Ok(iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .relay_mode(iroh::RelayMode::Disabled)
        .clear_ip_transports()
        .bind_addr(std::net::SocketAddr::new(std::net::Ipv4Addr::LOCALHOST.into(), 0))?
        .secret_key(iroh::SecretKey::from_bytes(&crate::config::secret(is_client)))
        .alpns(vec![crate::config::protocol(1, "positive")?.alpn.into_bytes()])
        .bind()
        .await?)
}

#[derive(Clone, Debug)]
pub(super) struct Ledger {
    barrier: super::Barrier,
    started: std::time::Instant,
    pub(super) barrier_at: Option<std::time::Instant>,
    pub(super) terminal_observed_at: Option<std::time::Instant>,
    pub(super) interrupted_at: Option<std::time::Instant>,
    pub(super) partial_header_bytes: usize,
    pub(super) submitted_body_bytes: usize,
    pub(super) local_bytes: Vec<u8>,
    pub(super) remote_bytes: Vec<u8>,
    pub(super) stream_fin_observed: bool,
    pub(super) socket_released: bool,
    pub(super) unread_local_bytes_at_release: bool,
    pub(super) transport_closed: bool,
    pub(super) unfinished_resource: Option<&'static str>,
}

impl Ledger {
    pub(super) fn new(barrier: super::Barrier, clock: fn() -> std::time::Instant) -> Self {
        Self {
            started: clock(),
            barrier,
            barrier_at: None,
            terminal_observed_at: None,
            interrupted_at: None,
            partial_header_bytes: 0,
            submitted_body_bytes: 0,
            local_bytes: Vec::new(),
            remote_bytes: Vec::new(),
            stream_fin_observed: false,
            socket_released: false,
            unread_local_bytes_at_release: false,
            transport_closed: false,
            unfinished_resource: None,
        }
    }

    // This oracle consumes socket/wire observations, never Evidence.delivery.
    pub(super) fn validate(&self, inbound: &[u8], outbound: &[u8]) -> crate::TestResult {
        if self.barrier_at.is_none()
            || (self.interrupted_at.is_none() && self.terminal_observed_at.is_none())
            || !self.socket_released
        {
            return Err("missing barrier/interruption observation or protected-socket release".into());
        }
        if !self.transport_closed && self.unfinished_resource.is_none() {
            return Err("unobserved transport was silently counted as cleanup".into());
        }
        let is_progress_retained = match self.barrier {
            super::Barrier::PartialHeader => self.partial_header_bytes == 1,
            super::Barrier::PartialBody => self.submitted_body_bytes == 5,
            super::Barrier::Shutdown => self.stream_fin_observed,
            super::Barrier::LocalWrite | super::Barrier::DelayedAck => true,
        };
        if !is_progress_retained {
            return Err("named barrier lacks independent retained progress".into());
        }
        let expected_local = if matches!(self.barrier, super::Barrier::LocalWrite) {
            inbound
        } else {
            &[]
        };
        let expected_remote = if matches!(self.barrier, super::Barrier::DelayedAck) {
            outbound
        } else {
            &[]
        };
        if self.local_bytes != expected_local || self.remote_bytes != expected_remote {
            return Err("independent payload/progress mismatch (loss or duplicate dispatch)".into());
        }
        Ok(())
    }

    pub(super) fn record(
        &self,
        interrupt: super::Interrupt,
        receipt: Option<&molten::fabric_transport::direct::Evidence>,
    ) {
        println!(
            "direct-stream-cancellation-observation {}",
            serde_json::json!({
                "case": format!("{:?}-{:?}", self.barrier, interrupt), "generation": 1,
                "barrier_after_us": self.barrier_at.map(|at| at.duration_since(self.started).as_micros()),
                "signal_or_drop_requested_after_us": self.interrupted_at.map(|at| at.duration_since(self.started).as_micros()),
                "terminal_observed_after_us": self.terminal_observed_at.map(|at| at.duration_since(self.started).as_micros()),
                "partial_header_bytes_observed": self.partial_header_bytes,
                "partial_body_bytes_submitted_not_proven_consumed": self.submitted_body_bytes,
                "independent_protected_effect_bytes": self.local_bytes.len(),
                "independent_remote_payload_bytes": self.remote_bytes.len(),
                "stream_fin_observed": self.stream_fin_observed, "protected_socket_released": self.socket_released,
                "unread_local_bytes_at_release": self.unread_local_bytes_at_release,
                "independent_transport_close_observed": self.transport_closed,
                "delivery": receipt.map(|value| format!("{:?}", value.delivery)),
                "terminal": receipt.map(|value| format!("{:?}", value.terminal)),
                "local_consumed_bytes": receipt.map(|value| value.local_consumed_bytes),
                "received_bytes": receipt.map(|value| value.received_bytes),
                "cleanup_observed": receipt.map(|value| value.cleanup_observed),
                "unfinished_resource": self.unfinished_resource,
            })
        );
    }
}

fn handoff(endpoint: &iroh::Endpoint) -> crate::TestResult<molten::fabric_transport::Endpoint> {
    let input = crate::config::listener_input(&role())?;
    let address = endpoint.addr();
    let locators = address
        .ip_addrs()
        .map(|address| molten::fabric_transport::EndpointLocator {
            class: molten::fabric_transport::EndpointLocatorClass::Ip,
            value: format!("ip:{address}"),
        })
        .collect();
    let bindings = molten::fabric_transport::EndpointDescriptorBindings {
        public_endpoint_identity: format!("iroh:{}", address.id),
        listener_identity_ref: input.listener_identity_ref,
        expected_peer_context_ref: input.expected_peer_context_ref,
        locator_cohort_ref: input.locator_cohort_ref,
        locators,
        disclosure: input.disclosure,
        resources: molten::fabric_transport::EndpointResourceBounds {
            max_sessions: input.profile.profile.limits.max_sessions,
            max_frame_bytes: input.protocol.framing.max_frame_bytes,
            max_queued_bytes: input.profile.profile.limits.max_queued_bytes,
            max_inflight_bytes: input.profile.profile.limits.max_inflight_bytes,
        },
        validity: input.validity,
    };
    Ok(molten::fabric_transport::endpoint(&input.profile.profile, &input.protocol, &bindings)?)
}

/// Drives the actual outbound cleanup-timeout branch: the raw listener completes
/// the stream but withholds connection close past the admitted cleanup budget.
pub(super) async fn cleanup_negative_control(clock: fn() -> std::time::Instant) -> crate::TestResult {
    let endpoint = endpoint(false).await?;
    let handoff = handoff(&endpoint)?;
    let mut bounds = crate::config::bounds();
    // A smaller cleanup budget is sufficient because the peer explicitly never
    // closes. Operation, idle and memory bounds are unchanged, not inflated.
    bounds.cleanup_timeout_ms = 100;
    let outbound = molten::fabric_transport::direct::Outbound {
        input: crate::config::client_input(&role(), handoff)?,
        peer: crate::config::peer(true, 1)?,
        bounds: bounds.clone(),
        request: crate::config::request(1)?,
        clock,
    };
    let (mut service, mut peer, _events) = service(true)?;
    let (_cancel, cancelled) = tokio::sync::watch::channel(false);
    let operation = molten::fabric_transport::direct::proxy(outbound, &mut service, cancelled);
    let (receipt, (raw, cleanup_started), ()) = tokio::time::timeout(crate::CHILD_BUDGET, async {
        tokio::try_join!(
            async { Ok::<_, Box<dyn std::error::Error + Send + Sync>>(operation.await?) },
            withhold_close(&endpoint, &bounds, clock),
            local_application(&mut peer),
        )
    })
    .await??;
    let terminal_at = clock();
    assert!(terminal_at.duration_since(cleanup_started) < crate::CHILD_BUDGET);
    assert!(receipt.duration_ms >= bounds.cleanup_timeout_ms);
    assert_eq!(receipt.terminal, molten::fabric_transport::DirectStreamTerminal::Clean);
    assert_eq!(receipt.delivery, molten::fabric_transport::DeliveryOutcome::Delivered);
    assert!(!receipt.cleanup_observed, "withheld peer close was counted as cleanup");
    assert_eq!(receipt.local_consumed_bytes, super::OUTBOUND.len() as u64);
    assert_eq!(receipt.sent_frames, 1);
    reject_broken_ledgers(cleanup_started, clock)?;
    println!(
        "direct-stream-cancellation-observation {}",
        serde_json::json!({
            "case": "actual-outbound-cleanup-timeout-negative-controls", "generation": 1,
            "deliberately_broken_progress_detected": true, "deliberately_unaccounted_cleanup_detected": true,
            "stream_completion_not_connection_cleanup": true,
            "delivery": format!("{:?}", receipt.delivery), "terminal": format!("{:?}", receipt.terminal),
            "local_consumed_bytes": receipt.local_consumed_bytes, "cleanup_observed": receipt.cleanup_observed,
            "configured_cleanup_deadline_ms": bounds.cleanup_timeout_ms,
            "terminal_observed_after_cleanup_barrier_us": terminal_at.duration_since(cleanup_started).as_micros(),
            "unfinished_resource": "peer-close withheld at clean stream barrier",
        })
    );
    raw.close().await?;
    tokio::time::timeout(crate::CHILD_BUDGET, endpoint.close()).await?;
    Ok(())
}

/// Completes the raw listener's half of the stream, then retains the real
/// connection without closing it. No fake cleanup future or transport hook.
async fn withhold_close(
    endpoint: &iroh::Endpoint,
    bounds: &molten::fabric_transport::DirectStreamBounds,
    clock: fn() -> std::time::Instant,
) -> crate::TestResult<(Raw, std::time::Instant)> {
    let connection = endpoint.accept().await.ok_or("raw listener closed")?.await?;
    let (send, receive) = connection.accept_bi().await?;
    let mut raw = Raw {
        endpoint: endpoint.clone(),
        connection: Some(connection),
        send: Some(send),
        receive: Some(receive),
    };
    let digest = crate::raw::operation_digest_for_bounds(1, "positive", bounds.clone())?;
    let mut observed = [0; 32];
    raw.receive_mut()?.read_exact(&mut observed).await?;
    assert_eq!(observed, digest);
    raw.send_mut()?.write_all(&digest).await?;
    assert_eq!(raw.data_received().await?, super::OUTBOUND);
    raw.send_ack(1, blake3::hash(super::OUTBOUND)).await?;
    assert_eq!(crate::raw::discard_frame(raw.receive_mut()?).await?, (3, 0));
    raw.send_ack(2, blake3::hash(&[])).await?;
    crate::raw::header(raw.send_mut()?, 3, 0).await?;
    assert_eq!(raw.ack().await?, (1, *blake3::hash(&[]).as_bytes()));
    raw.send_mut()?.finish()?;
    let mut byte = [0; 1];
    assert_eq!(raw.receive_mut()?.read(&mut byte).await?, None);
    assert_eq!(raw.send_mut()?.stopped().await?, None);
    assert!(raw.connection()?.close_reason().is_none(), "stream completion was mistaken for connection cleanup");
    Ok((raw, clock()))
}

async fn local_application(peer: &mut tokio::net::UnixStream) -> crate::TestResult {
    peer.write_all(super::OUTBOUND).await?;
    peer.shutdown().await?;
    let mut bytes = Vec::new();
    (&mut *peer).take(4097).read_to_end(&mut bytes).await?;
    assert!(bytes.is_empty());
    Ok(())
}

/// Broken progress and cleanup records are rejected without calling or
/// reading the production receipt classifier. Both are grounded in the real
/// frame and pending connection the caller observed.
fn reject_broken_ledgers(barrier_at: std::time::Instant, clock: fn() -> std::time::Instant) -> crate::TestResult {
    let mut ledger = Ledger::new(super::Barrier::DelayedAck, clock);
    ledger.barrier_at = Some(barrier_at);
    ledger.terminal_observed_at = Some(clock());
    ledger.remote_bytes = super::OUTBOUND.to_vec();
    ledger.socket_released = true;
    ledger.unfinished_resource = Some("peer-close withheld at clean stream barrier");
    ledger.validate(super::PAYLOAD, super::OUTBOUND)?;
    let mut broken_progress = ledger.clone();
    broken_progress.remote_bytes.clear();
    assert!(broken_progress.validate(super::PAYLOAD, super::OUTBOUND).is_err());
    let mut broken_cleanup = ledger.clone();
    broken_cleanup.unfinished_resource = None;
    assert!(broken_cleanup.validate(super::PAYLOAD, super::OUTBOUND).is_err());
    Ok(())
}
