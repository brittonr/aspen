use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

#[path = "transfer/fixture.rs"]
mod fixture;

const PAYLOAD: &[u8] = b"protected-transfer-once";
const OUTBOUND: &[u8] = b"retained-local-progress";

#[derive(Clone, Copy, Debug)]
enum Barrier {
    PartialHeader,
    PartialBody,
    LocalWrite,
    DelayedAck,
    Shutdown,
}

#[derive(Clone, Copy, Debug)]
enum Interrupt {
    Cancel,
    Timeout,
    Drop,
}

/// Independent observers that the protected operation does not own.
struct Probe<'a> {
    raw: &'a mut fixture::Raw,
    peer: &'a mut tokio::net::UnixStream,
    events: &'a mut fixture::Events,
    ledger: &'a mut fixture::Ledger,
}

async fn qualify(barrier: Barrier, interrupt: Interrupt, clock: fn() -> std::time::Instant) -> crate::TestResult {
    let mut fixture = fixture::Fixture::new().await?;
    let (cancel, cancelled) = tokio::sync::watch::channel(false);
    let mut ledger = fixture::Ledger::new(barrier, clock);
    let mut raw = fixture::Raw::new().await?;
    let handoff = fixture.listener.handoff().clone();
    let request = crate::config::request(1)?;
    let receipt = {
        let operation = fixture.listener.accept_proxy(request, &mut fixture.service, cancelled);
        tokio::pin!(operation);
        {
            let probe = Probe {
                raw: &mut raw,
                peer: &mut fixture.peer,
                events: &mut fixture.events,
                ledger: &mut ledger,
            };
            let scenario = reach(probe, barrier, &handoff);
            tokio::pin!(scenario);
            tokio::select! {
                result = &mut scenario => result?,
                result = &mut operation => return Err(format!("{barrier:?} ended before its barrier: {result:?}").into()),
                _ = tokio::time::sleep(crate::CHILD_BUDGET) => return Err("transfer barrier exceeded child budget".into()),
            }
        }
        ledger.barrier_at = Some(clock());
        if !matches!(interrupt, Interrupt::Timeout) {
            ledger.interrupted_at = ledger.barrier_at;
        }
        if matches!(interrupt, Interrupt::Cancel) {
            cancel.send(true)?;
        }
        // Exiting this block drops the actual pinned operation owner; otherwise
        // keep polling that same operation through its bounded terminal result.
        if matches!(interrupt, Interrupt::Drop) {
            None
        } else {
            Some(tokio::time::timeout(crate::CHILD_BUDGET, &mut operation).await??)
        }
    };
    // The public future does not expose its internal timer firing instant.
    // Retain terminal-observation time rather than inventing that timestamp.
    ledger.terminal_observed_at = receipt.as_ref().map(|_| clock());
    observe_release(barrier, &mut fixture.peer, &mut ledger).await?;
    if let Some(receipt) = &receipt {
        assert_receipt(receipt, barrier, interrupt, &ledger);
        tokio::time::timeout(crate::CHILD_BUDGET, raw.connection()?.closed()).await?;
        ledger.transport_closed = true;
    } else {
        // Dropping accept_proxy does not close its externally owned listener.
        // Do not infer transport cleanup from protected socket EOF.
        ledger.unfinished_resource = Some("listener-owned Iroh connection/reservation after outer drop");
    }
    ledger.validate(PAYLOAD, OUTBOUND)?;
    ledger.record(interrupt, receipt.as_ref());
    raw.close().await?;
    let cleanup = tokio::time::timeout(
        crate::CHILD_BUDGET,
        fixture.listener.drain_and_close(molten::fabric_transport::ListenerDrainReason::OperatorRequest),
    )
    .await?;
    if matches!(interrupt, Interrupt::Drop) {
        assert!(cleanup.is_err(), "outer drop released an unobserved connection reservation");
    } else {
        cleanup?;
    }
    Ok(())
}

