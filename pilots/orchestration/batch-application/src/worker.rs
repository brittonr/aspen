// SPDX-License-Identifier: AGPL-3.0-or-later
use molten_batch_core::*;

use crate::*;

/// Non-cloneable proof that THIS invocation observed a definite acceptance
/// commit. It is not a capability and cannot be constructed by a driving adapter.
pub struct LaunchTicket {
    request: ExecutionRequest,
    request_ref: Id,
    boot: u64,
}
pub struct WorkerService {
    ports: WorkerPorts,
}
impl WorkerService {
    pub fn new(ports: WorkerPorts) -> Self {
        Self { ports }
    }
    pub fn state(&self) -> Result<WorkerState, Error> {
        self.ports.journal.load()
    }
    /// Composition must invoke once after EXCLUSIVE database open, before RPCs.
    /// Only the no-import, in-process component profile supports this recovery.
    pub fn recover_after_exclusive_open(&self) -> Result<(), Error> {
        self.commit(&WorkerCommand::RecoverAfterProcessExit).map(|_| ())
    }
    pub fn info(&self) -> Result<WorkerInfo, Error> {
        let s = self.state()?;
        Ok(WorkerInfo {
            node: s.node,
            cluster: s.cluster,
            generation: s.generation,
            policy_ref: s.policy_ref,
            capacity: s.capacity,
            boot: s.boot,
            draining: s.draining,
        })
    }
    pub fn put_content(&self, c: &Content) -> Result<Id, Error> {
        self.ports.content.put(c)
    }
    pub fn get_content(&self, id: Id) -> Result<Content, Error> {
        self.ports.content.get(id)
    }
    pub fn observe(&self, request: &ExecutionRequest) -> Result<WorkerObservation, Error> {
        let request_ref = self.ports.identity.request(request)?;
        let state = self.state()?;
        match state.records.get(&request.fence.assignment) {
            None => Ok(WorkerObservation::Missing),
            Some(r) if r.request == *request && r.request_ref == request_ref => Ok(observation(r)),
            _ => Err(Error::Invalid("attempt identity reused".into())),
        }
    }
    pub fn start(&self, request: ExecutionRequest) -> Result<(WorkerObservation, Option<LaunchTicket>), Error> {
        validate_request(&request)?;
        let request_ref = self.ports.identity.request(&request)?;
        for _ in 0..8 {
            let old = self.state()?;
            if old.boot == 0 {
                return Err(Error::Invalid("worker not recovered".into()));
            }
            if let Some(record) = old.records.get(&request.fence.assignment) {
                if record.request != request || record.request_ref != request_ref {
                    return Err(Error::Invalid("request substitution".into()));
                }
                return Ok((observation(record), None));
            }
            self.ports.admission.assignment(&request, AdmissionAction::Execute)?;
            let next = worker_apply(&old, &WorkerCommand::Accept {
                request: request.clone(),
                request_ref,
            })?;
            match self.ports.journal.compare_commit(&old, &next)? {
                Commit::Conflict => continue,
                Commit::Unknown => return Err(Error::Unknown),
                Commit::Applied => {
                    let record = &next.records[&request.fence.assignment];
                    let ticket = if record.phase == WorkerPhase::Accepted {
                        Some(LaunchTicket {
                            request,
                            request_ref,
                            boot: next.boot,
                        })
                    } else {
                        None
                    };
                    return Ok((observation(record), ticket));
                }
            }
        }
        Err(Error::Conflict)
    }
    pub fn cancel(&self, request: &ExecutionRequest) -> Result<WorkerObservation, Error> {
        self.ports.admission.assignment(request, AdmissionAction::Cancel)?;
        let request_ref = self.ports.identity.request(request)?;
        let state = self.commit(&WorkerCommand::Cancel {
            request: request.clone(),
            request_ref,
        })?;
        let record = &state.records[&request.fence.assignment];
        if record.phase == WorkerPhase::Running {
            self.ports.execution.cancel(request.fence.assignment)?;
        }
        Ok(observation(record))
    }
    pub fn resume(&self) -> Result<(), Error> {
        self.commit(&WorkerCommand::Resume).map(|_| ())
    }
    pub fn drain(&self) -> Result<(), Error> {
        let state = self.commit(&WorkerCommand::Drain)?;
        for record in state.records.values() {
            if !matches!(record.phase, WorkerPhase::Terminal(_)) {
                self.cancel(&record.request)?;
            }
        }
        Ok(())
    }
    pub async fn execute(&self, ticket: LaunchTicket) -> Result<(), Error> {
        let request = &ticket.request;
        let id = request.fence.assignment;
        // Materialization is outside the core. A pre-start error may terminate
        // the accepted attempt, but may not permit reuse of the same identity.
        let prepared = self.prepare(request);
        let (component, inputs) = match prepared {
            Ok(value) => value,
            Err(_) => {
                return self.finish(id, ExecutionResult {
                    request_ref: ticket.request_ref,
                    outcome: ExecutionOutcome::RejectedBeforeStart,
                    stopped: true,
                })
            }
        };
        let mut claimed = false;
        for _ in 0..8 {
            let state = self.state()?;
            let record = state.records.get(&id).ok_or(Error::Invalid("missing acceptance".into()))?;
            if matches!(record.phase, WorkerPhase::Terminal(_)) {
                return Ok(());
            }
            if record.request != *request || record.request_ref != ticket.request_ref {
                return Err(Error::Invalid("ticket substitution".into()));
            }
            let next = worker_apply(&state, &WorkerCommand::Begin {
                attempt: id,
                request_ref: ticket.request_ref,
                boot: ticket.boot,
            })?;
            match self.ports.journal.compare_commit(&state, &next)? {
                Commit::Applied => {
                    claimed = true;
                    break;
                }
                Commit::Conflict => continue,
                Commit::Unknown => return Err(Error::Unknown),
            }
        }
        if !claimed {
            return Err(Error::Conflict);
        }
        let current = self.state()?;
        let cancelled = current.records[&id].cancel_requested;
        if cancelled || self.ports.admission.assignment(request, AdmissionAction::Execute).is_err() {
            return self.finish(id, ExecutionResult {
                request_ref: ticket.request_ref,
                outcome: ExecutionOutcome::RejectedBeforeStart,
                stopped: true,
            });
        }
        let output = self.ports.execution.run(ticket.request.clone(), component, inputs).await?;
        if !output.stopped {
            return Err(Error::Unknown);
        }
        let outcome = match output.output {
            Ok(bytes) if bytes.len() as u64 <= request.max_output_bytes => {
                let published = self.ports.content.make_data(bytes).and_then(|c| self.ports.content.put(&c));
                match published {
                    Ok(id) => ExecutionOutcome::Succeeded { output: id },
                    Err(_) => ExecutionOutcome::Failed {
                        code: FailureCode::Internal,
                    },
                }
            }
            Ok(_) => ExecutionOutcome::Failed {
                code: FailureCode::OutputLimit,
            },
            Err(code) => ExecutionOutcome::Failed { code },
        };
        self.finish(id, ExecutionResult {
            request_ref: ticket.request_ref,
            outcome,
            stopped: true,
        })
    }
    fn prepare(&self, r: &ExecutionRequest) -> Result<(Content, Vec<Vec<u8>>), Error> {
        self.ports.admission.assignment(r, AdmissionAction::Execute)?;
        let component = self.ports.content.get(r.component)?;
        if component.manifest.kind != ContentKind::Component {
            return Err(Error::Invalid("component kind".into()));
        }
        let mut total = 0_usize;
        let mut inputs = Vec::with_capacity(r.inputs.len());
        for id in &r.inputs {
            let c = self.ports.content.get(*id)?;
            total = total.checked_add(c.bytes.len()).ok_or(Error::Bound)?;
            if c.manifest.kind != ContentKind::Data || total > MAX_CONTENT_BYTES {
                return Err(Error::Bound);
            }
            inputs.push(c.bytes);
        }
        Ok((component, inputs))
    }
    fn finish(&self, id: Id, result: ExecutionResult) -> Result<(), Error> {
        // A racing cancel may have durably produced its own stopped result.
        // Do not replace that authoritative terminal record.
        let state = self.state()?;
        if state.records.get(&id).is_some_and(|r| matches!(r.phase, WorkerPhase::Terminal(_))) {
            return Ok(());
        }
        self.commit(&WorkerCommand::Finish { attempt: id, result }).map(|_| ())
    }
    fn commit(&self, command: &WorkerCommand) -> Result<WorkerState, Error> {
        for _ in 0..8 {
            let old = self.state()?;
            let next = worker_apply(&old, command)?;
            if next == old {
                return Ok(old);
            }
            match self.ports.journal.compare_commit(&old, &next)? {
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
fn observation(r: &WorkerRecord) -> WorkerObservation {
    match r.phase {
        WorkerPhase::Accepted => WorkerObservation::Accepted {
            request_ref: r.request_ref,
        },
        WorkerPhase::Running => WorkerObservation::Running {
            request_ref: r.request_ref,
        },
        WorkerPhase::Terminal(result) => WorkerObservation::Terminal {
            request_ref: r.request_ref,
            result,
        },
    }
}
