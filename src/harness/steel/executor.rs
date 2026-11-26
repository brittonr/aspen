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

/// Admits `source` against the closed sandbox language and renders the instrumented program text,
/// plus the private global name of `callable`. The callable is validated as an identifier and
/// resolved as a top-level definition; it is never spliced into program text.
fn compile_program(source: &str, callable: &str) -> std::result::Result<(String, String), String> {
    validate_identifier(callable).map_err(|issue| format!("callable {issue}"))?;
    validate_bracket_depth(source)?;
    let forms = Engine::emit_ast(source).map_err(|error| format!("source does not parse: {error}"))?;
    if forms.is_empty() {
        return Err("source defines nothing".to_string());
    }
    if forms.len() > STEEL_MAX_DEFINITIONS {
        return Err(format!("source has more than {STEEL_MAX_DEFINITIONS} top-level definitions"));
    }
    let mut definitions = Vec::with_capacity(forms.len());
    let mut globals = std::collections::BTreeSet::new();
    for form in &forms {
        let Expr::Define(define) = form else {
            return Err("only top-level definitions are admitted".to_string());
        };
        let name = binder_name(&define.name)?;
        if !globals.insert(name) {
            return Err(format!("{name} is defined more than once"));
        }
        definitions.push((name, &define.body));
    }
    if !globals.contains(callable) {
        return Err(format!("callable {callable} is not a top-level definition"));
    }
    let mut renderer = Renderer {
        globals: &globals,
        scope: Vec::new(),
        text: String::with_capacity(source.len().saturating_mul(2)),
    };
    for (name, body) in definitions {
        renderer.text.push_str("(define ");
        renderer.push_user_identifier(name);
        renderer.text.push(' ');
        renderer.expression(body, 0)?;
        renderer.text.push_str(")\n");
    }
    Ok((renderer.text, format!("{USER_NAMESPACE}{callable}")))
}

