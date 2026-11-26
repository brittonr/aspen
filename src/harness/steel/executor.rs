//! Reviewed Steel actor execution in a capability-free, metered VM.
//!
//! The source is parsed with Steel's reader and admitted only if it is a sequence of top-level
//! definitions in a closed core language (`define`, `lambda`, `if` with both branches, `begin`,
//! `quote`, application, integer, string, and boolean literals). Every user identifier is renamed
//! into a private namespace, so a script can reach only its own bindings and the metered primitive
//! table below; nothing else is registered in the raw VM, which has no prelude, no modules, and no
//! filesystem, process, environment, network, clock, or thread primitives. Each lambda
//! body is instrumented with a fuel tick, so every loop (all iteration is application) spends
//! deterministic fuel; every primitive charges fuel and its allocation to the same meter, and
//! hostcalls are counted before dispatch.

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use steel::rvals::IntoSteelVal;
use steel::steel_vm::register_fn::RegisterFn;

type Engine = steel::steel_vm::engine::Engine;
type Expr = steel::parser::ast::ExprKind;
type Token = steel::parser::tokens::TokenType<steel::parser::interner::InternedString>;
type SteelValue = steel::SteelVal;
type SteelError = steel::SteelErr;
type SteelResult<T, E = SteelError> = std::result::Result<T, E>;
type Shared<T> = std::sync::Arc<T>;
type Lock<T> = std::sync::Mutex<T>;
type PreservesValue = preserves::IOValue;
type Request = super::core::AdmissionRequest;
type Step = super::core::CoreStep;
type ActorConfig = super::schema::ActorExecutorConfig;
type ActorMode = super::schema::ActorKind;
type Suite = super::schema::Suite;
type RunReceipt<'a> = super::schema::SteelExecutionReceiptInput<'a>;
type ResourceReceipt = super::schema::SteelResourceReceiptInput;
type SourceConfig = super::schema::SteelExecutorConfig;
type MoltenError = crate::error::Failure;
type Result<T> = crate::error::Result<T>;

fn validate_bound_request(hostcall_request: &PreservesValue, operation: &str) -> Result<()> {
    super::schema::validate_hostcall_effect_binding_request(hostcall_request, operation)
}

fn receipt_value(input: RunReceipt<'_>) -> PreservesValue {
    super::schema::steel_execution_receipt_value(input)
}

fn source_ref(config: &SourceConfig) -> Result<String> {
    super::schema::steel_source_ref(config)
}

fn canonical_hash(value: &PreservesValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn string(value: &str) -> PreservesValue {
    crate::preserves_rail::string(value)
}

fn to_text(value: &PreservesValue) -> Result<String> {
    crate::preserves_rail::to_text(value)
}

/// Fuel available to one actor step: one unit per lambda entry, closure creation, and primitive
/// call.
const STEEL_FUEL_LIMIT: u64 = 16 * 1024;
/// Bytes one actor step may allocate through primitives and closure creation, including its input;
/// frees are never credited.
const STEEL_MEMORY_LIMIT_BYTES: u64 = 64 * 1024;
const STEEL_MAX_SOURCE_BYTES: usize = 8 * 1024;
const STEEL_MAX_INPUT_BYTES: usize = 8 * 1024;
const STEEL_MAX_OUTPUT_BYTES: usize = 8 * 1024;
/// The largest single string a primitive may produce.
const STEEL_MAX_STRING_BYTES: usize = STEEL_MAX_OUTPUT_BYTES;
const STEEL_MAX_HOSTCALLS: u64 = 8;
const STEEL_MAX_DEFINITIONS: usize = 128;
const STEEL_MAX_NESTING: usize = 64;
/// Bracket depth admitted before Steel's recursive reader runs; generous against
/// `STEEL_MAX_NESTING` because definition and lambda forms add brackets without adding expression
/// depth.
const STEEL_MAX_BRACKET_DEPTH: usize = 2 * STEEL_MAX_NESTING;
const STEEL_MAX_IDENTIFIER_BYTES: usize = 64;
/// Parameters per lambda and arguments per application, so every call frame has a bounded size.
const STEEL_MAX_ARITY: usize = 16;
/// Allocation charged for a primitive result that is not a string.
const STEEL_SCALAR_BYTES: u64 = 16;
/// Allocation charged for creating a closure, plus `STEEL_SCALAR_BYTES` per variable it can capture
/// or bind.
const STEEL_CLOSURE_BYTES: u64 = 64;
/// Stack for the VM thread. Charged closures bound any closure chain to a few thousand links, far
/// below what Steel's recursive drop of such a chain needs from this stack.
const STEEL_VM_STACK_BYTES: usize = 64 * 1024 * 1024;

/// Prefix that moves every user identifier out of the VM's global and keyword namespace.
const USER_NAMESPACE: &str = "molten-user-";
const FUEL_TICK: &str = "molten-prim-fuel-tick";
const CLOSURE: &str = "molten-prim-closure";
const PRIM_HOSTCALL: &str = "molten-prim-hostcall";
const PRIM_STRING_APPEND: &str = "molten-prim-string-append";
const PRIM_STRING_LENGTH: &str = "molten-prim-string-length";
const PRIM_SUBSTRING: &str = "molten-prim-substring";
const PRIM_STRING_EQUAL: &str = "molten-prim-string-equal";
const PRIM_EQUAL: &str = "molten-prim-equal";
const PRIM_NOT: &str = "molten-prim-not";
const PRIM_ADD: &str = "molten-prim-add";
const PRIM_SUBTRACT: &str = "molten-prim-subtract";
const PRIM_MULTIPLY: &str = "molten-prim-multiply";
const PRIM_INTEGER_EQUAL: &str = "molten-prim-integer-equal";
const PRIM_LESS: &str = "molten-prim-less";
/// The complete set of names a reviewed script may reference besides its own bindings, each mapped
/// to the private global it is registered under. Steel's compiler never recognizes a private name
/// as a built-in, so every call (including `(+ local 1)`) reaches the metered, overflow-checked
/// primitive.
const STEEL_PRIMITIVES: &[(&str, &str)] = &[
    ("molten-hostcall", PRIM_HOSTCALL),
    ("string-append", PRIM_STRING_APPEND),
    ("string-length", PRIM_STRING_LENGTH),
    ("substring", PRIM_SUBSTRING),
    ("string=?", PRIM_STRING_EQUAL),
    ("equal?", PRIM_EQUAL),
    ("not", PRIM_NOT),
    ("+", PRIM_ADD),
    ("-", PRIM_SUBTRACT),
    ("*", PRIM_MULTIPLY),
    ("=", PRIM_INTEGER_EQUAL),
    ("<", PRIM_LESS),
];

const _: () = assert!(STEEL_MAX_DEFINITIONS <= 1_024);
const _: () = assert!((STEEL_MAX_INPUT_BYTES as u64) < STEEL_MEMORY_LIMIT_BYTES);

pub fn execute_steel_actor_step(
    suite: &Suite,
    step: &Step,
    actor_input: &PreservesValue,
    hostcall_request: &PreservesValue,
) -> Result<Option<PreservesValue>> {
    let actor_id = step.primary_actor();
    let Some(actor) = suite.actors.iter().find(|actor| actor.id == actor_id) else {
        return Err(MoltenError::invalid_harness(format!("actor {actor_id} missing from executor registry")));
    };
    if actor.kind != ActorMode::Steel {
        return Ok(None);
    }
    let Some(ActorConfig::Steel(config)) = actor.executor.as_ref() else {
        return Err(MoltenError::invalid_harness(format!(
            "steel actor {actor_id} missing reviewed Steel executor preflight fixture"
        )));
    };

    let prepared = prepare_run(actor_id, &config.source, &config.callable, actor_input)?;
    let operation = Request::from_step(step).action.as_str().to_string();
    validate_bound_request(hostcall_request, &operation)?;
    let execution = run_vm(&prepared, &operation, &config.allowed_hostcalls).into_result(actor_id)?;
    let source_ref = source_ref(config)?;

    Ok(Some(finish_value(FinishInput {
        actor_id,
        source_ref: &source_ref,
        callable: &config.callable,
        source_bytes: config.source.len() as u64,
        operation: &operation,
        actor_input,
        input_text: &prepared.input_text,
        execution: &execution,
    })?))
}

struct Prepared {
    input_text: String,
    program: String,
    entry: String,
}

struct Execution {
    output_text: String,
    hostcall_count: u64,
    fuel_remaining: u64,
}

/// The ceiling that stopped a run. Exhaustion is sticky: once set, every metered operation fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Exhaustion {
    Fuel,
    Memory,
    Hostcalls,
}