/// Drives the raw peer and local socket to `barrier` while the operation runs.
async fn reach(probe: Probe<'_>, barrier: Barrier, handoff: &molten::fabric_transport::Endpoint) -> crate::TestResult {
    let Probe {
        raw,
        peer,
        events,
        ledger,
    } = probe;
    raw.connect(handoff).await?;
    fixture::opened(events).await?;
    // This check is the first pump-loop observation, after open returned.
    fixture::checked(events).await?;
    match barrier {
        Barrier::PartialHeader => {
            raw.send_mut()?.write_all(&fixture::data_header(PAYLOAD.len())[..1]).await?;
            // No local readiness or other frame can cause the next loop.
            // Its check therefore proves that read() retained one byte.
            fixture::checked(events).await?;
            ledger.partial_header_bytes = 1;
        }
        Barrier::PartialBody => {
            raw.send_mut()?.write_all(&fixture::data_header(PAYLOAD.len())).await?;
            raw.send_mut()?.write_all(blake3::hash(PAYLOAD).as_bytes()).await?;
            raw.send_mut()?.write_all(&PAYLOAD[..5]).await?;
            // QUIC submission is not proof that read_exact consumed five
            // bytes. The incomplete body cannot be dispatched locally.
            ledger.submitted_body_bytes = 5;
        }
        Barrier::LocalWrite => {
            raw.data(PAYLOAD).await?;
            ledger.local_bytes.resize(PAYLOAD.len(), 0);
            peer.read_exact(&mut ledger.local_bytes).await?;
            assert_eq!(ledger.local_bytes, PAYLOAD);
            assert_eq!(raw.ack().await?, (1, *blake3::hash(PAYLOAD).as_bytes()));
        }
        Barrier::DelayedAck => {
            peer.write_all(OUTBOUND).await?;
            ledger.remote_bytes = raw.data_received().await?;
            assert_eq!(ledger.remote_bytes, OUTBOUND);
            // Hold the application ACK. No second local frame can be
            // consumed while this actual hashed frame is outstanding.
            peer.write_all(b"must-remain-unconsumed").await?;
        }
        Barrier::Shutdown => {
            peer.shutdown().await?;
            assert_eq!(crate::raw::discard_frame(raw.receive_mut()?).await?, (3, 0));
            raw.send_ack(1, blake3::hash(&[])).await?;
            crate::raw::header(raw.send_mut()?, 3, 0).await?;
            assert_eq!(raw.ack().await?, (1, *blake3::hash(&[]).as_bytes()));
            let mut byte = [0_u8; 1];
            assert_eq!(peer.read(&mut byte).await?, 0);
            assert_eq!(raw.receive_mut()?.read(&mut byte).await?, None);
            // Both FINs and the final ACK were observed. Withhold QUIC
            // send EOF: the production finish future is still pending.
            ledger.stream_fin_observed = true;
        }
    }
    Ok(())
}

/// The outer-drop case intentionally has no receipt. The protected peer
/// independently observes release of the socket held by the operation.
async fn observe_release(
    barrier: Barrier,
    peer: &mut tokio::net::UnixStream,
    ledger: &mut fixture::Ledger,
) -> crate::TestResult {
    let mut tail = Vec::new();
    let released = tokio::time::timeout(crate::CHILD_BUDGET, (&mut *peer).take(4097).read_to_end(&mut tail)).await?;
    if matches!(barrier, Barrier::DelayedAck) {
        // Linux resets a Unix stream peer whose counterpart closes with unread
        // bytes: the bytes queued behind the unacknowledged frame were never
        // consumed or replayed.
        let error = released.expect_err("bytes queued behind an unacknowledged frame were consumed");
        assert_eq!(error.kind(), std::io::ErrorKind::ConnectionReset);
        ledger.unread_local_bytes_at_release = true;
    } else {
        released?;
    }
    ledger.socket_released = true;
    assert!(tail.is_empty(), "partial frame was dispatched or local bytes were duplicated: {tail:?}");
    Ok(())
}

