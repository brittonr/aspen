use tokio::io::AsyncReadExt;

struct Abandoned {
    listener: molten::fabric_transport::direct::Acceptor,
    service: super::service::Service,
    started: std::time::Instant,
    dropped_after: std::time::Duration,
    client: molten::fabric_transport::direct::Evidence,
}

async fn abandon_at_protected_effect(clock: fn() -> std::time::Instant) -> crate::TestResult<Abandoned> {
    let super::Pair {
        mut listener,
        outbound,
        mut listener_service,
        mut client_service,
        mut listener_peer,
        client_peer,
    } = super::pair(super::service::OpenAction::HoldAfterEffect).await?;
    let (_listener_owner, listener_cancel) = tokio::sync::watch::channel(false);
    let (_client_owner, client_cancel) = tokio::sync::watch::channel(false);
    let started = clock();
    let client_run = molten::fabric_transport::direct::proxy(outbound, &mut client_service, client_cancel);
    tokio::pin!(client_run);
    {
        let listener_run = listener.accept_proxy(crate::config::request(1)?, &mut listener_service, listener_cancel);
        tokio::pin!(listener_run);
        let mut observed = [0_u8; 1];
        tokio::select! {
            effect = tokio::time::timeout(crate::CHILD_BUDGET, listener_peer.read_exact(&mut observed)) => {
                effect??;
                assert_eq!(observed, [super::service::OPEN_EFFECT]);
            }
            _ = &mut listener_run => return Err("listener ended before the outer-drop barrier".into()),
            _ = &mut client_run => return Err("client ended before the outer-drop barrier".into()),
        }
        // The owned future, not just a cancellation signal or JoinHandle, is
        // dropped here. No post-select cleanup/evidence code gets to run.
    }
    let dropped_after = clock().duration_since(started);
    let mut remaining = [0_u8; 1];
    assert_eq!(
        tokio::time::timeout(crate::CHILD_BUDGET, listener_peer.read(&mut remaining)).await??,
        0,
        "dropped open retained its protected fd or replayed the observed byte"
    );
    let client = tokio::time::timeout(crate::CHILD_BUDGET, &mut client_run).await??;
    drop(client_peer);
    Ok(Abandoned {
        listener,
        service: listener_service,
        started,
        dropped_after,
        client,
    })
}

// r[verify molten.fabric_transport.direct_stream_bounds]
#[tokio::test]
async fn dropped_outer_open_quarantines_capacity_and_rejects_stale_generation() -> crate::TestResult {
    let Abandoned {
        mut listener,
        mut service,
        started,
        dropped_after,
        client,
    } = abandon_at_protected_effect(std::time::Instant::now).await?;
    let next_outbound = super::outbound(&listener)?;
    let (socket, _peer) = tokio::net::UnixStream::pair()?;
    let mut next_service = super::service::Service::new(socket, true, super::service::OpenAction::Ready)?;
    let (_listener_owner, listener_cancel) = tokio::sync::watch::channel(false);
    let (_client_owner, client_cancel) = tokio::sync::watch::channel(false);
    let request = crate::config::request(1)?;
    let retry = async {
        tokio::join!(
            listener.accept_proxy(request, &mut service, listener_cancel),
            molten::fabric_transport::direct::proxy(next_outbound, &mut next_service, client_cancel),
        )
    };
    let (retry_listener, retry_client) = tokio::time::timeout(crate::CHILD_BUDGET, retry).await?;
    let retry_listener = retry_listener?;
    let retry_client = retry_client?;
    assert_eq!(
        retry_listener.terminal,
        molten::fabric_transport::DirectStreamTerminal::Denied,
        "abandonment released capacity before terminal cleanup"
    );
    assert_eq!(retry_listener.delivery, molten::fabric_transport::DeliveryOutcome::NotDelivered);
    assert!(retry_client.cleanup_observed);
    let (_stale_owner, stale_cancel) = tokio::sync::watch::channel(false);
    let stale = listener
        .accept_proxy(crate::config::request(2)?, &mut service, stale_cancel)
        .await
        .expect_err("new-generation request entered an abandoned old listener");
    assert!(matches!(stale, molten::fabric::FabricPortError::MalformedObservation { .. }));
    let cleanup = listener
        .drain_and_close(molten::fabric_transport::ListenerDrainReason::OperatorRequest)
        .await
        .expect_err("an abandoned reservation was reported as cleanly closed");
    assert!(matches!(cleanup, molten::fabric::FabricPortError::MalformedObservation { .. }));
    println!(
        "direct-stream-cancellation-observation {}",
        serde_json::json!({
            "case": "outer-drop-during-open", "generation": 1,
            "dropped_after_us": dropped_after.as_micros(), "terminal_observed_after_us": started.elapsed().as_micros(),
            "independent_protected_effect_bytes": 1, "protected_eof": true,
            "terminal_receipt": null, "cleanup_observed": false,
            "peer_delivery": format!("{:?}", client.delivery), "peer_cleanup_observed": client.cleanup_observed,
            "retry_terminal": format!("{:?}", retry_listener.terminal),
            "stale_generation_rejected": true, "unfinished_resources": ["quarantined-listener-reservation"],
        })
    );
    Ok(())
}
