use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use preserves::IOValue;
use steel::steel_vm::engine::Engine;
use steel::steel_vm::register_fn::RegisterFn;

use super::core::AdmissionRequest;
use super::core::CoreStep;
use super::schema::ActorExecutorConfig;
use super::schema::ActorKind;
use super::schema::HarnessSuite;
use super::schema::SteelExecutionReceiptInput;
use super::schema::SteelResourceReceiptInput;
use super::schema::steel_execution_receipt_value;
use super::schema::steel_source_ref;
use super::schema::validate_hostcall_effect_binding_request;
use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::string;
use crate::preserves_rail::to_text;

const STEEL_FUEL_LIMIT: u64 = 16 * 1024;
const STEEL_MAX_SOURCE_BYTES: usize = 8 * 1024;
const STEEL_MAX_INPUT_BYTES: usize = 8 * 1024;
const STEEL_MAX_OUTPUT_BYTES: usize = 8 * 1024;
const STEEL_MAX_HOSTCALLS: u64 = 8;
const STEEL_MAX_DEFINED_FUNCTIONS: usize = 128;

const _: () = assert!(STEEL_MAX_DEFINED_FUNCTIONS <= 1_024);

pub fn execute_steel_actor_step(
    suite: &HarnessSuite,
    step: &CoreStep,
    actor_input: &IOValue,
    hostcall_request: &IOValue,
) -> Result<Option<IOValue>> {
    let actor_id = step.primary_actor();
    let Some(actor) = suite.actors.iter().find(|actor| actor.id == actor_id) else {
        return Err(MoltenError::invalid_harness(format!("actor {actor_id} missing from executor registry")));
    };
    if actor.kind != ActorKind::Steel {
        return Ok(None);
    }
    let Some(ActorExecutorConfig::Steel(config)) = actor.executor.as_ref() else {
        return Err(MoltenError::invalid_harness(format!(
            "steel actor {actor_id} missing reviewed Steel executor preflight fixture"
        )));
    };

    if config.source.len() > STEEL_MAX_SOURCE_BYTES {
        return Err(MoltenError::invalid_harness(format!(
            "Steel executor source for actor {actor_id} exceeds deterministic resource limit"
        )));
    }
    validate_steel_resource_shape(actor_id, &config.callable, &config.source)?;
    let input_text = to_text(actor_input)?;
    if input_text.len() > STEEL_MAX_INPUT_BYTES {
        return Err(MoltenError::invalid_harness(format!(
            "Steel executor input for actor {actor_id} exceeds deterministic resource limit"
        )));
    }
    let script = format!("{}\n({} \"{}\")", config.source, config.callable, escape_steel_string(&input_text));
    let estimated_fuel = estimate_steel_fuel(&config.source, &input_text);
    if estimated_fuel > STEEL_FUEL_LIMIT {
        return Err(MoltenError::invalid_harness(format!(
            "Steel executor estimated fuel for actor {actor_id} exceeds deterministic resource limit"
        )));
    }
    let allowed_hostcalls = config.allowed_hostcalls.clone();
    let expected_operation = AdmissionRequest::from_step(step).action.as_str().to_string();
    validate_hostcall_effect_binding_request(hostcall_request, &expected_operation)?;
    let hostcall_counter = Arc::new(AtomicU64::new(0));
    let hostcall_counter_for_vm = Arc::clone(&hostcall_counter);
    let mut engine = Engine::new();
    engine.register_fn("molten-hostcall", move |operation: String, envelope: String| -> String {
        hostcall_counter_for_vm.fetch_add(1, Ordering::SeqCst);
        if operation == expected_operation && allowed_hostcalls.iter().any(|allowed| allowed == &operation) {
            format!("<steel-hostcall-response \"pass\" \"{}\">", escape_steel_string(&envelope))
        } else {
            format!("<steel-hostcall-response \"deny\" \"{}\">", escape_steel_string(&operation))
        }
    });
    let values = engine.run(script).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "Steel executor actor {actor_id} callable {} failed in reviewed VM: {error}",
            config.callable
        ))
    })?;
    let output_text = values.last().map_or_else(|| "#<void>".to_string(), ToString::to_string);
    if output_text.len() > STEEL_MAX_OUTPUT_BYTES {
        return Err(MoltenError::invalid_harness(format!(
            "Steel executor output for actor {actor_id} exceeds deterministic resource limit"
        )));
    }
    let hostcall_count = hostcall_counter.load(Ordering::SeqCst);
    if hostcall_count > STEEL_MAX_HOSTCALLS {
        return Err(MoltenError::invalid_harness(format!(
            "Steel executor hostcall count for actor {actor_id} exceeds deterministic resource limit"
        )));
    }
    let output_value = string(&output_text);
    let input_ref = canonical_hash(actor_input)?;
    let output_ref = canonical_hash(&output_value)?;
    let source_ref = steel_source_ref(config)?;
    let operation = AdmissionRequest::from_step(step).action.as_str().to_string();
    let hostcalls = vec![operation.clone()];

    Ok(Some(steel_execution_receipt_value(SteelExecutionReceiptInput {
        actor_id,
        source_ref: &source_ref,
        callable: &config.callable,
        operation: &operation,
        input_ref: &input_ref,
        output_ref: &output_ref,
        hostcalls: &hostcalls,
        resource_limits: SteelResourceReceiptInput {
            fuel_limit: STEEL_FUEL_LIMIT,
            fuel_remaining: STEEL_FUEL_LIMIT.saturating_sub(estimated_fuel),
            source_bytes: config.source.len() as u64,
            input_bytes: input_text.len() as u64,
            output_bytes: output_text.len() as u64,
            hostcall_limit: STEEL_MAX_HOSTCALLS,
            hostcall_count,
        },
    })))
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