/// Rejects bracket nesting deeper than the reviewed grammar admits before Steel's recursive reader
/// runs. Brackets inside string literals, comments, and character literals do not count.
fn validate_bracket_depth(source: &str) -> std::result::Result<(), String> {
    let mut depth = 0_usize;
    let mut characters = source.chars();
    while let Some(character) = characters.next() {
        match character {
            '"' => {
                while let Some(inner) = characters.next() {
                    match inner {
                        '\\' => {
                            characters.next();
                        }
                        '"' => break,
                        _ => {}
                    }
                }
            }
            ';' => {
                for inner in characters.by_ref() {
                    if inner == '\n' {
                        break;
                    }
                }
            }
            '#' if characters.clone().next() == Some('\\') => {
                characters.next();
                characters.next();
            }
            '(' | '[' | '{' => {
                depth += 1;
                if depth > STEEL_MAX_BRACKET_DEPTH {
                    return Err(format!("brackets nest deeper than {STEEL_MAX_BRACKET_DEPTH}"));
                }
            }
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Ok(())
}

/// Admits a binding name: a plain identifier from a closed character set. Collisions with
/// primitives or VM names are impossible because every binding is renamed into `USER_NAMESPACE`.
fn validate_identifier(name: &str) -> std::result::Result<(), String> {
    let Some(first) = name.chars().next() else {
        return Err("is empty".to_string());
    };
    if name.len() > STEEL_MAX_IDENTIFIER_BYTES {
        return Err(format!("exceeds {STEEL_MAX_IDENTIFIER_BYTES} bytes"));
    }
    if !first.is_ascii_alphabetic() {
        return Err(format!("{name:?} must start with an ASCII letter"));
    }
    if !name.chars().all(|character| character.is_ascii_alphanumeric() || "-_?!*<>=/".contains(character)) {
        return Err(format!("{name:?} contains characters outside the reviewed identifier set"));
    }
    Ok(())
}

fn binder_name(expr: &Expr) -> std::result::Result<&str, String> {
    let Expr::Atom(atom) = expr else {
        return Err("definition and parameter names must be identifiers".to_string());
    };
    let Token::Identifier(name) = &atom.syn.ty else {
        return Err("definition and parameter names must be identifiers".to_string());
    };
    let name = name.resolve();
    validate_identifier(name).map_err(|issue| format!("binding {issue}"))?;
    Ok(name)
}

struct Renderer<'a> {
    globals: &'a std::collections::BTreeSet<&'a str>,
    scope: Vec<&'a str>,
    text: String,
}

impl<'a> Renderer<'a> {
    fn push_user_identifier(&mut self, name: &str) {
        self.text.push_str(USER_NAMESPACE);
        self.text.push_str(name);
    }

    fn expression(&mut self, expr: &'a Expr, depth: usize) -> std::result::Result<(), String> {
        if depth > STEEL_MAX_NESTING {
            return Err(format!("expressions nest deeper than {STEEL_MAX_NESTING}"));
        }
        let next = depth + 1;
        match expr {
            Expr::Atom(atom) => self.atom(&atom.syn.ty),
            Expr::If(branch) => {
                if is_implicit_else(&branch.else_expr) {
                    return Err("if requires an explicit else branch".to_string());
                }
                self.text.push_str("(if ");
                self.expression(&branch.test_expr, next)?;
                self.text.push(' ');
                self.expression(&branch.then_expr, next)?;
                self.text.push(' ');
                self.expression(&branch.else_expr, next)?;
                self.text.push(')');
                Ok(())
            }
            Expr::LambdaFunction(lambda) => {
                if lambda.rest || lambda.kwargs {
                    return Err("rest and keyword parameters are not admitted".to_string());
                }
                if lambda.args.len() > STEEL_MAX_ARITY {
                    return Err(format!("lambdas take at most {STEEL_MAX_ARITY} parameters"));
                }
                let scope_depth = self.scope.len();
                // Every closure evaluation is charged before the closure escapes: the base cost plus one slot
                // for each variable it can capture (everything in scope) or bind.
                let slots = u64::try_from(scope_depth + lambda.args.len()).unwrap_or(u64::MAX);
                let closure_bytes = STEEL_CLOSURE_BYTES.saturating_add(STEEL_SCALAR_BYTES.saturating_mul(slots));
                self.text.push('(');
                self.text.push_str(CLOSURE);
                self.text.push(' ');
                self.text.push_str(&closure_bytes.to_string());
                self.text.push_str(" (lambda (");
                for (index, parameter) in lambda.args.iter().enumerate() {
                    let name = binder_name(parameter)?;
                    if self.scope[scope_depth..].contains(&name) {
                        return Err(format!("parameter {name} is bound more than once"));
                    }
                    if index > 0 {
                        self.text.push(' ');
                    }
                    self.push_user_identifier(name);
                    self.scope.push(name);
                }
                self.text.push_str(") (");
                self.text.push_str(FUEL_TICK);
                self.text.push_str(") ");
                let body = self.expression(&lambda.body, next);
                self.scope.truncate(scope_depth);
                body?;
                self.text.push_str("))");
                Ok(())
            }
            Expr::Begin(begin) => {
                if begin.exprs.is_empty() {
                    return Err("begin requires at least one expression".to_string());
                }
                self.text.push_str("(begin");
                for expr in &begin.exprs {
                    self.text.push(' ');
                    self.expression(expr, next)?;
                }
                self.text.push(')');
                Ok(())
            }
            Expr::Quote(quote) => {
                self.text.push_str("(quote ");
                self.datum(&quote.expr, next)?;
                self.text.push(')');
                Ok(())
            }
            Expr::List(list) => {
                if list.improper || list.args.is_empty() {
                    return Err("applications must be proper and non-empty".to_string());
                }
                if list.args.len() > STEEL_MAX_ARITY + 1 {
                    return Err(format!("applications pass at most {STEEL_MAX_ARITY} arguments"));
                }
                self.text.push('(');
                for (index, expr) in list.args.iter().enumerate() {
                    if index > 0 {
                        self.text.push(' ');
                    }
                    self.expression(expr, next)?;
                }
                self.text.push(')');
                Ok(())
            }
            Expr::Define(_) => Err("definitions are admitted only at top level".to_string()),
            Expr::Let(_) => Err("plain-let is not admitted".to_string()),
            Expr::Return(_) => Err("return! is not admitted".to_string()),
            Expr::Macro(_) | Expr::SyntaxRules(_) => Err("macros are not admitted".to_string()),
            Expr::Set(_) => Err("set! is not admitted".to_string()),
            Expr::Require(_) => Err("require is not admitted".to_string()),
            Expr::Vector(_) => Err("vector literals are not admitted".to_string()),
        }
    }

    fn atom(&mut self, token: &Token) -> std::result::Result<(), String> {
        match token {
            Token::Identifier(name) => {
                let name = name.resolve();
                if self.scope.contains(&name) || self.globals.contains(name) {
                    self.push_user_identifier(name);
                } else if let Some((_, private)) = STEEL_PRIMITIVES.iter().find(|(public, _)| *public == name) {
                    self.text.push_str(private);
                } else {
                    return Err(format!("identifier {name} is not available in the Steel sandbox"));
                }
                Ok(())
            }
            literal => self.literal(literal),
        }
    }

    fn literal(&mut self, token: &Token) -> std::result::Result<(), String> {
        match token {
            Token::StringLiteral(text) => {
                self.text.push('"');
                self.text.push_str(&escape_steel_string(text.resolve()));
                self.text.push('"');
            }
            Token::Number(number) => {
                let rendered = number.to_string();
                let integer = rendered
                    .parse::<isize>()
                    .map_err(|_| format!("number {rendered} is not an admitted fixed-width integer"))?;
                self.text.push_str(&integer.to_string());
            }
            Token::BooleanLiteral(value) => self.text.push_str(if *value { "#true" } else { "#false" }),
            other => return Err(format!("token {other} is not admitted")),
        }
        Ok(())
    }

    fn datum(&mut self, expr: &Expr, depth: usize) -> std::result::Result<(), String> {
        if depth > STEEL_MAX_NESTING {
            return Err(format!("quoted data nests deeper than {STEEL_MAX_NESTING}"));
        }
        match expr {
            Expr::Atom(atom) => match &atom.syn.ty {
                Token::Identifier(name) => {
                    let name = name.resolve();
                    validate_identifier(name).map_err(|issue| format!("quoted symbol {issue}"))?;
                    self.text.push_str(name);
                    Ok(())
                }
                literal => self.literal(literal),
            },
            Expr::List(list) if !list.improper => {
                self.text.push('(');
                for (index, item) in list.args.iter().enumerate() {
                    if index > 0 {
                        self.text.push(' ');
                    }
                    self.datum(item, depth + 1)?;
                }
                self.text.push(')');
                Ok(())
            }
            _ => Err("quoted data must be symbols, literals, or proper lists".to_string()),
        }
    }
}

/// The reader fills a missing else branch with a private void primitive; the sandbox requires an
/// explicit branch.
fn is_implicit_else(expr: &Expr) -> bool {
    matches!(expr, Expr::Atom(atom) if matches!(&atom.syn.ty, Token::Identifier(name) if name.resolve().starts_with('#')))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MeterState {
    fuel_remaining: u64,
    allocated_bytes: u64,
    hostcall_count: u64,
    exhaustion: Option<Exhaustion>,
}

/// The deterministic resource meter shared by every primitive of one run.
#[derive(Clone)]
struct Meter {
    state: Shared<Lock<MeterState>>,
    interrupt: Shared<dyn Fn() + Send + Sync>,
}

impl Meter {
    fn exhaust(&self, state: &mut MeterState, exhaustion: Exhaustion) -> SteelError {
        if state.exhaustion.is_none() {
            state.exhaustion = Some(exhaustion);
        }
        // Stop the VM at its next instruction as well, so no script path can continue past the ceiling.
        (self.interrupt)();
        sandbox_error(format!("Steel sandbox {} limit exhausted", exhaustion.as_str()))
    }

    fn charge(&self, fuel: u64, bytes: u64) -> SteelResult<()> {
        let mut state = self.state.lock().map_err(|_| sandbox_error("Steel sandbox meter is unavailable"))?;
        if let Some(exhaustion) = state.exhaustion {
            return Err(self.exhaust(&mut state, exhaustion));
        }
        let Some(fuel_remaining) = state.fuel_remaining.checked_sub(fuel) else {
            state.fuel_remaining = 0;
            return Err(self.exhaust(&mut state, Exhaustion::Fuel));
        };
        let allocated_bytes = state.allocated_bytes.saturating_add(bytes);
        if allocated_bytes > STEEL_MEMORY_LIMIT_BYTES {
            return Err(self.exhaust(&mut state, Exhaustion::Memory));
        }
        state.fuel_remaining = fuel_remaining;
        state.allocated_bytes = allocated_bytes;
        Ok(())
    }

    /// Admits one hostcall before it is dispatched; the call past the limit is refused and never
    /// dispatched.
    // r[impl molten.bounded_execution.steel_sandbox_and_real_bounds]
    fn admit_hostcall(&self) -> SteelResult<()> {
        let mut state = self.state.lock().map_err(|_| sandbox_error("Steel sandbox meter is unavailable"))?;
        if let Some(exhaustion) = state.exhaustion {
            return Err(self.exhaust(&mut state, exhaustion));
        }
        if state.hostcall_count >= STEEL_MAX_HOSTCALLS {
            return Err(self.exhaust(&mut state, Exhaustion::Hostcalls));
        }
        state.hostcall_count += 1;
        Ok(())
    }

    /// Charges a produced string, refusing any string larger than the single-string ceiling.
    fn charge_string(&self, bytes: usize) -> SteelResult<()> {
        if bytes > STEEL_MAX_STRING_BYTES {
            let mut state = self.state.lock().map_err(|_| sandbox_error("Steel sandbox meter is unavailable"))?;
            return Err(self.exhaust(&mut state, Exhaustion::Memory));
        }
        self.charge(1, u64::try_from(bytes).unwrap_or(u64::MAX))
    }

    fn charge_scalar(&self) -> SteelResult<()> {
        self.charge(1, STEEL_SCALAR_BYTES)
    }
}

fn sandbox_error(message: impl Into<String>) -> SteelError {
    SteelError::new(steel::rerrs::ErrorKind::Generic, message.into())
}

fn integer_overflow() -> SteelError {
    sandbox_error("Steel sandbox integer arithmetic overflowed")
}

/// Runs `work` on a dedicated thread whose stack does not depend on the caller's.
fn on_sandbox_thread<T: Send>(work: impl FnOnce() -> T + Send) -> std::result::Result<T, String> {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .name("molten-steel-sandbox".to_string())
            .stack_size(STEEL_VM_STACK_BYTES)
            .spawn_scoped(scope, work)
            .map_err(|error| format!("Steel sandbox thread could not start: {error}"))?
            .join()
            .map_err(|_| "Steel sandbox thread panicked".to_string())
    })
}

