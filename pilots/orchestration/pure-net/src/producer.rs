// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::*;

pub(crate) struct Ids;
struct Hasher(blake3::Hasher);
impl noble_ngraph::identity::IdentityHasher for Hasher {
    fn update(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }
    fn finish(self: Box<Self>) -> [u8; 32] {
        *self.0.finalize().as_bytes()
    }
}
impl noble_ngraph::identity::IdentityScheme for Ids {
    fn name(&self) -> &'static str {
        "blake3"
    }
    fn start(&self) -> Box<dyn noble_ngraph::identity::IdentityHasher + '_> {
        Box::new(Hasher(blake3::Hasher::new()))
    }
}
// r[impl molten.pure_net.producer]
pub fn compile(source: &str, input_arity: u32) -> Result<Vec<u8>, Error> {
    encode_program(&Program {
        source: source.to_owned(),
        input_arity,
    })
}
pub fn encode_program(program: &Program) -> Result<Vec<u8>, Error> {
    let extraction = checked(program, &Limits::default(), &mut |_, _| Ok(()))?;
    let checked_ir = noble_ngraph::format::graph::print(extraction.checked.graph());
    let ir = identity("pure-net-eval/checked-source-ir/v1", &checked_ir)?;
    encode(PROGRAM, &MaterializedProgram {
        program: program.clone(),
        producer: PRODUCER_REVISION.into(),
        checked_ir,
        ir,
    })
}
pub(crate) fn diagnostic(d: &noble_ngraph::diag::Diagnostic) -> Error {
    match d.code {
        noble_ngraph::diag::Code::Budget(noble_ngraph::diag::BudgetKind::Inputs) => Error::Exhausted(Resource::Inputs),
        noble_ngraph::diag::Code::Budget(noble_ngraph::diag::BudgetKind::Outputs) => {
            Error::Exhausted(Resource::Readback)
        }
        noble_ngraph::diag::Code::Budget(
            noble_ngraph::diag::BudgetKind::ForwardScratch | noble_ngraph::diag::BudgetKind::BackwardScratch,
        ) => Error::Exhausted(Resource::Memory),
        noble_ngraph::diag::Code::Budget(_) => Error::Exhausted(Resource::Ir),
        noble_ngraph::diag::Code::GraphMalformed(_) | noble_ngraph::diag::Code::Format => Error::Malformed,
        _ => Error::Unsupported,
    }
}
pub(crate) fn checked(
    program: &Program,
    limits: &Limits,
    phase: &mut dyn FnMut(Phase, u64) -> Result<(), Error>,
) -> Result<noble_ngraph::extract::Extraction, Error> {
    if program.source.len()
        > usize::try_from(limits.max_source_bytes).map_err(|_| Error::Exhausted(Resource::Source))?
    {
        return Err(Error::Exhausted(Resource::Source));
    }
    if program.input_arity > limits.max_inputs {
        return Err(Error::Exhausted(Resource::Inputs));
    }
    // Reserve the bounded frontend meter before invoking the producer; the
    // producer itself enforces bytes/nodes/depth/work before arena growth.
    phase(Phase::ParseCheck, u64::from(limits.max_frontend_work))?;
    let session = noble_contracts::source::Session::new();
    let prepared = session
        .prepare(
            program.source.as_bytes(),
            &noble_ngraph::extract::source::i64s(
                usize::try_from(program.input_arity).map_err(|_| Error::Exhausted(Resource::Inputs))?,
            ),
            noble_contracts::Limits {
                bytes: limits.max_source_bytes,
                nodes: limits.max_source_nodes,
                depth: limits.max_source_depth,
                work: limits.max_frontend_work,
            },
        )
        .map_err(|e| match e.diagnostic().kind {
            noble_contracts::DiagnosticKind::Exhausted => Error::Exhausted(Resource::Source),
            noble_contracts::DiagnosticKind::Unsupported => Error::Unsupported,
            _ => Error::Malformed,
        })?;
    let submission = prepared.submission().ok_or(Error::Unsupported)?;
    let mut closure: Vec<u64> = submission.definitions.iter().map(|d| d.identity).collect();
    closure.sort_unstable();
    closure.dedup();
    // Kernel acceptance has its own producer-declared shared work meter.
    // Reserve every body's full cap plus bounded graph-checker traversals;
    // extraction separately enforces its actual symbolic instruction meter.
    let kernel_work = u64::from(submission.request.limits.work).saturating_mul(submission.definitions.len() as u64 + 1);
    phase(
        Phase::Extract,
        limits
            .max_extraction_work
            .saturating_add(kernel_work)
            .saturating_add(limits.max_ir_nodes.saturating_mul(limits.max_ir_nodes)),
    )?;
    let admitted = noble_ngraph::extract::admit(&Ids, submission, &noble_ngraph::extract::Request {
        profile: "ngraph-i64/0",
        claimed_closure: &closure,
        budget: noble_ngraph::extract::Budget {
            work_units: limits.max_extraction_work,
            expanded_nodes: limits.max_ir_nodes,
            inline_depth: u64::from(limits.max_source_depth),
        },
    })
    .map_err(|d| diagnostic(&d))?;
    let budgets = checker_budgets(limits);
    let result = noble_ngraph::extract::extract(&Ids, &admitted, &budgets).map_err(|d| diagnostic(&d))?;
    if result.checked.graph().outputs.len()
        > usize::try_from(limits.max_outputs).map_err(|_| Error::Exhausted(Resource::Readback))?
    {
        return Err(Error::Exhausted(Resource::Readback));
    }
    noble_ngraph::net::check_profile(&result.checked).map_err(refusal)?;
    Ok(result)
}
fn checker_budgets(limits: &Limits) -> noble_ngraph::check::Budgets {
    noble_ngraph::check::Budgets {
        work_units: limits.max_extraction_work,
        expanded_nodes: limits.max_ir_nodes,
        live_nodes: limits.max_ir_nodes,
        inline_depth: u64::from(limits.max_source_depth),
        layers: limits.max_ir_nodes,
        inputs: u64::from(limits.max_inputs),
        outputs: u64::from(limits.max_outputs),
        graph_bytes: limits.max_ir_bytes,
        affine_terms: 4_096,
        table_length_entries: 65_536,
        table_entries: 1_048_576,
        facts: 262_144,
        snapshot_bytes: 4 * 1024 * 1024,
        forward_scratch: 64 * 1024 * 1024,
        backward_scratch: 128 * 1024 * 1024,
        optimizer_steps: 1_000_000,
        residual_nodes: 262_144,
        variants: 1,
    }
}
pub(crate) fn refusal(value: noble_ngraph::net::Refusal) -> Error {
    match value {
        noble_ngraph::net::Refusal::Exhausted { limit, .. } => exhausted(limit),
        noble_ngraph::net::Refusal::InternalInvalid(invalid) => invalid_error(invalid),
        noble_ngraph::net::Refusal::Interface => Error::Malformed,
        _ => Error::Unsupported,
    }
}
pub(crate) fn exhausted(limit: noble_ngraph::net::Limit) -> Error {
    Error::Exhausted(match limit {
        noble_ngraph::net::Limit::Fuel => Resource::Reduction,
        noble_ngraph::net::Limit::Agents | noble_ngraph::net::Limit::ArenaReservation => Resource::Nodes,
        noble_ngraph::net::Limit::Wires => Resource::Wires,
        noble_ngraph::net::Limit::WorklistReservation => Resource::Queue,
        noble_ngraph::net::Limit::PlanScratch => Resource::Memory,
    })
}
pub(crate) fn invalid_error(value: noble_ngraph::net::Invalid) -> Error {
    match value {
        noble_ngraph::net::Invalid::ReadBack | noble_ngraph::net::Invalid::LeftoverAgent => Error::Incomplete,
        noble_ngraph::net::Invalid::StuckPair => Error::Stuck,
        noble_ngraph::net::Invalid::PlanInterrupted => Error::Cancelled,
        noble_ngraph::net::Invalid::PlanReservationFailure => Error::Exhausted(Resource::Memory),
        _ => Error::Malformed,
    }
}
pub(crate) fn values(values: Vec<noble_ngraph::sem::Value>) -> Result<Vec<Value>, Error> {
    values
        .into_iter()
        .map(|v| match v {
            noble_ngraph::sem::Value::I64(v) => Ok(Value::I64(v)),
            noble_ngraph::sem::Value::Bool(v) => Ok(Value::Bool(v)),
            noble_ngraph::sem::Value::F32(_) => Err(Error::Unsupported),
        })
        .collect()
}

