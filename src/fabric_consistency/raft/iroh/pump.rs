use super::*;

impl<S: ReplicaFrameSource> ReplicaIngressPump<S> {
    /// Pump frames from `source` until cancellation or a transport failure. Deliveries draw from a
    /// budget of `delivery_limit` per `delivery_window`; a spent budget stops polling until the
    /// window ends and then refills. Undecodable frames are dropped, counted and logged.
    // r[impl molten.consensus.long_running_service_bounds]
    pub fn spawn_source(source: S, config: ReplicaIngressConfig) -> crate::error::Result<Self> {
        validate_ingress_config(&config)?;
        let (event_sender, events) = tokio::sync::mpsc::channel(config.event_capacity);
        let (cancel, mut cancellation) = tokio::sync::oneshot::channel();
        let dropped_frames = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        let dropped = std::sync::Arc::clone(&dropped_frames);
        let task = tokio::spawn(async move {
            let mut source = source;
            let mut budget = config.delivery_limit;
            let mut window = tokio::time::interval(config.delivery_window);
            window.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            // The first tick establishes the window without spending an interval.
            window.tick().await;
            loop {
                if budget == 0 {
                    tokio::select! {
                        _ = &mut cancellation => return Ok(source),
                        _ = window.tick() => {
                            budget = config.delivery_limit;
                            continue;
                        }
                    }
                }
                let polled = tokio::select! {
                    _ = &mut cancellation => return Ok(source),
                    polled = source.poll_frame(&config.session_ref, config.accept_timeout) => polled?,
                };
                match polled {
                    Ok(Some(event)) => {
                        tokio::select! {
                            _ = &mut cancellation => return Ok(source),
                            sent = event_sender.send(event) => {
                                sent.map_err(|_| crate::error::Failure::invalid_harness("live Raft ingress consumer closed"))?;
                            }
                        }
                        // A pending read or queue admission may have crossed the refill boundary.
                        if std::future::poll_fn(|cx| std::task::Poll::Ready(window.poll_tick(cx).is_ready())).await {
                            budget = config.delivery_limit;
                        }
                        budget -= 1;
                    }
                    Err(error) => {
                        let count = dropped.fetch_add(1, std::sync::atomic::Ordering::Relaxed).saturating_add(1);
                        tracing::warn!(
                            target: "molten_fabric_consistency",
                            session_ref = %config.session_ref,
                            dropped_frames = count,
                            error = %error,
                            "dropped undecodable live Raft ingress frame"
                        );
                    }
                    Ok(None) => {}
                }
            }
        });
        Ok(Self {
            events,
            cancel: Some(cancel),
            task: Some(task),
            dropped_frames,
        })
    }

    pub async fn next(&mut self) -> crate::error::Result<ReceivedReplicaEvent> {
        self.events
            .recv()
            .await
            .ok_or_else(|| crate::error::Failure::invalid_harness("live Raft ingress pump terminated before delivery"))
    }

    /// Undecodable frames dropped since the pump started.
    pub fn dropped_frames(&self) -> u64 {
        self.dropped_frames.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub async fn shutdown(mut self) -> crate::error::Result<S> {
        let cancel = self
            .cancel
            .take()
            .ok_or_else(|| crate::error::Failure::invalid_harness("live Raft ingress cancellation handle is absent"))?;
        let _cancel_result = cancel.send(());
        let task = self
            .task
            .take()
            .ok_or_else(|| crate::error::Failure::invalid_harness("live Raft ingress task handle is absent"))?;
        task.await.map_err(|error| {
            crate::error::Failure::invalid_harness(format!("live Raft ingress task failed: {error}"))
        })?
    }
}

impl<S: ReplicaFrameSource> Drop for ReplicaIngressPump<S> {
    fn drop(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            let _cancel_result = cancel.send(());
        }
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}
