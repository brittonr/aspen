//! Bounded sequential I64 interaction nets (`ngraph-net-i64/3`).
//!
//! Lowering accepts only independently checked I64 graphs with I64 inputs,
//! I64 constants, wrapping binary I64 operations, equality, and ordered
//! scalar outputs. The candidate is rebuilt from that checked graph before
//! every reduction; the build key is not treated as proof of correspondence.

use crate::identity::IdentityScheme;

#[path = "net/rewrite.rs"]
mod rewrite;

/// Selected graph-to-net and rule contract.
pub const CONTRACT_REVISION: &str = "ngraph-net-i64/3";
/// Canonical graph-to-agent and use-order mapping.
pub const LOWERING_REVISION: &str = "ngraph-net-i64-lowering/0";
/// Version of the emitted, independently checked in-place rewrite plans.
pub const PLAN_REVISION: &str = "ngraph-net-plan/1";
/// Version of the exact arithmetic, fanout, and erasure rules compiled into plans.
pub const PLAN_RULES_REVISION: &str = "noble-net-i64-rules/3";
/// Version of the concrete arena/worklist representation used by the plan path.
pub const PLAN_REPRESENTATION_REVISION: &str = "noble-net-arena/1";
/// Version of the independent byte decoder and plan checker.
pub const PLAN_CHECKER_REVISION: &str = "noble-net-plan-checker/1";
/// Version of the plan executor.
pub const PLAN_EXECUTOR_REVISION: &str = "noble-net-plan-executor/1";

/// A checked I64 operation carried by an `op2`/`op1` agent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Op {
    /// Wrapping addition.
    Add,
    /// Wrapping subtraction: operand 1 minus operand 2.
    Sub,
    /// Wrapping multiplication.
    Mul,
    /// I64 equality, represented by 0 or 1 in the net.
    Eq,
}

impl Op {
    /// Stable operation spelling.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Sub => "sub",
            Self::Mul => "mul",
            Self::Eq => "eq",
        }
    }
}

/// Revision-3 interaction-system symbols.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Symbol {
    /// Exact 64-bit payload.
    Num(u64),
    /// Principal port consumes operand 1; auxiliary ports receive operand 2
    /// and the result consumer, respectively.
    Op2(Op),
    /// Staged operation with operand 1 retained as exact bits.
    Op1(Op, u64),
    /// Two-copy fanout.
    Dup,
    /// Erasure of one unused value.
    Era,
}

impl Symbol {
    /// Number of agent ports, including the principal port.
    #[must_use]
    pub const fn port_count(self) -> usize {
        match self {
            Self::Num(_) | Self::Era => 1,
            Self::Op1(..) => 2,
            Self::Op2(_) | Self::Dup => 3,
        }
    }
}

/// Source role retained on each translated agent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Origin {
    /// One source graph node's value producer or operation.
    Node(crate::ir::NodeId),
    /// A copy in the canonical left-leaning fanout chain for a source value.
    Dup {
        /// Value-producing graph node.
        value: crate::ir::NodeId,
        /// Zero-based chain position.
        ordinal: u32,
    },
    /// The eraser for a source value that has no consumers or outputs.
    Era(crate::ir::NodeId),
}

impl Origin {
    fn key(self) -> (u32, u8, u32) {
        match self {
            Self::Node(node) => (node.0, 0, 0),
            Self::Dup { value, ordinal } => (value.0, 1, ordinal),
            Self::Era(value) => (value.0, 2, 0),
        }
    }
}

/// Agent port reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PortIndex(u8);

impl From<u8> for PortIndex {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl From<PortIndex> for usize {
    fn from(value: PortIndex) -> Self {
        usize::from(value.0)
    }
}

impl From<PortIndex> for u8 {
    fn from(value: PortIndex) -> Self {
        value.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PortRef {
    /// Index in [`InteractionGraph::agents`].
    pub agent: u32,
    /// Port 0 is principal; ports 1 and 2 are auxiliary.
    pub port_index: PortIndex,
}

/// The other end of one wire; `Output` is an ordered free interface port.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Endpoint {
    /// Another agent port.
    Port(PortRef),
    /// Ordered free output port.
    Output(u32),
}

/// One translated agent and the endpoint incident to each of its ports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Agent {
    /// Graph node or generated fanout/erase source.
    pub origin: Origin,
    /// Interaction-system symbol.
    pub symbol: Symbol,
    /// Principal and auxiliary wire endpoints; unused array entries are `None`.
    pub ports: [Option<Endpoint>; 3],
}

/// One graph- and input-specific lowered net.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InteractionGraph {
    /// Checked graph identity.
    pub graph_id: crate::ir::Digest,
    /// Contract identity carried by this candidate.
    pub contract_revision: &'static str,
    /// Lowering identity carried by this candidate.
    pub lowering_revision: &'static str,
    /// Build key over graph identity, contract/lowering revisions, and inputs.
    pub build_key: crate::ir::Digest,
    /// Input values in the checked graph's interface order.
    pub input_values: alloc::vec::Vec<i64>,
    /// Source nodes first, then generated duplicators/erasers.
    pub agents: alloc::vec::Vec<Agent>,
    /// Each entry points at the net-side end of the corresponding free output.
    pub outputs: alloc::vec::Vec<PortRef>,
    /// Type used when decoding each ordered output.
    pub output_types: alloc::vec::Vec<crate::ir::ScalarType>,
}

impl InteractionGraph {
    /// Number of undirected agent/wire and agent/output connections.
    #[must_use]
    pub fn wire_count(&self) -> Option<u64> {
        wire_count(&self.agents)
    }

    /// A capacity-aware estimate of the retained Rust-owned net storage.
    #[must_use]
    pub fn retained_bytes(&self) -> u64 {
        let mut bytes = u64::try_from(core::mem::size_of::<Self>()).unwrap_or(u64::MAX);
        for (element_slots, element) in [
            (self.input_values.capacity(), core::mem::size_of::<i64>()),
            (self.agents.capacity(), core::mem::size_of::<Agent>()),
            (self.outputs.capacity(), core::mem::size_of::<PortRef>()),
            (
                self.output_types.capacity(),
                core::mem::size_of::<crate::ir::ScalarType>(),
            ),
        ] {
            let element_bytes = u64::try_from(element).unwrap_or(u64::MAX);
            let element_slots = u64::try_from(element_slots).unwrap_or(u64::MAX);
            bytes = bytes.saturating_add(element_slots.saturating_mul(element_bytes));
        }
        bytes
    }

    /// `(ops, duplicators, erasers)` in the original translated net.
    #[must_use]
    pub fn rule_counts(&self) -> (u64, u64, u64) {
        let mut ops = 0u64;
        let mut dups = 0u64;
        let mut eras = 0u64;
        for agent in &self.agents {
            match agent.symbol {
                Symbol::Op2(_) => ops = ops.saturating_add(1),
                Symbol::Dup => dups = dups.saturating_add(1),
                Symbol::Era => eras = eras.saturating_add(1),
                Symbol::Num(_) | Symbol::Op1(..) => {}
            }
        }
        (ops, dups, eras)
    }

    /// Contract interaction count `2 * ops + dups + eras` for the initial net.
    #[must_use]
    pub fn expected_interactions(&self) -> Option<u64> {
        let (ops, dups, eras) = self.rule_counts();
        ops.checked_mul(2)?.checked_add(dups)?.checked_add(eras)
    }
}

/// Net resource limits charged before allocation/growth.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Maximum initial agents and fixed reducer arena slots.
    pub max_agent_slots: u64,
    /// Maximum initial wires, including free output connections.
    pub max_wires: u64,
    /// Maximum interaction steps.
    pub interaction_fuel: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_agent_slots: 1_048_576,
            max_wires: 2_097_152,
            interaction_fuel: 2_097_152,
        }
    }
}

/// Resource whose limit was reached.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Limit {
    /// Interaction steps.
    Fuel,
    /// Initial agents / fixed arena slots.
    Agents,
    /// Initial wire connections.
    Wires,
    /// Additional arena slots required by an allocating plan.
    ArenaReservation,
    /// Additional active-pair queue entries required by a plan.
    WorklistReservation,
    /// Fixed plan-decoder scratch requirement.
    PlanScratch,
}

impl Limit {
    /// Stable limit spelling.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Fuel => "fuel",
            Self::Agents => "agents",
            Self::Wires => "wires",
            Self::ArenaReservation => "arena-reservation",
            Self::WorklistReservation => "worklist-reservation",
            Self::PlanScratch => "plan-scratch",
        }
    }
}

/// Storage policy used by the checked plan executor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StoragePolicy {
    /// Reuse consumed slots whenever possible; this is the default policy.
    #[default]
    Reuse,
    /// Allocate fresh result slots and free every consumed slot, as a baseline.
    Allocate,
}

/// A malformed candidate-net condition detected before or during reduction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Invalid {
    /// Candidate is tied to another graph.
    GraphIdentity,
    /// Candidate uses a different net-contract revision.
    ContractRevision,
    /// Candidate uses a different lowering revision.
    LoweringRevision,
    /// Candidate build key is stale or forged.
    BuildKey,
    /// Candidate's stored input values differ from the supplied row.
    InputValues,
    /// Agent or port index is outside the candidate.
    UnknownPort,
    /// An agent has a missing or extra port for its declared symbol.
    PortArity,
    /// A wire is not reciprocal.
    NonReciprocal,
    /// A port points to itself.
    SelfWire,
    /// Output boundary has a missing, repeated or wrong endpoint.
    OutputBoundary,
    /// More than one agent has the same source origin.
    DuplicateOrigin,
    /// Agent origins do not correspond to the checked graph translation.
    SourceOrigin,
    /// Agent symbol or exact numeric payload differs from the checked graph.
    SourceSymbol,
    /// Wire endpoint or ordered operand/output mapping differs from the graph.
    SourceWiring,
    /// Candidate exceeds the configured agent cap.
    AgentCapacity,
    /// Candidate exceeds the configured wire cap.
    WireCapacity,
    /// An active principal pair has no rule in revision 3.
    StuckPair,
    /// A complete reduction leaves an agent outside the output boundary.
    LeftoverAgent,
    /// A normal-form boundary does not point to one exact numeric value.
    ReadBack,
    /// A checked counter could not be represented or advanced.
    CounterOverflow,
    /// The active-pair worklist exceeded its fixed agent-derived bound.
    WorklistBound,
    /// A plan's bytes are malformed or contain an unsupported encoding.
    PlanEncoding,
    /// A decoded plan does not describe the selected live interaction.
    PlanMismatch,
    /// A plan boundary aliases a consumed slot or another captured boundary.
    PlanBoundaryAlias,
    /// A plan names a stale or already freed arena slot.
    StalePlanSlot,
    /// A plan frees one consumed slot more than once.
    DuplicateFree,
    /// A plan publishes an active pair more than once.
    DoubleEnqueue,
    /// A plan version is not supported.
    PlanVersion,
    /// A required memory reservation failed before commit.
    PlanReservationFailure,
    /// An injected in-flight interruption rolled back before commit.
    PlanInterrupted,
}

impl Invalid {
    /// Stable failure-class spelling.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::GraphIdentity => "graph-identity",
            Self::ContractRevision => "contract-revision",
            Self::LoweringRevision => "lowering-revision",
            Self::BuildKey => "build-key",
            Self::InputValues => "input-values",
            Self::UnknownPort => "unknown-port",
            Self::PortArity => "port-arity",
            Self::NonReciprocal => "non-reciprocal-wire",
            Self::SelfWire => "self-wire",
            Self::OutputBoundary => "output-boundary",
            Self::DuplicateOrigin => "duplicate-origin",
            Self::SourceOrigin => "source-origin",
            Self::SourceSymbol => "source-symbol",
            Self::SourceWiring => "source-wiring",
            Self::AgentCapacity => "agent-capacity",
            Self::WireCapacity => "wire-capacity",
            Self::StuckPair => "stuck-active-pair",
            Self::LeftoverAgent => "leftover-agent",
            Self::ReadBack => "read-back",
            Self::CounterOverflow => "counter-overflow",
            Self::WorklistBound => "worklist-bound",
            Self::PlanEncoding => "plan-encoding",
            Self::PlanMismatch => "plan-mismatch",
            Self::PlanBoundaryAlias => "plan-boundary-alias",
            Self::StalePlanSlot => "stale-plan-slot",
            Self::DuplicateFree => "duplicate-free",
            Self::DoubleEnqueue => "double-enqueue",
            Self::PlanVersion => "plan-version",
            Self::PlanReservationFailure => "plan-reservation-failure",
            Self::PlanInterrupted => "plan-interrupted",
        }
    }
}

/// Admission, interface, or capacity refusal before reduction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Graph profile is not the revision-3 I64 profile.
    Profile,
    /// At least one source operation is outside the explicit whitelist.
    Operation,
    /// The graph carries facts, which this net profile does not implement.
    Facts,
    /// The graph contains param slots.
    Params,
    /// Inputs do not match the ordered I64 input interface.
    Interface,
    /// A checked resource limit would be exceeded.
    Exhausted {
        /// Resource cap.
        limit: Limit,
        /// Configured maximum.
        cap: u64,
        /// Required count (or the first interaction over fuel).
        observed: u64,
    },
    /// Lowering/validation discovered an internal inconsistency.
    InternalInvalid(Invalid),
}

impl Refusal {
    /// Stable class spelling.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Profile => "profile",
            Self::Operation => "operation",
            Self::Facts => "facts",
            Self::Params => "params",
            Self::Interface => "interface",
            Self::Exhausted { limit, .. } => limit.name(),
            Self::InternalInvalid(invalid) => invalid.name(),
        }
    }
}

/// Worklist selection policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Schedule {
    /// Last active pair first.
    Lifo,
    /// First active pair first.
    Fifo,
    /// Uniform-index choice driven by the named deterministic generator.
    SplitMix64 {
        /// Recorded seed.
        seed: u64,
    },
}

/// Callback observing the exact emitted plan bytes after independent checking.
pub type PlanObserver<'a> = &'a dyn Fn(&[u8]);

/// Per-invocation execution controls.
#[derive(Clone, Copy, Default)]
pub struct Plan<'a> {
    /// Pair selection (defaults to LIFO).
    pub schedule: Option<Schedule>,
    /// Optional stricter fuel limit, in addition to [`Limits::interaction_fuel`].
    pub interaction_fuel: Option<u64>,
    /// Called immediately before the next firing with the completed step count.
    pub cancel: Option<&'a dyn Fn(u64) -> bool>,
    /// Optional storage-policy override; absent selects [`StoragePolicy::Reuse`].
    pub storage_policy: Option<StoragePolicy>,
    /// Optional upper bound for per-plan scratch bytes.
    pub max_plan_scratch_bytes: Option<u64>,
    /// Optional upper bound for additional queue entries reserved by one plan.
    pub max_worklist_entries: Option<u64>,
    /// Optional observer for the exact plan bytes after independent checking.
    pub plan_observer: Option<PlanObserver<'a>>,
}