// Exact canonical net representation. Producer native byte-stream digests are
// retained separately; Debug, pointer/layout and JSON rendering never define it.
#[derive(serde::Serialize)]
enum Origin {
    Node(u32),
    Dup(u32, u32),
    Era(u32),
}
#[derive(serde::Serialize)]
enum Symbol {
    Num(u64),
    Op2(&'static str),
    Op1(&'static str, u64),
    Dup,
    Era,
}
#[derive(serde::Serialize)]
enum Endpoint {
    Port(u32, u8),
    Output(u32),
}
#[derive(serde::Serialize)]
struct Agent {
    origin: Origin,
    symbol: Symbol,
    ports: [Option<Endpoint>; 3],
}
#[derive(serde::Serialize)]
struct Net<'a> {
    producer_graph_digest: molten_batch_core::Id,
    contract: &'a str,
    lowering: &'a str,
    inputs: &'a [i64],
    agents: Vec<Agent>,
    outputs: Vec<(u32, u8)>,
    output_types: Vec<&'static str>,
}
fn op(op: noble_ngraph::net::Op) -> &'static str {
    match op {
        noble_ngraph::net::Op::Add => "add",
        noble_ngraph::net::Op::Sub => "sub",
        noble_ngraph::net::Op::Mul => "mul",
        noble_ngraph::net::Op::Eq => "eq",
    }
}
pub(crate) fn net_identity(net: &noble_ngraph::net::InteractionGraph) -> Result<molten_batch_core::Id, Error> {
    let agents = net
        .agents
        .iter()
        .map(|a| Agent {
            origin: match a.origin {
                noble_ngraph::net::Origin::Node(n) => Origin::Node(n.0),
                noble_ngraph::net::Origin::Dup { value, ordinal } => Origin::Dup(value.0, ordinal),
                noble_ngraph::net::Origin::Era(n) => Origin::Era(n.0),
            },
            symbol: match a.symbol {
                noble_ngraph::net::Symbol::Num(v) => Symbol::Num(v),
                noble_ngraph::net::Symbol::Op2(o) => Symbol::Op2(op(o)),
                noble_ngraph::net::Symbol::Op1(o, v) => Symbol::Op1(op(o), v),
                noble_ngraph::net::Symbol::Dup => Symbol::Dup,
                noble_ngraph::net::Symbol::Era => Symbol::Era,
            },
            ports: a.ports.map(|p| {
                p.map(|e| match e {
                    noble_ngraph::net::Endpoint::Port(p) => Endpoint::Port(p.agent, u8::from(p.port_index)),
                    noble_ngraph::net::Endpoint::Output(i) => Endpoint::Output(i),
                })
            }),
        })
        .collect();
    let canonical = Net {
        producer_graph_digest: molten_batch_core::Id(net.graph_id),
        contract: net.contract_revision,
        lowering: net.lowering_revision,
        inputs: &net.input_values,
        agents,
        outputs: net.outputs.iter().map(|p| (p.agent, u8::from(p.port_index))).collect(),
        output_types: net
            .output_types
            .iter()
            .map(|t| match t {
                noble_ngraph::ir::ScalarType::I64 => "i64",
                noble_ngraph::ir::ScalarType::Bool => "bool",
                noble_ngraph::ir::ScalarType::F32 => "f32",
            })
            .collect(),
    };
    identity("pure-net-eval/compiled-net/v1", &canonical)
}
