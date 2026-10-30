//! Retained native effects: fresh scope authority, real Raft, durable completion.

#[path = "runtime/authority.rs"]
mod authority;
#[path = "runtime/config.rs"]
mod config;
#[path = "runtime/live.rs"]
mod live;
#[path = "runtime/proof.rs"]
mod proof;
#[path = "runtime/worker.rs"]
mod worker;
pub use config::*;

pub(super) type Replica = crate::fabric_consistency::raft::ScopedLiveReplica<
    crate::fabric_consistency::raft::ConcreteReplicaPortBundle<
        crate::fabric_time::OperatingSystemEntropySource,
        super::Application,
    >,
>;
pub(super) struct Parts {
    service: Replica,
    ingress: crate::fabric_consistency::raft::ReplicaIngressPump,
    sender: tokio::sync::mpsc::Sender<crate::fabric_consistency::raft::ReplicaEvent>,
    control: tokio::sync::mpsc::Receiver<crate::fabric_consistency::raft::ReplicaObservation>,
    installation: super::Installation,
    timeout: std::time::Duration,
    authority: authority::config::Bound,
}
pub(super) struct Driver {
    service: Replica,
    ingress: crate::fabric_consistency::raft::ReplicaIngressPump,
    sender: tokio::sync::mpsc::Sender<crate::fabric_consistency::raft::ReplicaEvent>,
    control: tokio::sync::mpsc::Receiver<crate::fabric_consistency::raft::ReplicaObservation>,
    installation: super::Installation,
    timeout: std::time::Duration,
    authority: authority::config::Bound,
    port: super::native::CommitPort,
    is_uncertain: bool,
}

enum Message {
    Route(
        Box<crate::fabric::CanonicalPortBinding>,
        crate::system_extension::TypedEffectRequest,
        std::sync::mpsc::SyncSender<Result<crate::system_extension::PortEffectOutput, String>>,
    ),
    Read(
        super::RequestRef,
        std::sync::mpsc::SyncSender<Result<crate::fabric_consistency::raft::ReplicaObservation, String>>,
    ),
    Stop,
}

/// Opening is an effect-shell operation. The public host checks the independent
/// current install-effects grant before calling it; source bytes grant nothing.
pub struct Worker {
    sender: Option<tokio::sync::mpsc::Sender<Message>>,
    thread: Option<std::thread::JoinHandle<()>>,
    timeout: std::time::Duration,
    is_uncertain: bool,
}
impl Worker {
    pub fn open(root: &crate::node_state::DirectoryRoot, plan: AdmittedPlan) -> crate::error::Result<Self> {
        let (sender, receiver) = tokio::sync::mpsc::channel(16);
        let (ready, startup) = std::sync::mpsc::sync_channel(1);
        let root = root.clone();
        let thread = std::thread::Builder::new()
            .name("collaboration-scope".to_string())
            .spawn(move || worker::run(root, plan, receiver, ready))
            .map_err(|_| super::denied(super::Denial::Uncertain))?;
        let timeout = startup
            .recv_timeout(std::time::Duration::from_secs(120))
            .map_err(|_| super::denied(super::Denial::Uncertain))?
            .map_err(|_| super::denied(super::Denial::Uncertain))?;
        Ok(Self {
            sender: Some(sender),
            thread: Some(thread),
            timeout,
            is_uncertain: false,
        })
    }
    /// A real quorum challenge, never a reusable authorization projection.
    pub fn observe_read(
        &mut self,
        request: super::RequestRef,
    ) -> crate::error::Result<crate::fabric_consistency::raft::ReplicaObservation> {
        let (reply, result) = std::sync::mpsc::sync_channel(1);
        self.send(Message::Read(request, reply)).map_err(|_| super::denied(super::Denial::Uncertain))?;
        self.receive(result).map_err(|_| super::denied(super::Denial::Uncertain))
    }
    fn send(&self, message: Message) -> Result<(), String> {
        if self.is_uncertain {
            return Err("collaboration storage or worker outcome uncertain".to_string());
        }
        self.sender
            .as_ref()
            .ok_or_else(|| "collaboration runtime closed".to_string())?
            .try_send(message)
            .map_err(|_| "collaboration runtime queue unavailable".to_string())
    }
    fn receive<T>(&mut self, result: std::sync::mpsc::Receiver<Result<T, String>>) -> Result<T, String> {
        match result.recv_timeout(self.timeout + std::time::Duration::from_secs(1)) {
            Ok(result) => result,
            Err(_) => {
                self.is_uncertain = true;
                Err("collaboration worker completion uncertain".to_string())
            }
        }
    }
}
impl crate::system_extension::FabricEffectPort for Worker {
    fn route(
        &mut self,
        binding: &crate::fabric::CanonicalPortBinding,
        effect: &crate::system_extension::TypedEffectRequest,
    ) -> crate::fabric::FabricPortResult<crate::system_extension::PortEffectOutput> {
        let (reply, result) = std::sync::mpsc::sync_channel(1);
        self.send(Message::Route(Box::new(binding.clone()), effect.clone(), reply))
            .map_err(crate::fabric::FabricPortError::malformed)?;
        self.receive(result).map_err(crate::fabric::FabricPortError::malformed)
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        if let Some(sender) = self.sender.take() {
            let _closed = sender.try_send(Message::Stop);
        }
        if let Some(thread) = self.thread.take() {
            let _stopped = thread.join();
        }
    }
}
