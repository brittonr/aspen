use crate::*;

pub async fn run(config: config::RoleConfig) -> TestResult {
    let is_client = config.role != "listener";
    if config.role == "raw" {
        let endpoint = read_endpoint(&config.root.join(if config.case == "restart" {
            "old-handoff.preserves"
        } else {
            "handoff.preserves"
        }))?;
        return tokio::time::timeout(std::time::Duration::from_secs(12), raw::run(&config, endpoint)).await?;
    }
    let socket_dir =
        config
            .root
            .join(format!("{}-socket-{}", if is_client { "client" } else { "server" }, config.generation,));
    private_directory(&socket_dir)?;
    let socket_path = socket_dir.join("service.sock");
    let local_listener = protected_socket(&socket_path)?;
    let revoked = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut service = service::SocketService {
        path: socket_path,
        expected: config::request(config.generation)?,
        peer: config::peer(is_client, config.generation)?,
        revoked: revoked.clone(),
        opens: 0,
    };
    if config.case == "scope" && !is_client {
        service.expected.resource_ref = reference("different-protected-resource")?;
    }
    let application = spawn_application(local_listener, &config, is_client, revoked);
    let (cancel, cancellation) = tokio::sync::watch::channel(config.case == "cancel");
    let evidence = proxy(&config, is_client, &mut service, cancellation).await?;
    drop(cancel);
    assert_bounds(&evidence);
    if config.case == "positive" {
        assert_positive(&config, is_client, &service, &evidence);
        tokio::time::timeout(std::time::Duration::from_secs(2), application).await???;
    } else {
        application.abort();
        let _stopped = application.await;
        assert_fault(&config.case, is_client, &service, &evidence)?;
    }
    canonical_output(
        &config.root.join(if is_client {
            "client-evidence.preserves"
        } else {
            "listener-evidence.preserves"
        }),
        &evidence,
    )?;
    Ok(())
}

