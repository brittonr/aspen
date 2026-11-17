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
    fault: ProcessFault,
}
impl ObservabilityPort for Counters {
    fn record(&self, event: Diagnostic) {
        if let Diagnostic::Boundary(point) = event {
            self.fault.hit(point);
            return;
        }
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
    fault: ProcessFault,
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
                Request::Status { run } => {
                    let mut state = controller.state()?;
                    Ok(match run {
                        Some(id) => Reply::Run(state.runs.remove(&id).ok_or(Error::Unavailable)?),
                        None => Reply::State(state),
                    })
                }
                Request::ProgressSnapshot { run } => Ok(Reply::ProgressSnapshot(controller.progress_snapshot(run)?)),
                Request::ProgressRead { cursor, limit } => {
                    let page = controller.progress_read(cursor, limit)?;
                    if matches!(&page, ProgressPage::Events { events, .. } if !events.is_empty()) {
                        self.fault.hit("before-progress-reply");
                    }
                    Ok(Reply::ProgressPage(page))
                }
                Request::ProgressArtifact { id } => match controller.get_content(id) {
                    Ok(content) => Ok(Reply::Content(WireContent::from_content(content)?)),
                    Err(Error::Unavailable) => Ok(Reply::ArtifactGap { id }),
                    Err(error) => Err(error),
                },
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
                Request::ProgressArtifact { id } => match worker.get_content(id) {
                    Ok(content) => Ok(Reply::Content(WireContent::from_content(content)?)),
                    Err(Error::Unavailable) => Ok(Reply::ArtifactGap { id }),
                    Err(error) => Err(error),
                },
                Request::Observe { request } => {
                    let observation = worker.observe(&request)?;
                    if matches!(observation, WorkerObservation::Terminal { .. }) {
                        self.fault.hit("before-worker-reply");
                    }
                    Ok(Reply::Observation(observation.into()))
                }
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
    fn reply_closed(&self, reply: &Reply) {
        if matches!(reply, Reply::ProgressPage(ProgressPage::Events { events, .. }) if !events.is_empty()) {
            self.fault.hit("after-progress-ack");
        }
        if self.worker.is_some() && matches!(reply, Reply::Observation(WireObservation::Terminal { .. })) {
            self.fault.hit("after-worker-ack");
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
    open_service_with_fault(root, resume, ProcessFault::default()).await
}
pub async fn open_service_with_fault(
    root: CapabilityRoot,
    resume: bool,
    fault: ProcessFault,
) -> Result<RunningService, Error> {
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
    validate_policy(&policy)?;
    let installed = if policy.profile == PURE_NET_PROFILE {
        Some(crate::pure_net::load_pure_net(&root, &policy)?)
    } else {
        None
    };
    if let Some(native) = &installed {
        if config.role == Role::Worker {
            if config.capacity.slots > native.manifest.limits.max_concurrent_evaluations {
                return Err(Error::Bound);
            }
            let limits = &native.manifest.limits;
            let observed = observe_process_limits(
                limits.process_address_space_bytes,
                limits.process_cpu_soft_seconds,
                limits.process_cpu_hard_seconds,
            )?;
            eprintln!(
                "pure-net-worker-process-observation {}",
                serde_json::json!({
                    "manifest": native.manifest_ref,
                    "limits": observed,
                    "trusted_native_not_sandboxed": true
                })
            );
        }
    }
    let disk = Disk::open_with_fault(&root, fault.clone())?;
    let content = RedbContent::new(disk.clone());
    if let Some(native) = &installed {
        content.put(&native.content)?;
    }
    let identity = Arc::new(CanonicalIdentity);
    let admission = Arc::new(FileAdmission::new(root.clone(), policy_ref)?);
    let counters = Arc::new(Counters {
        fault: fault.clone(),
        ..Counters::default()
    });
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
            let execution: Arc<dyn ExecutionPort> = match &installed {
                Some(native) => Arc::new(PureNetRuntime::for_worker(disk, native, counters.clone())),
                None => Arc::new(PureComponentRuntime::for_worker(disk)),
            };
            let worker = Arc::new(WorkerService::new(WorkerPorts {
                journal,
                content,
                identity,
                admission,
                execution,
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
            fault,
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