/// Runs one actor step on the sandbox thread. The engine, every value it built, and their
/// (recursive) drops all stay on that thread.
// r[impl molten.bounded_execution.steel_sandbox_and_real_bounds]
fn run_vm(prepared: &Prepared, expected_operation: &str, allowed_hostcalls: &[String]) -> SteelRun {
    on_sandbox_thread(|| run_vm_on_current_thread(prepared, expected_operation, allowed_hostcalls)).unwrap_or_else(
        |detail| SteelRun {
            outcome: Err(detail),
            meter: MeterState {
                fuel_remaining: 0,
                allocated_bytes: 0,
                hostcall_count: 0,
                exhaustion: None,
            },
        },
    )
}

fn run_vm_on_current_thread(prepared: &Prepared, expected_operation: &str, allowed_hostcalls: &[String]) -> SteelRun {
    // A raw engine registers no prelude, modules, or ambient primitives; the sandbox table below is all
    // it has.
    let mut engine = Engine::new_raw_no_kernel();
    let controller = engine.get_thread_state_controller();
    let meter = Meter {
        state: Shared::new(Lock::new(MeterState {
            fuel_remaining: STEEL_FUEL_LIMIT,
            allocated_bytes: 0,
            hostcall_count: 0,
            exhaustion: None,
        })),
        interrupt: Shared::new(move || controller.interrupt()),
    };
    register_sandbox(&mut engine, &meter, expected_operation, allowed_hostcalls);
    let outcome = meter
        .charge(0, prepared.input_text.len() as u64)
        .and_then(|()| engine.run(prepared.program.clone()))
        .and_then(|_| prepared.input_text.clone().into_steelval())
        .and_then(|input| engine.call_function_by_name_with_args(&prepared.entry, vec![input]))
        .map(|value| value.to_string())
        .map_err(|error| error.to_string());
    match meter.state.lock() {
        Ok(state) => SteelRun { outcome, meter: *state },
        Err(_) => SteelRun {
            outcome: Err("Steel sandbox meter is unavailable".to_string()),
            meter: MeterState {
                fuel_remaining: 0,
                allocated_bytes: 0,
                hostcall_count: 0,
                exhaustion: None,
            },
        },
    }
}