fn spawn_application(
    listener: tokio::net::UnixListener,
    config: &config::RoleConfig,
    is_client: bool,
    revoked: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> tokio::task::JoinHandle<TestResult> {
    let case = config.case.clone();
    tokio::spawn(async move {
        if case == "ack-loss" && !is_client {
            let (mut socket, _) = listener.accept().await?;
            socket.write_all(config::RESPONSE).await?;
            // Keep the local producer alive until its owner cancels the fault fixture.
            std::future::pending::<()>().await;
            Ok(())
        } else if case == "revoked" && !is_client {
            let (mut socket, _) = listener.accept().await?;
            let mut bytes = Vec::new();
            (&mut socket).take(MAX_ARTIFACT_BYTES).read_to_end(&mut bytes).await?;
            assert_eq!(bytes, config::REQUEST);
            revoked.store(true, std::sync::atomic::Ordering::SeqCst);
            socket.write_all(config::RESPONSE).await?;
            socket.shutdown().await?;
            Ok(())
        } else {
            service::application_exchange(listener, is_client, config::REQUEST, config::RESPONSE).await
        }
    })
}

async fn proxy(
    config: &config::RoleConfig,
    is_client: bool,
    service: &mut service::SocketService,
    cancellation: tokio::sync::watch::Receiver<bool>,
) -> TestResult<molten::fabric_transport::direct::Evidence> {
    if is_client {
        let endpoint = read_endpoint(&config.root.join("handoff.preserves"))?;
        return Ok(molten::fabric_transport::direct::proxy(
            molten::fabric_transport::direct::Outbound {
                input: config::client_input(config, endpoint)?,
                peer: config::peer(true, config.client_generation)?,
                bounds: config::bounds(),
                request: config::request(config.client_generation)?,
                clock: std::time::Instant::now,
            },
            service,
            cancellation,
        )
        .await?);
    }
    let mut peer = config::peer(false, config.generation)?;
    if config.case == "wrong-peer" {
        peer.endpoint_id = iroh::SecretKey::from_bytes(&[0x65; 32]).public().to_string();
    }
    let mut listener = molten::fabric_transport::direct::Acceptor::bind(molten::fabric_transport::direct::Config {
        listener: config::listener_input(config)?,
        peer,
        bounds: config::bounds(),
        receive_budget: molten::fabric_transport::ReceiveBudget {
            burst: 16,
            refill_ticks: 100,
            max_sources: 4,
        },
        clock: std::time::Instant::now,
    })
    .await?;
    atomic_value(&config.root.join("handoff.preserves"), &listener.handoff().value)?;
    if config.case == "drain" {
        listener.begin_drain()?;
    }
    let result = listener.accept_proxy(config::request(config.generation)?, service, cancellation).await;
    let _cleanup = listener.drain_and_close(molten::fabric_transport::ListenerDrainReason::OperatorRequest).await?;
    Ok(result?)
}

fn assert_bounds(evidence: &molten::fabric_transport::direct::Evidence) {
    assert_eq!(evidence.automatic_retry_count, 0, "base stream must never redial/replay");
    assert!(evidence.sent_bytes <= config::bounds().max_total_bytes);
    assert!(evidence.received_bytes <= config::bounds().max_total_bytes);
    assert!(evidence.sent_frames <= config::bounds().max_frames);
    assert!(evidence.received_frames <= config::bounds().max_frames);
    assert!(evidence.cleanup_observed, "terminal evidence omitted owned transport cleanup");
}

fn assert_positive(
    config: &config::RoleConfig,
    is_client: bool,
    service: &service::SocketService,
    evidence: &molten::fabric_transport::direct::Evidence,
) {
    assert_eq!(evidence.terminal, molten::fabric_transport::DirectStreamTerminal::Clean);
    assert_eq!(evidence.delivery, molten::fabric_transport::DeliveryOutcome::Delivered);
    assert_eq!(service.opens, 1);
    let (sent, received) = if is_client {
        (config::REQUEST.len(), config::RESPONSE.len())
    } else {
        (config::RESPONSE.len(), config::REQUEST.len())
    };
    assert_eq!(evidence.sent_bytes, sent as u64);
    assert_eq!(evidence.received_bytes, received as u64);
    assert!(evidence.sent_frames > 1);
    assert!(evidence.received_frames > 1);
    let expected_path = if config.bind_ip.is_loopback() {
        molten::fabric_transport::DirectStreamPath::DirectLoopback
    } else {
        molten::fabric_transport::DirectStreamPath::DirectIp
    };
    assert_eq!(evidence.actual_path, expected_path);
}

fn assert_fault(
    case: &str,
    is_client: bool,
    service: &service::SocketService,
    evidence: &molten::fabric_transport::direct::Evidence,
) -> TestResult {
    if is_client {
        assert_ne!(evidence.terminal, molten::fabric_transport::DirectStreamTerminal::Clean);
        return Ok(());
    }
    match case {
        "wrong-peer" | "stale" | "scope" | "restart" => {
            assert_eq!(evidence.terminal, molten::fabric_transport::DirectStreamTerminal::Denied);
            assert_eq!(service.opens, 0, "denied peer/operation opened protected socket");
        }
        "revoked" => {
            assert_eq!(evidence.terminal, molten::fabric_transport::DirectStreamTerminal::Denied);
            assert_eq!(service.opens, 1, "revocation must fence an already admitted operation");
        }
        "cancel" => {
            assert_eq!(evidence.terminal, molten::fabric_transport::DirectStreamTerminal::Cancelled);
            assert_eq!(service.opens, 0);
        }
        "drain" => {
            assert_eq!(evidence.terminal, molten::fabric_transport::DirectStreamTerminal::Denied);
            assert_eq!(service.opens, 0);
        }
        "alpn" => {
            assert_ne!(evidence.terminal, molten::fabric_transport::DirectStreamTerminal::Clean);
            assert_eq!(service.opens, 0);
        }
        "over-bound" => assert_eq!(evidence.terminal, molten::fabric_transport::DirectStreamTerminal::OverBound),
        "slow" | "ack-loss" => assert_eq!(evidence.terminal, molten::fabric_transport::DirectStreamTerminal::Timeout),
        "disconnect" => assert_eq!(evidence.terminal, molten::fabric_transport::DirectStreamTerminal::Disconnected),
        "concurrent" => {
            assert_eq!(evidence.terminal, molten::fabric_transport::DirectStreamTerminal::Disconnected);
            assert_eq!(service.opens, 1, "excess connection opened a second protected socket");
        }
        _ => return Err("unknown qualification case".into()),
    }
    if case == "ack-loss" {
        assert!(evidence.local_consumed_bytes > 0, "ack loss must follow real local byte consumption");
        assert_eq!(evidence.delivery, molten::fabric_transport::DeliveryOutcome::Uncertain);
    }
    Ok(())
}
