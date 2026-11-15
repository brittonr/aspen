// SPDX-License-Identifier: AGPL-3.0-or-later
//! Versioned observation DTOs. No observation grants execution or artifact authority.
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use serde::Deserialize;
use serde::Serialize;

use crate::*;

pub const PROGRESS_SCHEMA: &str = "batch-progress.v1";
pub const MAX_PROGRESS_PAGE: u32 = 64;
pub const MAX_BATCH_PROGRESS_EVENTS: usize = 256;
pub const MAX_PROGRESS_REPLY_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressCursor {
    pub service_id: String,
    pub stream_epoch: u64,
    pub revision: u64,
    pub ordinal: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgressPayload {
    AcceptedOutput {
        fence: Fence,
        request: Id,
        result: Id,
        output: Id,
    },
    RunTerminal {
        cluster: Id,
        generation: u64,
        run: Id,
        spec_ref: Id,
        phase: RunPhase,
        required_outputs: BTreeMap<String, Id>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressEvent {
    pub event_ref: String,
    pub cursor: ProgressCursor,
    pub payload_ref: Id,
    pub committed_at_tick: u64,
    pub payload: ProgressPayload,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgressTask {
    Pending {
        ready_at: u64,
    },
    Dispatching {
        assignment: Id,
        epoch: u32,
    },
    Running {
        assignment: Id,
        epoch: u32,
    },
    Uncertain {
        assignment: Id,
        epoch: u32,
    },
    Accepted {
        output: Id,
        result: Id,
        fence: Box<Fence>,
        request: Id,
    },
    Failed {
        result: Id,
    },
    Cancelled,
    Blocked,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressSnapshot {
    pub schema: String,
    pub cluster: Id,
    pub generation: u64,
    pub run: Id,
    pub spec_ref: Id,
    pub phase: RunPhase,
    pub cancel_requested: bool,
    pub tasks: BTreeMap<String, ProgressTask>,
    pub state_ref: Id,
    pub state_revision: u64,
    pub watermark: ProgressCursor,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgressPage {
    Events {
        events: Vec<ProgressEvent>,
        next: ProgressCursor,
    },
    ResnapshotRequired {
        from: ProgressCursor,
    },
    StaleEpoch {
        current_epoch: u64,
    },
    ForeignService,
}

pub fn terminal_run(phase: RunPhase) -> bool {
    matches!(phase, RunPhase::Succeeded | RunPhase::Failed | RunPhase::Cancelled)
}

fn accepted_attempt<'a>(
    state: &'a State,
    run: Id,
    task: &str,
    output: Id,
    result: Id,
) -> Result<&'a Attempt, Violation> {
    let mut matching = state.attempts.values().filter(|a| {
        a.request.fence.run == run && a.request.fence.task == task && matches!(a.phase,
            AttemptPhase::Terminal { result_ref, result: ExecutionResult { outcome: ExecutionOutcome::Succeeded { output: o }, stopped: true, .. } }
            if result_ref == result && o == output)
    });
    let a = matching.next().ok_or(Violation::Malformed("accepted attempt missing"))?;
    if matching.next().is_some() {
        return Err(Violation::Malformed("ambiguous accepted attempt"));
    }
    Ok(a)
}

pub fn progress_tasks(state: &State, run: &Run) -> Result<BTreeMap<String, ProgressTask>, Violation> {
    run.tasks
        .iter()
        .map(|(name, task)| {
            let projected = match task.phase {
                TaskPhase::Pending { ready_at } => ProgressTask::Pending { ready_at },
                TaskPhase::Active { attempt } => {
                    let a = state.attempts.get(&attempt).ok_or(Violation::Malformed("active attempt missing"))?;
                    let epoch = a.request.fence.epoch;
                    match a.phase {
                        AttemptPhase::Dispatching => ProgressTask::Dispatching {
                            assignment: attempt,
                            epoch,
                        },
                        AttemptPhase::Running => ProgressTask::Running {
                            assignment: attempt,
                            epoch,
                        },
                        _ => ProgressTask::Uncertain {
                            assignment: attempt,
                            epoch,
                        },
                    }
                }
                TaskPhase::Succeeded { output, result } => {
                    let a = accepted_attempt(state, run.id, name, output, result)?;
                    ProgressTask::Accepted {
                        output,
                        result,
                        fence: Box::new(a.request.fence.clone()),
                        request: a.request_ref,
                    }
                }
                TaskPhase::Failed { result } => ProgressTask::Failed { result },
                TaskPhase::Cancelled => ProgressTask::Cancelled,
                TaskPhase::Blocked => ProgressTask::Blocked,
            };
            Ok((name.clone(), projected))
        })
        .collect()
}

/// Bootstrap uses no previous state; subsequent commits only project newly accepted outputs.
pub fn progress_delta(previous: Option<&State>, next: &State) -> Result<Vec<ProgressPayload>, Violation> {
    let mut out = Vec::new();
    for run in next.runs.values() {
        let old = previous.and_then(|s| s.runs.get(&run.id));
        for (name, task) in &run.tasks {
            if let TaskPhase::Succeeded { output, result } = task.phase {
                if old.and_then(|r| r.tasks.get(name)).is_some_and(|t| t.phase == task.phase) {
                    continue;
                }
                let a = accepted_attempt(next, run.id, name, output, result)?;
                out.push(ProgressPayload::AcceptedOutput {
                    fence: a.request.fence.clone(),
                    request: a.request_ref,
                    result,
                    output,
                });
            }
        }
        let phase = run.phase();
        if terminal_run(phase) && old.is_none_or(|r| r.phase() != phase) {
            let required_outputs = run
                .tasks
                .iter()
                .filter_map(|(name, task)| match task.phase {
                    TaskPhase::Succeeded { output, .. } => Some((name.clone(), output)),
                    _ => None,
                })
                .collect();
            out.push(ProgressPayload::RunTerminal {
                cluster: next.cluster,
                generation: next.generation,
                run: run.id,
                spec_ref: run.spec_ref,
                phase,
                required_outputs,
            });
        }
    }
    Ok(out)
}
