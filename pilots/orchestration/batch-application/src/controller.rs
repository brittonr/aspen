// SPDX-License-Identifier: AGPL-3.0-or-later
use std::sync::Mutex;

use molten_batch_core::*;

use crate::*;

const CAS_ATTEMPTS: usize = 8;
const MAX_DISPATCHES_PER_TICK: usize = 32;

pub struct Controller {
    ports: ControllerPorts,
    last_placement: Mutex<Option<PlacementCursor>>,
}
impl Controller {
    pub fn new(ports: ControllerPorts) -> Self {
        Self {
            ports,
            last_placement: Mutex::new(None),
        }
    }
    pub fn state(&self) -> Result<State, Error> {
        self.ports.store.load()
    }
    pub fn progress_snapshot(&self, run: Id) -> Result<ProgressSnapshot, Error> {
        self.ports.store.progress_snapshot(run)
    }
    pub fn progress_read(&self, cursor: ProgressCursor, limit: u32) -> Result<ProgressPage, Error> {
        self.ports.store.progress_read(cursor, limit)
    }
    pub fn submit(&self, key: String, spec: JobSpec) -> Result<Id, Error> {
        validate_job(&spec)?;
        let state = self.state()?;
        self.ports.admission.submit(&spec, state.policy_ref)?;
        let spec_ref = self.ports.identity.spec(&spec)?;
        let run = self.ports.identity.run(state.cluster, &key)?;
        for task in &spec.tasks {
            match task.workload {
                Workload::Component { artifact } => {
                    self.verified_content(artifact, ContentKind::Component)?;
                }
                Workload::PureNet { manifest, artifact } => {
                    self.verified_content(manifest, ContentKind::PureNetManifest)?;
                    self.verified_content(artifact, ContentKind::PureNetProgram)?;
                }
            }
            for input in &task.inputs {
                self.verified_content(*input, ContentKind::Data)?;
            }
        }
        self.commit(&Command::Submit {
            key,
            spec_ref,
            run,
            spec,
        })?;
        Ok(run)
    }
    pub fn register(&self, node: Node) -> Result<(), Error> {
        self.commit(&Command::Register(node)).map(|_| ())
    }
    pub fn drain(&self, node: Id, draining: bool) -> Result<(), Error> {
        self.commit(&Command::Drain { node, draining }).map(|_| ())
    }
    pub fn cancel(&self, run: Id) -> Result<(), Error> {
        self.commit(&Command::CancelRun { run }).map(|_| ())
    }
    pub fn put_content(&self, content: &Content) -> Result<Id, Error> {
        self.ports.content.put(content)
    }
    pub fn get_content(&self, id: Id) -> Result<Content, Error> {
        self.ports.content.get(id)
    }

    /// Exactly one driving loop calls tick. Concurrent API mutations still use
    /// whole-state CAS. Logical time advances here, not from a worker clock.
    pub async fn tick(&self) -> Result<(), Error> {
        self.commit(&Command::Tick)?;
        let snapshot = self.state()?;
        for (id, node) in &snapshot.nodes {
            match self.ports.workers.info(*id).await {
                Ok(info)
                    if info.node == *id
                        && info.cluster == snapshot.cluster
                        && info.generation == snapshot.generation
                        && info.policy_ref == snapshot.policy_ref
                        && info.capacity == node.capacity =>
                {
                    if info.draining {
                        self.drain(*id, true)?;
                    }
                    self.commit(&Command::ObserveNode {
                        node: *id,
                        boot: info.boot,
                    })?;
                }
                _ => self.ports.observations.record(Diagnostic::Transport { node: *id }),
            }
        }
        // Missing work is NOT proof that a delayed start cannot still arrive.
        // A worker-side durable cancellation tombstone fences that arrival.
        let snapshot = self.state()?;
        for (id, attempt) in &snapshot.attempts {
            if matches!(attempt.phase, AttemptPhase::Terminal { .. }) {
                continue;
            }
            let cancel = snapshot.runs[&attempt.request.fence.run].cancel_requested;
            let observation = if cancel {
                self.ports.workers.cancel(&attempt.request).await
            } else {
                self.ports.workers.observe(&attempt.request).await
            };
            let observation = match observation {
                Ok(WorkerObservation::Missing) => self.ports.workers.cancel(&attempt.request).await,
                other => other,
            };
            match observation {
                Ok(value) => {
                    if self.accept_observation(attempt, value).await.is_err() {
                        self.ports.observations.record(Diagnostic::PendingReconciliation { attempt: *id });
                    }
                }
                Err(_) => self.ports.observations.record(Diagnostic::PendingReconciliation { attempt: *id }),
            }
        }
        let mut first_denied: Option<PlacementCursor> = None;
        for _ in 0..MAX_DISPATCHES_PER_TICK {
            let state = self.state()?;
            let placement = {
                let cursor = self.last_placement.lock().expect("placement cursor poisoned");
                self.ports.placement.next(&state, *cursor)?
            };
            let Some(placement) = placement else {
                break;
            };
            validate_placement(&state, &placement)?;
            let position = PlacementCursor::at(&state, &placement)?;
            if first_denied == Some(position) {
                break;
            }
            let request = match self.request(&state, &placement) {
                Ok(request) => request,
                Err(_) => {
                    self.ports.observations.record(Diagnostic::ServiceError);
                    break;
                }
            };
            match self.ports.admission.assignment(&request, AdmissionAction::Reserve) {
                Ok(()) => {}
                Err(Error::Denied) => {
                    if first_denied.is_none() {
                        first_denied = Some(position);
                    }
                    *self.last_placement.lock().expect("placement cursor poisoned") = Some(position);
                    continue;
                }
                Err(_) => {
                    self.ports.observations.record(Diagnostic::ServiceError);
                    break;
                }
            }
            // A failing peer does not turn a failed/unknown call into a retry.
            // This bounded tick stops proposing new work; later ticks reconcile.
            if self.dispatch(state, &placement, request).await.is_err() {
                self.ports.observations.record(Diagnostic::ServiceError);
                break;
            }
            *self.last_placement.lock().expect("placement cursor poisoned") = Some(position);
        }
        Ok(())
    }

