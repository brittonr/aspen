// SPDX-License-Identifier: AGPL-3.0-or-later
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;

pub const MAX_RUNS: usize = 128;
pub const MAX_TASKS: usize = 64;
pub const MAX_NODES: usize = 32;
pub const MAX_ATTEMPTS: u32 = 8;
pub const MAX_INPUTS: usize = 64;
pub const MAX_ACTIVE: usize = 128;
pub const MAX_CONTENT_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_STATE_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_FRAME_BYTES: usize = 40 * 1024 * 1024;
pub const MAX_CONTENT_OBJECTS: u64 = 4096;
pub const MAX_CONTENT_TOTAL_BYTES: u64 = 1024 * 1024 * 1024;
pub const MAX_WORKER_RECORDS: usize = 4096;
pub const PROFILE: &str = "molten-orchestrator/pure-component-batch/v1";
pub const ABI: &str = "run(list<list<u8>>)->list<u8>";

/// Public identity bytes. Parsing an identity never confers authority; its
/// domain is bound by its named field and the versioned canonical schema.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Id(pub [u8; 32]);
impl Id {
    pub fn hex(self) -> String {
        let mut result = String::with_capacity(64);
        for b in self.0 {
            use core::fmt::Write;
            let _ = write!(&mut result, "{b:02x}");
        }
        result
    }
    pub fn parse(s: &str) -> Result<Self, Violation> {
        if s.len() != 64 {
            return Err(Violation::Malformed("identity"));
        }
        let mut bytes = [0; 32];
        for (i, pair) in s.as_bytes().chunks_exact(2).enumerate() {
            let digit = |v| match v {
                b'0'..=b'9' => Ok(v - b'0'),
                b'a'..=b'f' => Ok(v - b'a' + 10),
                _ => Err(Violation::Malformed("identity")),
            };
            bytes[i] = digit(pair[0])? * 16 + digit(pair[1])?;
        }
        Ok(Self(bytes))
    }
}
impl fmt::Debug for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.hex())
    }
}
impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.hex())
    }
}
impl Serialize for Id {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.hex())
    }
}
impl<'de> Deserialize<'de> for Id {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::parse(&String::deserialize(d)?)
            .map_err(|_| serde::de::Error::custom("expected 64 lowercase hex characters"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resources {
    pub cpu_millis: u64,
    pub memory_bytes: u64,
    pub slots: u32,
}
impl Resources {
    pub fn fits(self, r: Self) -> bool {
        self.cpu_millis >= r.cpu_millis && self.memory_bytes >= r.memory_bytes && self.slots >= r.slots
    }
    // r[impl molten.batch_orchestration.repairs]
    pub fn checked_add(self, r: Self) -> Result<Self, Violation> {
        Ok(Self {
            cpu_millis: self.cpu_millis.checked_add(r.cpu_millis).ok_or(Violation::Overflow)?,
            memory_bytes: self.memory_bytes.checked_add(r.memory_bytes).ok_or(Violation::Overflow)?,
            slots: self.slots.checked_add(r.slots).ok_or(Violation::Overflow)?,
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentKind {
    Component,
    Data,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentManifest {
    pub kind: ContentKind,
    pub raw_digest: Id,
    pub length: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpec {
    pub id: String,
    pub component: Id,
    pub inputs: Vec<Id>,
    pub dependencies: Vec<String>,
    pub resources: Resources,
    pub required_labels: BTreeMap<String, String>,
    pub fuel: u64,
    pub timeout_ms: u64,
    pub max_output_bytes: u64,
    pub max_attempts: u32,
    pub retry_delay_ticks: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobSpec {
    pub name: String,
    pub tasks: Vec<TaskSpec>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskPhase {
    Pending { ready_at: u64 },
    Active { attempt: Id },
    Succeeded { output: Id, result: Id },
    Failed { result: Id },
    Cancelled,
    Blocked,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskState {
    pub number: u32,
    pub phase: TaskPhase,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Submission {
    pub spec_ref: Id,
    pub run: Id,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Run {
    pub id: Id,
    pub spec_ref: Id,
    pub spec: JobSpec,
    pub cancel_requested: bool,
    pub tasks: BTreeMap<String, TaskState>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunPhase {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelling,
    Cancelled,
}
impl Run {
    pub fn phase(&self) -> RunPhase {
        let active = self.tasks.values().any(|t| matches!(t.phase, TaskPhase::Active { .. }));
        if self.cancel_requested {
            return if active {
                RunPhase::Cancelling
            } else {
                RunPhase::Cancelled
            };
        }
        if active {
            return RunPhase::Running;
        }
        if self.tasks.values().any(|t| matches!(t.phase, TaskPhase::Pending { .. })) {
            return RunPhase::Pending;
        }
        if self.tasks.values().all(|t| matches!(t.phase, TaskPhase::Succeeded { .. })) {
            RunPhase::Succeeded
        } else {
            RunPhase::Failed
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: Id,
    pub capacity: Resources,
    pub labels: BTreeMap<String, String>,
    pub draining: bool,
    pub enabled: bool,
    pub last_seen: Option<u64>,
    pub boot: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fence {
    pub cluster: Id,
    pub generation: u64,
    pub run: Id,
    pub task: String,
    pub assignment: Id,
    pub spec_ref: Id,
    pub node: Id,
    pub epoch: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionRequest {
    pub fence: Fence,
    pub policy_ref: Id,
    pub component: Id,
    pub inputs: Vec<Id>,
    pub resources: Resources,
    pub fuel: u64,
    pub timeout_ms: u64,
    pub max_output_bytes: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureCode {
    Rejected,
    Compile,
    Instantiate,
    Trap,
    FuelOrDeadline,
    OutputLimit,
    Interrupted,
    Internal,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionOutcome {
    Succeeded { output: Id },
    Failed { code: FailureCode },
    Cancelled,
    RejectedBeforeStart,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionResult {
    pub request_ref: Id,
    pub outcome: ExecutionOutcome,
    pub stopped: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttemptPhase {
    Dispatching,
    Running,
    Unknown,
    Terminal { result_ref: Id, result: ExecutionResult },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attempt {
    pub request: ExecutionRequest,
    pub request_ref: Id,
    pub phase: AttemptPhase,
    pub execution_revision: u64,
    pub reservation_held: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub cluster: Id,
    pub generation: u64,
    pub policy_ref: Id,
    pub revision: u64,
    pub tick: u64,
    pub freshness_ticks: u64,
    pub nodes: BTreeMap<Id, Node>,
    pub submissions: BTreeMap<String, Submission>,
    pub runs: BTreeMap<Id, Run>,
    pub attempts: BTreeMap<Id, Attempt>,
}
impl State {
    pub fn empty(cluster: Id, policy_ref: Id, generation: u64, freshness_ticks: u64) -> Result<Self, Violation> {
        if generation == 0 || freshness_ticks == 0 {
            return Err(Violation::Malformed("generation/freshness"));
        }
        Ok(Self {
            cluster,
            generation,
            policy_ref,
            revision: 0,
            tick: 0,
            freshness_ticks,
            nodes: BTreeMap::new(),
            submissions: BTreeMap::new(),
            runs: BTreeMap::new(),
            attempts: BTreeMap::new(),
        })
    }
    pub fn usage(&self, node: Id) -> Result<Resources, Violation> {
        self.attempts
            .values()
            .filter(|a| a.reservation_held && a.request.fence.node == node)
            .try_fold(Resources::default(), |sum, a| sum.checked_add(a.request.resources))
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    Malformed(&'static str),
    Bound(&'static str),
    Overflow,
    Missing(&'static str),
    Conflict(&'static str),
    FenceMismatch,
    Capacity,
    Dependency,
    Cycle,
    IllegalTransition,
}
impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("{self:?}"))
    }
}

pub fn valid_name(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64 && s.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}
pub fn validate_labels(m: &BTreeMap<String, String>) -> Result<(), Violation> {
    if m.len() > 32 || m.iter().any(|(k, v)| !valid_name(k) || !valid_name(v)) {
        return Err(Violation::Malformed("labels"));
    }
    Ok(())
}
pub fn validate_resources(r: Resources) -> Result<(), Violation> {
    if r.cpu_millis == 0
        || r.cpu_millis > 128_000
        || r.memory_bytes < 65_536
        || r.memory_bytes > 1024 * 1024 * 1024
        || r.slots != 1
    {
        return Err(Violation::Bound("task resources"));
    }
    Ok(())
}
pub fn validate_request(r: &ExecutionRequest) -> Result<(), Violation> {
    validate_resources(r.resources)?;
    if r.fence.generation == 0 || r.fence.epoch == 0 || r.fence.epoch > MAX_ATTEMPTS || !valid_name(&r.fence.task) {
        return Err(Violation::FenceMismatch);
    }
    if r.inputs.len() > MAX_INPUTS
        || r.fuel == 0
        || r.fuel > 100_000_000_000
        || r.timeout_ms == 0
        || r.timeout_ms > 3_600_000
        || r.max_output_bytes > MAX_CONTENT_BYTES as u64
    {
        return Err(Violation::Bound("execution limits"));
    }
    Ok(())
}
pub fn validate_node(n: &Node) -> Result<(), Violation> {
    validate_labels(&n.labels)?;
    if n.capacity.slots == 0 || n.capacity.slots > 64 || n.capacity.cpu_millis == 0 || n.capacity.memory_bytes < 65_536
    {
        return Err(Violation::Bound("node capacity"));
    }
    Ok(())
}
pub fn validate_job(s: &JobSpec) -> Result<(), Violation> {
    use alloc::collections::BTreeSet;
    if !valid_name(&s.name) || s.tasks.is_empty() || s.tasks.len() > MAX_TASKS {
        return Err(Violation::Bound("job"));
    }
    if s.tasks.windows(2).any(|w| w[0].id >= w[1].id) {
        return Err(Violation::Malformed("tasks must be sorted unique"));
    }
    let ids: BTreeSet<_> = s.tasks.iter().map(|t| t.id.as_str()).collect();
    for t in &s.tasks {
        validate_resources(t.resources)?;
        validate_labels(&t.required_labels)?;
        if !valid_name(&t.id)
            || t.max_attempts == 0
            || t.max_attempts > MAX_ATTEMPTS
            || t.inputs.len() + t.dependencies.len() > MAX_INPUTS
        {
            return Err(Violation::Bound("task"));
        }
        if t.fuel == 0
            || t.fuel > 100_000_000_000
            || t.timeout_ms == 0
            || t.timeout_ms > 3_600_000
            || t.max_output_bytes > MAX_CONTENT_BYTES as u64
        {
            return Err(Violation::Bound("task limits"));
        }
        if t.dependencies.windows(2).any(|w| w[0] >= w[1])
            || t.dependencies.iter().any(|d| d == &t.id || !ids.contains(d.as_str()))
        {
            return Err(Violation::Dependency);
        }
    }
    let mut ready = BTreeSet::new();
    for _ in 0..s.tasks.len() {
        let before = ready.len();
        for t in &s.tasks {
            if t.dependencies.iter().all(|d| ready.contains(d.as_str())) {
                ready.insert(t.id.as_str());
            }
        }
        if ready.len() == ids.len() {
            return Ok(());
        }
        if ready.len() == before {
            break;
        }
    }
    Err(Violation::Cycle)
}
