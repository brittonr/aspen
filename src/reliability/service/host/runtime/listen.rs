//! The long-running serve loop: timer, peer and bounded client intake, one request at a time.

use tokio::io::AsyncWriteExt as _;

enum Incoming {
    Timer(crate::fabric_consistency::raft::ReplicaEvent),
    Peer(crate::fabric_consistency::raft::ReplicaEvent),
    Client(super::connections::Admitted),
}

impl super::Host {
    /// Receives only in select; effects are awaited outside select and never cancelled midway.
    ///
    /// The host is a long-running service: it stops on an operator stop or teardown request, on
    /// cancellation, or fail-stop when an admitted replica effect fails or Raft ingress ends with a
    /// transport failure. Connection counts and peer traffic volume never end it.
    pub async fn serve(mut self) -> crate::error::Result<()> {
        self.context.authorization.require("serve")?;
        let listener = tokio::net::TcpListener::bind(self.admission.endpoint).await.map_err(super::io_error)?;
        super::super::control::started(&self.context, &self.admission)?;
        let result = self.serve_bound(&listener).await;
        let stop_result = self.apply(crate::fabric_consistency::raft::ReplicaEvent::Stop).await;
        self.replica.ports_mut().time.cancel_all();
        let ingress_result = self.ingress.shutdown().await;
        super::super::control::publish(
            &self.context,
            if result.is_ok() && stop_result.is_ok() && ingress_result.is_ok() {
                "stopped"
            } else {
                "failed"
            },
        )?;
        stop_result?;
        result?;
        ingress_result?;
        Ok(())
    }

    // r[impl molten.consensus.long_running_service_bounds]
    async fn serve_bound(&mut self, listener: &tokio::net::TcpListener) -> crate::error::Result<()> {
        let mut connections =
            super::connections::Intake::new(self.admission.max_connections, std::time::Duration::from_secs(5))?;
        let mut control = tokio::time::interval(std::time::Duration::from_millis(100));
        loop {
            let incoming = tokio::select! {
                event = self.timers.recv() => Incoming::Timer(event.ok_or_else(|| crate::error::Failure::invalid_harness("timer queue closed"))?),
                event = self.ingress.next() => Incoming::Peer(event?.event),
                accepted = listener.accept(), if connections.has_capacity() => {
                    connections.admit(accepted.map_err(super::io_error)?.0)?;
                    continue;
                }
                admitted = connections.next() => Incoming::Client(admitted),
                _ = control.tick() => {
                    if super::super::control::should_stop(&self.context)? { return Ok(()); }
                    continue;
                }
            };
            match incoming {
                // r[impl molten.consensus.raft_timer_lifecycle_safety]
                // A timer armed before a role change or timer reset can still be queued when it is
                // received; it no longer applies, so the host ignores it instead of failing stop.
                Incoming::Timer(event)
                    if crate::fabric_consistency::raft::stale_timer_event(self.replica.state(), &event).is_some() => {}
                Incoming::Timer(event) | Incoming::Peer(event) => {
                    self.apply(event).await?;
                }
                Incoming::Client(admitted) => {
                    if self.connection(admitted).await? {
                        return Ok(());
                    }
                }
            }
            while self.observations.try_recv().is_ok() {}
        }
    }

    /// Answer one fully read request; its connection permit is released when this returns. A
    /// fail-stop latched while serving it ends the host after the response is sent.
    async fn connection(&mut self, admitted: super::connections::Admitted) -> crate::error::Result<bool> {
        let super::connections::Admitted { frame, mut stream, .. } = admitted;
        let Ok(request) = super::parse_request(&frame) else {
            return Ok(false);
        };
        let response = match self.admit(&request) {
            Err(diagnostic) => super::denied(&request, diagnostic),
            Ok(()) => self.execute(&request).await,
        };
        let should_stop = request.phase == super::super::super::protocol::Phase::Teardown
            && response.outcome == super::super::super::protocol::Outcome::Ok;
        let bytes =
            serde_json::to_vec(&response).map_err(|error| crate::error::Failure::invalid_harness(error.to_string()))?;
        // Losing the response is uncertainty for the client, not a server-side rollback.
        let _delivery = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            stream.write_all(&bytes).await?;
            stream.write_all(b"\n").await
        })
        .await;
        if let Some(failure) = self.fail_stop.take() {
            return Err(failure);
        }
        Ok(should_stop)
    }
}
