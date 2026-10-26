// SPDX-License-Identifier: AGPL-3.0-or-later
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::num::NonZeroU64;

use molten_orchestrator_core as execution;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement {
    pub run: Id,
    pub task: String,
    pub node: Id,
    pub number: u32,
    pub inputs: Vec<Id>,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PendingReason {
    pub run: Id,
    pub task: String,
    pub reason: &'static str,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command<'a> {
    Tick,
    Register(Node),
    ObserveNode {
        node: Id,
        boot: u64,
    },
    Drain {
        node: Id,
        draining: bool,
    },
    Submit {
        key: String,
        spec_ref: Id,
        run: Id,
        spec: JobSpec,
    },
    Schedule {
        placement: Placement,
        request: &'a ExecutionRequest,
        request_ref: Id,
    },
    Started {
        attempt: Id,
        request_ref: Id,
    },
    Uncertain {
        attempt: Id,
        request_ref: Id,
    },
    Terminal {
        attempt: Id,
        result_ref: Id,
        result: ExecutionResult,
    },
    CancelRun {
        run: Id,
    },
}

fn allocation(a: &Attempt) -> Result<execution::Allocation, Violation> {
    let f = &a.request.fence;
    let fence = execution::Fence {
        run: execution::RunId(f.assignment.0),
        assignment: execution::AssignmentRef(f.assignment.0),
        workload: execution::WorkloadRef(f.spec_ref.0),
        node: execution::NodeRef(f.node.0),
        generation: NonZeroU64::new(f.generation).ok_or(Violation::FenceMismatch)?,
        epoch: NonZeroU64::new(u64::from(f.epoch)).ok_or(Violation::FenceMismatch)?,
    };
    let phase = match a.phase {
        AttemptPhase::Dispatching => execution::Phase::Dispatching,
        AttemptPhase::Running => execution::Phase::Running,
        AttemptPhase::Unknown => execution::Phase::Unknown,
        AttemptPhase::Terminal { result_ref, result } => {
            execution::Phase::Terminal(terminal_result(result_ref, result))
        }
    };
    execution::Allocation::restore(fence, a.execution_revision, phase).map_err(|_| Violation::IllegalTransition)
}
fn terminal_result(id: Id, result: ExecutionResult) -> execution::TerminalResult {
    match result.outcome {
        ExecutionOutcome::Succeeded { .. } => execution::TerminalResult::Succeeded(execution::ResultRef(id.0)),
        _ => execution::TerminalResult::Failed(execution::ResultRef(id.0)),
    }
}
fn advance(a: &mut Attempt, event: execution::Event) -> Result<(), Violation> {
    let old = allocation(a)?;
    a.execution_revision = execution::transition(&old, old.fence(), event)
        .map_err(|_| Violation::IllegalTransition)?
        .next()
        .revision();
    Ok(())
}

/// Check restored records before they can produce any effects. Content hashes
/// are additionally rechecked by the canonical store adapter.
pub fn validate_state(s: &State) -> Result<(), Violation> {
    if s.tick > s.revision {
        return Err(Violation::Malformed("logical tick history"));
    }
    if s.generation == 0 || s.freshness_ticks == 0 {
        return Err(Violation::Malformed("state"));
    }
    if s.nodes.len() > MAX_NODES
        || s.runs.len() > MAX_RUNS
        || s.submissions.len() != s.runs.len()
        || s.attempts.len() > MAX_RUNS * MAX_TASKS * MAX_ATTEMPTS as usize
    {
        return Err(Violation::Bound("state inventory"));
    }
    if s.attempts.values().filter(|a| a.reservation_held).count() > MAX_ACTIVE {
        return Err(Violation::Bound("active attempts"));
    }
    for (id, n) in &s.nodes {
        validate_node(n)?;
        if *id != n.id || n.last_seen.is_some_and(|t| t > s.tick) {
            return Err(Violation::Malformed("node observation"));
        }
        if !n.capacity.fits(s.usage(*id)?) {
            return Err(Violation::Capacity);
        }
    }
    let mut submitted = alloc::collections::BTreeSet::new();
    for (key, sub) in &s.submissions {
        if !valid_name(key) || !submitted.insert(sub.run) {
            return Err(Violation::Malformed("submission"));
        }
        if s.runs.get(&sub.run).ok_or(Violation::Missing("submitted run"))?.spec_ref != sub.spec_ref {
            return Err(Violation::FenceMismatch);
        }
    }
    for (id, run) in &s.runs {
        if *id != run.id || !submitted.contains(id) {
            return Err(Violation::FenceMismatch);
        }
        validate_job(&run.spec)?;
        if run.tasks.len() != run.spec.tasks.len() {
            return Err(Violation::Malformed("task inventory"));
        }
        for spec in &run.spec.tasks {
            let t = run.tasks.get(&spec.id).ok_or(Violation::Missing("task state"))?;
            if t.number > spec.max_attempts {
                return Err(Violation::Bound("task counter"));
            }
            let count = s
                .attempts
                .values()
                .filter(|a| a.request.fence.run == *id && a.request.fence.task == spec.id)
                .count();
            if count != t.number as usize {
                return Err(Violation::Malformed("attempt history count"));
            }
            if let TaskPhase::Active { attempt } = t.phase {
                let a = s.attempts.get(&attempt).ok_or(Violation::Missing("active attempt"))?;
                if !a.reservation_held
                    || a.request.fence.run != *id
                    || a.request.fence.task != spec.id
                    || a.request.fence.epoch != t.number
                {
                    return Err(Violation::FenceMismatch);
                }
            }
            if let TaskPhase::Succeeded { output, result } = t.phase {
                if !s.attempts.values().any(|a| a.request.fence.run==*id && a.request.fence.task==spec.id && a.request.fence.epoch==t.number && matches!(a.phase,AttemptPhase::Terminal{result_ref,result:r} if result_ref==result && r.outcome==(ExecutionOutcome::Succeeded{output}))) {return Err(Violation::Malformed("success without current completion"));}
            }
        }
    }
    let mut epochs = alloc::collections::BTreeSet::new();
    for (id, a) in &s.attempts {
        validate_request(&a.request)?;
        allocation(a)?;
        let f = &a.request.fence;
        if *id != f.assignment
            || f.cluster != s.cluster
            || f.generation != s.generation
            || a.request.policy_ref != s.policy_ref
            || !s.nodes.contains_key(&f.node)
        {
            return Err(Violation::FenceMismatch);
        }
        if !epochs.insert((f.run, f.task.clone(), f.epoch)) {
            return Err(Violation::Conflict("duplicate attempt epoch"));
        }
        let r = s.runs.get(&f.run).ok_or(Violation::Missing("attempt run"))?;
        let spec = r.spec.tasks.iter().find(|t| t.id == f.task).ok_or(Violation::Missing("attempt task"))?;
        let t = &r.tasks[&f.task];
        if f.spec_ref != r.spec_ref
            || f.epoch > t.number
            || a.request.component != spec.component
            || a.request.resources != spec.resources
            || a.request.fuel != spec.fuel
            || a.request.timeout_ms != spec.timeout_ms
            || a.request.max_output_bytes != spec.max_output_bytes
        {
            return Err(Violation::FenceMismatch);
        }
        let mut inputs = spec.inputs.clone();
        for d in &spec.dependencies {
            match r.tasks[d].phase {
                TaskPhase::Succeeded { output, .. } => inputs.push(output),
                _ => return Err(Violation::Dependency),
            }
        }
        if a.request.inputs != inputs {
            return Err(Violation::FenceMismatch);
        }
        match a.phase {
            AttemptPhase::Terminal { result, .. } => {
                if a.reservation_held || !result.stopped || result.request_ref != a.request_ref {
                    return Err(Violation::FenceMismatch);
                }
            }
            _ => {
                if !a.reservation_held || t.phase != (TaskPhase::Active { attempt: *id }) {
                    return Err(Violation::Malformed("orphan reservation"));
                }
            }
        }
    }
    Ok(())
}

pub fn validate_placement(s: &State, p: &Placement) -> Result<(), Violation> {
    let r = s.runs.get(&p.run).ok_or(Violation::Missing("run"))?;
    let spec = r.spec.tasks.iter().find(|t| t.id == p.task).ok_or(Violation::Missing("task"))?;
    let t = &r.tasks[&p.task];
    if r.cancel_requested || t.number.checked_add(1) != Some(p.number) || p.number > spec.max_attempts {
        return Err(Violation::Conflict("attempt"));
    }
    match t.phase {
        TaskPhase::Pending { ready_at } if ready_at <= s.tick => {}
        _ => return Err(Violation::IllegalTransition),
    }
    let mut inputs = spec.inputs.clone();
    for d in &spec.dependencies {
        match r.tasks[d].phase {
            TaskPhase::Succeeded { output, .. } => inputs.push(output),
            _ => return Err(Violation::Dependency),
        }
    }
    if inputs != p.inputs {
        return Err(Violation::FenceMismatch);
    }
    let n = s.nodes.get(&p.node).ok_or(Violation::Missing("node"))?;
    if !n.enabled || n.draining || !n.last_seen.is_some_and(|t| s.tick.saturating_sub(t) <= s.freshness_ticks) {
        return Err(Violation::Conflict("node eligibility"));
    }
    if spec.required_labels.iter().any(|(k, v)| n.labels.get(k) != Some(v)) {
        return Err(Violation::Conflict("labels"));
    }
    if !n.capacity.fits(s.usage(p.node)?.checked_add(spec.resources)?)
        || s.attempts.values().filter(|a| a.reservation_held).count() >= MAX_ACTIVE
    {
        return Err(Violation::Capacity);
    }
    Ok(())
}
/// One placement per snapshot. The application asks again after each commit,
/// preventing overcommit through a stale multi-placement plan.
pub fn next_placement(s: &State) -> Result<Option<Placement>, Violation> {
    validate_state(s)?;
    for (id, r) in &s.runs {
        if r.cancel_requested {
            continue;
        }
        for spec in &r.spec.tasks {
            let t = &r.tasks[&spec.id];
            if !matches!(t.phase,TaskPhase::Pending{ready_at} if ready_at<=s.tick) {
                continue;
            }
            let mut inputs = spec.inputs.clone();
            let mut ready = true;
            for d in &spec.dependencies {
                match r.tasks[d].phase {
                    TaskPhase::Succeeded { output, .. } => inputs.push(output),
                    _ => {
                        ready = false;
                        break;
                    }
                }
            }
            if !ready {
                continue;
            }
            for node in s.nodes.keys() {
                let p = Placement {
                    run: *id,
                    task: spec.id.clone(),
                    node: *node,
                    number: t.number + 1,
                    inputs: inputs.clone(),
                };
                if validate_placement(s, &p).is_ok() {
                    return Ok(Some(p));
                }
            }
        }
    }
    Ok(None)
}
pub fn explain(s: &State) -> Vec<PendingReason> {
    let mut out = Vec::new();
    for (id, r) in &s.runs {
        for (task, t) in &r.tasks {
            if let TaskPhase::Pending { ready_at } = t.phase {
                let spec = r.spec.tasks.iter().find(|v| &v.id == task);
                let reason = if ready_at > s.tick {
                    "retry-delay"
                } else if spec.is_some_and(|v| {
                    v.dependencies.iter().any(|d| !matches!(r.tasks[d].phase, TaskPhase::Succeeded { .. }))
                }) {
                    "dependencies"
                } else {
                    "capacity-or-node-eligibility"
                };
                out.push(PendingReason {
                    run: *id,
                    task: task.clone(),
                    reason,
                });
            }
        }
    }
    out
}

pub fn apply(s: &State, c: &Command<'_>) -> Result<State, Violation> {
    validate_state(s)?;
    let mut n = s.clone();
    match c {
        Command::Tick => n.tick = s.tick.checked_add(1).ok_or(Violation::Overflow)?,
        Command::Register(node) => {
            validate_node(node)?;
            if node.last_seen.is_some() || node.boot != 0 {
                return Err(Violation::Malformed("fabricated liveness"));
            }
            if let Some(old) = s.nodes.get(&node.id) {
                return if old.capacity == node.capacity && old.labels == node.labels {
                    Ok(s.clone())
                } else {
                    Err(Violation::Conflict("node descriptor"))
                };
            }
            if n.nodes.len() >= MAX_NODES {
                return Err(Violation::Bound("nodes"));
            }
            n.nodes.insert(node.id, node.clone());
        }
        Command::ObserveNode { node, boot } => {
            let node = n.nodes.get_mut(node).ok_or(Violation::Missing("node"))?;
            if *boot == 0 || *boot < node.boot {
                return Err(Violation::Conflict("worker boot"));
            }
            node.boot = *boot;
            node.last_seen = Some(s.tick);
        }
        Command::Drain { node, draining } => {
            n.nodes.get_mut(node).ok_or(Violation::Missing("node"))?.draining = *draining
        }
        Command::Submit {
            key,
            spec_ref,
            run,
            spec,
        } => {
            validate_job(spec)?;
            if !valid_name(key) {
                return Err(Violation::Malformed("submission key"));
            }
            if let Some(old) = s.submissions.get(key) {
                return if old.spec_ref == *spec_ref && old.run == *run && s.runs[run].spec == *spec {
                    Ok(s.clone())
                } else {
                    Err(Violation::Conflict("submission key"))
                };
            }
            if s.runs.len() >= MAX_RUNS || s.runs.contains_key(run) {
                return Err(Violation::Bound("run inventory"));
            }
            let tasks = spec
                .tasks
                .iter()
                .map(|t| {
                    (t.id.clone(), TaskState {
                        number: 0,
                        phase: TaskPhase::Pending { ready_at: s.tick },
                    })
                })
                .collect::<BTreeMap<_, _>>();
            n.runs.insert(*run, Run {
                id: *run,
                spec_ref: *spec_ref,
                spec: spec.clone(),
                cancel_requested: false,
                tasks,
            });
            n.submissions.insert(key.clone(), Submission {
                spec_ref: *spec_ref,
                run: *run,
            });
        }
        Command::Schedule {
            placement: p,
            request: r,
            request_ref,
        } => {
            let r = *r;
            validate_placement(s, p)?;
            validate_request(r)?;
            let spec = s.runs[&p.run].spec.tasks.iter().find(|t| t.id == p.task).ok_or(Violation::Missing("task"))?;
            let f = &r.fence;
            if f.cluster != s.cluster
                || f.generation != s.generation
                || f.run != p.run
                || f.task != p.task
                || f.node != p.node
                || f.epoch != p.number
                || f.spec_ref != s.runs[&p.run].spec_ref
                || r.policy_ref != s.policy_ref
                || r.component != spec.component
                || r.inputs != p.inputs
                || r.resources != spec.resources
                || r.fuel != spec.fuel
                || r.timeout_ms != spec.timeout_ms
                || r.max_output_bytes != spec.max_output_bytes
            {
                return Err(Violation::FenceMismatch);
            }
            if s.attempts.contains_key(&f.assignment) {
                return Err(Violation::Conflict("used assignment"));
            }
            let t = n
                .runs
                .get_mut(&p.run)
                .ok_or(Violation::Missing("run"))?
                .tasks
                .get_mut(&p.task)
                .ok_or(Violation::Missing("task"))?;
            t.number = p.number;
            t.phase = TaskPhase::Active { attempt: f.assignment };
            n.attempts.insert(f.assignment, Attempt {
                request: r.clone(),
                request_ref: *request_ref,
                phase: AttemptPhase::Dispatching,
                execution_revision: 1,
                reservation_held: true,
            });
        }
        Command::Started { attempt, request_ref } => {
            let a = n.attempts.get_mut(attempt).ok_or(Violation::Missing("attempt"))?;
            if a.request_ref != *request_ref {
                return Err(Violation::FenceMismatch);
            }
            if matches!(a.phase, AttemptPhase::Dispatching) {
                advance(a, execution::Event::StartObserved)?;
                a.phase = AttemptPhase::Running;
            }
        }
        Command::Uncertain { attempt, request_ref } => {
            let a = n.attempts.get_mut(attempt).ok_or(Violation::Missing("attempt"))?;
            if a.request_ref != *request_ref {
                return Err(Violation::FenceMismatch);
            }
            if matches!(a.phase, AttemptPhase::Dispatching) {
                advance(a, execution::Event::StartUncertain)?;
                a.phase = AttemptPhase::Unknown;
            }
        }
        Command::Terminal {
            attempt,
            result_ref,
            result,
        } => finish(&mut n, *attempt, *result_ref, *result)?,
        Command::CancelRun { run } => {
            let r = n.runs.get_mut(run).ok_or(Violation::Missing("run"))?;
            if matches!(r.phase(), RunPhase::Succeeded | RunPhase::Failed | RunPhase::Cancelled) {
                return Ok(s.clone());
            }
            r.cancel_requested = true;
            for t in r.tasks.values_mut() {
                if matches!(t.phase, TaskPhase::Pending { .. } | TaskPhase::Blocked) {
                    t.phase = TaskPhase::Cancelled;
                }
            }
        }
    }
    for r in n.runs.values_mut() {
        for _ in 0..r.spec.tasks.len() {
            let mut changed = false;
            for spec in &r.spec.tasks {
                if matches!(r.tasks[&spec.id].phase, TaskPhase::Pending { .. })
                    && spec.dependencies.iter().any(|d| {
                        matches!(r.tasks[d].phase, TaskPhase::Failed { .. } | TaskPhase::Blocked | TaskPhase::Cancelled)
                    })
                {
                    if let Some(t) = r.tasks.get_mut(&spec.id) {
                        t.phase = TaskPhase::Blocked;
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
    }
    if n == *s {
        return Ok(n);
    }
    n.revision = s.revision.checked_add(1).ok_or(Violation::Overflow)?;
    validate_state(&n)?;
    Ok(n)
}
fn finish(s: &mut State, id: Id, result_ref: Id, result: ExecutionResult) -> Result<(), Violation> {
    let a = s.attempts.get_mut(&id).ok_or(Violation::Missing("attempt"))?;
    if result.request_ref != a.request_ref || !result.stopped {
        return Err(Violation::FenceMismatch);
    }
    if let AttemptPhase::Terminal {
        result_ref: old,
        result: old_result,
    } = a.phase
    {
        return if old == result_ref && old_result == result {
            Ok(())
        } else {
            Err(Violation::Conflict("authoritative completion"))
        };
    }
    advance(a, execution::Event::Complete(terminal_result(result_ref, result)))?;
    a.phase = AttemptPhase::Terminal { result_ref, result };
    a.reservation_held = false;
    let r = s.runs.get_mut(&a.request.fence.run).ok_or(Violation::Missing("run"))?;
    let spec = r.spec.tasks.iter().find(|t| t.id == a.request.fence.task).ok_or(Violation::Missing("task"))?;
    let t = r.tasks.get_mut(&a.request.fence.task).ok_or(Violation::Missing("task state"))?;
    if t.phase != (TaskPhase::Active { attempt: id }) {
        return Err(Violation::FenceMismatch);
    }
    t.phase = if r.cancel_requested {
        TaskPhase::Cancelled
    } else {
        match result.outcome {
            ExecutionOutcome::Succeeded { output } => TaskPhase::Succeeded {
                output,
                result: result_ref,
            },
            _ if t.number < spec.max_attempts => TaskPhase::Pending {
                ready_at: s.tick.checked_add(spec.retry_delay_ticks).ok_or(Violation::Overflow)?,
            },
            _ => TaskPhase::Failed { result: result_ref },
        }
    };
    Ok(())
}
