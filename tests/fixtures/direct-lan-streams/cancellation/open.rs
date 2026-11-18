use tokio::io::AsyncReadExt;

#[derive(Clone, Copy)]
enum Interruption {
    Cancel,
    IdleDeadline,
    ServiceError,
}

impl Interruption {
    fn terminal(self) -> molten::fabric_transport::DirectStreamTerminal {
        match self {
            Self::Cancel => molten::fabric_transport::DirectStreamTerminal::Cancelled,
            Self::IdleDeadline => molten::fabric_transport::DirectStreamTerminal::Timeout,
            Self::ServiceError => molten::fabric_transport::DirectStreamTerminal::Disconnected,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Cancel => "open-cancel",
            Self::IdleDeadline => "open-idle-deadline",
            Self::ServiceError => "open-error-after-effect",
        }
    }

    fn action(self) -> super::service::OpenAction {
        match self {
            Self::ServiceError => super::service::OpenAction::FailAfterEffect,
            Self::Cancel | Self::IdleDeadline => super::service::OpenAction::HoldAfterEffect,
        }
    }
}

/// Independent observations of one interrupted protected open.
struct Observed {
    effect_at: std::time::Duration,
    cancellation_at: Option<std::time::Duration>,
    terminal_at: std::time::Duration,
    is_protected_eof: bool,
    listener: molten::fabric_transport::direct::Evidence,
    client: molten::fabric_transport::direct::Evidence,
    cleanup_terminal: String,
}

// r[verify molten.fabric_transport.direct_stream_bounds]
#[tokio::test]
async fn interrupted_open_never_reports_definite_non_delivery() -> crate::TestResult {
    interrupted_open(Interruption::Cancel, std::time::Instant::now).await
}

// r[verify molten.fabric_transport.direct_stream_bounds]
#[tokio::test]
async fn timed_out_open_retains_possible_protected_effects() -> crate::TestResult {
    interrupted_open(Interruption::IdleDeadline, std::time::Instant::now).await
}

// r[verify molten.fabric_transport.direct_stream_bounds]
#[tokio::test]
async fn failed_open_retains_observed_protected_effects() -> crate::TestResult {
    interrupted_open(Interruption::ServiceError, std::time::Instant::now).await
}

async fn interrupted_open(interruption: Interruption, clock: fn() -> std::time::Instant) -> crate::TestResult {
    let super::Pair {
        mut listener,
        outbound,
        mut listener_service,
        mut client_service,
        mut listener_peer,
        client_peer,
    } = super::pair(interruption.action()).await?;
    let release = listener_service.release();
    let (cancel_listener, listener_cancel) = tokio::sync::watch::channel(false);
    let (cancel_client, client_cancel) = tokio::sync::watch::channel(false);
    let started = clock();
    let effect_at;
    let mut cancellation_at = None;
    let (listener_result, client_result) = {
        let listener_run = listener.accept_proxy(crate::config::request(1)?, &mut listener_service, listener_cancel);
        let client_run = molten::fabric_transport::direct::proxy(outbound, &mut client_service, client_cancel);
        let operation = async { tokio::join!(listener_run, client_run) };
        tokio::pin!(operation);
        let mut observed = [0_u8; 1];
        tokio::select! {
            effect = tokio::time::timeout(crate::CHILD_BUDGET, listener_peer.read_exact(&mut observed)) => {
                effect??;
                assert_eq!(observed, [super::service::OPEN_EFFECT]);
                effect_at = clock().duration_since(started);
            }
            _ = &mut operation => return Err("operation ended before the protected-effect barrier".into()),
        }
        match interruption {
            Interruption::Cancel => {
                cancellation_at = Some(clock().duration_since(started));
                cancel_listener.send(true)?;
            }
            Interruption::ServiceError => release.notify_one(),
            Interruption::IdleDeadline => {}
        }
        tokio::time::timeout(crate::CHILD_BUDGET, &mut operation).await?
    };
    let terminal_at = clock().duration_since(started);
    drop(cancel_client);
    drop(client_peer);
    let listener_result = listener_result?;
    let client_result = client_result?;
    let mut remaining = [0_u8; 1];
    let count = tokio::time::timeout(crate::CHILD_BUDGET, listener_peer.read(&mut remaining)).await??;
    let cleanup = listener.drain_and_close(molten::fabric_transport::ListenerDrainReason::OperatorRequest).await?;
    assert_interrupted_open(interruption, &Observed {
        effect_at,
        cancellation_at,
        terminal_at,
        is_protected_eof: count == 0,
        listener: listener_result,
        client: client_result,
        cleanup_terminal: format!("{:?}", cleanup.terminal_class),
    });
    Ok(())
}

