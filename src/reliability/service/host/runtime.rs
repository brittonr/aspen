use tokio::io::AsyncBufReadExt as _;
use tokio::io::AsyncReadExt as _;

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
            || !(1..=MAX_CONCURRENT_CONNECTIONS).contains(&self.max_connections)
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
    pub(super) fail_stop: FailStop,
}

/// Sticky fail-stop latch. A quarantined replica (`ReplicaExecutionOutcome::Failed`) or a failed
/// snapshot stops the host once the current response is answered, whichever path observed it:
/// the serve loop, a public operation, its wait, or a forwarded exchange.
// r[impl molten.consensus.reliability_replay_and_failstop]
#[derive(Debug, Default)]
pub(super) struct FailStop(Option<crate::error::Failure>);

impl FailStop {
    /// `Applied` continues; `Denied` is an error its caller answers; `Failed` latches fail-stop.
    pub(super) fn settle(
        &mut self,
        outcome: crate::fabric_consistency::raft::ReplicaExecutionOutcome,
    ) -> crate::error::Result<()> {
        match outcome {
            crate::fabric_consistency::raft::ReplicaExecutionOutcome::Applied(_) => Ok(()),
            crate::fabric_consistency::raft::ReplicaExecutionOutcome::Denied { diagnostic, .. } => {
                Err(crate::error::Failure::invalid_harness(diagnostic))
            }
            crate::fabric_consistency::raft::ReplicaExecutionOutcome::Failed(failed) => {
                Err(self.latch(quarantined(&failed.diagnostic)))
            }
        }
    }

    /// Latch `failure` unless an earlier one is latched, returning the latched failure.
    pub(super) fn latch(&mut self, failure: crate::error::Failure) -> crate::error::Failure {
        self.0.get_or_insert(failure).clone()
    }

    pub(super) fn latched(&self) -> Option<&crate::error::Failure> {
        self.0.as_ref()
    }

    pub(super) fn take(&mut self) -> Option<crate::error::Failure> {
        self.0.take()
    }
}

fn quarantined(diagnostic: &str) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(format!("replica quarantined; host fail-stops: {diagnostic}"))
}

/// The response that ends a public operation once the replica handled its proposal or read, or
/// `None` to await the replica's observation. A quarantine latches fail-stop: the client hears an
/// uncertain outcome, because the failed effect may have happened, and the host stops after it.
pub(super) fn operation_response(
    fail_stop: &mut FailStop,
    request: &super::super::protocol::Request,
    outcome: crate::fabric_consistency::raft::ReplicaExecutionOutcome,
) -> Option<super::super::protocol::Response> {
    match outcome {
        crate::fabric_consistency::raft::ReplicaExecutionOutcome::Applied(_) => None,
        crate::fabric_consistency::raft::ReplicaExecutionOutcome::Denied { diagnostic, .. } => {
            Some(denied(request, diagnostic))
        }
        crate::fabric_consistency::raft::ReplicaExecutionOutcome::Failed(failed) => {
            let failure = fail_stop.latch(quarantined(&failed.diagnostic));
            Some(super::super::protocol::uncertain(request, failure.to_string()))
        }
    }
}

/// Request scope checks that precede authority: the admitted run, cohort, profile and key. A
/// `Teardown` stops only the host it names: a signed teardown for one endpoint replayed to another
/// voter is denied there.
// r[impl molten.consensus.reliability_replay_and_failstop]
pub(super) fn admit_scope(
    admission: &HostAdmission,
    request: &super::super::protocol::Request,
    object_key: &str,
) -> std::result::Result<(), String> {
    let endpoint = super::super::protocol::validate_request(request)?;
    if request.run_id != admission.run_id || !admission.peers.values().any(|peer| peer == &endpoint) {
        return Err("request-outside-admitted-host".into());
    }
    if request.phase == super::super::protocol::Phase::Teardown && endpoint != admission.endpoint {
        return Err("teardown-outside-local-endpoint".into());
    }
    if request.profile.as_ref() != Some(&admission.profile) {
        return Err("request-profile-substitution".into());
    }
    if let Some(operation) = &request.operation
        && operation.get("object_key").and_then(|value| value.as_str()) != Some(object_key)
    {
        return Err("request-outside-admitted-key".into());
    }
    Ok(())
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
            fail_stop: FailStop::default(),
        }
    }
    pub fn observation(&self) -> crate::error::Result<super::diagnostics::HostObservation> {
        super::diagnostics::observe(&self.replica, &self.context.source_gate, self.admission.endpoint)
    }

    fn admit(&self, request: &super::super::protocol::Request) -> std::result::Result<(), String> {
        admit_scope(&self.admission, request, self.replica.ports().application.handler().object_key())?;
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
        let outcome = self.replica.handle_event(event).await;
        self.fail_stop.settle(outcome)?;
        self.snapshot_if_due().await
    }

    /// Snapshot with the registry's state ref once the retained applied entries reach the profile's
    /// threshold, so the log keeps room for new proposals. A denied or failed snapshot latches the
    /// same fail-stop as any other failed replica effect.
    // r[impl molten.consensus.snapshot_threshold]
    async fn snapshot_if_due(&mut self) -> crate::error::Result<()> {
        if !crate::fabric_consistency::raft::is_snapshot_due(self.replica.state()) {
            return Ok(());
        }
        let application_state_ref = self.replica.ports().application.handler().state_ref()?;
        let outcome = self
            .replica
            .handle_event(crate::fabric_consistency::raft::ReplicaEvent::CreateSnapshot { application_state_ref })
            .await;
        self.fail_stop.settle(outcome).map_err(|failure| self.fail_stop.latch(failure))
    }
}

#[cfg(test)]
pub(super) async fn receive(
    stream: &mut tokio::net::TcpStream,
) -> crate::error::Result<super::super::protocol::Request> {
    parse_request(&read_frame(stream).await?)
}

/// Read one newline-terminated public request frame of bounded size.
async fn read_frame(stream: &mut tokio::net::TcpStream) -> crate::error::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut reader = tokio::io::BufReader::new(stream).take(super::super::protocol::MAX_FRAME_BYTES + 1);
    reader.read_until(b'\n', &mut bytes).await.map_err(io_error)?;
    if bytes.len() as u64 > super::super::protocol::MAX_FRAME_BYTES || bytes.last() != Some(&b'\n') {
        return Err(crate::error::Failure::invalid_harness("invalid bounded public request frame"));
    }
    Ok(bytes)
}

fn parse_request(frame: &[u8]) -> crate::error::Result<super::super::protocol::Request> {
    serde_json::from_slice(frame).map_err(|error| crate::error::Failure::invalid_harness(error.to_string()))
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

/// Simultaneous public connections a host may serve; later connections wait in the listen backlog.
const MAX_CONCURRENT_CONNECTIONS: u64 = 256;

#[path = "runtime/connections.rs"]
mod connections;
#[path = "runtime/listen.rs"]
mod listen;
#[path = "runtime/operation.rs"]
mod operation;
