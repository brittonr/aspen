use tokio::io::AsyncBufReadExt as _;
use tokio::io::AsyncReadExt as _;
use tokio::io::AsyncWriteExt as _;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostAdmission {
    pub run_id: String,
    pub endpoint: std::net::SocketAddr,
    pub max_connections: u64,
    pub peers: std::collections::BTreeMap<String, std::net::SocketAddr>,
    pub profile: super::super::profile::Register,
}

impl HostAdmission {
    pub fn validate(&self) -> crate::error::Result<()> {
        super::super::profile::admit(&self.profile)
            .map_err(|issue| crate::error::Failure::invalid_harness(format!("invalid registry workload: {issue:?}")))?;
        if self.run_id.is_empty()
            || self.run_id.len() > 256
            || self.endpoint.port() == 0
            || !(1..=65_536).contains(&self.max_connections)
        {
            return Err(crate::error::Failure::invalid_harness("invalid bounded public registry host admission"));
        }
        if self.peers.len() != 3
            || self.peers.values().any(|endpoint| endpoint.port() == 0)
            || self.peers.values().collect::<std::collections::BTreeSet<_>>().len() != 3
            || self.profile.endpoints.iter().cloned().collect::<std::collections::BTreeSet<_>>()
                != self.peers.values().map(ToString::to_string).collect()
        {
            return Err(crate::error::Failure::invalid_harness(
                "registry public endpoints do not match exact three-voter profile",
            ));
        }
        Ok(())
    }
}

pub struct Host {
    pub(super) admission: HostAdmission,
    pub(super) replica: crate::fabric_consistency::raft::ScopedLiveReplica<super::launch::Ports>,
    pub(super) ingress: crate::fabric_consistency::raft::ReplicaIngressPump,
    pub(super) timers: tokio::sync::mpsc::Receiver<crate::fabric_consistency::raft::ReplicaEvent>,
    pub(super) observations: tokio::sync::mpsc::Receiver<crate::fabric_consistency::raft::ReplicaObservation>,
    pub(super) context: super::session::Context,
}

enum Incoming {
    Timer(crate::fabric_consistency::raft::ReplicaEvent),
    Peer(crate::fabric_consistency::raft::ReplicaEvent),
    Client(tokio::net::TcpStream),
}

impl Host {
    pub(super) fn new(
        admission: HostAdmission,
        replica: crate::fabric_consistency::raft::ScopedLiveReplica<super::launch::Ports>,
        ingress: crate::fabric_consistency::raft::ReplicaIngressPump,
        channels: (
            tokio::sync::mpsc::Receiver<crate::fabric_consistency::raft::ReplicaEvent>,
            tokio::sync::mpsc::Receiver<crate::fabric_consistency::raft::ReplicaObservation>,
        ),
        context: super::session::Context,
    ) -> Self {
        Self {
            admission,
            replica,
            ingress,
            timers: channels.0,
            observations: channels.1,
            context,
        }
    }
    pub fn observation(&self) -> crate::error::Result<super::diagnostics::HostObservation> {
        super::diagnostics::observe(&self.replica, &self.context.source_gate, self.admission.endpoint)
    }

    /// Receives only in select; effects are awaited outside select and never cancelled midway.
    pub async fn serve(mut self) -> crate::error::Result<()> {
        self.context.authorization.require("serve")?;
        let listener = tokio::net::TcpListener::bind(self.admission.endpoint).await.map_err(io_error)?;
        super::control::started(&self.context, &self.admission)?;
        let result = self.serve_bound(&listener).await;
        let stop_result = self.apply(crate::fabric_consistency::raft::ReplicaEvent::Stop).await;
        self.replica.ports_mut().time.cancel_all();
        let ingress_result = self.ingress.shutdown().await;
        super::control::publish(
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

    async fn serve_bound(&mut self, listener: &tokio::net::TcpListener) -> crate::error::Result<()> {
        let mut connections = 0;
        let mut control = tokio::time::interval(std::time::Duration::from_millis(100));
        loop {
            let incoming = tokio::select! {
                event = self.timers.recv() => Incoming::Timer(event.ok_or_else(|| crate::error::Failure::invalid_harness("timer queue closed"))?),
                event = self.ingress.next() => Incoming::Peer(event?.event),
                accepted = listener.accept(), if connections < self.admission.max_connections => Incoming::Client(accepted.map_err(io_error)?.0),
                _ = control.tick() => {
                    if super::control::should_stop(&self.context)? { return Ok(()); }
                    continue;
                }
            };
            match incoming {
                Incoming::Timer(event) | Incoming::Peer(event) => {
                    self.apply(event).await?;
                }
                Incoming::Client(stream) => {
                    connections += 1;
                    if self.connection(stream).await? {
                        return Ok(());
                    }
                    if connections == self.admission.max_connections {
                        return Err(crate::error::Failure::invalid_harness(
                            "public registry connection bound exhausted",
                        ));
                    }
                }
            }
            while self.observations.try_recv().is_ok() {}
        }
    }

    async fn connection(&mut self, mut stream: tokio::net::TcpStream) -> crate::error::Result<bool> {
        let request = match self.read_request(&mut stream).await {
            Some(request) => request,
            None => return Ok(false),
        };
        let response = match self.admit(&request) {
            Err(diagnostic) => denied(&request, diagnostic),
            Ok(()) => self.execute(&request).await,
        };
        let should_stop = request.phase == super::super::protocol::Phase::Teardown
            && response.outcome == super::super::protocol::Outcome::Ok;
        let bytes =
            serde_json::to_vec(&response).map_err(|error| crate::error::Failure::invalid_harness(error.to_string()))?;
        // Losing the response is uncertainty for the client, not a server-side rollback.
        let _delivery = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            stream.write_all(&bytes).await?;
            stream.write_all(b"\n").await
        })
        .await;
        Ok(should_stop)
    }
    async fn read_request(&mut self, stream: &mut tokio::net::TcpStream) -> Option<super::super::protocol::Request> {
        let incoming = receive(stream);
        tokio::pin!(incoming);
        let expiry = tokio::time::sleep(std::time::Duration::from_secs(5));
        tokio::pin!(expiry);
        loop {
            tokio::select! {
                request = &mut incoming => return request.ok(),
                _ = &mut expiry => return None,
                event = self.timers.recv() => { self.apply(event?).await.ok()?; }
                event = self.ingress.next() => { self.apply(event.ok()?.event).await.ok()?; }
            }
        }
    }