fn assert_interrupted_open(interruption: Interruption, observed: &Observed) {
    println!(
        "direct-stream-cancellation-observation {}",
        serde_json::json!({
            "case": interruption.label(), "generation": 1,
            "effect_barrier_after_us": observed.effect_at.as_micros(),
            "cancellation_sent_after_us": observed.cancellation_at.map(|at| at.as_micros()),
            "configured_idle_deadline_ms": crate::config::bounds().idle_timeout_ms,
            "terminal_observed_after_us": observed.terminal_at.as_micros(),
            "independent_protected_effect_bytes": 1, "protected_eof": observed.is_protected_eof,
            "delivery": format!("{:?}", observed.listener.delivery),
            "terminal": format!("{:?}", observed.listener.terminal),
            "local_consumed_bytes": observed.listener.local_consumed_bytes,
            "received_bytes": observed.listener.received_bytes,
            "cleanup_observed": observed.listener.cleanup_observed,
            "peer_cleanup_observed": observed.client.cleanup_observed,
            "listener_cleanup_terminal": observed.cleanup_terminal,
            "unfinished_resources": [],
        })
    );
    assert!(observed.is_protected_eof, "interrupted open retained its protected socket or wrote duplicate bytes");
    assert!(observed.listener.cleanup_observed);
    assert!(observed.client.cleanup_observed);
    assert_eq!(observed.listener.terminal, interruption.terminal());
    assert_eq!(observed.listener.local_consumed_bytes, 0);
    assert_eq!(observed.listener.received_bytes, 0);
    assert_eq!(
        observed.listener.delivery,
        molten::fabric_transport::DeliveryOutcome::Uncertain,
        "the independent protected peer observed a write before open completed"
    );
}

// r[verify molten.fabric_transport.direct_stream_bounds]
#[tokio::test]
async fn cancellation_before_admission_retains_known_non_delivery() -> crate::TestResult {
    let super::Pair {
        mut listener,
        mut listener_service,
        mut listener_peer,
        ..
    } = super::pair(super::service::OpenAction::HoldAfterEffect).await?;
    let (_owner, cancel) = tokio::sync::watch::channel(true);
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(
        crate::CHILD_BUDGET,
        listener.accept_proxy(crate::config::request(1)?, &mut listener_service, cancel),
    )
    .await??;
    let mut byte = [0_u8; 1];
    assert_eq!(
        listener_peer.try_read(&mut byte).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock,
        "pre-admission cancellation reached the protected peer"
    );
    drop(listener_service);
    assert_eq!(tokio::time::timeout(crate::CHILD_BUDGET, listener_peer.read(&mut byte)).await??, 0);
    let cleanup = listener.drain_and_close(molten::fabric_transport::ListenerDrainReason::OperatorRequest).await?;
    println!(
        "direct-stream-cancellation-observation {}",
        serde_json::json!({
            "case": "before-admission-cancel", "generation": 1, "cancellation_sent_after_us": 0,
            "terminal_observed_after_us": started.elapsed().as_micros(),
            "independent_protected_effect_bytes": 0, "protected_eof": true,
            "delivery": format!("{:?}", result.delivery), "terminal": format!("{:?}", result.terminal),
            "local_consumed_bytes": result.local_consumed_bytes, "received_bytes": result.received_bytes,
            "cleanup_observed": result.cleanup_observed,
            "listener_cleanup_terminal": format!("{:?}", cleanup.terminal_class), "unfinished_resources": [],
        })
    );
    assert_eq!(result.terminal, molten::fabric_transport::DirectStreamTerminal::Cancelled);
    assert_eq!(result.delivery, molten::fabric_transport::DeliveryOutcome::NotDelivered);
    assert!(result.cleanup_observed);
    Ok(())
}
