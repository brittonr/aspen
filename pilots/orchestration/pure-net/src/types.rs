// SPDX-License-Identifier: AGPL-3.0-or-later

pub const PRODUCER_REVISION: &str = "f7af9e972aa2968d22978794a667d267cf2745e8";
pub const REQUESTED_INETS: &str = "01bd554a43504b58dc64c607f46dfe4acc7735cd";
pub const PRODUCER_INETS: &str = "98bbb01070664e4462a35efa5d62de6f27143572";
pub const OVERLAY_REVISION: &str = "aspen-noble-sequential-observation/1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Backend {
    SequentialNet,
    IndependentReference,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostBindings {
    pub executable: molten_batch_core::Id,
    pub dependency_closure: molten_batch_core::Id,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionGrants {
    pub fuel: u64,
    pub memory_bytes: u64,
    pub max_output_bytes: u64,
    pub max_attempts: u32,
}

/// Borrowed program/input bytes, measured host bindings and bounded request grants.
pub struct ExecutionInput<'a> {
    pub manifest_bytes: &'a [u8],
    pub program_bytes: &'a [u8],
    pub input_bytes: &'a [u8],
    pub host: &'a HostBindings,
    pub grants: &'a ExecutionGrants,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bindings {
    pub producer: String,
    pub requested_inets_reference: String,
    pub actual_producer_inets_reference: String,
    pub arithmetic: String,
    pub rules: String,
    pub lowering: String,
    pub net_contract: String,
    pub plan_format: String,
    pub plan_checker: String,
    pub plan_executor: String,
    pub arena_representation: String,
    pub reducer: String,
    pub readback: String,
    pub reference_evaluator: String,
    pub canonicalization: String,
    pub host: HostBindings,
}
impl Bindings {
    pub(crate) fn exact(host: HostBindings) -> Self {
        Self {
            producer: PRODUCER_REVISION.into(),
            requested_inets_reference: REQUESTED_INETS.into(),
            actual_producer_inets_reference: PRODUCER_INETS.into(),
            arithmetic: "wrapping-i64-mod-2^64/source-operand-order/v1".into(),
            rules: noble_ngraph::net::PLAN_RULES_REVISION.into(),
            lowering: noble_ngraph::net::LOWERING_REVISION.into(),
            net_contract: noble_ngraph::net::CONTRACT_REVISION.into(),
            plan_format: noble_ngraph::net::PLAN_REVISION.into(),
            plan_checker: noble_ngraph::net::PLAN_CHECKER_REVISION.into(),
            plan_executor: noble_ngraph::net::PLAN_EXECUTOR_REVISION.into(),
            arena_representation: noble_ngraph::net::PLAN_REPRESENTATION_REVISION.into(),
            reducer: OVERLAY_REVISION.into(),
            readback: "noble-net-complete-ordered-scalar-readback/3".into(),
            reference_evaluator: "noble-ngraph-source-extracted-ir/reference/0+aspen-node-hook/1".into(),
            canonicalization: "molten.orchestration.document.v1/preserves-packed/serde-json-projection/blake3".into(),
            host,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub version: String,
    pub backend: Backend,
    pub bindings: Bindings,
    pub limits: crate::budget::Limits,
}
impl Manifest {
    pub fn validate(&self) -> Result<(), Error> {
        if self.version != "pure-net-eval/v1" || self.bindings != Bindings::exact(self.bindings.host.clone()) {
            return Err(Error::Bindings);
        }
        self.limits.validate()
    }
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Program {
    pub source: String,
    pub input_arity: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MaterializedProgram {
    pub program: Program,
    pub producer: String,
    pub checked_ir: String,
    pub ir: molten_batch_core::Id,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OrderedInputs {
    pub values: Vec<i64>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Phase {
    Decode,
    ParseCheck,
    Extract,
    Lower,
    Validate,
    Setup,
    Reduce,
    Readback,
    Reference,
    Complete,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    Continue,
    Cancel,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub phase: Phase,
    pub work: u64,
    pub interactions: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Resource {
    Decode,
    Source,
    Inputs,
    Ir,
    Nodes,
    Wires,
    Queue,
    Memory,
    Setup,
    Validation,
    Reduction,
    Reference,
    Readback,
    Observations,
    TotalWork,
    Attempts,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Error {
    Malformed,
    Unsupported,
    Bindings,
    InvalidLimits,
    Incomplete,
    Stuck,
    Cancelled,
    Exhausted(Resource),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Value {
    I64(i64),
    Bool(bool),
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Outcome {
    Completed(Vec<Value>),
    Rejected(Error),
}
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Counters {
    pub total_work: u64,
    pub setup_work: u64,
    pub validation_work: u64,
    pub readback_work: u64,
    pub queue_pops: u64,
    pub interactions: u64,
    pub reference_nodes: u64,
    pub extraction_work: u64,
    pub expanded_nodes: u64,
    pub initial_agents: u64,
    pub peak_live_agents: u64,
    pub arena_slots_high_water: u64,
    pub queue_high_water: u64,
    pub allocated_slots: u64,
    pub reused_slots: u64,
    pub freed_slots: u64,
    pub plan_bytes: u64,
    pub plan_microsteps: u64,
    pub plan_scratch_bytes_high_water: u64,
    pub transient_live_agents_high_water: u64,
    pub initial_net_retained_bytes: u64,
    pub logical_memory_reservation: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub version: String,
    pub backend: Backend,
    pub manifest: molten_batch_core::Id,
    pub program: molten_batch_core::Id,
    pub ordered_inputs: molten_batch_core::Id,
    pub manifest_semantics: molten_batch_core::Id,
    pub program_semantics: molten_batch_core::Id,
    pub ordered_input_semantics: molten_batch_core::Id,
    pub bindings: Bindings,
    pub limits: crate::budget::Limits,
    pub grants: ExecutionGrants,
    pub checked_ir: Option<molten_batch_core::Id>,
    pub compiled_net: Option<molten_batch_core::Id>,
    pub producer_graph_digest: Option<molten_batch_core::Id>,
    pub producer_plan_stream_digest: Option<molten_batch_core::Id>,
    pub outcome: Outcome,
    pub counters: Counters,
    pub observations: Vec<Observation>,
}