fn register_sandbox(engine: &mut Engine, meter: &Meter, expected_operation: &str, allowed_hostcalls: &[String]) {
    let tick = meter.clone();
    engine.register_fn(FUEL_TICK, move || -> SteelResult<bool> { tick.charge(1, 0).map(|()| true) });

    let closure = meter.clone();
    engine.register_fn(CLOSURE, move |bytes: isize, value: SteelValue| -> SteelResult<SteelValue> {
        closure.charge(1, u64::try_from(bytes).unwrap_or(u64::MAX))?;
        Ok(value)
    });

    let hostcall = meter.clone();
    let expected_operation = expected_operation.to_string();
    let allowed_hostcalls = allowed_hostcalls.to_vec();
    engine.register_fn(PRIM_HOSTCALL, move |operation: String, envelope: String| -> SteelResult<String> {
        hostcall.admit_hostcall()?;
        let response = if operation == expected_operation && allowed_hostcalls.contains(&operation) {
            format!("<steel-hostcall-response \"pass\" \"{}\">", escape_steel_string(&envelope))
        } else {
            format!("<steel-hostcall-response \"deny\" \"{}\">", escape_steel_string(&operation))
        };
        hostcall.charge_string(response.len())?;
        Ok(response)
    });

    let append = meter.clone();
    engine.register_fn(PRIM_STRING_APPEND, move |left: String, right: String| -> SteelResult<String> {
        append.charge_string(left.len().saturating_add(right.len()))?;
        Ok(left + &right)
    });
    let length = meter.clone();
    engine.register_fn(PRIM_STRING_LENGTH, move |text: String| -> SteelResult<isize> {
        length.charge_scalar()?;
        isize::try_from(text.chars().count()).map_err(|_| integer_overflow())
    });
    let slice = meter.clone();
    engine.register_fn(PRIM_SUBSTRING, move |text: String, start: isize, end: isize| -> SteelResult<String> {
        let (Ok(start), Ok(end)) = (usize::try_from(start), usize::try_from(end)) else {
            return Err(sandbox_error("substring bounds must be non-negative"));
        };
        if start > end || end > text.chars().count() {
            return Err(sandbox_error("substring bounds are outside the string"));
        }
        let result = text.chars().skip(start).take(end - start).collect::<String>();
        slice.charge_string(result.len())?;
        Ok(result)
    });
    let same_text = meter.clone();
    engine.register_fn(PRIM_STRING_EQUAL, move |left: String, right: String| -> SteelResult<bool> {
        same_text.charge_scalar()?;
        Ok(left == right)
    });
    let equal = meter.clone();
    engine.register_fn(PRIM_EQUAL, move |left: SteelValue, right: SteelValue| -> SteelResult<bool> {
        equal.charge_scalar()?;
        Ok(left == right)
    });
    let negate = meter.clone();
    engine.register_fn(PRIM_NOT, move |value: SteelValue| -> SteelResult<bool> {
        negate.charge_scalar()?;
        Ok(matches!(value, SteelValue::BoolV(false)))
    });
    register_integer(engine, meter, PRIM_ADD, isize::checked_add);
    register_integer(engine, meter, PRIM_SUBTRACT, isize::checked_sub);
    register_integer(engine, meter, PRIM_MULTIPLY, isize::checked_mul);
    let equal_integer = meter.clone();
    engine.register_fn(PRIM_INTEGER_EQUAL, move |left: isize, right: isize| -> SteelResult<bool> {
        equal_integer.charge_scalar()?;
        Ok(left == right)
    });
    let less = meter.clone();
    engine.register_fn(PRIM_LESS, move |left: isize, right: isize| -> SteelResult<bool> {
        less.charge_scalar()?;
        Ok(left < right)
    });
}

