use steel::steel_vm::register_fn::RegisterFn;

type Engine = steel::steel_vm::engine::Engine;
type Shared<T> = std::sync::Arc<T>;
type Counter = std::sync::atomic::AtomicU64;
type MemoryOrder = std::sync::atomic::Ordering;
type PreservesValue = preserves::IOValue;
type Request = super::core::AdmissionRequest;
type Step = super::core::CoreStep;
type ActorConfig = super::schema::ActorExecutorConfig;
type ActorMode = super::schema::ActorKind;
type Suite = super::schema::Suite;
type RunReceipt<'a> = super::schema::SteelExecutionReceiptInput<'a>;
type ResourceReceipt = super::schema::SteelResourceReceiptInput;
type SourceConfig = super::schema::SteelExecutorConfig;
type MoltenError = crate::error::MoltenError;
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

const STEEL_FUEL_LIMIT: u64 = 16 * 1024;
const STEEL_MAX_SOURCE_BYTES: usize = 8 * 1024;
const STEEL_MAX_INPUT_BYTES: usize = 8 * 1024;
const STEEL_MAX_OUTPUT_BYTES: usize = 8 * 1024;
const STEEL_MAX_HOSTCALLS: u64 = 8;
const STEEL_MAX_DEFINED_FUNCTIONS: usize = 128;

const _: () = assert!(STEEL_MAX_DEFINED_FUNCTIONS <= 1_024);

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
    let execution = run_vm(actor_id, &config.callable, prepared.script, &operation, &config.allowed_hostcalls)?;
    let source_ref = source_ref(config)?;

    Ok(Some(finish_value(FinishInput {
        actor_id,
        source_ref: &source_ref,
        callable: &config.callable,
        source_bytes: config.source.len() as u64,
        operation: &operation,
        actor_input,
        input_text: &prepared.input_text,
        output_text: &execution.output_text,
        estimated_fuel: prepared.estimated_fuel,
        hostcall_count: execution.hostcall_count,
    })?))
}

struct Prepared {
    input_text: String,
    script: String,
    estimated_fuel: u64,
}

struct Execution {
    output_text: String,
    hostcall_count: u64,
}

struct FinishInput<'a> {
    actor_id: &'a str,
    source_ref: &'a str,
    callable: &'a str,
    source_bytes: u64,
    operation: &'a str,
    actor_input: &'a PreservesValue,
    input_text: &'a str,
    output_text: &'a str,
    estimated_fuel: u64,
    hostcall_count: u64,
}

fn prepare_run(actor_id: &str, source: &str, callable: &str, actor_input: &PreservesValue) -> Result<Prepared> {
    if source.len() > STEEL_MAX_SOURCE_BYTES {
        return Err(MoltenError::invalid_harness(format!(
            "Steel executor source for actor {actor_id} exceeds deterministic resource limit"
        )));
    }
    validate_steel_resource_shape(actor_id, callable, source)?;
    let input_text = to_text(actor_input)?;
    if input_text.len() > STEEL_MAX_INPUT_BYTES {
        return Err(MoltenError::invalid_harness(format!(
            "Steel executor input for actor {actor_id} exceeds deterministic resource limit"
        )));
    }
    let script = format!("{}\n({} \"{}\")", source, callable, escape_steel_string(&input_text));
    let estimated_fuel = estimate_steel_fuel(source, &input_text);
    if estimated_fuel > STEEL_FUEL_LIMIT {
        return Err(MoltenError::invalid_harness(format!(
            "Steel executor estimated fuel for actor {actor_id} exceeds deterministic resource limit"
        )));
    }
    Ok(Prepared {
        input_text,
        script,
        estimated_fuel,
    })
}