/// Borrowed checked inputs and controls for one reduction.
#[derive(Clone, Copy)]
pub struct ReduceRequest<'a> {
    /// Independently checked source graph.
    pub checked: &'a crate::check::CheckedGraph,
    /// Lowered candidate to validate and reduce.
    pub candidate: &'a InteractionGraph,
    /// Input values in the checked graph's interface order.
    pub inputs: &'a [crate::sem::Value],
    /// Admission and execution resource limits.
    pub limits: &'a Limits,
    /// Schedule, optional tighter fuel and host cancellation hook.
    pub plan: Plan<'a>,
}

/// Complete net observation. A successful output vector is never partial.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Obs {
    /// Ordered output values from a fully reduced net.
    Normal(alloc::vec::Vec<crate::sem::Value>),
    /// Refusal before reduction.
    Refused(Refusal),
    /// Refusal shape retained for parity with `eval::Obs`; guards are outside
    /// the revision-3 admitted subset.
    GuardFailed {
        /// Guard node (unreachable for admitted nets).
        node: crate::ir::NodeId,
        /// Guard fact.
        fact: crate::ir::Fact,
    },
    /// Fuel or host cancellation stopped the reduction without read-back.
    Exhausted {
        /// Exhausted resource.
        limit: Limit,
        /// Configured cap.
        cap: u64,
        /// First amount required beyond the cap.
        observed: u64,
    },
    /// Host cancellation stopped the reduction without read-back.
    Cancelled,
    /// A malformed/stuck net; no partial outputs are returned.
    InternalInvalid(Invalid),
}

/// Reduction result and resource observations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// Complete observation.
    pub obs: Obs,
    /// Interactions actually fired.
    pub interactions: u64,
    /// Initial and peak live agents (no rule increases this count).
    pub peak_live_agents: u64,
    /// Final active wire count when one was available.
    pub final_wires: Option<u64>,
    /// Initial net's retained storage estimate.
    pub retained_bytes: u64,
    /// Counters and identities for the checked in-place plan path.
    pub plan_stats: PlanStats,
}

/// Allocation, reuse, queue, scratch, and identity observations for one reduction.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlanStats {
    /// Number of result slots allocated after the initial arena.
    pub allocated_slots: u64,
    /// Number of consumed slots reused for results.
    pub reused_slots: u64,
    /// Number of consumed slots freed by committed plans.
    pub freed_slots: u64,
    /// Live nodes at the most recent commit (zero before the first commit).
    pub committed_live_nodes: u64,
    /// Maximum live nodes held during any plan transaction.
    pub transient_slots: u64,
    /// Maximum physical arena length, including free slots.
    pub arena_slots_high_water: u64,
    /// Maximum pending active-pair entries.
    pub queue_high_water: u64,
    /// Maximum bytes in an emitted plan.
    pub scratch_bytes_high_water: u64,
    /// Sum of bytes emitted across all checked plans.
    pub emitted_plan_bytes: u64,
    /// Commit and internal microstep count (reported separately from interactions).
    pub microsteps: u64,
    /// Rolling digest over the exact checked plan byte stream.
    pub plan_digest: crate::ir::Digest,
    /// Digest of the graph/input-bound net identity followed by the plan stream.
    pub replay_digest: crate::ir::Digest,
}

impl PlanStats {
    fn record_commit_counts(&mut self, execution: &rewrite::Execution) -> Result<(), Invalid> {
        let allocated = self
            .allocated_slots
            .checked_add(u64::from(execution.allocated_slots))
            .ok_or(Invalid::CounterOverflow)?;
        let reused = self
            .reused_slots
            .checked_add(u64::from(execution.reused_slots))
            .ok_or(Invalid::CounterOverflow)?;
        let freed = self
            .freed_slots
            .checked_add(u64::from(execution.freed_slots))
            .ok_or(Invalid::CounterOverflow)?;
        let microsteps = self
            .microsteps
            .checked_add(u64::from(execution.microsteps))
            .ok_or(Invalid::CounterOverflow)?;
        self.allocated_slots = allocated;
        self.reused_slots = reused;
        self.freed_slots = freed;
        self.microsteps = microsteps;
        Ok(())
    }
}

/// Lowers a checked graph and one input row into its canonical revision-3 net.
///
/// Admission and exact input/interface validation happen before net allocation.
/// The returned net is bound to the checked graph, revisions and input row.
pub fn lower(
    ids: &dyn IdentityScheme,
    checked: &crate::check::CheckedGraph,
    inputs: &[crate::sem::Value],
    limits: &Limits,
) -> Result<InteractionGraph, Refusal> {
    build_expected(ids, checked, inputs, limits)
}

/// Independently rebuilds the expected net from the checked graph and supplied
/// row, then validates structure and compares source roles and connections up
/// to an alpha-renaming of agent indices.
pub fn validate(
    ids: &dyn IdentityScheme,
    checked: &crate::check::CheckedGraph,
    net: &InteractionGraph,
    inputs: &[crate::sem::Value],
    limits: &Limits,
) -> Result<(), Refusal> {
    admit(checked)?;
    let input_values = input_values(checked, inputs)?;
    if net.graph_id != checked.id() {
        return Err(Refusal::InternalInvalid(Invalid::GraphIdentity));
    }
    if net.contract_revision != CONTRACT_REVISION {
        return Err(Refusal::InternalInvalid(Invalid::ContractRevision));
    }
    if net.lowering_revision != LOWERING_REVISION {
        return Err(Refusal::InternalInvalid(Invalid::LoweringRevision));
    }
    if net.input_values != input_values {
        return Err(Refusal::InternalInvalid(Invalid::InputValues));
    }
    let key = build_key(ids, checked.id(), &input_values).map_err(Refusal::InternalInvalid)?;
    if net.build_key != key {
        return Err(Refusal::InternalInvalid(Invalid::BuildKey));
    }
    validate_structure(net, limits).map_err(Refusal::InternalInvalid)?;
    let expected = build_expected_from_i64(ids, checked, input_values, limits)?;
    compare_up_to_renaming(&expected, net).map_err(Refusal::InternalInvalid)
}

/// Reduces a lowered net after re-admitting and independently validating it.
///
/// Every successful schedule fully normalizes and returns the same ordered
/// outputs and interaction count. Fuel is checked before the next firing.
pub fn reduce(ids: &dyn IdentityScheme, request: ReduceRequest<'_>) -> Report {
    let ReduceRequest {
        checked,
        candidate: net,
        inputs,
        limits,
        plan,
    } = request;
    let retained_bytes = net.retained_bytes();
    let validation = validate(ids, checked, net, inputs, limits);
    if let Err(refusal) = validation {
        let obs = match refusal {
            Refusal::InternalInvalid(invalid) => Obs::InternalInvalid(invalid),
            other => Obs::Refused(other),
        };
        return report(obs, 0, 0, None, retained_bytes);
    }
    let initial_agents = match u64::try_from(net.agents.len()) {
        Ok(value) => value,
        Err(_) => {
            return report(
                Obs::InternalInvalid(Invalid::CounterOverflow),
                0,
                0,
                None,
                retained_bytes,
            );
        }
    };
    let initial_wires = match net.wire_count() {
        Some(value) => value,
        None => {
            return report(
                Obs::InternalInvalid(Invalid::CounterOverflow),
                0,
                initial_agents,
                None,
                retained_bytes,
            );
        }
    };
    let mut slots = alloc::vec::Vec::with_capacity(net.agents.len());
    for agent in &net.agents {
        slots.push(Slot {
            agent: Some(RuntimeAgent {
                origin: agent.origin,
                symbol: agent.symbol,
                ports: agent.ports,
            }),
            generation: 0,
            queued_generation: None,
        });
    }
    let mut outputs = net.outputs.clone();
    let mut worklist = ActiveWorklist {
        entries: alloc::vec::Vec::with_capacity(net.agents.len()),
    };
    let storage_policy = plan.storage_policy.unwrap_or(StoragePolicy::Reuse);
    let mut plan_stats = PlanStats {
        committed_live_nodes: initial_agents,
        transient_slots: initial_agents,
        arena_slots_high_water: initial_agents,
        plan_digest: digest_bytes(
            ids,
            &[
                b"noble-net-plan-stream/1",
                PLAN_RULES_REVISION.as_bytes(),
                PLAN_REVISION.as_bytes(),
            ],
        ),
        ..PlanStats::default()
    };
    plan_stats.replay_digest = replay_digest(ids, net, plan_stats.plan_digest);
    let mut random = match plan.schedule.unwrap_or(Schedule::Lifo) {
        Schedule::SplitMix64 { seed } => Some(crate::prng::SplitMix64::new(seed)),
        Schedule::Lifo | Schedule::Fifo => None,
    };
    for index in 0..slots.len() {
        if let Err(invalid) = enqueue_active(&mut slots, &mut worklist, index) {
            return report_with_stats(ReportInput {
                obs: Obs::InternalInvalid(invalid),
                interactions: 0,
                peak_live_agents: initial_agents,
                final_wires: Some(initial_wires),
                retained_bytes,
                plan_stats,
            });
        }
    }
    plan_stats.queue_high_water = u64::try_from(worklist.entries.len()).unwrap_or(u64::MAX);
    let schedule = plan.schedule.unwrap_or(Schedule::Lifo);
    let fuel = plan
        .interaction_fuel
        .map_or(limits.interaction_fuel, |requested| {
            requested.min(limits.interaction_fuel)
        });
    let mut interactions = 0u64;
    let mut live_agents = initial_agents;
    let mut live_wires = initial_wires;
    while let Some(pair) = pop_pair(&mut worklist, schedule, &mut random) {
        if !pair_is_current(&mut slots, pair) {
            continue;
        }
        if plan.cancel.is_some_and(|cancel| cancel(interactions)) {
            return report_with_stats(ReportInput {
                obs: Obs::Cancelled,
                interactions,
                peak_live_agents: initial_agents,
                final_wires: Some(live_wires),
                retained_bytes,
                plan_stats,
            });
        }
        if interactions >= fuel {
            let observed = match interactions.checked_add(1) {
                Some(value) => value,
                None => {
                    return report_with_stats(ReportInput {
                        obs: Obs::InternalInvalid(Invalid::CounterOverflow),
                        interactions,
                        peak_live_agents: initial_agents,
                        final_wires: Some(live_wires),
                        retained_bytes,
                        plan_stats,
                    });
                }
            };
            return report_with_stats(ReportInput {
                obs: Obs::Exhausted {
                    limit: Limit::Fuel,
                    cap: fuel,
                    observed,
                },
                interactions,
                peak_live_agents: initial_agents,
                final_wires: Some(live_wires),
                retained_bytes,
                plan_stats,
            });
        }
        let bytes = match rewrite::compile(&slots, &outputs, pair, storage_policy) {
            Ok(bytes) => bytes,
            Err(invalid) => {
                return report_with_stats(ReportInput {
                    obs: Obs::InternalInvalid(invalid),
                    interactions,
                    peak_live_agents: initial_agents,
                    final_wires: Some(live_wires),
                    retained_bytes,
                    plan_stats,
                });
            }
        };
        let checked_plan = match rewrite::check_with_worklist(rewrite::CheckRequest {
            bytes: &bytes,
            slots: &slots,
            outputs: &outputs,
            worklist: &worklist,
            pair,
            policy: storage_policy,
        }) {
            Ok(checked) => checked,
            Err(invalid) => {
                return report_with_stats(ReportInput {
                    obs: Obs::InternalInvalid(invalid),
                    interactions,
                    peak_live_agents: initial_agents,
                    final_wires: Some(live_wires),
                    retained_bytes,
                    plan_stats,
                });
            }
        };
        if let Some(observer) = plan.plan_observer {
            observer(bytes.as_bytes());
        }
        let emitted = u64::try_from(bytes.as_bytes().len()).unwrap_or(u64::MAX);
        plan_stats.emitted_plan_bytes = match plan_stats.emitted_plan_bytes.checked_add(emitted) {
            Some(total) => total,
            None => {
                return report_with_stats(ReportInput {
                    obs: Obs::InternalInvalid(Invalid::CounterOverflow),
                    interactions,
                    peak_live_agents: initial_agents,
                    final_wires: Some(live_wires),
                    retained_bytes,
                    plan_stats,
                });
            }
        };
        plan_stats.scratch_bytes_high_water = plan_stats
            .scratch_bytes_high_water
            .max(u64::from(checked_plan.scratch_reservation));
        plan_stats.plan_digest = digest_bytes(ids, &[&plan_stats.plan_digest, bytes.as_bytes()]);
        plan_stats.replay_digest = replay_digest(ids, net, plan_stats.plan_digest);
        let scratch_limit_bytes = plan
            .max_plan_scratch_bytes
            .unwrap_or(rewrite::MAX_PLAN_BYTES as u64);
        let max_queue_entries = plan.max_worklist_entries.unwrap_or(limits.max_agent_slots);
        let execution = match rewrite::execute(
            checked_plan,
            &mut rewrite::ExecutionState {
                slots: &mut slots,
                outputs: &mut outputs,
                worklist: &mut worklist,
            },
            rewrite::ExecutionLimits {
                arena_slots: limits.max_agent_slots,
                scratch_bytes: scratch_limit_bytes,
                worklist_entries: max_queue_entries,
            },
        ) {
            Ok(execution) => execution,
            Err(rewrite::ExecuteError::Invalid(invalid)) => {
                return report_with_stats(ReportInput {
                    obs: Obs::InternalInvalid(invalid),
                    interactions,
                    peak_live_agents: initial_agents,
                    final_wires: Some(live_wires),
                    retained_bytes,
                    plan_stats,
                });
            }
            Err(rewrite::ExecuteError::Limited {
                limit,
                cap,
                observed,
            }) => {
                return report_with_stats(ReportInput {
                    obs: Obs::Exhausted {
                        limit,
                        cap,
                        observed,
                    },
                    interactions,
                    peak_live_agents: initial_agents,
                    final_wires: Some(live_wires),
                    retained_bytes,
                    plan_stats,
                });
            }
        };
        interactions = match interactions.checked_add(1) {
            Some(value) => value,
            None => {
                return report_with_stats(ReportInput {
                    obs: Obs::InternalInvalid(Invalid::CounterOverflow),
                    interactions,
                    peak_live_agents: initial_agents,
                    final_wires: Some(live_wires),
                    retained_bytes,
                    plan_stats,
                });
            }
        };
        live_agents = match live_agents.checked_sub(u64::from(execution.live_decrease)) {
            Some(value) => value,
            None => {
                return report_with_stats(ReportInput {
                    obs: Obs::InternalInvalid(Invalid::CounterOverflow),
                    interactions,
                    peak_live_agents: initial_agents,
                    final_wires: Some(live_wires),
                    retained_bytes,
                    plan_stats,
                });
            }
        };
        if let Err(invalid) = plan_stats.record_commit_counts(&execution) {
            return report_with_stats(ReportInput {
                obs: Obs::InternalInvalid(invalid),
                interactions,
                peak_live_agents: initial_agents,
                final_wires: Some(live_wires),
                retained_bytes,
                plan_stats,
            });
        }
        plan_stats.committed_live_nodes = live_agents;
        plan_stats.transient_slots = plan_stats
            .transient_slots
            .max(u64::from(execution.transient_live_nodes));
        plan_stats.arena_slots_high_water = plan_stats
            .arena_slots_high_water
            .max(u64::try_from(slots.len()).unwrap_or(u64::MAX));
        plan_stats.queue_high_water = plan_stats
            .queue_high_water
            .max(u64::try_from(worklist.entries.len()).unwrap_or(u64::MAX));
        live_wires = match live_wires.checked_sub(1) {
            Some(value) => value,
            None => {
                return report_with_stats(ReportInput {
                    obs: Obs::InternalInvalid(Invalid::CounterOverflow),
                    interactions,
                    peak_live_agents: initial_agents,
                    final_wires: None,
                    retained_bytes,
                    plan_stats,
                });
            }
        };
    }
    match read_back(&slots, &outputs, &net.output_types) {
        Ok(values) => {
            let actual_live = slots.iter().filter(|slot| slot.agent.is_some()).count();
            if u64::try_from(actual_live).ok() != Some(live_agents) || live_agents > initial_agents
            {
                return report_with_stats(ReportInput {
                    obs: Obs::InternalInvalid(Invalid::LeftoverAgent),
                    interactions,
                    peak_live_agents: initial_agents,
                    final_wires: Some(live_wires),
                    retained_bytes,
                    plan_stats,
                });
            }
            plan_stats.replay_digest = replay_digest(ids, net, plan_stats.plan_digest);
            report_with_stats(ReportInput {
                obs: Obs::Normal(values),
                interactions,
                peak_live_agents: initial_agents,
                final_wires: Some(live_wires),
                retained_bytes,
                plan_stats,
            })
        }
        Err(invalid) => report_with_stats(ReportInput {
            obs: Obs::InternalInvalid(invalid),
            interactions,
            peak_live_agents: initial_agents,
            final_wires: Some(live_wires),
            retained_bytes,
            plan_stats,
        }),
    }
}

