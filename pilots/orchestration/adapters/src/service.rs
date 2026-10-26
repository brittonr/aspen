// SPDX-License-Identifier: AGPL-3.0-or-later
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use molten_batch_application::*;
use molten_batch_core::*;
use tokio::sync::watch;
use tokio::sync::Mutex;
use tokio::task::JoinSet;

use crate::*;

#[derive(Default)]
pub struct Counters {
    transport: AtomicU64,
    reconciliation: AtomicU64,
    admission: AtomicU64,
    errors: AtomicU64,
}
impl ObservabilityPort for Counters {
    fn record(&self, event: Diagnostic) {
        let counter = match event {
            Diagnostic::Transport { .. } => &self.transport,
            Diagnostic::PendingReconciliation { .. } => &self.reconciliation,
            Diagnostic::Admission { .. } => &self.admission,
            _ => &self.errors,
        };
        let _ = counter.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| Some(v.saturating_add(1)));
    }
}
impl Counters {
    pub fn snapshot(&self) -> [u64; 4] {
        [
            self.transport.load(Ordering::Relaxed),
            self.reconciliation.load(Ordering::Relaxed),
            self.admission.load(Ordering::Relaxed),
            self.errors.load(Ordering::Relaxed),
        ]
    }
}

pub struct ServiceHandler {
    pub controller: Option<Arc<Controller>>,
    pub worker: Option<Arc<WorkerService>>,
    config: NodeConfig,
    executions: Mutex<JoinSet<Result<(), Error>>>,
}
impl ServiceHandler {
    async fn dispatch(&self, request: Request) -> Result<Reply, Error> {
        // Reap completed execution handles; their semantic outcomes are journaled
        // independently. No detached process owns workload state.
        {
            let mut tasks = self.executions.lock().await;
            while tasks.try_join_next().is_some() {}
        }
        if let Some(controller) = &self.controller {
            return match request {
                Request::Put { id, content } => {
                    let c = content.content()?;
                    if content_id(&c)? != id {
                        return Err(Error::Invalid("put identity".into()));
                    }
                    Ok(Reply::Id(controller.put_content(&c)?))
                }
                Request::Get { id } => Ok(Reply::Content(WireContent::from_content(controller.get_content(id)?)?)),
                Request::Submit { key, spec } => Ok(Reply::Id(controller.submit(key, spec)?)),
                Request::Status => Ok(Reply::State(controller.state()?)),
                Request::CancelRun { run } => {
                    controller.cancel(run)?;
                    Ok(Reply::Ok)
                }
                Request::DrainNode { node, draining } => {
                    controller.drain(node, draining)?;
                    Ok(Reply::Ok)
                }
                _ => Err(Error::Unsupported),
            };
        }
        if let Some(worker) = &self.worker {
            return match request {
                Request::Info => Ok(Reply::Info(worker.info()?.into())),
                Request::Put { id, content } => {
                    let c = content.content()?;
                    if content_id(&c)? != id {
                        return Err(Error::Invalid("put identity".into()));
                    }
                    Ok(Reply::Id(worker.put_content(&c)?))
                }
                Request::Get { id } => Ok(Reply::Content(WireContent::from_content(worker.get_content(id)?)?)),
                Request::Observe { request } => Ok(Reply::Observation(worker.observe(&request)?.into())),
                Request::CancelAttempt { request } => Ok(Reply::Observation(worker.cancel(&request)?.into())),
                Request::Start { request } => {
                    let (observation, ticket) = worker.start(request)?;
                    if let Some(ticket) = ticket {
                        let service = worker.clone();
                        self.executions.lock().await.spawn(async move { service.execute(ticket).await });
                    }
                    Ok(Reply::Observation(observation.into()))
                }
                Request::WorkerDrain => {
                    worker.drain()?;
                    Ok(Reply::Ok)
                }
                _ => Err(Error::Unsupported),
            };
        }
        Err(Error::Unsupported)
    }
    pub async fn shutdown(&self) -> Result<(), Error> {
        if let Some(worker) = &self.worker {
            worker.drain()?;
        }
        let mut tasks = self.executions.lock().await;
        while let Some(result) = tasks.join_next().await {
            if !matches!(result, Ok(Ok(()))) {
                eprintln!("orchestrator: execution requires journal reconciliation");
            }
        }
        Ok(())
    }
}
#[async_trait]
impl RequestHandler for ServiceHandler {
    fn permits(&self, peer: Id) -> bool {
        match self.config.role {
            Role::Controller => self.config.operators.contains(&peer),
            Role::Worker => self.config.controller == Some(peer),
            Role::Client => false,
        }
    }
    async fn handle(&self, peer: Id, request: Request) -> Reply {
        if !self.permits(peer) {
            return error_reply(Error::Denied);
        }
        match self.dispatch(request).await {
            Ok(r) => r,
            Err(e) => error_reply(e),
        }
    }
}
pub struct RunningService {
    pub endpoint: iroh::Endpoint,
    pub handler: Arc<ServiceHandler>,
    pub counters: Arc<Counters>,
    config: NodeConfig,
}
pub fn secret(root: &CapabilityRoot) -> Result<iroh::SecretKey, Error> {
    let bytes = root.read("identity.key", 32)?;
    let array: [u8; 32] = bytes.try_into().map_err(|_| Error::Invalid("key length".into()))?;
    Ok(iroh::SecretKey::from_bytes(&array))
}
pub fn public_id(root: &CapabilityRoot) -> Result<Id, Error> {
    Ok(Id(*secret(root)?.public().as_bytes()))
}
/// Initialization is an explicit local operator action. Never automatically
/// replaces a key, root, policy, generation, or existing configuration.
pub fn initialize(root: &CapabilityRoot, config: &NodeConfig, policy: &Policy) -> Result<Id, Error> {
    config.validate()?;
    validate_policy(policy)?;
    if config.cluster != policy.cluster || config.generation != policy.generation {
        return Err(Error::Denied);
    }
    let key = iroh::SecretKey::generate();
    let id = Id(*key.public().as_bytes());
    root.create_new("identity.key", &key.to_bytes())?;
    root.create_new("config.json", &serde_json::to_vec_pretty(config).map_err(crate::invalid)?)?;
    root.create_new("policy.json", &serde_json::to_vec_pretty(policy).map_err(crate::invalid)?)?;
    root.create_new("revoked.json", b"[]")?;
    Ok(id)
}
pub async fn open_service(root: CapabilityRoot, resume: bool) -> Result<RunningService, Error> {
    let config: NodeConfig = root.read_json("config.json")?;
    config.validate()?;
    if config.role == Role::Client {
        return Err(Error::Unsupported);
    }
    let policy: Policy = root.read_json("policy.json")?;
    let policy_ref = policy_id(&policy)?;
    if policy.cluster != config.cluster || policy.generation != config.generation {
        return Err(Error::Denied);
    }
    let disk = Disk::open(&root)?;
    let content = RedbContent::new(disk.clone());
    let identity = Arc::new(CanonicalIdentity);
    let admission = Arc::new(FileAdmission::new(root.clone(), policy_ref)?);
    let counters = Arc::new(Counters::default());
    let key = secret(&root)?;
    let node = Id(*key.public().as_bytes());
    let endpoint = bind_endpoint(key, config.bind).await?;
    let (controller, worker) = match config.role {
        Role::Controller => {
            let store = RedbControllerStore::open(
                disk,
                State::empty(config.cluster, policy_ref, config.generation, config.freshness_ticks)?,
            )?;
            let workers = Arc::new(IrohWorkers::new(
                endpoint.clone(),
                config.workers.iter().map(|e| e.peer.clone()).collect(),
                Duration::from_millis(config.rpc_timeout_ms),
            )?);
            let controller = Arc::new(Controller::new(ControllerPorts {
                store,
                content,
                identity,
                placement: Arc::new(FirstFitPlacement),
                admission,
                workers,
                observations: counters.clone(),
            }));
            for e in &config.workers {
                controller.register(Node {
                    id: e.peer.id,
                    capacity: e.capacity,
                    labels: e.labels.clone(),
                    draining: false,
                    enabled: true,
                    last_seen: None,
                    boot: 0,
                })?;
            }
            (Some(controller), None)
        }
        Role::Worker => {
            let journal = RedbWorkerJournal::open(
                disk.clone(),
                WorkerState::empty(config.cluster, node, config.generation, policy_ref, config.capacity)?,
            )?;
            let worker = Arc::new(WorkerService::new(WorkerPorts {
                journal,
                content,
                identity,
                admission,
                execution: Arc::new(PureComponentRuntime::for_worker(disk)),
                observations: counters.clone(),
            }));
            worker.recover_after_exclusive_open()?;
            if resume {
                worker.resume()?;
            }
            (None, Some(worker))
        }
        Role::Client => return Err(Error::Unsupported),
    };
    Ok(RunningService {
        endpoint,
        handler: Arc::new(ServiceHandler {
            controller,
            worker,
            config: config.clone(),
            executions: Mutex::new(JoinSet::new()),
        }),
        counters,
        config,
    })
}
impl RunningService {
    pub async fn run(self, stop: watch::Receiver<bool>) -> Result<(), Error> {
        let (driver_stop, mut driver_stopped) = watch::channel(false);
        let driver = if let Some(controller) = &self.handler.controller {
            let controller = controller.clone();
            let mut stop = stop.clone();
            let interval = self.config.tick_ms;
            let counters = self.counters.clone();
            Some(tokio::spawn(async move {
                let mut turns = 0_u64;
                loop {
                    if *stop.borrow() || *driver_stopped.borrow() {
                        break;
                    }
                    if controller.tick().await.is_err() {
                        counters.record(Diagnostic::ServiceError);
                    }
                    turns = turns.saturating_add(1);
                    if turns.is_multiple_of(60) {
                        eprintln!(
                            "orchestrator counters [transport,reconciliation,admission,error]={:?}",
                            counters.snapshot()
                        );
                    }
                    tokio::select! {changed=stop.changed()=>{if changed.is_err()||*stop.borrow(){break;}},_=driver_stopped.changed()=>{break;},_=tokio::time::sleep(Duration::from_millis(interval))=>{}}
                }
            }))
        } else {
            None
        };
        let result =
            serve(self.endpoint.clone(), self.handler.clone(), Duration::from_millis(self.config.rpc_timeout_ms), stop)
                .await;
        let _ = driver_stop.send(true);
        if let Some(driver) = driver {
            let _ = driver.await;
        }
        let shutdown = self.handler.shutdown().await;
        self.endpoint.close().await;
        result.and(shutdown)
    }
}
pub fn iroh_key_for_namespace() -> Id {
    Id(*iroh::SecretKey::generate().public().as_bytes())
}