    fn admit(&self, request: &super::super::protocol::Request) -> std::result::Result<(), String> {
        let endpoint = super::super::protocol::validate_request(request)?;
        if request.run_id != self.admission.run_id || !self.admission.peers.values().any(|peer| peer == &endpoint) {
            return Err("request-outside-admitted-host".into());
        }
        if request.profile.as_ref() != Some(&self.admission.profile) {
            return Err("request-profile-substitution".into());
        }
        if let Some(operation) = &request.operation
            && operation.get("object_key").and_then(|value| value.as_str())
                != Some(self.replica.ports().application.handler().object_key())
        {
            return Err("request-outside-admitted-key".into());
        }
        self.context.require_current("serve").map_err(|error| error.to_string())?;
        self.context.authorization.request(request).map_err(|error| error.to_string())?;
        Ok(())
    }

    async fn execute(&mut self, request: &super::super::protocol::Request) -> super::super::protocol::Response {
        if request.phase == super::super::protocol::Phase::Teardown {
            return success(request, serde_json::Value::Null);
        }
        if request.phase == super::super::protocol::Phase::Probe {
            return self.probe(request).await;
        }
        if self.replica.state().role != crate::fabric_consistency::raft::ReplicaRole::Leader {
            return self.forward(request).await;
        }
        if request.phase == super::super::protocol::Phase::Recover {
            return self.recover(request).await;
        }
        if request.phase == super::super::protocol::Phase::Setup {
            // Readiness uses the same quorum-current public read as normal operations.
            return self.perform(request, false).await;
        }
        let is_write =
            request.operation.as_ref().and_then(|value| value.get("function")).and_then(|value| value.as_str())
                == Some("write");
        self.perform(request, is_write).await
    }
    async fn forward(&mut self, request: &super::super::protocol::Request) -> super::super::protocol::Response {
        let endpoint = self.replica.state().leader_id.as_ref().and_then(|node| self.admission.peers.get(node)).copied();
        let Some(endpoint) = endpoint else {
            return super::super::protocol::uncertain(request, "leader-not-observed".into());
        };
        if endpoint == self.admission.endpoint {
            return super::super::protocol::uncertain(request, "leader-not-current".into());
        }
        self.exchange_at(request, endpoint).await
    }

    pub(super) async fn apply(
        &mut self,
        event: crate::fabric_consistency::raft::ReplicaEvent,
    ) -> crate::error::Result<()> {
        self.context.require_current("serve")?;
        match self.replica.handle_event(event).await {
            crate::fabric_consistency::raft::ReplicaExecutionOutcome::Applied(_) => Ok(()),
            crate::fabric_consistency::raft::ReplicaExecutionOutcome::Denied { diagnostic, .. } => {
                Err(crate::error::Failure::invalid_harness(diagnostic))
            }
            crate::fabric_consistency::raft::ReplicaExecutionOutcome::Failed(failure) => {
                Err(crate::error::Failure::invalid_harness(failure.diagnostic))
            }
        }
    }
}

pub(super) async fn receive(
    stream: &mut tokio::net::TcpStream,
) -> crate::error::Result<super::super::protocol::Request> {
    let mut bytes = Vec::new();
    let mut reader = tokio::io::BufReader::new(stream).take(super::super::protocol::MAX_FRAME_BYTES + 1);
    reader.read_until(b'\n', &mut bytes).await.map_err(io_error)?;
    if bytes.len() as u64 > super::super::protocol::MAX_FRAME_BYTES || bytes.last() != Some(&b'\n') {
        return Err(crate::error::Failure::invalid_harness("invalid bounded public request frame"));
    }
    serde_json::from_slice(&bytes).map_err(|error| crate::error::Failure::invalid_harness(error.to_string()))
}

fn success(request: &super::super::protocol::Request, value: serde_json::Value) -> super::super::protocol::Response {
    super::super::protocol::response(request, super::super::protocol::Outcome::Ok, value, None)
}
fn denied(request: &super::super::protocol::Request, diagnostic: String) -> super::super::protocol::Response {
    super::super::protocol::response(
        request,
        super::super::protocol::Outcome::Fail,
        serde_json::Value::Null,
        Some(diagnostic),
    )
}
fn io_error(error: std::io::Error) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(error.to_string())
}

#[path = "runtime/operation.rs"]
mod operation;