fn report(
    obs: Obs,
    interactions: u64,
    peak_live_agents: u64,
    final_wires: Option<u64>,
    retained_bytes: u64,
) -> Report {
    report_with_stats(ReportInput {
        obs,
        interactions,
        peak_live_agents,
        final_wires,
        retained_bytes,
        plan_stats: PlanStats::default(),
    })
}

struct ReportInput {
    obs: Obs,
    interactions: u64,
    peak_live_agents: u64,
    final_wires: Option<u64>,
    retained_bytes: u64,
    plan_stats: PlanStats,
}

fn report_with_stats(input: ReportInput) -> Report {
    Report {
        obs: input.obs,
        interactions: input.interactions,
        peak_live_agents: input.peak_live_agents,
        final_wires: input.final_wires,
        retained_bytes: input.retained_bytes,
        plan_stats: input.plan_stats,
    }
}

fn digest_bytes(ids: &dyn IdentityScheme, parts: &[&[u8]]) -> crate::ir::Digest {
    let mut hasher = ids.start();
    for part in parts {
        hasher.update(part);
    }
    hasher.finish()
}

fn replay_digest(
    ids: &dyn IdentityScheme,
    net: &InteractionGraph,
    plan_digest: crate::ir::Digest,
) -> crate::ir::Digest {
    digest_bytes(
        ids,
        &[
            b"noble-net-plan-replay/1",
            net.build_key.as_ref(),
            CONTRACT_REVISION.as_bytes(),
            LOWERING_REVISION.as_bytes(),
            PLAN_RULES_REVISION.as_bytes(),
            PLAN_REVISION.as_bytes(),
            PLAN_REPRESENTATION_REVISION.as_bytes(),
            PLAN_CHECKER_REVISION.as_bytes(),
            PLAN_EXECUTOR_REVISION.as_bytes(),
            &plan_digest,
        ],
    )
}

fn admit(checked: &crate::check::CheckedGraph) -> Result<(), Refusal> {
    let graph = checked.graph();
    if graph.profile != crate::ir::SubProfile::I64 {
        return Err(Refusal::Profile);
    }
    if !graph.facts.is_empty() {
        return Err(Refusal::Facts);
    }
    if graph
        .slots
        .iter()
        .any(|slot| slot.kind != crate::ir::SlotKind::Input)
    {
        return Err(Refusal::Params);
    }
    if graph
        .slots
        .iter()
        .any(|slot| slot.ty != crate::ir::ScalarType::I64)
    {
        return Err(Refusal::Interface);
    }
    for node in &graph.nodes {
        match &node.op {
            crate::ir::Op::Input(_) | crate::ir::Op::Const(crate::ir::Literal::I64(_)) => {}
            crate::ir::Op::I64Add(..)
            | crate::ir::Op::I64Sub(..)
            | crate::ir::Op::I64Mul(..)
            | crate::ir::Op::I64Eq(..) => {}
            _ => return Err(Refusal::Operation),
        }
    }
    for &output in &graph.outputs {
        match checked.ty(output) {
            Some(crate::ir::ScalarType::I64 | crate::ir::ScalarType::Bool) => {}
            Some(crate::ir::ScalarType::F32) | None => return Err(Refusal::Operation),
        }
    }
    Ok(())
}

fn input_values(
    checked: &crate::check::CheckedGraph,
    inputs: &[crate::sem::Value],
) -> Result<alloc::vec::Vec<i64>, Refusal> {
    let expected = checked.graph().slots.len();
    if inputs.len() != expected {
        return Err(Refusal::Interface);
    }
    let mut values = alloc::vec::Vec::with_capacity(inputs.len());
    for input in inputs {
        match input {
            crate::sem::Value::I64(value) => values.push(*value),
            crate::sem::Value::Bool(_) | crate::sem::Value::F32(_) => {
                return Err(Refusal::Interface);
            }
        }
    }
    Ok(values)
}

#[derive(Clone, Copy)]
struct Counts {
    agents: u64,
    wires: u64,
}

fn counts(checked: &crate::check::CheckedGraph) -> Result<(Counts, alloc::vec::Vec<u64>), Refusal> {
    let graph = checked.graph();
    let mut uses = alloc::vec![0u64; graph.nodes.len()];
    for node in &graph.nodes {
        let (a, b) = match &node.op {
            crate::ir::Op::I64Add(a, b)
            | crate::ir::Op::I64Sub(a, b)
            | crate::ir::Op::I64Mul(a, b)
            | crate::ir::Op::I64Eq(a, b) => (a, b),
            _ => continue,
        };
        for operand in [*a, *b] {
            let use_count = uses
                .get_mut(operand.index())
                .ok_or(Refusal::InternalInvalid(Invalid::UnknownPort))?;
            *use_count = use_count
                .checked_add(1)
                .ok_or(Refusal::InternalInvalid(Invalid::CounterOverflow))?;
        }
    }
    for &output in &graph.outputs {
        let use_count = uses
            .get_mut(output.index())
            .ok_or(Refusal::InternalInvalid(Invalid::UnknownPort))?;
        *use_count = use_count
            .checked_add(1)
            .ok_or(Refusal::InternalInvalid(Invalid::CounterOverflow))?;
    }
    let mut agents = u64::try_from(graph.nodes.len())
        .map_err(|_| Refusal::InternalInvalid(Invalid::CounterOverflow))?;
    let mut wires = 0u64;
    for use_count in &uses {
        match *use_count {
            0 => {
                agents = agents
                    .checked_add(1)
                    .ok_or(Refusal::InternalInvalid(Invalid::CounterOverflow))?;
                wires = wires
                    .checked_add(1)
                    .ok_or(Refusal::InternalInvalid(Invalid::CounterOverflow))?;
            }
            1 => {
                wires = wires
                    .checked_add(1)
                    .ok_or(Refusal::InternalInvalid(Invalid::CounterOverflow))?;
            }
            n => {
                let copies = n
                    .checked_sub(1)
                    .ok_or(Refusal::InternalInvalid(Invalid::CounterOverflow))?;
                agents = agents
                    .checked_add(copies)
                    .ok_or(Refusal::InternalInvalid(Invalid::CounterOverflow))?;
                wires = wires
                    .checked_add(
                        n.checked_mul(2)
                            .and_then(|v| v.checked_sub(1))
                            .ok_or(Refusal::InternalInvalid(Invalid::CounterOverflow))?,
                    )
                    .ok_or(Refusal::InternalInvalid(Invalid::CounterOverflow))?;
            }
        }
    }
    Ok((Counts { agents, wires }, uses))
}

fn build_expected(
    ids: &dyn IdentityScheme,
    checked: &crate::check::CheckedGraph,
    inputs: &[crate::sem::Value],
    limits: &Limits,
) -> Result<InteractionGraph, Refusal> {
    admit(checked)?;
    let values = input_values(checked, inputs)?;
    build_expected_from_i64(ids, checked, values, limits)
}

fn build_expected_from_i64(
    ids: &dyn IdentityScheme,
    checked: &crate::check::CheckedGraph,
    input_values: alloc::vec::Vec<i64>,
    limits: &Limits,
) -> Result<InteractionGraph, Refusal> {
    let (counts, use_counts) = counts(checked)?;
    if counts.agents > limits.max_agent_slots {
        return Err(Refusal::Exhausted {
            limit: Limit::Agents,
            cap: limits.max_agent_slots,
            observed: counts.agents,
        });
    }
    if counts.wires > limits.max_wires {
        return Err(Refusal::Exhausted {
            limit: Limit::Wires,
            cap: limits.max_wires,
            observed: counts.wires,
        });
    }
    let agent_slots = usize::try_from(counts.agents)
        .map_err(|_| Refusal::InternalInvalid(Invalid::CounterOverflow))?;
    let graph = checked.graph();
    let source_count = graph.nodes.len();
    let mut agents = alloc::vec::Vec::with_capacity(agent_slots);
    for (index, node) in graph.nodes.iter().enumerate() {
        let node_id = crate::ir::NodeId(
            u32::try_from(index).map_err(|_| Refusal::InternalInvalid(Invalid::CounterOverflow))?,
        );
        let symbol = match &node.op {
            crate::ir::Op::Input(slot) => {
                let value = input_values
                    .get(
                        usize::try_from(*slot)
                            .map_err(|_| Refusal::InternalInvalid(Invalid::CounterOverflow))?,
                    )
                    .ok_or(Refusal::InternalInvalid(Invalid::UnknownPort))?;
                Symbol::Num(u64::from_le_bytes(value.to_le_bytes()))
            }
            crate::ir::Op::Const(crate::ir::Literal::I64(value)) => {
                Symbol::Num(u64::from_le_bytes(value.to_le_bytes()))
            }
            crate::ir::Op::I64Add(..) => Symbol::Op2(Op::Add),
            crate::ir::Op::I64Sub(..) => Symbol::Op2(Op::Sub),
            crate::ir::Op::I64Mul(..) => Symbol::Op2(Op::Mul),
            crate::ir::Op::I64Eq(..) => Symbol::Op2(Op::Eq),
            _ => return Err(Refusal::Operation),
        };
        agents.push(Agent {
            origin: Origin::Node(node_id),
            symbol,
            ports: [None, None, None],
        });
    }
    let mut first_dup = alloc::vec![None; source_count];
    let mut era_agent = alloc::vec![None; source_count];
    for (index, use_count) in use_counts.iter().enumerate() {
        let value = crate::ir::NodeId(
            u32::try_from(index).map_err(|_| Refusal::InternalInvalid(Invalid::CounterOverflow))?,
        );
        match *use_count {
            0 => {
                let agent = u32::try_from(agents.len())
                    .map_err(|_| Refusal::InternalInvalid(Invalid::CounterOverflow))?;
                era_agent[index] = Some(agent);
                agents.push(Agent {
                    origin: Origin::Era(value),
                    symbol: Symbol::Era,
                    ports: [None, None, None],
                });
            }
            1 => {}
            n => {
                let first = u32::try_from(agents.len())
                    .map_err(|_| Refusal::InternalInvalid(Invalid::CounterOverflow))?;
                first_dup[index] = Some(first);
                for ordinal in 0..n - 1 {
                    let ordinal = u32::try_from(ordinal)
                        .map_err(|_| Refusal::InternalInvalid(Invalid::CounterOverflow))?;
                    agents.push(Agent {
                        origin: Origin::Dup { value, ordinal },
                        symbol: Symbol::Dup,
                        ports: [None, None, None],
                    });
                }
            }
        }
    }
    if agents.len() != agent_slots {
        return Err(Refusal::InternalInvalid(Invalid::SourceOrigin));
    }
    let total_uses = use_counts.iter().try_fold(0usize, |sum, count| {
        let count = usize::try_from(*count).ok()?;
        sum.checked_add(count)
    });
    let total_uses = total_uses.ok_or(Refusal::InternalInvalid(Invalid::CounterOverflow))?;
    let mut use_entry_offsets = alloc::vec::Vec::with_capacity(source_count.saturating_add(1));
    use_entry_offsets.push(0usize);
    for count in &use_counts {
        let count = usize::try_from(*count)
            .map_err(|_| Refusal::InternalInvalid(Invalid::CounterOverflow))?;
        let next_cursor = use_entry_offsets
            .last()
            .copied()
            .and_then(|previous_entry_offset| previous_entry_offset.checked_add(count))
            .ok_or(Refusal::InternalInvalid(Invalid::CounterOverflow))?;
        use_entry_offsets.push(next_cursor);
    }
    let mut cursors = alloc::vec::Vec::with_capacity(source_count);
    for entry_offset in use_entry_offsets.iter().take(source_count) {
        cursors.push(*entry_offset);
    }
    let mut uses = alloc::vec![None; total_uses];
    for (index, node) in graph.nodes.iter().enumerate() {
        let node_index =
            u32::try_from(index).map_err(|_| Refusal::InternalInvalid(Invalid::CounterOverflow))?;
        let (a, b) = match &node.op {
            crate::ir::Op::I64Add(a, b)
            | crate::ir::Op::I64Sub(a, b)
            | crate::ir::Op::I64Mul(a, b)
            | crate::ir::Op::I64Eq(a, b) => (*a, *b),
            _ => continue,
        };
        insert_use(
            &mut uses,
            &mut cursors,
            a,
            Endpoint::Port(PortRef {
                agent: node_index,
                port_index: PortIndex::from(0),
            }),
        )?;
        insert_use(
            &mut uses,
            &mut cursors,
            b,
            Endpoint::Port(PortRef {
                agent: node_index,
                port_index: PortIndex::from(1),
            }),
        )?;
    }
    for (index, output) in graph.outputs.iter().enumerate() {
        insert_use(
            &mut uses,
            &mut cursors,
            *output,
            Endpoint::Output(
                u32::try_from(index)
                    .map_err(|_| Refusal::InternalInvalid(Invalid::CounterOverflow))?,
            ),
        )?;
    }
    let mut output_links = alloc::vec![None; graph.outputs.len()];
    for (index, use_count) in use_counts.iter().enumerate() {
        let source = match agents.get(index) {
            Some(agent) => match agent.symbol {
                Symbol::Num(_) => PortRef {
                    agent: u32::try_from(index)
                        .map_err(|_| Refusal::InternalInvalid(Invalid::CounterOverflow))?,
                    port_index: PortIndex::from(0),
                },
                Symbol::Op2(_) => PortRef {
                    agent: u32::try_from(index)
                        .map_err(|_| Refusal::InternalInvalid(Invalid::CounterOverflow))?,
                    port_index: PortIndex::from(2),
                },
                _ => return Err(Refusal::InternalInvalid(Invalid::SourceSymbol)),
            },
            None => return Err(Refusal::InternalInvalid(Invalid::UnknownPort)),
        };
        let use_index = *use_entry_offsets
            .get(index)
            .ok_or(Refusal::InternalInvalid(Invalid::UnknownPort))?;
        match *use_count {
            0 => {
                let era =
                    era_agent[index].ok_or(Refusal::InternalInvalid(Invalid::SourceOrigin))?;
                connect(
                    &mut agents,
                    &mut output_links,
                    source,
                    Endpoint::Port(PortRef {
                        agent: era,
                        port_index: PortIndex::from(0),
                    }),
                )?;
            }
            1 => {
                let target = *uses
                    .get(use_index)
                    .ok_or(Refusal::InternalInvalid(Invalid::UnknownPort))?
                    .as_ref()
                    .ok_or(Refusal::InternalInvalid(Invalid::SourceWiring))?;
                connect(&mut agents, &mut output_links, source, target)?;
            }
            n => {
                let first =
                    first_dup[index].ok_or(Refusal::InternalInvalid(Invalid::SourceOrigin))?;
                connect(
                    &mut agents,
                    &mut output_links,
                    source,
                    Endpoint::Port(PortRef {
                        agent: first,
                        port_index: PortIndex::from(0),
                    }),
                )?;
                let duplicate_count = n - 1;
                for ordinal in 0..duplicate_count {
                    let ordinal_u32 = u32::try_from(ordinal)
                        .map_err(|_| Refusal::InternalInvalid(Invalid::CounterOverflow))?;
                    let duplicate = first
                        .checked_add(ordinal_u32)
                        .ok_or(Refusal::InternalInvalid(Invalid::CounterOverflow))?;
                    let first_use_index = use_index
                        .checked_add(
                            usize::try_from(ordinal)
                                .map_err(|_| Refusal::InternalInvalid(Invalid::CounterOverflow))?,
                        )
                        .ok_or(Refusal::InternalInvalid(Invalid::CounterOverflow))?;
                    let first_use = *uses
                        .get(first_use_index)
                        .ok_or(Refusal::InternalInvalid(Invalid::UnknownPort))?
                        .as_ref()
                        .ok_or(Refusal::InternalInvalid(Invalid::SourceWiring))?;
                    connect(
                        &mut agents,
                        &mut output_links,
                        PortRef {
                            agent: duplicate,
                            port_index: PortIndex::from(1),
                        },
                        first_use,
                    )?;
                    let second_copy = if ordinal + 1 < duplicate_count {
                        Endpoint::Port(PortRef {
                            agent: duplicate
                                .checked_add(1)
                                .ok_or(Refusal::InternalInvalid(Invalid::CounterOverflow))?,
                            port_index: PortIndex::from(0),
                        })
                    } else {
                        let last_index =
                            use_index
                                .checked_add(usize::try_from(n - 1).map_err(|_| {
                                    Refusal::InternalInvalid(Invalid::CounterOverflow)
                                })?)
                                .ok_or(Refusal::InternalInvalid(Invalid::CounterOverflow))?;
                        *uses
                            .get(last_index)
                            .ok_or(Refusal::InternalInvalid(Invalid::UnknownPort))?
                            .as_ref()
                            .ok_or(Refusal::InternalInvalid(Invalid::SourceWiring))?
                    };
                    connect(
                        &mut agents,
                        &mut output_links,
                        PortRef {
                            agent: duplicate,
                            port_index: PortIndex::from(2),
                        },
                        second_copy,
                    )?;
                }
            }
        }
    }
    let mut outputs = alloc::vec::Vec::with_capacity(output_links.len());
    for output in output_links {
        outputs.push(output.ok_or(Refusal::InternalInvalid(Invalid::OutputBoundary))?);
    }
    let mut output_types = alloc::vec::Vec::with_capacity(graph.outputs.len());
    for output in &graph.outputs {
        output_types.push(
            checked
                .ty(*output)
                .ok_or(Refusal::InternalInvalid(Invalid::SourceSymbol))?,
        );
    }
    let net = InteractionGraph {
        graph_id: checked.id(),
        contract_revision: CONTRACT_REVISION,
        lowering_revision: LOWERING_REVISION,
        build_key: build_key(ids, checked.id(), &input_values).map_err(Refusal::InternalInvalid)?,
        input_values,
        agents,
        outputs,
        output_types,
    };
    if net.wire_count() != Some(counts.wires) {
        return Err(Refusal::InternalInvalid(Invalid::SourceWiring));
    }
    Ok(net)
}