    fn request(&self, state: &State, p: &Placement) -> Result<ExecutionRequest, Error> {
        let run = &state.runs[&p.run];
        let task = run.spec.tasks.iter().find(|t| t.id == p.task).ok_or(Error::Invalid("task missing".into()))?;
        Ok(ExecutionRequest {
            fence: Fence {
                cluster: state.cluster,
                generation: state.generation,
                run: p.run,
                task: p.task.clone(),
                assignment: self.ports.identity.assignment(state.cluster, state.generation, p, run.spec_ref)?,
                spec_ref: run.spec_ref,
                node: p.node,
                epoch: p.number,
            },
            policy_ref: state.policy_ref,
            workload: task.workload,
            inputs: p.inputs.clone(),
            resources: task.resources,
            fuel: task.fuel,
            timeout_ms: task.timeout_ms,
            max_output_bytes: task.max_output_bytes,
            max_attempts: task.max_attempts,
        })
    }
    async fn dispatch(&self, state: State, placement: &Placement, request: ExecutionRequest) -> Result<(), Error> {
        // Content publication is idempotent and cannot execute a workload.
        // Complete materialization before claiming the one-shot start slot.
        let artifact = request.workload.artifact();
        let kind = match request.workload {
            Workload::Component { .. } => ContentKind::Component,
            Workload::PureNet { manifest, .. } => {
                let content = self.verified_content(manifest, ContentKind::PureNetManifest)?;
                self.ports.workers.put(placement.node, manifest, content).await?;
                ContentKind::PureNetProgram
            }
        };
        self.ports.workers.put(placement.node, artifact, self.verified_content(artifact, kind)?).await?;
        for input in &request.inputs {
            self.ports
                .workers
                .put(placement.node, *input, self.verified_content(*input, ContentKind::Data)?)
                .await?;
        }
        let request_ref = self.ports.identity.request(&request)?;
        let next = apply(&state, &Command::Schedule {
            placement: placement.clone(),
            request: &request,
            request_ref,
        })?;
        match self.ports.store.compare_commit(&state, &next)? {
            Commit::Applied => {}
            Commit::Conflict => return Ok(()),
            Commit::Unknown => {
                self.ports.observations.record(Diagnostic::CommitUnknown);
                return Err(Error::Unknown);
            }
        }
        self.ports.observations.record(Diagnostic::Boundary("before-dispatch"));
        let id = request.fence.assignment;
        let current = self.state()?;
        let observation = if current.runs[&request.fence.run].cancel_requested
            || self.ports.admission.assignment(&request, AdmissionAction::Execute).is_err()
        {
            self.ports.observations.record(Diagnostic::Admission { attempt: id });
            self.ports.workers.cancel(&request).await
        } else {
            self.ports.workers.start(request.clone()).await
        };
        self.ports.observations.record(Diagnostic::Boundary("after-dispatch"));
        let attempt = &next.attempts[&id];
        match observation {
            Ok(value) => self.accept_observation(attempt, value).await,
            Err(_) => {
                self.commit(&Command::Uncertain {
                    attempt: id,
                    request_ref,
                })?;
                Ok(())
            }
        }
    }
    async fn accept_observation(&self, attempt: &Attempt, observation: WorkerObservation) -> Result<(), Error> {
        let id = attempt.request.fence.assignment;
        match observation {
            WorkerObservation::Missing => {
                self.ports.observations.record(Diagnostic::PendingReconciliation { attempt: id })
            }
            WorkerObservation::Accepted { request_ref } => {
                if request_ref != attempt.request_ref {
                    return Err(Error::Invalid("acceptance substitution".into()));
                }
            }
            WorkerObservation::Running { request_ref } => {
                if request_ref != attempt.request_ref {
                    return Err(Error::Invalid("running substitution".into()));
                }
                self.commit(&Command::Started {
                    attempt: id,
                    request_ref,
                })?;
            }
            WorkerObservation::Terminal {
                request_ref,
                mut result,
            } => {
                if request_ref != attempt.request_ref || result.request_ref != request_ref || !result.stopped {
                    return Err(Error::Invalid("terminal fence/stopped mismatch".into()));
                }
                self.ports.admission.assignment(&attempt.request, AdmissionAction::Complete)?;
                if let Some(report) = result.report {
                    let content = self.ports.workers.get(attempt.request.fence.node, report).await?;
                    if content.manifest.kind != ContentKind::Data
                        || content.bytes.len() as u64 > attempt.request.max_output_bytes
                        || self.ports.identity.content(&content)? != report
                    {
                        return Err(Error::Invalid("report kind/identity/bound".into()));
                    }
                    // Stopped failure receipts remain reconcilable after grant
                    // revocation and must be durable before their terminal ID.
                    self.ports.observations.record(Diagnostic::Boundary("before-output-persist"));
                    if self.ports.content.put(&content)? != report {
                        return Err(Error::Invalid("report identity".into()));
                    }
                    self.ports.observations.record(Diagnostic::Boundary("after-output-persist"));
                }
                if matches!(result.outcome, ExecutionOutcome::Succeeded { .. })
                    && self.ports.admission.assignment(&attempt.request, AdmissionAction::Publish).is_err()
                {
                    result.outcome = ExecutionOutcome::Failed {
                        code: FailureCode::PublicationDenied,
                    };
                }
                if let ExecutionOutcome::Succeeded { output } = result.outcome {
                    let content = self.ports.workers.get(attempt.request.fence.node, output).await?;
                    if content.manifest.kind != ContentKind::Data
                        || content.bytes.len() as u64 > attempt.request.max_output_bytes
                        || self.ports.identity.content(&content)? != output
                    {
                        return Err(Error::Invalid("output kind/bound".into()));
                    }
                    // The DAG cannot publish a successful stage before its output
                    // is durably available from the controller's content port.
                    self.ports.observations.record(Diagnostic::Boundary("before-output-persist"));
                    if self.ports.admission.assignment(&attempt.request, AdmissionAction::Publish).is_err() {
                        result.outcome = ExecutionOutcome::Failed {
                            code: FailureCode::PublicationDenied,
                        };
                    } else {
                        if self.ports.content.put(&content)? != output {
                            return Err(Error::Invalid("output identity".into()));
                        }
                        self.ports.observations.record(Diagnostic::Boundary("after-output-persist"));
                    }
                }
                self.ports.observations.record(Diagnostic::Boundary("before-terminal-commit"));
                if matches!(result.outcome, ExecutionOutcome::Succeeded { .. })
                    && self.ports.admission.assignment(&attempt.request, AdmissionAction::Publish).is_err()
                {
                    result.outcome = ExecutionOutcome::Failed {
                        code: FailureCode::PublicationDenied,
                    };
                }
                let result_ref = self.ports.identity.result(&result)?;
                self.commit(&Command::Terminal {
                    attempt: id,
                    result_ref,
                    result,
                })?;
                self.ports.observations.record(Diagnostic::Boundary("after-terminal-commit"));
            }
        }
        Ok(())
    }
    fn verified_content(&self, id: Id, kind: ContentKind) -> Result<Content, Error> {
        let content = self.ports.content.get(id)?;
        if content.manifest.kind != kind || self.ports.identity.content(&content)? != id {
            return Err(Error::Invalid("content kind/identity".into()));
        }
        Ok(content)
    }
    fn commit(&self, command: &Command<'_>) -> Result<State, Error> {
        for _ in 0..CAS_ATTEMPTS {
            let current = self.state()?;
            // A conflicting API mutation may force another terminal CAS. Each
            // new successful publication proposal requires current authority;
            // a previously accepted terminal record remains immutable.
            let terminal;
            let command = match command {
                Command::Terminal { attempt, result, .. }
                    if matches!(result.outcome, ExecutionOutcome::Succeeded { .. })
                        && !current
                            .attempts
                            .get(attempt)
                            .is_some_and(|a| matches!(a.phase, AttemptPhase::Terminal { .. })) =>
                {
                    let request =
                        &current.attempts.get(attempt).ok_or(Error::Invalid("missing attempt".into()))?.request;
                    let mut result = *result;
                    if self.ports.admission.assignment(request, AdmissionAction::Publish).is_err() {
                        result.outcome = ExecutionOutcome::Failed {
                            code: FailureCode::PublicationDenied,
                        };
                    }
                    terminal = Command::Terminal {
                        attempt: *attempt,
                        result_ref: self.ports.identity.result(&result)?,
                        result,
                    };
                    &terminal
                }
                _ => command,
            };
            let next = apply(&current, command)?;
            if next == current {
                return Ok(current);
            }
            match self.ports.store.compare_commit(&current, &next)? {
                Commit::Applied => return Ok(next),
                Commit::Conflict => continue,
                Commit::Unknown => {
                    self.ports.observations.record(Diagnostic::CommitUnknown);
                    return Err(Error::Unknown);
                }
            }
        }
        Err(Error::Conflict)
    }
}
