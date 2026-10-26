// SPDX-License-Identifier: AGPL-3.0-or-later
use molten_batch_core::*;

use crate::*;

const CAS_ATTEMPTS: usize = 8;
const MAX_DISPATCHES_PER_TICK: usize = 32;

pub struct Controller {
    ports: ControllerPorts,
}
impl Controller {
    pub fn new(ports: ControllerPorts) -> Self {
        Self { ports }
    }
    pub fn state(&self) -> Result<State, Error> {
        self.ports.store.load()
    }
    pub fn submit(&self, key: String, spec: JobSpec) -> Result<Id, Error> {
        validate_job(&spec)?;
        let state = self.state()?;
        self.ports.admission.submit(&spec, state.policy_ref)?;
        let spec_ref = self.ports.identity.spec(&spec)?;
        let run = self.ports.identity.run(state.cluster, &key)?;
        for task in &spec.tasks {
            if self.ports.content.get(task.component)?.manifest.kind != ContentKind::Component {
                return Err(Error::Invalid("component kind".into()));
            }
            for input in &task.inputs {
                if self.ports.content.get(*input)?.manifest.kind != ContentKind::Data {
                    return Err(Error::Invalid("input kind".into()));
                }
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
        for _ in 0..MAX_DISPATCHES_PER_TICK {
            let state = self.state()?;
            let Some(placement) = self.ports.placement.next(&state)? else {
                break;
            };
            validate_placement(&state, &placement)?;
            // A failing peer does not turn a failed/unknown call into a retry.
            // This bounded tick stops proposing new work; later ticks reconcile.
            if self.dispatch(state, placement).await.is_err() {
                self.ports.observations.record(Diagnostic::ServiceError);
                break;
            }
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
            component: task.component,
            inputs: p.inputs.clone(),
            resources: task.resources,
            fuel: task.fuel,
            timeout_ms: task.timeout_ms,
            max_output_bytes: task.max_output_bytes,
        })
    }
    async fn dispatch(&self, state: State, placement: Placement) -> Result<(), Error> {
        let request = self.request(&state, &placement)?;
        self.ports.admission.assignment(&request, AdmissionAction::Reserve)?;
        // Content publication is idempotent and cannot execute a component.
        // Complete materialization before claiming the one-shot start slot.
        let component = self.ports.content.get(request.component)?;
        self.ports.workers.put(placement.node, request.component, component).await?;
        for input in &request.inputs {
            self.ports.workers.put(placement.node, *input, self.ports.content.get(*input)?).await?;
        }
        let request_ref = self.ports.identity.request(&request)?;
        let next = apply(&state, &Command::Schedule {
            placement,
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
            WorkerObservation::Terminal { request_ref, result } => {
                if request_ref != attempt.request_ref || result.request_ref != request_ref || !result.stopped {
                    return Err(Error::Invalid("terminal fence/stopped mismatch".into()));
                }
                self.ports.admission.assignment(&attempt.request, AdmissionAction::Complete)?;
                if let ExecutionOutcome::Succeeded { output } = result.outcome {
                    let content = self.ports.workers.get(attempt.request.fence.node, output).await?;
                    if content.manifest.kind != ContentKind::Data
                        || content.bytes.len() as u64 > attempt.request.max_output_bytes
                    {
                        return Err(Error::Invalid("output kind/bound".into()));
                    }
                    // The DAG cannot publish a successful stage before its output
                    // is durably available from the controller's content port.
                    if self.ports.content.put(&content)? != output {
                        return Err(Error::Invalid("output identity".into()));
                    }
                }
                let result_ref = self.ports.identity.result(&result)?;
                self.commit(&Command::Terminal {
                    attempt: id,
                    result_ref,
                    result,
                })?;
            }
        }
        Ok(())
    }
    fn commit(&self, command: &Command<'_>) -> Result<State, Error> {
        for _ in 0..CAS_ATTEMPTS {
            let current = self.state()?;
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