fn insert_use(
    uses: &mut [Option<Endpoint>],
    cursors: &mut [usize],
    source: crate::ir::NodeId,
    target: Endpoint,
) -> Result<(), Refusal> {
    let cursor = cursors
        .get_mut(source.index())
        .ok_or(Refusal::InternalInvalid(Invalid::UnknownPort))?;
    let use_slot = uses
        .get_mut(*cursor)
        .ok_or(Refusal::InternalInvalid(Invalid::UnknownPort))?;
    if use_slot.is_some() {
        return Err(Refusal::InternalInvalid(Invalid::SourceWiring));
    }
    *use_slot = Some(target);
    *cursor = cursor
        .checked_add(1)
        .ok_or(Refusal::InternalInvalid(Invalid::CounterOverflow))?;
    Ok(())
}

fn connect(
    agents: &mut [Agent],
    outputs: &mut [Option<PortRef>],
    source: PortRef,
    target: Endpoint,
) -> Result<(), Refusal> {
    let source_agent = agents
        .get_mut(source.agent as usize)
        .ok_or(Refusal::InternalInvalid(Invalid::UnknownPort))?;
    let source_endpoint = source_agent
        .ports
        .get_mut(usize::from(source.port_index))
        .ok_or(Refusal::InternalInvalid(Invalid::UnknownPort))?;
    if source_endpoint.is_some() {
        return Err(Refusal::InternalInvalid(Invalid::SourceWiring));
    }
    *source_endpoint = Some(target);
    match target {
        Endpoint::Port(other) => {
            if source.agent == other.agent {
                return Err(Refusal::InternalInvalid(Invalid::SelfWire));
            }
            let target_agent = agents
                .get_mut(other.agent as usize)
                .ok_or(Refusal::InternalInvalid(Invalid::UnknownPort))?;
            let target_endpoint = target_agent
                .ports
                .get_mut(usize::from(other.port_index))
                .ok_or(Refusal::InternalInvalid(Invalid::UnknownPort))?;
            if target_endpoint.is_some() {
                return Err(Refusal::InternalInvalid(Invalid::SourceWiring));
            }
            *target_endpoint = Some(Endpoint::Port(source));
        }
        Endpoint::Output(index) => {
            let output = outputs
                .get_mut(index as usize)
                .ok_or(Refusal::InternalInvalid(Invalid::OutputBoundary))?;
            if output.is_some() {
                return Err(Refusal::InternalInvalid(Invalid::OutputBoundary));
            }
            *output = Some(source);
        }
    }
    Ok(())
}

fn build_key(
    ids: &dyn IdentityScheme,
    graph_id: crate::ir::Digest,
    inputs: &[i64],
) -> Result<crate::ir::Digest, Invalid> {
    let mut canonical = crate::ir::Canon::new(ids, "noble.ngraph.net-i64.build.v0");
    canonical.bytes(&graph_id);
    canonical.str(CONTRACT_REVISION);
    canonical.str(LOWERING_REVISION);
    canonical.u64(u64::try_from(inputs.len()).map_err(|_| Invalid::CounterOverflow)?);
    for input in inputs {
        canonical.i64(*input);
    }
    Ok(canonical.finish())
}

fn validate_structure(net: &InteractionGraph, limits: &Limits) -> Result<(), Invalid> {
    let agents = u64::try_from(net.agents.len()).map_err(|_| Invalid::CounterOverflow)?;
    if agents > limits.max_agent_slots {
        return Err(Invalid::AgentCapacity);
    }
    let output_count = u64::try_from(net.outputs.len()).map_err(|_| Invalid::CounterOverflow)?;
    if output_count > limits.max_wires {
        return Err(Invalid::WireCapacity);
    }
    let mut output_seen = alloc::vec![false; net.outputs.len()];
    let mut wires = 0u64;
    for (index, agent) in net.agents.iter().enumerate() {
        let source = PortRef {
            agent: u32::try_from(index).map_err(|_| Invalid::CounterOverflow)?,
            port_index: PortIndex::from(0),
        };
        for (port_index, endpoint) in agent.ports.iter().enumerate() {
            if port_index >= agent.symbol.port_count() {
                if endpoint.is_some() {
                    return Err(Invalid::PortArity);
                }
                continue;
            }
            let endpoint = endpoint.ok_or(Invalid::PortArity)?;
            let source_port = PortRef {
                port_index: PortIndex::from(
                    u8::try_from(port_index).map_err(|_| Invalid::CounterOverflow)?,
                ),
                ..source
            };
            match endpoint {
                Endpoint::Port(target) => {
                    if source_port.agent == target.agent {
                        return Err(Invalid::SelfWire);
                    }
                    let target_agent = net
                        .agents
                        .get(target.agent as usize)
                        .ok_or(Invalid::UnknownPort)?;
                    if usize::from(target.port_index) >= target_agent.symbol.port_count() {
                        return Err(Invalid::UnknownPort);
                    }
                    let reciprocal = target_agent
                        .ports
                        .get(usize::from(target.port_index))
                        .ok_or(Invalid::UnknownPort)?;
                    if *reciprocal != Some(Endpoint::Port(source_port)) {
                        return Err(Invalid::NonReciprocal);
                    }
                    if (source_port.agent, source_port.port_index)
                        < (target.agent, target.port_index)
                    {
                        wires = wires.checked_add(1).ok_or(Invalid::CounterOverflow)?;
                    }
                }
                Endpoint::Output(output) => {
                    let seen = output_seen
                        .get_mut(output as usize)
                        .ok_or(Invalid::OutputBoundary)?;
                    if *seen {
                        return Err(Invalid::OutputBoundary);
                    }
                    *seen = true;
                    if net.outputs.get(output as usize) != Some(&source_port) {
                        return Err(Invalid::OutputBoundary);
                    }
                    wires = wires.checked_add(1).ok_or(Invalid::CounterOverflow)?;
                }
            }
        }
    }
    if output_seen.iter().any(|seen| !seen) {
        return Err(Invalid::OutputBoundary);
    }
    for (index, output) in net.outputs.iter().enumerate() {
        let agent = net
            .agents
            .get(output.agent as usize)
            .ok_or(Invalid::OutputBoundary)?;
        if usize::from(output.port_index) >= agent.symbol.port_count()
            || agent.ports[usize::from(output.port_index)]
                != Some(Endpoint::Output(
                    u32::try_from(index).map_err(|_| Invalid::CounterOverflow)?,
                ))
        {
            return Err(Invalid::OutputBoundary);
        }
    }
    if wires > limits.max_wires {
        return Err(Invalid::WireCapacity);
    }
    Ok(())
}

fn compare_up_to_renaming(
    expected: &InteractionGraph,
    candidate: &InteractionGraph,
) -> Result<(), Invalid> {
    if expected.graph_id != candidate.graph_id {
        return Err(Invalid::GraphIdentity);
    }
    if expected.contract_revision != candidate.contract_revision {
        return Err(Invalid::ContractRevision);
    }
    if expected.lowering_revision != candidate.lowering_revision {
        return Err(Invalid::LoweringRevision);
    }
    if expected.input_values != candidate.input_values {
        return Err(Invalid::InputValues);
    }
    if expected.output_types != candidate.output_types {
        return Err(Invalid::SourceWiring);
    }
    if expected.agents.len() != candidate.agents.len()
        || expected.outputs.len() != candidate.outputs.len()
    {
        return Err(Invalid::SourceOrigin);
    }
    let mut indices = alloc::vec::Vec::with_capacity(candidate.agents.len());
    for index in 0..candidate.agents.len() {
        indices.push(u32::try_from(index).map_err(|_| Invalid::CounterOverflow)?);
    }
    indices.sort_unstable_by_key(|index| candidate.agents[*index as usize].origin.key());
    for pair in indices.windows(2) {
        let first = candidate
            .agents
            .get(pair[0] as usize)
            .ok_or(Invalid::UnknownPort)?
            .origin
            .key();
        let second = candidate
            .agents
            .get(pair[1] as usize)
            .ok_or(Invalid::UnknownPort)?
            .origin
            .key();
        if first == second {
            return Err(Invalid::DuplicateOrigin);
        }
    }
    for expected_agent in &expected.agents {
        let key = expected_agent.origin.key();
        let candidate_index = indices
            .binary_search_by_key(&key, |index| candidate.agents[*index as usize].origin.key())
            .map_err(|_| Invalid::SourceOrigin)?;
        let candidate_agent = candidate
            .agents
            .get(indices[candidate_index] as usize)
            .ok_or(Invalid::UnknownPort)?;
        if candidate_agent.symbol != expected_agent.symbol {
            return Err(Invalid::SourceSymbol);
        }
        for port in 0..expected_agent.symbol.port_count() {
            match (expected_agent.ports[port], candidate_agent.ports[port]) {
                (Some(Endpoint::Output(a)), Some(Endpoint::Output(b))) if a == b => {}
                (Some(Endpoint::Port(expected_port)), Some(Endpoint::Port(candidate_port))) => {
                    if expected_port.port_index != candidate_port.port_index {
                        return Err(Invalid::SourceWiring);
                    }
                    let expected_target = expected
                        .agents
                        .get(expected_port.agent as usize)
                        .ok_or(Invalid::UnknownPort)?;
                    let candidate_target = candidate
                        .agents
                        .get(candidate_port.agent as usize)
                        .ok_or(Invalid::UnknownPort)?;
                    if expected_target.origin != candidate_target.origin {
                        return Err(Invalid::SourceWiring);
                    }
                }
                (None, None) => {}
                _ => return Err(Invalid::SourceWiring),
            }
        }
    }
    for (expected_port, candidate_port) in expected.outputs.iter().zip(&candidate.outputs) {
        if expected_port.port_index != candidate_port.port_index {
            return Err(Invalid::OutputBoundary);
        }
        let expected_agent = expected
            .agents
            .get(expected_port.agent as usize)
            .ok_or(Invalid::UnknownPort)?;
        let candidate_agent = candidate
            .agents
            .get(candidate_port.agent as usize)
            .ok_or(Invalid::UnknownPort)?;
        if expected_agent.origin != candidate_agent.origin {
            return Err(Invalid::OutputBoundary);
        }
    }
    Ok(())
}