fn register_integer(
    engine: &mut Engine,
    meter: &Meter,
    name: &'static str,
    operation: fn(isize, isize) -> Option<isize>,
) {
    let meter = meter.clone();
    engine.register_fn(name, move |left: isize, right: isize| -> SteelResult<isize> {
        meter.charge_scalar()?;
        operation(left, right).ok_or_else(integer_overflow)
    });
}

fn finish_value(input: FinishInput<'_>) -> Result<PreservesValue> {
    let output_value = string(&input.execution.output_text);
    let input_ref = canonical_hash(input.actor_input)?;
    let output_ref = canonical_hash(&output_value)?;
    let hostcalls = vec![input.operation.to_string()];

    Ok(receipt_value(RunReceipt {
        actor_id: input.actor_id,
        source_ref: input.source_ref,
        callable: input.callable,
        operation: input.operation,
        input_ref: &input_ref,
        output_ref: &output_ref,
        hostcalls: &hostcalls,
        resource_limits: ResourceReceipt {
            fuel_limit: STEEL_FUEL_LIMIT,
            fuel_remaining: input.execution.fuel_remaining,
            source_bytes: input.source_bytes,
            input_bytes: input.input_text.len() as u64,
            output_bytes: input.execution.output_text.len() as u64,
            hostcall_limit: STEEL_MAX_HOSTCALLS,
            hostcall_count: input.execution.hostcall_count,
        },
    }))
}

fn escape_steel_string(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for character in input.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push(other),
        }
    }
    escaped
}
