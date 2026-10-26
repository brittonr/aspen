// SPDX-License-Identifier: AGPL-3.0-or-later
use alloc::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerPhase {
    Accepted,
    Running,
    Terminal(ExecutionResult),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerRecord {
    pub request: ExecutionRequest,
    pub request_ref: Id,
    pub phase: WorkerPhase,
    pub cancel_requested: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerState {
    pub cluster: Id,
    pub node: Id,
    pub generation: u64,
    pub policy_ref: Id,
    pub capacity: Resources,
    pub boot: u64,
    pub revision: u64,
    pub draining: bool,
    pub records: BTreeMap<Id, WorkerRecord>,
}
impl WorkerState {
    pub fn empty(
        cluster: Id,
        node: Id,
        generation: u64,
        policy_ref: Id,
        capacity: Resources,
    ) -> Result<Self, Violation> {
        let state = Self {
            cluster,
            node,
            generation,
            policy_ref,
            capacity,
            boot: 0,
            revision: 0,
            draining: false,
            records: BTreeMap::new(),
        };
        validate_worker(&state)?;
        Ok(state)
    }
    pub fn usage(&self) -> Result<Resources, Violation> {
        self.records
            .values()
            .filter(|r| !matches!(r.phase, WorkerPhase::Terminal(_)))
            .try_fold(Resources::default(), |sum, r| sum.checked_add(r.request.resources))
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerCommand {
    Accept {
        request: ExecutionRequest,
        request_ref: Id,
    },
    Begin {
        attempt: Id,
        request_ref: Id,
        boot: u64,
    },
    Cancel {
        request: ExecutionRequest,
        request_ref: Id,
    },
    Finish {
        attempt: Id,
        result: ExecutionResult,
    },
    /// Legal only after exclusive reopening of a process-owned pure-Wasm journal.
    /// No detached native execution or hostcalls are allowed by this profile.
    RecoverAfterProcessExit,
    Drain,
    Resume,
}
pub fn validate_worker(s: &WorkerState) -> Result<(), Violation> {
    if s.generation == 0
        || s.records.len() > MAX_WORKER_RECORDS
        || s.capacity.slots == 0
        || s.capacity.slots > 64
        || s.capacity.memory_bytes < 65_536
        || s.capacity.cpu_millis == 0
    {
        return Err(Violation::Bound("worker state"));
    }
    if s.boot > s.revision || (s.boot == 0 && !s.records.is_empty()) {
        return Err(Violation::Malformed("worker boot history"));
    }
    for (id, r) in &s.records {
        validate_request(&r.request)?;
        let f = &r.request.fence;
        if f.assignment != *id
            || f.cluster != s.cluster
            || f.node != s.node
            || f.generation != s.generation
            || r.request.policy_ref != s.policy_ref
        {
            return Err(Violation::FenceMismatch);
        }
        if let WorkerPhase::Terminal(result) = r.phase {
            if result.request_ref != r.request_ref || !result.stopped {
                return Err(Violation::FenceMismatch);
            }
        }
    }
    if !s.capacity.fits(s.usage()?) {
        return Err(Violation::Capacity);
    }
    Ok(())
}
pub fn worker_apply(s: &WorkerState, c: &WorkerCommand) -> Result<WorkerState, Violation> {
    validate_worker(s)?;
    let mut n = s.clone();
    match c {
        WorkerCommand::Accept {
            request: r,
            request_ref,
        } => {
            check_request(s, r)?;
            if let Some(old) = s.records.get(&r.fence.assignment) {
                return if old.request == *r && old.request_ref == *request_ref {
                    Ok(s.clone())
                } else {
                    Err(Violation::FenceMismatch)
                };
            }
            if s.records.len() >= MAX_WORKER_RECORDS {
                return Err(Violation::Bound("worker tombstone inventory"));
            }
            let rejected = s.draining || !s.capacity.fits(s.usage()?.checked_add(r.resources)?);
            let phase = if rejected {
                WorkerPhase::Terminal(ExecutionResult {
                    request_ref: *request_ref,
                    outcome: ExecutionOutcome::RejectedBeforeStart,
                    stopped: true,
                })
            } else {
                WorkerPhase::Accepted
            };
            n.records.insert(r.fence.assignment, WorkerRecord {
                request: r.clone(),
                request_ref: *request_ref,
                phase,
                cancel_requested: false,
            });
        }
        WorkerCommand::Begin {
            attempt,
            request_ref,
            boot,
        } => {
            if *boot != s.boot || *boot == 0 {
                return Err(Violation::FenceMismatch);
            }
            let record = n.records.get_mut(attempt).ok_or(Violation::Missing("worker record"))?;
            if record.request_ref != *request_ref || record.cancel_requested || record.phase != WorkerPhase::Accepted {
                return Err(Violation::IllegalTransition);
            }
            record.phase = WorkerPhase::Running;
        }
        WorkerCommand::Cancel {
            request: r,
            request_ref,
        } => {
            check_request(s, r)?;
            if let Some(record) = n.records.get_mut(&r.fence.assignment) {
                if record.request != *r || record.request_ref != *request_ref {
                    return Err(Violation::FenceMismatch);
                }
                match record.phase {
                    WorkerPhase::Terminal(_) => return Ok(s.clone()),
                    WorkerPhase::Accepted => {
                        record.phase = WorkerPhase::Terminal(ExecutionResult {
                            request_ref: *request_ref,
                            outcome: ExecutionOutcome::Cancelled,
                            stopped: true,
                        })
                    }
                    WorkerPhase::Running => {}
                }
                record.cancel_requested = true;
            } else {
                if s.records.len() >= MAX_WORKER_RECORDS {
                    return Err(Violation::Bound("worker tombstone inventory"));
                }
                // Tombstone commits before the cancellation acknowledgement.
                // Any delayed Accept for the same identity returns this result.
                n.records.insert(r.fence.assignment, WorkerRecord {
                    request: r.clone(),
                    request_ref: *request_ref,
                    phase: WorkerPhase::Terminal(ExecutionResult {
                        request_ref: *request_ref,
                        outcome: ExecutionOutcome::RejectedBeforeStart,
                        stopped: true,
                    }),
                    cancel_requested: true,
                });
            }
        }
        WorkerCommand::Finish { attempt, result } => {
            let record = n.records.get_mut(attempt).ok_or(Violation::Missing("worker record"))?;
            if !result.stopped || result.request_ref != record.request_ref {
                return Err(Violation::FenceMismatch);
            }
            if let WorkerPhase::Terminal(old) = record.phase {
                return if old == *result {
                    Ok(s.clone())
                } else {
                    Err(Violation::Conflict("worker completion"))
                };
            }
            let actual = if record.cancel_requested {
                ExecutionResult {
                    outcome: ExecutionOutcome::Cancelled,
                    ..*result
                }
            } else {
                *result
            };
            record.phase = WorkerPhase::Terminal(actual);
        }
        WorkerCommand::RecoverAfterProcessExit => {
            n.boot = n.boot.checked_add(1).ok_or(Violation::Overflow)?;
            // Pure in-process component Stores cannot survive the old process.
            // This is not a rule for native processes, remote effects or WASI.
            for r in n.records.values_mut() {
                if !matches!(r.phase, WorkerPhase::Terminal(_)) {
                    r.phase = WorkerPhase::Terminal(ExecutionResult {
                        request_ref: r.request_ref,
                        outcome: ExecutionOutcome::Failed {
                            code: FailureCode::Interrupted,
                        },
                        stopped: true,
                    });
                }
            }
        }
        WorkerCommand::Drain => n.draining = true,
        WorkerCommand::Resume => n.draining = false,
    }
    if n == *s {
        return Ok(n);
    }
    n.revision = s.revision.checked_add(1).ok_or(Violation::Overflow)?;
    validate_worker(&n)?;
    Ok(n)
}
fn check_request(s: &WorkerState, r: &ExecutionRequest) -> Result<(), Violation> {
    validate_request(r)?;
    let f = &r.fence;
    if f.cluster != s.cluster || f.node != s.node || f.generation != s.generation || r.policy_ref != s.policy_ref {
        return Err(Violation::FenceMismatch);
    }
    Ok(())
}