fn assert_receipt(
    receipt: &molten::fabric_transport::direct::Evidence,
    barrier: Barrier,
    interrupt: Interrupt,
    ledger: &fixture::Ledger,
) {
    assert_eq!(receipt.terminal, match interrupt {
        Interrupt::Cancel => molten::fabric_transport::DirectStreamTerminal::Cancelled,
        Interrupt::Timeout => molten::fabric_transport::DirectStreamTerminal::Timeout,
        Interrupt::Drop => unreachable!(),
    });
    assert_eq!(
        receipt.delivery,
        molten::fabric_transport::DeliveryOutcome::Uncertain,
        "open already crossed the possible-effect boundary"
    );
    assert!(receipt.cleanup_observed);
    assert_eq!(receipt.received_bytes, ledger.local_bytes.len() as u64);
    assert_eq!(receipt.local_consumed_bytes, ledger.remote_bytes.len() as u64);
    assert_eq!(receipt.sent_bytes, ledger.remote_bytes.len() as u64);
    if matches!(barrier, Barrier::DelayedAck) {
        assert_eq!(receipt.sent_frames, 1, "delayed ACK allowed duplicate dispatch or a second consumed frame");
    }
}

// r[verify molten.fabric_transport.direct_stream_bounds]
#[tokio::test]
async fn partial_header_cancel_retains_progress_and_closes_socket() -> crate::TestResult {
    qualify(Barrier::PartialHeader, Interrupt::Cancel, std::time::Instant::now).await
}

#[tokio::test]
async fn partial_header_outer_drop_has_no_terminal_cleanup_claim() -> crate::TestResult {
    qualify(Barrier::PartialHeader, Interrupt::Drop, std::time::Instant::now).await
}

#[tokio::test]
async fn partial_body_timeout_never_dispatches_incomplete_frame() -> crate::TestResult {
    qualify(Barrier::PartialBody, Interrupt::Timeout, std::time::Instant::now).await
}

#[tokio::test]
async fn partial_body_outer_drop_never_dispatches_incomplete_frame() -> crate::TestResult {
    qualify(Barrier::PartialBody, Interrupt::Drop, std::time::Instant::now).await
}

#[tokio::test]
async fn protected_local_write_cancel_preserves_effect_and_uncertainty() -> crate::TestResult {
    qualify(Barrier::LocalWrite, Interrupt::Cancel, std::time::Instant::now).await
}

#[tokio::test]
async fn protected_local_write_outer_drop_releases_actual_socket() -> crate::TestResult {
    qualify(Barrier::LocalWrite, Interrupt::Drop, std::time::Instant::now).await
}

#[tokio::test]
async fn delayed_ack_cancel_retains_consumed_frame_without_replay() -> crate::TestResult {
    qualify(Barrier::DelayedAck, Interrupt::Cancel, std::time::Instant::now).await
}

#[tokio::test]
async fn delayed_ack_outer_drop_retains_unfinished_transport_owner() -> crate::TestResult {
    qualify(Barrier::DelayedAck, Interrupt::Drop, std::time::Instant::now).await
}

#[tokio::test]
async fn stream_shutdown_timeout_is_not_clean_completion() -> crate::TestResult {
    qualify(Barrier::Shutdown, Interrupt::Timeout, std::time::Instant::now).await
}

#[tokio::test]
async fn stream_shutdown_outer_drop_has_no_cleanup_receipt() -> crate::TestResult {
    qualify(Barrier::Shutdown, Interrupt::Drop, std::time::Instant::now).await
}