fn wire_count(agents: &[Agent]) -> Option<u64> {
    let mut wires = 0u64;
    for (index, agent) in agents.iter().enumerate() {
        let source_agent = u32::try_from(index).ok()?;
        for (port, endpoint) in agent
            .ports
            .iter()
            .take(agent.symbol.port_count())
            .enumerate()
        {
            match (*endpoint)? {
                Endpoint::Port(target) => {
                    let source_port = (source_agent, PortIndex::from(u8::try_from(port).ok()?));
                    if source_port < (target.agent, target.port_index) {
                        wires = wires.checked_add(1)?;
                    }
                }
                Endpoint::Output(_) => wires = wires.checked_add(1)?,
            }
        }
    }
    Some(wires)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RuntimeAgent {
    origin: Origin,
    symbol: Symbol,
    ports: [Option<Endpoint>; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Slot {
    agent: Option<RuntimeAgent>,
    generation: u64,
    queued_generation: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Pair {
    first: u32,
    first_generation: u64,
    second: u32,
    second_generation: u64,
}

/// Pending principal-port pairs, bounded by the fixed agent arena.
#[derive(Debug, PartialEq, Eq)]
struct ActiveWorklist {
    entries: alloc::vec::Vec<Pair>,
}

fn enqueue_active(
    slots: &mut [Slot],
    worklist: &mut ActiveWorklist,
    index: usize,
) -> Result<(), Invalid> {
    let slot = slots.get(index).ok_or(Invalid::UnknownPort)?;
    let agent = slot.agent.ok_or(Invalid::UnknownPort)?;
    let target = match agent.ports[0] {
        Some(Endpoint::Port(target)) if target.port_index == PortIndex::from(0) => target,
        Some(Endpoint::Port(_)) | Some(Endpoint::Output(_)) | None => return Ok(()),
    };
    if target.agent as usize == index {
        return Err(Invalid::SelfWire);
    }
    let other = slots
        .get(target.agent as usize)
        .ok_or(Invalid::UnknownPort)?;
    let other_agent = other.agent.ok_or(Invalid::UnknownPort)?;
    if other_agent.ports[0]
        != Some(Endpoint::Port(PortRef {
            agent: u32::try_from(index).map_err(|_| Invalid::CounterOverflow)?,
            port_index: PortIndex::from(0),
        }))
    {
        return Ok(());
    }
    let (first, second) = if index < target.agent as usize {
        (index, target.agent as usize)
    } else {
        (target.agent as usize, index)
    };
    let first_generation = slots.get(first).ok_or(Invalid::UnknownPort)?.generation;
    let second_generation = slots.get(second).ok_or(Invalid::UnknownPort)?.generation;
    let first_mark = slots
        .get(first)
        .ok_or(Invalid::UnknownPort)?
        .queued_generation;
    let second_mark = slots
        .get(second)
        .ok_or(Invalid::UnknownPort)?
        .queued_generation;
    if first_mark == Some(first_generation) && second_mark == Some(second_generation) {
        return Ok(());
    }
    if first_mark == Some(first_generation) || second_mark == Some(second_generation) {
        return Err(Invalid::WorklistBound);
    }
    if worklist.entries.len() >= slots.len() {
        return Err(Invalid::WorklistBound);
    }
    let first_slot = slots.get_mut(first).ok_or(Invalid::UnknownPort)?;
    first_slot.queued_generation = Some(first_generation);
    let second_slot = slots.get_mut(second).ok_or(Invalid::UnknownPort)?;
    second_slot.queued_generation = Some(second_generation);
    worklist.entries.push(Pair {
        first: u32::try_from(first).map_err(|_| Invalid::CounterOverflow)?,
        first_generation,
        second: u32::try_from(second).map_err(|_| Invalid::CounterOverflow)?,
        second_generation,
    });
    Ok(())
}

fn pop_pair(
    worklist: &mut ActiveWorklist,
    schedule: Schedule,
    random: &mut Option<crate::prng::SplitMix64>,
) -> Option<Pair> {
    match schedule {
        Schedule::Lifo => worklist.entries.pop(),
        Schedule::Fifo => {
            if worklist.entries.is_empty() {
                None
            } else {
                Some(worklist.entries.remove(0))
            }
        }
        Schedule::SplitMix64 { .. } => {
            if worklist.entries.is_empty() {
                None
            } else {
                let len = u64::try_from(worklist.entries.len()).ok()?;
                let index = random.as_mut()?.below(len).try_into().ok()?;
                Some(worklist.entries.swap_remove(index))
            }
        }
    }
}

fn pair_is_current(slots: &mut [Slot], pair: Pair) -> bool {
    let first = pair.first as usize;
    let second = pair.second as usize;
    let Some(first_slot) = slots.get_mut(first) else {
        return false;
    };
    let is_first_current = first_slot.generation == pair.first_generation
        && first_slot.agent.is_some()
        && first_slot.queued_generation == Some(pair.first_generation);
    if first_slot.queued_generation == Some(pair.first_generation) {
        first_slot.queued_generation = None;
    }
    let Some(second_slot) = slots.get_mut(second) else {
        return false;
    };
    let is_second_current = second_slot.generation == pair.second_generation
        && second_slot.agent.is_some()
        && second_slot.queued_generation == Some(pair.second_generation);
    if second_slot.queued_generation == Some(pair.second_generation) {
        second_slot.queued_generation = None;
    }
    if !is_first_current || !is_second_current {
        return false;
    }
    let Some(first_agent) = slots.get(first).and_then(|slot| slot.agent) else {
        return false;
    };
    let Some(second_agent) = slots.get(second).and_then(|slot| slot.agent) else {
        return false;
    };
    first_agent.ports[0]
        == Some(Endpoint::Port(PortRef {
            agent: pair.second,
            port_index: PortIndex::from(0),
        }))
        && second_agent.ports[0]
            == Some(Endpoint::Port(PortRef {
                agent: pair.first,
                port_index: PortIndex::from(0),
            }))
}

fn read_back(
    slots: &[Slot],
    outputs: &[PortRef],
    output_types: &[crate::ir::ScalarType],
) -> Result<alloc::vec::Vec<crate::sem::Value>, Invalid> {
    if outputs.len() != output_types.len() {
        return Err(Invalid::ReadBack);
    }
    let mut observed = alloc::vec::Vec::with_capacity(outputs.len());
    let mut output_agent_seen = alloc::vec![false; slots.len()];
    for (index, output) in outputs.iter().enumerate() {
        if output.port_index != PortIndex::from(0) {
            return Err(Invalid::ReadBack);
        }
        let agent = slots
            .get(output.agent as usize)
            .and_then(|slot| slot.agent)
            .ok_or(Invalid::ReadBack)?;
        let value = match agent.symbol {
            Symbol::Num(bits)
                if agent.ports[0]
                    == Some(Endpoint::Output(
                        u32::try_from(index).map_err(|_| Invalid::CounterOverflow)?,
                    )) =>
            {
                bits
            }
            Symbol::Num(_) | Symbol::Op2(_) | Symbol::Op1(..) | Symbol::Dup | Symbol::Era => {
                return Err(Invalid::ReadBack);
            }
        };
        let seen = output_agent_seen
            .get_mut(output.agent as usize)
            .ok_or(Invalid::ReadBack)?;
        if *seen {
            return Err(Invalid::OutputBoundary);
        }
        *seen = true;
        let value = match output_types[index] {
            crate::ir::ScalarType::I64 => {
                crate::sem::Value::I64(i64::from_le_bytes(value.to_le_bytes()))
            }
            crate::ir::ScalarType::Bool => match value {
                0 => crate::sem::Value::Bool(false),
                1 => crate::sem::Value::Bool(true),
                _ => return Err(Invalid::ReadBack),
            },
            crate::ir::ScalarType::F32 => return Err(Invalid::ReadBack),
        };
        observed.push(value);
    }
    for (index, slot) in slots.iter().enumerate() {
        if let Some(agent) = slot.agent {
            match agent.symbol {
                Symbol::Num(_) if output_agent_seen.get(index) == Some(&true) => {}
                Symbol::Num(_) | Symbol::Op2(_) | Symbol::Op1(..) | Symbol::Dup | Symbol::Era => {
                    return Err(Invalid::LeftoverAgent);
                }
            }
        }
    }
    Ok(observed)
}

#[cfg(test)]
#[expect(
    tigerstyle::no_panic,
    tigerstyle::no_unwrap,
    reason = "Owner: noble-ngraph-maintainers; these unit tests intentionally panic on a broken fixture or assertion so the failing contract case is visible."
)]
mod tests {
    use super::*;

    const REQUIRED_ALLOCATION_ARENA_SLOTS: u64 = 4;

    struct TestIds;

    struct TestHasher {
        state: [u8; 32],
        next_byte_index: usize,
    }

    impl crate::identity::IdentityHasher for TestHasher {
        fn update(&mut self, bytes: &[u8]) {
            for byte in bytes {
                let index = self.next_byte_index;
                self.state[index] = self.state[index]
                    .wrapping_mul(31)
                    .wrapping_add(*byte)
                    .wrapping_add(index as u8);
                self.next_byte_index = if index + 1 == self.state.len() {
                    0
                } else {
                    index + 1
                };
            }
        }

        fn finish(self: alloc::boxed::Box<Self>) -> crate::ir::Digest {
            self.state
        }
    }

    impl crate::identity::IdentityScheme for TestIds {
        fn name(&self) -> &'static str {
            "test-net"
        }

        fn start(&self) -> alloc::boxed::Box<dyn crate::identity::IdentityHasher + '_> {
            alloc::boxed::Box::new(TestHasher {
                state: [0; 32],
                next_byte_index: 0,
            })
        }
    }

    const IDS: TestIds = TestIds;

    fn sample() -> crate::check::CheckedGraph {
        let subject = crate::ir::Subject::authored(&IDS, "net-reducer-sample");
        let mut builder = crate::ir::GraphBuilder::new(crate::ir::SubProfile::I64, subject);
        let x = builder.input(crate::ir::ScalarType::I64);
        let y = builder.input(crate::ir::ScalarType::I64);
        let difference = builder.op(crate::ir::Op::I64Sub(x, y));
        let square = builder.op(crate::ir::Op::I64Mul(difference, difference));
        let equal = builder.op(crate::ir::Op::I64Eq(difference, square));
        builder.output(difference);
        builder.output(square);
        builder.output(equal);
        match crate::check::check(&IDS, builder.finish()) {
            Ok(checked) => checked,
            Err(error) => panic!("sample graph rejected: {error}"),
        }
    }

    fn empty() -> crate::check::CheckedGraph {
        let subject = crate::ir::Subject::authored(&IDS, "net-empty");
        let builder = crate::ir::GraphBuilder::new(crate::ir::SubProfile::I64, subject);
        match crate::check::check(&IDS, builder.finish()) {
            Ok(checked) => checked,
            Err(error) => panic!("empty graph rejected: {error}"),
        }
    }

    fn inputs(left_i64: i64, right_i64: i64) -> [crate::sem::Value; 2] {
        [
            crate::sem::Value::I64(left_i64),
            crate::sem::Value::I64(right_i64),
        ]
    }

    fn sample_interaction_graph(
        left_i64: i64,
        right_i64: i64,
    ) -> (
        crate::check::CheckedGraph,
        InteractionGraph,
        [crate::sem::Value; 2],
    ) {
        let checked = sample();
        let inputs = inputs(left_i64, right_i64);
        let net = match lower(&IDS, &checked, &inputs, &Limits::default()) {
            Ok(net) => net,
            Err(error) => panic!("sample net refused: {error:?}"),
        };
        (checked, net, inputs)
    }

    fn limits(agent_slots: u64, wire_count: u64, interaction_fuel: u64) -> Limits {
        Limits {
            max_agent_slots: agent_slots,
            max_wires: wire_count,
            interaction_fuel,
        }
    }

    fn plan_fixture(
        rule: Symbol,
        value: u64,
        reverse: bool,
    ) -> (alloc::vec::Vec<Slot>, alloc::vec::Vec<PortRef>, Pair) {
        let rule_slot = if reverse { 1u32 } else { 0u32 };
        let value_slot = 1 - rule_slot;
        let symbols = if reverse {
            [Symbol::Num(value), rule]
        } else {
            [rule, Symbol::Num(value)]
        };
        let pair = Pair {
            first: 0,
            first_generation: 0,
            second: 1,
            second_generation: 0,
        };
        let mut slots = alloc::vec![
            Slot {
                agent: None,
                generation: 0,
                queued_generation: None,
            };
            2
        ];
        let mut outputs = alloc::vec::Vec::with_capacity(rule.port_count().saturating_sub(1));
        let mut output_index = 0u32;
        for index in 0..2usize {
            let symbol = symbols[index];
            let peer = if index == 0 { 1 } else { 0 };
            let mut ports = [None; 3];
            ports[0] = Some(Endpoint::Port(PortRef {
                agent: u32::try_from(peer).expect("two fixture slots"),
                port_index: PortIndex::from(0),
            }));
            if u32::try_from(index).expect("two fixture slots") == rule_slot {
                for (port, endpoint) in ports
                    .iter_mut()
                    .enumerate()
                    .take(symbol.port_count())
                    .skip(1)
                {
                    *endpoint = Some(Endpoint::Output(output_index));
                    outputs.push(PortRef {
                        agent: u32::try_from(index).expect("two fixture slots"),
                        port_index: PortIndex::from(
                            u8::try_from(port).expect("three fixture ports"),
                        ),
                    });
                    output_index += 1;
                }
            }
            slots[index].agent = Some(RuntimeAgent {
                origin: Origin::Node(crate::ir::NodeId(
                    u32::try_from(index).expect("two fixture slots"),
                )),
                symbol,
                ports,
            });
        }
        assert_eq!(value_slot, if reverse { 0 } else { 1 });
        (slots, outputs, pair)
    }

    fn result_slot_range(
        plan: &rewrite::PlanBytes,
        result_index: usize,
    ) -> Option<core::ops::Range<usize>> {
        let bytes = plan.as_bytes();
        let mut cursor = 33usize;
        let boundary_count = usize::from(*bytes.get(cursor)?);
        cursor = cursor.checked_add(1)?;
        for _ in 0..boundary_count {
            cursor = cursor.checked_add(5)?;
            cursor = cursor.checked_add(match bytes.get(cursor).copied()? {
                0 => 6,
                1 => 5,
                _ => return None,
            })?;
        }
        let result_count = usize::from(*bytes.get(cursor)?);
        if result_index >= result_count {
            return None;
        }
        cursor = cursor.checked_add(1)?;
        cursor = cursor.checked_add(result_index.checked_mul(21)?)?;
        cursor = cursor.checked_add(1)?;
        Some(cursor..cursor.checked_add(4)?)
    }

    fn principal_boundary_fixture() -> (alloc::vec::Vec<Slot>, alloc::vec::Vec<PortRef>, Pair) {
        let pair = Pair {
            first: 0,
            first_generation: 0,
            second: 1,
            second_generation: 0,
        };
        let mut slots = alloc::vec![
            Slot {
                agent: None,
                generation: 0,
                queued_generation: None,
            };
            3
        ];
        let mut first_ports = [None; 3];
        first_ports[0] = Some(Endpoint::Port(PortRef {
            agent: 1,
            port_index: PortIndex::from(0),
        }));
        first_ports[1] = Some(Endpoint::Port(PortRef {
            agent: 2,
            port_index: PortIndex::from(0),
        }));
        first_ports[2] = Some(Endpoint::Output(0));
        slots[0].agent = Some(RuntimeAgent {
            origin: Origin::Node(crate::ir::NodeId(0)),
            symbol: Symbol::Op2(Op::Sub),
            ports: first_ports,
        });
        slots[1].agent = Some(RuntimeAgent {
            origin: Origin::Node(crate::ir::NodeId(1)),
            symbol: Symbol::Num(3),
            ports: [
                Some(Endpoint::Port(PortRef {
                    agent: 0,
                    port_index: PortIndex::from(0),
                })),
                None,
                None,
            ],
        });
        slots[2].agent = Some(RuntimeAgent {
            origin: Origin::Node(crate::ir::NodeId(2)),
            symbol: Symbol::Op2(Op::Add),
            ports: [
                Some(Endpoint::Port(PortRef {
                    agent: 0,
                    port_index: PortIndex::from(1),
                })),
                Some(Endpoint::Output(1)),
                Some(Endpoint::Output(2)),
            ],
        });
        let outputs = alloc::vec![
            PortRef {
                agent: 0,
                port_index: PortIndex::from(2),
            },
            PortRef {
                agent: 2,
                port_index: PortIndex::from(1),
            },
            PortRef {
                agent: 2,
                port_index: PortIndex::from(2),
            },
        ];
        (slots, outputs, pair)
    }

    fn duplicate_principal_fixture() -> (alloc::vec::Vec<Slot>, alloc::vec::Vec<PortRef>, Pair) {
        let pair = Pair {
            first: 0,
            first_generation: 0,
            second: 1,
            second_generation: 0,
        };
        let mut slots = alloc::vec![
            Slot {
                agent: None,
                generation: 0,
                queued_generation: None,
            };
            4
        ];
        slots[0].agent = Some(RuntimeAgent {
            origin: Origin::Node(crate::ir::NodeId(0)),
            symbol: Symbol::Dup,
            ports: [
                Some(Endpoint::Port(PortRef {
                    agent: 1,
                    port_index: PortIndex::from(0),
                })),
                Some(Endpoint::Port(PortRef {
                    agent: 2,
                    port_index: PortIndex::from(0),
                })),
                Some(Endpoint::Port(PortRef {
                    agent: 3,
                    port_index: PortIndex::from(0),
                })),
            ],
        });
        slots[1].agent = Some(RuntimeAgent {
            origin: Origin::Node(crate::ir::NodeId(1)),
            symbol: Symbol::Num(5),
            ports: [
                Some(Endpoint::Port(PortRef {
                    agent: 0,
                    port_index: PortIndex::from(0),
                })),
                None,
                None,
            ],
        });
        for index in [2usize, 3usize] {
            let output_index_base = u32::try_from((index - 2) * 2).expect("two fixture agents");
            slots[index].agent = Some(RuntimeAgent {
                origin: Origin::Node(crate::ir::NodeId(
                    u32::try_from(index).expect("four fixture slots"),
                )),
                symbol: Symbol::Op2(Op::Add),
                ports: [
                    Some(Endpoint::Port(PortRef {
                        agent: 0,
                        port_index: PortIndex::from(
                            u8::try_from(index - 1).expect("two DUP outputs"),
                        ),
                    })),
                    Some(Endpoint::Output(output_index_base)),
                    Some(Endpoint::Output(output_index_base + 1)),
                ],
            });
        }
        let outputs = alloc::vec![
            PortRef {
                agent: 2,
                port_index: PortIndex::from(1)
            },
            PortRef {
                agent: 2,
                port_index: PortIndex::from(2)
            },
            PortRef {
                agent: 3,
                port_index: PortIndex::from(1)
            },
            PortRef {
                agent: 3,
                port_index: PortIndex::from(2)
            },
        ];
        (slots, outputs, pair)
    }

    fn shared_boundary_worklist_fixture() -> (
        alloc::vec::Vec<Slot>,
        alloc::vec::Vec<PortRef>,
        ActiveWorklist,
        Pair,
    ) {
        let mut slots = alloc::vec![
            Slot {
                agent: None,
                generation: 0,
                queued_generation: None,
            };
            4
        ];
        slots[0].agent = Some(RuntimeAgent {
            origin: Origin::Node(crate::ir::NodeId(0)),
            symbol: Symbol::Op2(Op::Add),
            ports: [
                Some(Endpoint::Port(PortRef {
                    agent: 1,
                    port_index: PortIndex::from(0),
                })),
                Some(Endpoint::Port(PortRef {
                    agent: 2,
                    port_index: PortIndex::from(1),
                })),
                Some(Endpoint::Port(PortRef {
                    agent: 2,
                    port_index: PortIndex::from(2),
                })),
            ],
        });
        slots[1].agent = Some(RuntimeAgent {
            origin: Origin::Node(crate::ir::NodeId(1)),
            symbol: Symbol::Num(9),
            ports: [
                Some(Endpoint::Port(PortRef {
                    agent: 0,
                    port_index: PortIndex::from(0),
                })),
                None,
                None,
            ],
        });
        slots[2].agent = Some(RuntimeAgent {
            origin: Origin::Node(crate::ir::NodeId(2)),
            symbol: Symbol::Dup,
            ports: [
                Some(Endpoint::Port(PortRef {
                    agent: 3,
                    port_index: PortIndex::from(0),
                })),
                Some(Endpoint::Port(PortRef {
                    agent: 0,
                    port_index: PortIndex::from(1),
                })),
                Some(Endpoint::Port(PortRef {
                    agent: 0,
                    port_index: PortIndex::from(2),
                })),
            ],
        });
        slots[3].agent = Some(RuntimeAgent {
            origin: Origin::Node(crate::ir::NodeId(3)),
            symbol: Symbol::Num(17),
            ports: [
                Some(Endpoint::Port(PortRef {
                    agent: 2,
                    port_index: PortIndex::from(0),
                })),
                None,
                None,
            ],
        });
        slots[2].queued_generation = Some(0);
        slots[3].queued_generation = Some(0);
        let selected = Pair {
            first: 0,
            first_generation: 0,
            second: 1,
            second_generation: 0,
        };
        let queued = Pair {
            first: 2,
            first_generation: 0,
            second: 3,
            second_generation: 0,
        };
        let outputs = alloc::vec::Vec::new();
        let worklist = ActiveWorklist {
            entries: alloc::vec![queued],
        };
        (slots, outputs, worklist, selected)
    }

    #[test]
    fn plan_compiler_emits_checked_bytes_for_every_rule_and_orientation() {
        for op in [Op::Add, Op::Sub, Op::Mul, Op::Eq] {
            for reverse in [false, true] {
                for rule in [Symbol::Op2(op), Symbol::Op1(op, 19)] {
                    let (slots, outputs, pair) = plan_fixture(rule, 7, reverse);
                    for policy in [StoragePolicy::Reuse, StoragePolicy::Allocate] {
                        let bytes = rewrite::compile(&slots, &outputs, pair, policy)
                            .expect("rule must compile");
                        assert_eq!(&bytes.as_bytes()[..5], b"NPLN\x01");
                        assert_eq!(bytes.as_bytes().last(), Some(&0xc7));
                        assert!(rewrite::check(&bytes, &slots, &outputs, pair, policy).is_ok());
                    }
                }
            }
        }
        for rule in [Symbol::Dup, Symbol::Era] {
            for reverse in [false, true] {
                let (slots, outputs, pair) = plan_fixture(rule, 0xfeed, reverse);
                for policy in [StoragePolicy::Reuse, StoragePolicy::Allocate] {
                    let bytes = rewrite::compile(&slots, &outputs, pair, policy)
                        .expect("fanout/erasure rule must compile");
                    assert!(rewrite::check(&bytes, &slots, &outputs, pair, policy).is_ok());
                }
            }
        }
    }

    #[test]
    fn replay_identity_changes_for_a_new_plan_with_the_same_rule_identity() {
        let (_checked, net, _row) = sample_interaction_graph(2, 8);
        let (slots, outputs, pair) = principal_boundary_fixture();
        let reused =
            rewrite::compile(&slots, &outputs, pair, StoragePolicy::Reuse).expect("reuse plan");
        let allocated = rewrite::compile(&slots, &outputs, pair, StoragePolicy::Allocate)
            .expect("allocation plan");
        let initial = digest_bytes(
            &IDS,
            &[
                b"noble-net-plan-stream/1",
                PLAN_RULES_REVISION.as_bytes(),
                PLAN_REVISION.as_bytes(),
            ],
        );
        let reuse_plan_digest = digest_bytes(&IDS, &[&initial, reused.as_bytes()]);
        let allocate_plan_digest = digest_bytes(&IDS, &[&initial, allocated.as_bytes()]);
        assert_ne!(reuse_plan_digest, allocate_plan_digest);
        assert_ne!(
            replay_digest(&IDS, &net, reuse_plan_digest),
            replay_digest(&IDS, &net, allocate_plan_digest),
        );
    }

    #[test]
    fn plan_operation_counters_refuse_wrap_without_partial_accounting() {
        let execution = rewrite::Execution {
            live_decrease: 1,
            transient_live_nodes: 2,
            allocated_slots: 1,
            reused_slots: 1,
            freed_slots: 1,
            microsteps: 6,
        };
        for mut stats in [
            PlanStats {
                allocated_slots: u64::MAX,
                ..PlanStats::default()
            },
            PlanStats {
                reused_slots: u64::MAX,
                ..PlanStats::default()
            },
            PlanStats {
                freed_slots: u64::MAX,
                ..PlanStats::default()
            },
            PlanStats {
                microsteps: u64::MAX - 5,
                ..PlanStats::default()
            },
        ] {
            let before = stats.clone();
            assert_eq!(
                stats.record_commit_counts(&execution),
                Err(Invalid::CounterOverflow)
            );
            assert_eq!(stats, before);
        }
    }

    #[test]
    fn checker_preserves_distinct_ports_on_one_boundary_agent_and_queue_ownership() {
        let (slots, outputs, worklist, pair) = shared_boundary_worklist_fixture();
        let bytes = rewrite::compile(&slots, &outputs, pair, StoragePolicy::Reuse)
            .expect("shared boundary is legal");
        assert!(rewrite::check_with_worklist(rewrite::CheckRequest {
            bytes: &bytes,
            slots: &slots,
            outputs: &outputs,
            worklist: &worklist,
            pair,
            policy: StoragePolicy::Reuse,
        })
        .is_ok());

        let mut duplicate = ActiveWorklist {
            entries: worklist.entries.clone(),
        };
        duplicate.entries.push(pair);
        assert!(matches!(
            rewrite::check_with_worklist(rewrite::CheckRequest {
                bytes: &bytes,
                slots: &slots,
                outputs: &outputs,
                worklist: &duplicate,
                pair,
                policy: StoragePolicy::Reuse,
            }),
            Err(Invalid::DoubleEnqueue)
        ));

        let mut stale_queue = ActiveWorklist {
            entries: worklist.entries.clone(),
        };
        stale_queue.entries[0].second_generation += 1;
        assert!(matches!(
            rewrite::check_with_worklist(rewrite::CheckRequest {
                bytes: &bytes,
                slots: &slots,
                outputs: &outputs,
                worklist: &stale_queue,
                pair,
                policy: StoragePolicy::Reuse,
            }),
            Err(Invalid::StalePlanSlot)
        ));

        let mut freed_slots = slots.clone();
        freed_slots[3].agent = None;
        assert!(matches!(
            rewrite::check_with_worklist(rewrite::CheckRequest {
                bytes: &bytes,
                slots: &freed_slots,
                outputs: &outputs,
                worklist: &worklist,
                pair,
                policy: StoragePolicy::Reuse,
            }),
            Err(Invalid::StalePlanSlot)
        ));
    }

    #[test]
    fn independent_plan_checker_rejects_forged_bytes() {
        let (slots, outputs, pair) = plan_fixture(Symbol::Op2(Op::Sub), 8, false);
        let mut bytes =
            rewrite::compile(&slots, &outputs, pair, StoragePolicy::Reuse).expect("op2 plan");
        bytes.as_mut_bytes()[31] = 0;
        assert!(rewrite::check(&bytes, &slots, &outputs, pair, StoragePolicy::Reuse).is_err());

        let mut stale =
            rewrite::compile(&slots, &outputs, pair, StoragePolicy::Reuse).expect("op2 plan");
        stale.as_mut_bytes()[11] ^= 1;
        assert!(rewrite::check(&stale, &slots, &outputs, pair, StoragePolicy::Reuse).is_err());

        let mut wrong_reuse =
            rewrite::compile(&slots, &outputs, pair, StoragePolicy::Reuse).expect("op2 plan");
        let slot_range = result_slot_range(&wrong_reuse, 0).expect("first result slot range");
        wrong_reuse.as_mut_bytes()[slot_range].copy_from_slice(&pair.second.to_le_bytes());
        assert!(matches!(
            rewrite::check(&wrong_reuse, &slots, &outputs, pair, StoragePolicy::Reuse),
            Err(Invalid::PlanMismatch)
        ));

        let mut omitted_write =
            rewrite::compile(&slots, &outputs, pair, StoragePolicy::Reuse).expect("op2 plan");
        omitted_write.as_mut_bytes()[76] = 1;
        assert!(
            rewrite::check(&omitted_write, &slots, &outputs, pair, StoragePolicy::Reuse).is_err()
        );

        let mut extra_write =
            rewrite::compile(&slots, &outputs, pair, StoragePolicy::Reuse).expect("op2 plan");
        extra_write.as_mut_bytes()[76] = 3;
        assert!(
            rewrite::check(&extra_write, &slots, &outputs, pair, StoragePolicy::Reuse).is_err()
        );

        let mut aliased_boundary =
            rewrite::compile(&slots, &outputs, pair, StoragePolicy::Reuse).expect("op2 plan");
        aliased_boundary.as_mut_bytes()[50..54].copy_from_slice(&0u32.to_le_bytes());
        assert!(rewrite::check(
            &aliased_boundary,
            &slots,
            &outputs,
            pair,
            StoragePolicy::Reuse
        )
        .is_err());

        let (sub_slots, sub_outputs, sub_pair) = plan_fixture(Symbol::Op1(Op::Sub, 2), 8, false);
        let mut swapped =
            rewrite::compile(&sub_slots, &sub_outputs, sub_pair, StoragePolicy::Reuse)
                .expect("op1 subtraction plan");
        swapped.as_mut_bytes()[51..59].copy_from_slice(&6u64.to_le_bytes());
        assert!(rewrite::check(
            &swapped,
            &sub_slots,
            &sub_outputs,
            sub_pair,
            StoragePolicy::Reuse
        )
        .is_err());

        let (dup_slots, dup_outputs, dup_pair) = plan_fixture(Symbol::Dup, 5, false);
        let mut duplicate_free =
            rewrite::compile(&dup_slots, &dup_outputs, dup_pair, StoragePolicy::Reuse)
                .expect("dup plan");
        let first_range = result_slot_range(&duplicate_free, 0).expect("first dup slot range");
        let second_range = result_slot_range(&duplicate_free, 1).expect("second dup slot range");
        let mut first_destination = [0; 4];
        first_destination.copy_from_slice(&duplicate_free.as_bytes()[first_range]);
        duplicate_free.as_mut_bytes()[second_range].copy_from_slice(&first_destination);
        assert!(matches!(
            rewrite::check(
                &duplicate_free,
                &dup_slots,
                &dup_outputs,
                dup_pair,
                StoragePolicy::Reuse
            ),
            Err(Invalid::DuplicateFree)
        ));

        let (fanout_slots, fanout_outputs, fanout_pair) = duplicate_principal_fixture();
        let mut duplicate_enqueue = rewrite::compile(
            &fanout_slots,
            &fanout_outputs,
            fanout_pair,
            StoragePolicy::Reuse,
        )
        .expect("active fanout plan");
        let emitted = duplicate_enqueue.as_mut_bytes();
        for byte_index in 0..24 {
            emitted[150 + byte_index] = emitted[126 + byte_index];
        }
        assert!(rewrite::check(
            &duplicate_enqueue,
            &fanout_slots,
            &fanout_outputs,
            fanout_pair,
            StoragePolicy::Reuse,
        )
        .is_err());

        let mut insufficient = rewrite::compile(&slots, &outputs, pair, StoragePolicy::Allocate)
            .expect("allocating op2 plan");
        assert_eq!(insufficient.as_bytes()[90], 1);
        insufficient.as_mut_bytes()[90..94].copy_from_slice(&0u32.to_le_bytes());
        assert!(rewrite::check(
            &insufficient,
            &slots,
            &outputs,
            pair,
            StoragePolicy::Allocate
        )
        .is_err());
    }

    #[test]
    fn plan_resources_are_reserved_before_mutation_and_microsteps_roll_back() {
        for policy in [StoragePolicy::Reuse, StoragePolicy::Allocate] {
            for stop_at in 1..=6 {
                let (mut slots, mut outputs, pair) = principal_boundary_fixture();
                let bytes =
                    rewrite::compile(&slots, &outputs, pair, policy).expect("principal plan");
                let checked = rewrite::check(&bytes, &slots, &outputs, pair, policy)
                    .expect("principal plan checks");
                let slots_before = slots.clone();
                let outputs_before = outputs.clone();
                let mut worklist = ActiveWorklist {
                    entries: alloc::vec::Vec::with_capacity(slots.len()),
                };
                let queue_before = worklist.entries.clone();
                let interrupted = |step| step == stop_at;
                let max_arena_slots = u64::try_from(slots.len()).expect("fixture size")
                    + u64::from(policy == StoragePolicy::Allocate);
                let result = rewrite::execute_with_interrupt(
                    checked,
                    &mut rewrite::ExecutionState {
                        slots: &mut slots,
                        outputs: &mut outputs,
                        worklist: &mut worklist,
                    },
                    rewrite::ExecutionLimits {
                        arena_slots: max_arena_slots,
                        scratch_bytes: rewrite::MAX_PLAN_BYTES as u64,
                        worklist_entries: 1,
                    },
                    Some(&interrupted),
                );
                assert!(matches!(
                    result,
                    Err(rewrite::ExecuteError::Invalid(Invalid::PlanInterrupted))
                ));
                assert_eq!(slots, slots_before);
                assert_eq!(outputs, outputs_before);
                assert_eq!(worklist.entries, queue_before);
            }
        }

        let mut worklist = ActiveWorklist {
            entries: alloc::vec::Vec::with_capacity(1),
        };
        for fail_after_reservations in 0..=2 {
            let (mut slots, mut outputs, pair) = principal_boundary_fixture();
            let bytes = rewrite::compile(&slots, &outputs, pair, StoragePolicy::Allocate)
                .expect("allocating principal plan");
            let checked = rewrite::check(&bytes, &slots, &outputs, pair, StoragePolicy::Allocate)
                .expect("allocating principal plan checks");
            let slots_before = slots.clone();
            let outputs_before = outputs.clone();
            worklist.entries.clear();
            let result = rewrite::execute_with_reservation_failure(
                checked,
                &mut rewrite::ExecutionState {
                    slots: &mut slots,
                    outputs: &mut outputs,
                    worklist: &mut worklist,
                },
                rewrite::ExecutionLimits {
                    arena_slots: REQUIRED_ALLOCATION_ARENA_SLOTS,
                    scratch_bytes: rewrite::MAX_PLAN_BYTES as u64,
                    worklist_entries: 1,
                },
                fail_after_reservations,
            );
            assert!(matches!(
                result,
                Err(rewrite::ExecuteError::Invalid(
                    Invalid::PlanReservationFailure
                ))
            ));
            assert_eq!(slots, slots_before);
            assert_eq!(outputs, outputs_before);
            assert!(worklist.entries.is_empty());
        }

        let (mut slots, mut outputs, pair) = principal_boundary_fixture();
        let initial_slots = slots.len();
        let bytes = rewrite::compile(&slots, &outputs, pair, StoragePolicy::Reuse)
            .expect("fully reusable plan");
        let checked = rewrite::check(&bytes, &slots, &outputs, pair, StoragePolicy::Reuse)
            .expect("fully reusable plan checks");
        let mut worklist = ActiveWorklist {
            entries: alloc::vec::Vec::new(),
        };
        let result = rewrite::execute(
            checked,
            &mut rewrite::ExecutionState {
                slots: &mut slots,
                outputs: &mut outputs,
                worklist: &mut worklist,
            },
            rewrite::ExecutionLimits {
                arena_slots: u64::try_from(initial_slots).expect("fixture arena size"),
                scratch_bytes: rewrite::MAX_PLAN_BYTES as u64,
                worklist_entries: 1,
            },
        );
        let result = match result {
            Ok(result) => result,
            Err(_) => panic!("reusable plan fits with zero free arena slots"),
        };
        assert_eq!(result.allocated_slots, 0);
        assert_eq!(result.reused_slots, 1);
        assert_eq!(slots.len(), initial_slots);

        let (mut slots, mut outputs, pair) = principal_boundary_fixture();
        let bytes = rewrite::compile(&slots, &outputs, pair, StoragePolicy::Allocate)
            .expect("allocating principal plan");
        let checked = rewrite::check(&bytes, &slots, &outputs, pair, StoragePolicy::Allocate)
            .expect("allocating principal plan checks");
        let slots_before = slots.clone();
        let outputs_before = outputs.clone();
        let mut worklist = ActiveWorklist {
            entries: alloc::vec::Vec::with_capacity(slots.len()),
        };
        let max_arena_slots = u64::try_from(slots.len()).expect("fixture size");
        let result = rewrite::execute(
            checked,
            &mut rewrite::ExecutionState {
                slots: &mut slots,
                outputs: &mut outputs,
                worklist: &mut worklist,
            },
            rewrite::ExecutionLimits {
                arena_slots: max_arena_slots,
                scratch_bytes: rewrite::MAX_PLAN_BYTES as u64,
                worklist_entries: 1,
            },
        );
        assert!(matches!(
            result,
            Err(rewrite::ExecuteError::Limited {
                limit: Limit::ArenaReservation,
                ..
            })
        ));
        assert_eq!(slots, slots_before);
        assert_eq!(outputs, outputs_before);
        assert!(worklist.entries.is_empty());
    }

    #[test]
    fn plan_resource_limits_fail_below_and_succeed_at_or_above() {
        let required_scratch_bytes = rewrite::MAX_PLAN_BYTES as u64;
        for scratch_limit_bytes in [
            required_scratch_bytes - 1,
            required_scratch_bytes,
            required_scratch_bytes + 1,
        ] {
            let (mut slots, mut outputs, pair) = principal_boundary_fixture();
            let bytes =
                rewrite::compile(&slots, &outputs, pair, StoragePolicy::Reuse).expect("reuse plan");
            let checked = rewrite::check(&bytes, &slots, &outputs, pair, StoragePolicy::Reuse)
                .expect("checked plan");
            let mut worklist = ActiveWorklist {
                entries: alloc::vec::Vec::with_capacity(slots.len()),
            };
            let arena_slots = u64::try_from(slots.len()).expect("fixture arena size");
            let result = rewrite::execute(
                checked,
                &mut rewrite::ExecutionState {
                    slots: &mut slots,
                    outputs: &mut outputs,
                    worklist: &mut worklist,
                },
                rewrite::ExecutionLimits {
                    arena_slots,
                    scratch_bytes: scratch_limit_bytes,
                    worklist_entries: 1,
                },
            );
            if scratch_limit_bytes < required_scratch_bytes {
                assert!(matches!(
                    result,
                    Err(rewrite::ExecuteError::Limited {
                        limit: Limit::PlanScratch,
                        ..
                    })
                ));
                assert!(worklist.entries.is_empty());
            } else {
                assert!(result.is_ok());
                assert_eq!(worklist.entries.len(), 1);
            }
        }
        for max_queue_entries in [0u64, 1, 2] {
            let (mut slots, mut outputs, pair) = principal_boundary_fixture();
            let bytes =
                rewrite::compile(&slots, &outputs, pair, StoragePolicy::Reuse).expect("reuse plan");
            let checked = rewrite::check(&bytes, &slots, &outputs, pair, StoragePolicy::Reuse)
                .expect("checked plan");
            let slots_before = slots.clone();
            let outputs_before = outputs.clone();
            let mut worklist = ActiveWorklist {
                entries: alloc::vec::Vec::with_capacity(slots.len()),
            };
            let arena_slots = u64::try_from(slots.len()).expect("fixture arena size");
            let result = rewrite::execute(
                checked,
                &mut rewrite::ExecutionState {
                    slots: &mut slots,
                    outputs: &mut outputs,
                    worklist: &mut worklist,
                },
                rewrite::ExecutionLimits {
                    arena_slots,
                    scratch_bytes: required_scratch_bytes,
                    worklist_entries: max_queue_entries,
                },
            );
            if max_queue_entries == 0 {
                assert!(matches!(
                    result,
                    Err(rewrite::ExecuteError::Limited {
                        limit: Limit::WorklistReservation,
                        ..
                    })
                ));
                assert_eq!(slots, slots_before);
                assert_eq!(outputs, outputs_before);
                assert!(worklist.entries.is_empty());
            } else {
                assert!(result.is_ok());
                assert_eq!(worklist.entries.len(), 1);
            }
        }
        for max_arena_slots in [
            REQUIRED_ALLOCATION_ARENA_SLOTS - 1,
            REQUIRED_ALLOCATION_ARENA_SLOTS,
            REQUIRED_ALLOCATION_ARENA_SLOTS + 1,
        ] {
            let (mut slots, mut outputs, pair) = principal_boundary_fixture();
            let bytes = rewrite::compile(&slots, &outputs, pair, StoragePolicy::Allocate)
                .expect("allocating plan");
            let checked = rewrite::check(&bytes, &slots, &outputs, pair, StoragePolicy::Allocate)
                .expect("checked allocating plan");
            let mut worklist = ActiveWorklist {
                entries: alloc::vec::Vec::with_capacity(slots.len()),
            };
            let result = rewrite::execute(
                checked,
                &mut rewrite::ExecutionState {
                    slots: &mut slots,
                    outputs: &mut outputs,
                    worklist: &mut worklist,
                },
                rewrite::ExecutionLimits {
                    arena_slots: max_arena_slots,
                    scratch_bytes: required_scratch_bytes,
                    worklist_entries: 1,
                },
            );
            if max_arena_slots < REQUIRED_ALLOCATION_ARENA_SLOTS {
                assert!(matches!(
                    result,
                    Err(rewrite::ExecuteError::Limited {
                        limit: Limit::ArenaReservation,
                        ..
                    })
                ));
            } else {
                assert!(result.is_ok());
                assert_eq!(
                    slots.len(),
                    usize::try_from(REQUIRED_ALLOCATION_ARENA_SLOTS).expect("arena slots")
                );
            }
        }
    }

    #[test]
    fn lowering_is_canonical_and_alpha_renaming_is_accepted() {
        let (checked, net, row) = sample_interaction_graph(11, 4);
        assert_eq!(net.agents.len(), 9);
        assert_eq!(net.wire_count(), Some(13));
        assert_eq!(net.rule_counts(), (3, 4, 0));
        assert_eq!(net.expected_interactions(), Some(10));
        assert_eq!(
            validate(&IDS, &checked, &net, &row, &Limits::default()),
            Ok(())
        );

        let mut renamed = net.clone();
        let count = renamed.agents.len();
        let mut old_to_new = alloc::vec![0u32; count];
        let mut reordered = alloc::vec::Vec::with_capacity(count);
        for new_index in 0..count {
            let old_index = count - 1 - new_index;
            old_to_new[old_index] = u32::try_from(new_index).expect("bounded sample agent index");
            reordered.push(renamed.agents[old_index].clone());
        }
        for agent in &mut reordered {
            for endpoint in &mut agent.ports {
                if let Some(Endpoint::Port(port)) = endpoint {
                    port.agent = old_to_new[port.agent as usize];
                }
            }
        }
        for output in &mut renamed.outputs {
            output.agent = old_to_new[output.agent as usize];
        }
        renamed.agents = reordered;
        assert_eq!(
            validate(&IDS, &checked, &renamed, &row, &Limits::default()),
            Ok(())
        );
    }

    #[test]
    fn reduction_matches_reference_and_exact_interaction_bound() {
        let (checked, net, row) = sample_interaction_graph(11, 4);
        let plan = Plan {
            schedule: Some(Schedule::Lifo),
            ..Plan::default()
        };
        let limits = Limits::default();
        let report = reduce(
            &IDS,
            ReduceRequest {
                checked: &checked,
                candidate: &net,
                inputs: &row,
                limits: &limits,
                plan,
            },
        );
        assert_eq!(
            report.obs,
            Obs::Normal(alloc::vec![
                crate::sem::Value::I64(7),
                crate::sem::Value::I64(49),
                crate::sem::Value::Bool(false),
            ])
        );
        assert_eq!(
            report.interactions,
            net.expected_interactions().expect("count")
        );
        assert_eq!(report.peak_live_agents, net.agents.len() as u64);
        assert_eq!(report.final_wires, Some(net.outputs.len() as u64));
        let net_values = match report.obs {
            Obs::Normal(values) => values,
            other => panic!("sample net was not normal: {other:?}"),
        };
        let reference =
            crate::eval::reference(&checked, &row, None, &mut crate::eval::Control::default());
        assert!(matches!(reference.obs, crate::eval::Obs::Normal(values) if values == net_values));
    }

    #[test]
    fn schedules_have_identical_values_counts_and_live_bound() {
        let (checked, net, row) = sample_interaction_graph(i64::MIN, -1);
        let schedules = [
            Schedule::Lifo,
            Schedule::Fifo,
            Schedule::SplitMix64 { seed: 1 },
            Schedule::SplitMix64 { seed: 0x5eed },
        ];
        let limits = Limits::default();
        let mut observations = alloc::vec::Vec::with_capacity(schedules.len());
        for schedule in schedules {
            let plan = Plan {
                schedule: Some(schedule),
                ..Plan::default()
            };
            let report = reduce(
                &IDS,
                ReduceRequest {
                    checked: &checked,
                    candidate: &net,
                    inputs: &row,
                    limits: &limits,
                    plan,
                },
            );
            assert!(
                matches!(report.obs, Obs::Normal(_)),
                "{schedule:?}: {report:?}"
            );
            assert_eq!(report.interactions, 10, "{schedule:?}");
            assert!(report.peak_live_agents <= net.agents.len() as u64);
            observations.push(report.obs);
        }
        assert!(observations.windows(2).all(|pair| pair[0] == pair[1]));
    }

    #[test]
    fn schedule_fuel_boundary_refuses_without_partial_readback() {
        let (checked, net, row) = sample_interaction_graph(11, 4);
        let total = net.expected_interactions().expect("sample interactions");
        let initial_wires = net.wire_count().expect("sample wires");
        let reference =
            crate::eval::reference(&checked, &row, None, &mut crate::eval::Control::default());
        let expected = match reference.obs {
            crate::eval::Obs::Normal(values) => values,
            other => panic!("sample reference was not normal: {other:?}"),
        };
        let schedules = [
            Schedule::Lifo,
            Schedule::Fifo,
            Schedule::SplitMix64 { seed: 1 },
            Schedule::SplitMix64 { seed: 0x5eed },
        ];
        for cap in [total - 1, total, total + 1] {
            for schedule in schedules {
                let limits = Limits::default();
                let report = reduce(
                    &IDS,
                    ReduceRequest {
                        checked: &checked,
                        candidate: &net,
                        inputs: &row,
                        limits: &limits,
                        plan: Plan {
                            schedule: Some(schedule),
                            interaction_fuel: Some(cap),
                            ..Plan::default()
                        },
                    },
                );
                if cap < total {
                    assert_eq!(
                        report.obs,
                        Obs::Exhausted {
                            limit: Limit::Fuel,
                            cap,
                            observed: cap + 1,
                        },
                        "{schedule:?}"
                    );
                    assert_eq!(report.interactions, cap, "{schedule:?}");
                    assert_eq!(
                        report.final_wires,
                        Some(initial_wires - cap),
                        "{schedule:?}"
                    );
                } else {
                    assert!(
                        matches!(&report.obs, Obs::Normal(values) if values == &expected),
                        "{schedule:?}: {report:?}"
                    );
                    assert_eq!(report.interactions, total, "{schedule:?}");
                    assert_eq!(
                        report.final_wires,
                        Some(initial_wires - total),
                        "{schedule:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn budgets_refuse_below_and_succeed_at_or_above_each_limit() {
        let (checked, net, row) = sample_interaction_graph(8, 3);
        let agents = u64::try_from(net.agents.len()).expect("bounded sample agents");
        let wires = net.wire_count().expect("well-formed sample wires");
        let fuel = net.expected_interactions().expect("sample interactions");

        assert!(matches!(
            lower(&IDS, &checked, &row, &limits(agents - 1, wires, fuel)),
            Err(Refusal::Exhausted {
                limit: Limit::Agents,
                observed,
                ..
            }) if observed == agents
        ));
        assert!(lower(&IDS, &checked, &row, &limits(agents, wires, fuel)).is_ok());
        assert!(lower(&IDS, &checked, &row, &limits(agents + 1, wires, fuel)).is_ok());

        assert!(matches!(
            lower(&IDS, &checked, &row, &limits(agents, wires - 1, fuel)),
            Err(Refusal::Exhausted {
                limit: Limit::Wires,
                observed,
                ..
            }) if observed == wires
        ));
        assert!(lower(&IDS, &checked, &row, &limits(agents, wires, fuel)).is_ok());
        assert!(lower(&IDS, &checked, &row, &limits(agents, wires + 1, fuel)).is_ok());

        for (cap, expected) in [(fuel - 1, false), (fuel, true), (fuel + 1, true)] {
            let limits = limits(agents, wires, cap);
            let report = reduce(
                &IDS,
                ReduceRequest {
                    checked: &checked,
                    candidate: &net,
                    inputs: &row,
                    limits: &limits,
                    plan: Plan::default(),
                },
            );
            if expected {
                assert!(
                    matches!(report.obs, Obs::Normal(_)),
                    "cap {cap}: {report:?}"
                );
                assert_eq!(report.interactions, fuel);
            } else {
                assert_eq!(
                    report.obs,
                    Obs::Exhausted {
                        limit: Limit::Fuel,
                        cap,
                        observed: fuel,
                    }
                );
                assert_eq!(report.interactions, fuel - 1);
            }
        }
    }

    #[test]
    fn empty_unit_shape_reduces_to_an_empty_ordered_observation() {
        let checked = empty();
        let net = lower(&IDS, &checked, &[], &Limits::default()).expect("empty net");
        let limits = Limits::default();
        let report = reduce(
            &IDS,
            ReduceRequest {
                checked: &checked,
                candidate: &net,
                inputs: &[],
                limits: &limits,
                plan: Plan::default(),
            },
        );
        assert_eq!(report.obs, Obs::Normal(alloc::vec::Vec::new()));
        assert_eq!(report.interactions, 0);
        assert_eq!(report.final_wires, Some(0));
    }

    #[test]
    fn unsupported_profile_facts_and_operations_refuse_before_lowering() {
        let mut f32_builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::F32,
            crate::ir::Subject::authored(&IDS, "net-refuse-f32-profile"),
        );
        let f32_input = f32_builder.input(crate::ir::ScalarType::F32);
        f32_builder.output(f32_input);
        let f32_graph = match crate::check::check(&IDS, f32_builder.finish()) {
            Ok(graph) => graph,
            Err(error) => panic!("F32 control graph rejected: {error}"),
        };
        assert_eq!(
            lower(
                &IDS,
                &f32_graph,
                &[crate::sem::Value::F32(0)],
                &Limits::default()
            ),
            Err(Refusal::Profile)
        );

        let mut fact_builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::I64,
            crate::ir::Subject::authored(&IDS, "net-refuse-fact"),
        );
        let fact_input = fact_builder.input(crate::ir::ScalarType::I64);
        fact_builder.fact(
            fact_input,
            crate::ir::Fact::Range {
                lo: crate::ir::Literal::I64(0),
                hi: crate::ir::Literal::I64(9),
            },
            crate::ir::Provenance::InputValidated,
        );
        let fact_graph = match crate::check::check(&IDS, fact_builder.finish()) {
            Ok(graph) => graph,
            Err(error) => panic!("input-fact control graph rejected: {error}"),
        };
        assert_eq!(
            lower(
                &IDS,
                &fact_graph,
                &[crate::sem::Value::I64(4)],
                &Limits::default()
            ),
            Err(Refusal::Facts)
        );

        let mut operation_builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::I64,
            crate::ir::Subject::authored(&IDS, "net-refuse-guard"),
        );
        let operation_input = operation_builder.input(crate::ir::ScalarType::I64);
        let _guarded = operation_builder.guard(
            crate::ir::Fact::Range {
                lo: crate::ir::Literal::I64(0),
                hi: crate::ir::Literal::I64(9),
            },
            operation_input,
        );
        operation_builder.output(operation_input);
        let operation_graph = match crate::check::check(&IDS, operation_builder.finish()) {
            Ok(graph) => graph,
            Err(error) => panic!("guard control graph rejected: {error}"),
        };
        assert_eq!(
            lower(
                &IDS,
                &operation_graph,
                &[crate::sem::Value::I64(4)],
                &Limits::default()
            ),
            Err(Refusal::Operation)
        );
    }

    #[test]
    fn unused_input_fires_erase_and_returns_no_prefix() {
        let subject = crate::ir::Subject::authored(&IDS, "net-discarded-input");
        let mut builder = crate::ir::GraphBuilder::new(crate::ir::SubProfile::I64, subject);
        let _unused = builder.input(crate::ir::ScalarType::I64);
        let checked = match crate::check::check(&IDS, builder.finish()) {
            Ok(checked) => checked,
            Err(error) => panic!("discarded-input graph rejected: {error}"),
        };
        let row = [crate::sem::Value::I64(-17)];
        let net = lower(&IDS, &checked, &row, &Limits::default()).expect("discarded input net");
        assert_eq!(net.rule_counts(), (0, 0, 1));
        assert_eq!(net.expected_interactions(), Some(1));
        let limits = Limits::default();
        let report = reduce(
            &IDS,
            ReduceRequest {
                checked: &checked,
                candidate: &net,
                inputs: &row,
                limits: &limits,
                plan: Plan::default(),
            },
        );
        assert_eq!(report.obs, Obs::Normal(alloc::vec::Vec::new()));
        assert_eq!(report.interactions, 1);
        assert_eq!(report.final_wires, Some(0));
    }

    #[test]
    fn seeded_faults_are_rejected_before_reduction() {
        let (checked, original, row) = sample_interaction_graph(11, 4);
        let limits = Limits::default();
        let must_reject = |candidate: InteractionGraph, expected: Invalid| {
            let result = validate(&IDS, &checked, &candidate, &row, &limits);
            assert_eq!(result, Err(Refusal::InternalInvalid(expected)));
            let report = reduce(
                &IDS,
                ReduceRequest {
                    checked: &checked,
                    candidate: &candidate,
                    inputs: &row,
                    limits: &limits,
                    plan: Plan::default(),
                },
            );
            assert_eq!(report.obs, Obs::InternalInvalid(expected));
            assert_eq!(report.interactions, 0);
        };

        let mut swapped = original.clone();
        let sub = 2u32;
        swapped.agents[usize::from(0u8)].ports[0] = Some(Endpoint::Port(PortRef {
            agent: sub,
            port_index: PortIndex::from(1),
        }));
        swapped.agents[1].ports[0] = Some(Endpoint::Port(PortRef {
            agent: sub,
            port_index: PortIndex::from(0),
        }));
        swapped.agents[sub as usize].ports.swap(0, 1);
        must_reject(swapped, Invalid::SourceWiring);

        let mut dangling = original.clone();
        dangling.agents[2].ports[0] = None;
        must_reject(dangling, Invalid::NonReciprocal);

        let mut extra_port = original.clone();
        let narrow = extra_port
            .agents
            .iter()
            .position(|agent| agent.symbol.port_count() == 1)
            .expect("sample has a one-port agent");
        extra_port.agents[narrow].ports[1] = Some(Endpoint::Output(0));
        must_reject(extra_port, Invalid::PortArity);

        let mut duplicated = original.clone();
        duplicated.agents[1].ports[0] = Some(Endpoint::Port(PortRef {
            agent: sub,
            port_index: PortIndex::from(0),
        }));
        must_reject(duplicated, Invalid::NonReciprocal);

        let mut stale = original.clone();
        stale.build_key[0] ^= 1;
        must_reject(stale, Invalid::BuildKey);

        let mut payload = original.clone();
        payload.agents[0].symbol = Symbol::Num(99);
        must_reject(payload, Invalid::SourceSymbol);

        let mut wrong_order = original.clone();
        wrong_order.outputs.swap(0, 1);
        for agent in &mut wrong_order.agents {
            for endpoint in &mut agent.ports {
                if let Some(Endpoint::Output(output)) = endpoint {
                    match *output {
                        0 => *output = 1,
                        1 => *output = 0,
                        _ => {}
                    }
                }
            }
        }
        must_reject(wrong_order, Invalid::SourceWiring);

        let mut foreign = original;
        foreign.agents[0].origin = Origin::Node(crate::ir::NodeId(99));
        must_reject(foreign, Invalid::SourceOrigin);
    }

    #[test]
    fn unsupported_active_pair_is_internal_invalid() {
        let pair = Pair {
            first: 0,
            first_generation: 0,
            second: 1,
            second_generation: 0,
        };
        let first = RuntimeAgent {
            origin: Origin::Node(crate::ir::NodeId(0)),
            symbol: Symbol::Num(1),
            ports: [
                Some(Endpoint::Port(PortRef {
                    agent: 1,
                    port_index: PortIndex::from(0),
                })),
                None,
                None,
            ],
        };
        let second = RuntimeAgent {
            origin: Origin::Node(crate::ir::NodeId(1)),
            symbol: Symbol::Num(2),
            ports: [
                Some(Endpoint::Port(PortRef {
                    agent: 0,
                    port_index: PortIndex::from(0),
                })),
                None,
                None,
            ],
        };
        let slots = alloc::vec![
            Slot {
                agent: Some(first),
                generation: 0,
                queued_generation: None,
            },
            Slot {
                agent: Some(second),
                generation: 0,
                queued_generation: None,
            },
        ];
        let result = rewrite::compile(&slots, &[], pair, StoragePolicy::Reuse);
        assert_eq!(result, Err(Invalid::StuckPair));
    }

    #[test]
    fn cancellation_and_input_refusals_are_not_partial_results() {
        let (checked, net, row) = sample_interaction_graph(11, 4);
        let limits = Limits::default();
        let cancel_after_two = |steps| steps >= 2;
        let report = reduce(
            &IDS,
            ReduceRequest {
                checked: &checked,
                candidate: &net,
                inputs: &row,
                limits: &limits,
                plan: Plan {
                    cancel: Some(&cancel_after_two),
                    ..Plan::default()
                },
            },
        );
        assert_eq!(report.obs, Obs::Cancelled);
        assert_eq!(report.interactions, 2);

        let wrong = [crate::sem::Value::Bool(true), crate::sem::Value::I64(4)];
        assert_eq!(
            lower(&IDS, &checked, &wrong, &Limits::default()),
            Err(Refusal::Interface)
        );
        let report = reduce(
            &IDS,
            ReduceRequest {
                checked: &checked,
                candidate: &net,
                inputs: &wrong,
                limits: &limits,
                plan: Plan::default(),
            },
        );
        assert_eq!(report.obs, Obs::Refused(Refusal::Interface));
        assert_eq!(report.interactions, 0);

        let wrong_values = inputs(12, 4);
        let report = reduce(
            &IDS,
            ReduceRequest {
                checked: &checked,
                candidate: &net,
                inputs: &wrong_values,
                limits: &limits,
                plan: Plan::default(),
            },
        );
        assert_eq!(report.obs, Obs::InternalInvalid(Invalid::InputValues));
        assert_eq!(report.interactions, 0);
    }
}
