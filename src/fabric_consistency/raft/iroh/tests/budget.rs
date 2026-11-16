use super::*;

struct DelayedSource {
    source: ScriptedFrames,
    release: Option<tokio::sync::oneshot::Receiver<()>>,
}

impl ReplicaFrameSource for DelayedSource {
    fn poll_frame<'a>(&'a mut self, session_ref: &'a str, timeout: std::time::Duration) -> ReplicaFrameFuture<'a> {
        let release = self.release.take();
        let frame = self.source.poll_frame(session_ref, timeout);
        Box::pin(async move {
            if let Some(release) = release {
                release.await.map_err(|_| crate::error::Failure::invalid_harness("delayed source owner exited"))?;
            }
            frame.await
        })
    }
}

// r[verify molten.consensus.long_running_service_bounds]
#[tokio::test(start_paused = true)]
async fn pending_frame_uses_current_allowance() {
    let polls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (release, held) = tokio::sync::oneshot::channel();
    let source = DelayedSource {
        source: ScriptedFrames {
            frames: ["first", "second"].map(|label| Ok(Ok(Some(scripted_event(label))))).into(),
            polls: std::sync::Arc::clone(&polls),
        },
        release: Some(held),
    };
    let mut pump = ReplicaIngressPump::spawn_source(source, ReplicaIngressConfig {
        session_ref: super::super::tests::test_ref("scripted-session"),
        accept_timeout: std::time::Duration::from_secs(POSITIVE_TIMEOUT_SECONDS),
        event_capacity: INGRESS_TEST_CAPACITY,
        delivery_limit: 1,
        delivery_window: INGRESS_TEST_WINDOW,
    })
    .expect("delayed pump");
    tokio::task::yield_now().await;
    assert_eq!(polls.load(std::sync::atomic::Ordering::SeqCst), 1);
    tokio::time::advance(INGRESS_TEST_WINDOW).await;
    release.send(()).expect("release the pending frame");
    let first = pump.next().await.expect("first delivery after refill");
    assert_eq!(first.transport_evidence.request_ref, super::super::tests::test_ref("first"));
    assert!(tokio::time::timeout(INGRESS_TEST_WINDOW / 2, pump.next()).await.is_err());
    assert_eq!(polls.load(std::sync::atomic::Ordering::SeqCst), 1, "the current allowance was consumed");
    let second = pump.next().await.expect("next refill");
    assert_eq!(second.transport_evidence.request_ref, super::super::tests::test_ref("second"));
}

// r[verify molten.consensus.long_running_service_bounds]
#[tokio::test(start_paused = true)]
async fn shutdown_reclaims_source_while_consumer_queue_is_full() {
    let frames =
        ["first", "second", "third", "fourth", "pending", "untouched"].map(|label| Ok(Ok(Some(scripted_event(label)))));
    let (pump, polls) = scripted_pump(frames.into(), 6);
    tokio::task::yield_now().await;
    assert_eq!(polls.load(std::sync::atomic::Ordering::SeqCst), INGRESS_TEST_CAPACITY + 1);
    let mut source = tokio::time::timeout(INGRESS_TEST_WINDOW / 2, pump.shutdown())
        .await
        .expect("shutdown must not wait for the full consumer queue")
        .expect("source owner returned");
    let Some(Ok(Ok(Some(next)))) = source.frames.pop_front() else {
        panic!("shutdown consumed an untouched source frame");
    };
    assert_eq!(next.transport_evidence.request_ref, super::super::tests::test_ref("untouched"));
}