#[tokio::test]
async fn partial_header_survives_local_readiness_and_clean_completion() -> crate::TestResult {
    let mut fixture = fixture::Fixture::new().await?;
    let (_cancel, cancelled) = tokio::sync::watch::channel(false);
    let mut raw = fixture::Raw::new().await?;
    let handoff = fixture.listener.handoff().clone();
    let operation = fixture.listener.accept_proxy(crate::config::request(1)?, &mut fixture.service, cancelled);
    let scenario = async {
        raw.connect(&handoff).await?;
        fixture::opened(&mut fixture.events).await?;
        fixture::checked(&mut fixture.events).await?;
        let header = fixture::data_header(PAYLOAD.len());
        raw.send_mut()?.write_all(&header[..1]).await?;
        fixture::checked(&mut fixture.events).await?;
        fixture.peer.write_all(OUTBOUND).await?;
        assert_eq!(raw.data_received().await?, OUTBOUND);
        raw.send_mut()?.write_all(&header[1..]).await?;
        raw.send_mut()?.write_all(blake3::hash(PAYLOAD).as_bytes()).await?;
        raw.send_mut()?.write_all(&PAYLOAD[..5]).await?;
        raw.send_mut()?.write_all(&PAYLOAD[5..]).await?;
        let mut observed = vec![0; PAYLOAD.len()];
        fixture.peer.read_exact(&mut observed).await?;
        assert_eq!(observed, PAYLOAD);
        assert_eq!(raw.ack().await?, (1, *blake3::hash(PAYLOAD).as_bytes()));
        raw.send_ack(1, blake3::hash(OUTBOUND)).await?;
        fixture.peer.shutdown().await?;
        assert_eq!(crate::raw::discard_frame(raw.receive_mut()?).await?, (3, 0));
        raw.send_ack(2, blake3::hash(&[])).await?;
        crate::raw::header(raw.send_mut()?, 3, 0).await?;
        assert_eq!(raw.ack().await?, (2, *blake3::hash(&[]).as_bytes()));
        raw.send_mut()?.finish()?;
        let mut tail = Vec::new();
        (&mut fixture.peer).take(4097).read_to_end(&mut tail).await?;
        assert!(tail.is_empty(), "clean transfer duplicated protected bytes");
        let mut byte = [0; 1];
        assert_eq!(raw.receive_mut()?.read(&mut byte).await?, None);
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    };
    let (receipt, ()) = tokio::time::timeout(crate::CHILD_BUDGET, async {
        tokio::try_join!(async { Ok::<_, Box<dyn std::error::Error + Send + Sync>>(operation.await?) }, scenario)
    })
    .await??;
    assert_eq!(receipt.terminal, molten::fabric_transport::DirectStreamTerminal::Clean);
    assert_eq!(receipt.delivery, molten::fabric_transport::DeliveryOutcome::Delivered);
    assert_eq!(receipt.received_bytes, PAYLOAD.len() as u64);
    assert_eq!(receipt.local_consumed_bytes, OUTBOUND.len() as u64);
    assert_eq!((receipt.received_frames, receipt.sent_frames), (1, 1));
    assert!(receipt.cleanup_observed);
    tokio::time::timeout(crate::CHILD_BUDGET, raw.connection()?.closed()).await?;
    println!(
        "direct-stream-cancellation-observation {}",
        serde_json::json!({
            "case": "clean-partial-header-local-readiness", "generation": 1,
            "reached_barriers": ["partial-header", "local-readiness", "protected-write", "ACK", "FIN", "stream-EOF", "connection-close"],
            "independent_protected_effect_bytes": PAYLOAD.len(), "independent_remote_payload_bytes": OUTBOUND.len(),
            "protected_eof": true, "independent_transport_close_observed": true,
            "delivery": format!("{:?}", receipt.delivery), "terminal": format!("{:?}", receipt.terminal),
            "local_consumed_bytes": receipt.local_consumed_bytes, "received_bytes": receipt.received_bytes,
            "cleanup_observed": receipt.cleanup_observed, "unfinished_resources": [],
        })
    );
    raw.close().await?;
    tokio::time::timeout(
        crate::CHILD_BUDGET,
        fixture.listener.drain_and_close(molten::fabric_transport::ListenerDrainReason::OperatorRequest),
    )
    .await??;
    Ok(())
}

#[tokio::test]
async fn cleanup_timeout_and_broken_progress_are_rejected_by_resource_observer() -> crate::TestResult {
    fixture::cleanup_negative_control(std::time::Instant::now).await
}