fn run_vm(
    actor_id: &str,
    callable: &str,
    script: String,
    expected_operation: &str,
    allowed_hostcalls: &[String],
) -> Result<Execution> {
    let expected_operation = expected_operation.to_string();
    let allowed_hostcalls = allowed_hostcalls.to_vec();
    let hostcall_counter = Shared::new(Counter::new(0));
    let hostcall_counter_for_vm = Shared::clone(&hostcall_counter);
    let mut engine = Engine::new();
    engine.register_fn("molten-hostcall", move |operation: String, envelope: String| -> String {
        hostcall_counter_for_vm.fetch_add(1, MemoryOrder::SeqCst);
        if operation == expected_operation && allowed_hostcalls.iter().any(|allowed| allowed == &operation) {
            format!("<steel-hostcall-response \"pass\" \"{}\">", escape_steel_string(&envelope))
        } else {
            format!("<steel-hostcall-response \"deny\" \"{}\">", escape_steel_string(&operation))
        }
    });
    let values = engine.run(script).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "Steel executor actor {actor_id} callable {callable} failed in reviewed VM: {error}"
        ))
    })?;
    let output_text = values.last().map_or_else(|| "#<void>".to_string(), ToString::to_string);
    if output_text.len() > STEEL_MAX_OUTPUT_BYTES {
        return Err(MoltenError::invalid_harness(format!(
            "Steel executor output for actor {actor_id} exceeds deterministic resource limit"
        )));
    }
    let hostcall_count = hostcall_counter.load(MemoryOrder::SeqCst);
    if hostcall_count > STEEL_MAX_HOSTCALLS {
        return Err(MoltenError::invalid_harness(format!(
            "Steel executor hostcall count for actor {actor_id} exceeds deterministic resource limit"
        )));
    }
    Ok(Execution {
        output_text,
        hostcall_count,
    })
}

fn finish_value(input: FinishInput<'_>) -> Result<PreservesValue> {
    let output_value = string(input.output_text);
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
            fuel_remaining: STEEL_FUEL_LIMIT.saturating_sub(input.estimated_fuel),
            source_bytes: input.source_bytes,
            input_bytes: input.input_text.len() as u64,
            output_bytes: input.output_text.len() as u64,
            hostcall_limit: STEEL_MAX_HOSTCALLS,
            hostcall_count: input.hostcall_count,
        },
    }))
}

fn validate_steel_resource_shape(actor_id: &str, callable: &str, source: &str) -> Result<()> {
    for token in FORBIDDEN_STEEL_RESOURCE_TOKENS {
        if source.contains(token) {
            return Err(MoltenError::invalid_harness(format!(
                "Steel executor source for actor {actor_id} references unbounded resource token {token}; reviewed Steel resource gate remains fail-closed"
            )));
        }
    }
    for function in steel_defined_functions(source)? {
        let self_call = format!("({function}");
        if source.matches(&self_call).count() > 1 {
            return Err(MoltenError::invalid_harness(format!(
                "Steel executor source for actor {actor_id} references recursive callable {function}; reviewed Steel resource gate remains fail-closed"
            )));
        }
    }
    let self_call = format!("({callable}");
    if source.matches(&self_call).count() > 1 {
        return Err(MoltenError::invalid_harness(format!(
            "Steel executor source for actor {actor_id} references recursive callable {callable}; reviewed Steel resource gate remains fail-closed"
        )));
    }
    Ok(())
}

fn steel_defined_functions(source: &str) -> Result<Vec<&str>> {
    let mut functions = Vec::with_capacity(STEEL_MAX_DEFINED_FUNCTIONS);
    let mut remainder = source;
    while let Some(position) = remainder.find("(define (") {
        let after_prefix = &remainder[position + "(define (".len()..];
        let end = after_prefix
            .find(|character: char| character.is_whitespace() || character == ')' || character == '(')
            .unwrap_or(after_prefix.len());
        if end > 0 {
            if functions.len() >= STEEL_MAX_DEFINED_FUNCTIONS {
                return Err(MoltenError::invalid_harness(format!(
                    "Steel executor source defines more than {STEEL_MAX_DEFINED_FUNCTIONS} functions"
                )));
            }
            functions.push(&after_prefix[..end]);
        }
        remainder = &after_prefix[end..];
    }
    Ok(functions)
}

const FORBIDDEN_STEEL_RESOURCE_TOKENS: &[&str] = &[
    "letrec",
    "let loop",
    "named-let",
    "do ",
    "while",
    "for-each",
    "stream",
    "delay",
    "force",
    "call/cc",
    "make-string",
    "make-vector",
    "vector-grow",
    "range",
];

fn estimate_steel_fuel(source: &str, input: &str) -> u64 {
    let structural_tokens = source.bytes().filter(|byte| matches!(byte, b'(' | b')' | b' ' | b'\n' | b'\t')).count();
    source.len() as u64 + input.len() as u64 + structural_tokens as u64
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