impl Exhaustion {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Fuel => "fuel",
            Self::Memory => "memory",
            Self::Hostcalls => "hostcall",
        }
    }
}

/// Everything a run observed, including the meter, whether or not the script completed.
struct SteelRun {
    outcome: std::result::Result<String, String>,
    meter: MeterState,
}

impl SteelRun {
    fn into_result(self, actor_id: &str) -> Result<Execution> {
        if let Some(exhaustion) = self.meter.exhaustion {
            return Err(MoltenError::invalid_harness(format!(
                "Steel executor actor {actor_id} exhausted its deterministic {} limit; the run is denied",
                exhaustion.as_str()
            )));
        }
        let output_text = self.outcome.map_err(|error| {
            MoltenError::invalid_harness(format!("Steel executor actor {actor_id} failed in the sandboxed VM: {error}"))
        })?;
        if output_text.len() > STEEL_MAX_OUTPUT_BYTES {
            return Err(MoltenError::invalid_harness(format!(
                "Steel executor output for actor {actor_id} exceeds deterministic resource limit"
            )));
        }
        Ok(Execution {
            output_text,
            hostcall_count: self.meter.hostcall_count,
            fuel_remaining: self.meter.fuel_remaining,
        })
    }
}

struct FinishInput<'a> {
    actor_id: &'a str,
    source_ref: &'a str,
    callable: &'a str,
    source_bytes: u64,
    operation: &'a str,
    actor_input: &'a PreservesValue,
    input_text: &'a str,
    execution: &'a Execution,
}

fn prepare_run(actor_id: &str, source: &str, callable: &str, actor_input: &PreservesValue) -> Result<Prepared> {
    if source.len() > STEEL_MAX_SOURCE_BYTES {
        return Err(MoltenError::invalid_harness(format!(
            "Steel executor source for actor {actor_id} exceeds deterministic resource limit"
        )));
    }
    let input_text = to_text(actor_input)?;
    if input_text.len() > STEEL_MAX_INPUT_BYTES {
        return Err(MoltenError::invalid_harness(format!(
            "Steel executor input for actor {actor_id} exceeds deterministic resource limit"
        )));
    }
    // Parsing and rendering recurse over the source, so they run on the sandbox thread as well.
    let compiled =
        on_sandbox_thread(|| compile_program(source, callable))
            .and_then(|compiled| compiled)
            .map_err(|issue| {
                MoltenError::invalid_harness(format!(
                    "Steel executor source for actor {actor_id} is outside the reviewed Steel sandbox: {issue}"
                ))
            })?;
    let (program, entry) = compiled;
    Ok(Prepared {
        input_text,
        program,
        entry,
    })
}

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/harness/steel/parts/executor/p000/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/harness/steel/parts/executor/p001/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/harness/steel/parts/executor/p002/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/harness/steel/parts/executor/p003/body.rs"));