async fn blocked_write(interrupt: Interrupt, clock: fn() -> std::time::Instant) -> crate::TestResult {
    let started = clock();
    let mut fixture = fixture::Fixture::new().await?;
    let prefill = tokio::time::timeout(crate::CHILD_BUDGET, fixture.service.fill_protected_send_buffer()).await??;
    let mut raw = fixture::Raw::new().await?;
    let handoff = fixture.listener.handoff().clone();
    let (cancel, cancelled) = tokio::sync::watch::channel(false);
    let interrupted_at;
    let receipt = {
        let operation = fixture.listener.accept_proxy(crate::config::request(1)?, &mut fixture.service, cancelled);
        tokio::pin!(operation);
        {
            let barrier = pend_protected_write(&mut raw, &mut fixture.events, &handoff);
            tokio::pin!(barrier);
            tokio::select! {
                result = &mut barrier => result?,
                result = &mut operation => return Err(format!("blocked local write ended early: {result:?}").into()),
                _ = tokio::time::sleep(crate::CHILD_BUDGET) => return Err("blocked-write barrier deadline".into()),
            }
        }
        interrupted_at = clock();
        if matches!(interrupt, Interrupt::Cancel) {
            cancel.send(true)?;
            Some(tokio::time::timeout(crate::CHILD_BUDGET, &mut operation).await??)
        } else {
            None
        }
    };
    let mut observed = Vec::new();
    tokio::time::timeout(
        crate::CHILD_BUDGET,
        (&mut fixture.peer).take(crate::config::SATURATION_BYTES + 1).read_to_end(&mut observed),
    )
    .await??;
    let terminal_at = clock();
    assert_eq!(observed, prefill, "blocked protected write produced unaccounted bytes");
    assert!(terminal_at.duration_since(interrupted_at) < crate::CHILD_BUDGET);
    if let Some(receipt) = &receipt {
        assert_eq!(receipt.terminal, molten::fabric_transport::DirectStreamTerminal::Cancelled);
        assert_eq!(receipt.delivery, molten::fabric_transport::DeliveryOutcome::Uncertain);
        assert_eq!(receipt.received_bytes, PAYLOAD.len() as u64, "potential local effect lost before pending write");
        assert_eq!(receipt.received_frames, 1);
        assert!(receipt.cleanup_observed);
    }
    let after = (interrupted_at.duration_since(started), terminal_at.duration_since(started));
    record_blocked_write(interrupt, after, observed.len(), receipt.as_ref());
    raw.close().await?;
    let cleanup = tokio::time::timeout(
        crate::CHILD_BUDGET,
        fixture.listener.drain_and_close(molten::fabric_transport::ListenerDrainReason::OperatorRequest),
    )
    .await?;
    if matches!(interrupt, Interrupt::Drop) {
        assert!(cleanup.is_err(), "unobserved dropped operation released its reservation");
    } else {
        cleanup?;
    }
    Ok(())
}

/// Delivers one complete frame whose protected local write must then wait for
/// the already full kernel send buffer.
async fn pend_protected_write(
    raw: &mut fixture::Raw,
    events: &mut fixture::Events,
    handoff: &molten::fabric_transport::Endpoint,
) -> crate::TestResult {
    raw.connect(handoff).await?;
    fixture::opened(events).await?;
    fixture::checked(events).await?;
    raw.data(PAYLOAD).await?;
    // Complete digest/body decoding calls current authority immediately
    // before progress accounting and write_all. Full kernel send-buffer
    // backpressure forces that write to yield before any ACK is sent.
    fixture::checked(events).await
}

fn record_blocked_write(
    interrupt: Interrupt,
    after: (std::time::Duration, std::time::Duration),
    prefilled_bytes: usize,
    receipt: Option<&molten::fabric_transport::direct::Evidence>,
) {
    println!(
        "direct-stream-cancellation-observation {}",
        serde_json::json!({
            "case": format!("pending-protected-write-{interrupt:?}"), "generation": 1,
            "signal_or_drop_requested_after_us": after.0.as_micros(),
            "terminal_observed_after_us": after.1.as_micros(),
            "independent_prefilled_bytes": prefilled_bytes, "additional_protected_payload_bytes": 0,
            "protected_eof": true, "possible_protected_write": true,
            "delivery": receipt.map(|value| format!("{:?}", value.delivery)),
            "local_consumed_bytes": receipt.map(|value| value.local_consumed_bytes),
            "received_bytes": receipt.map(|value| value.received_bytes),
            "cleanup_observed": receipt.map(|value| value.cleanup_observed),
            "unfinished_resource": matches!(interrupt, Interrupt::Drop).then_some("quarantined listener reservation"),
        })
    );
}

#[tokio::test]
async fn pending_protected_write_cancel_retains_possible_effect_progress() -> crate::TestResult {
    blocked_write(Interrupt::Cancel, std::time::Instant::now).await
}

#[tokio::test]
async fn pending_protected_write_outer_drop_releases_socket_not_reservation() -> crate::TestResult {
    blocked_write(Interrupt::Drop, std::time::Instant::now).await
}
