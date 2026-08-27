use std::io::Read;
use std::io::Write;

use molten::fabric::FabricPortKey;
use molten::system_extension::CallbackKind;
use molten::system_extension::CallbackOutcome;
use molten::system_extension::EffectTarget;
use molten::system_extension::HealthState;
use molten::system_extension::TypedEffectRequest;
use molten::system_extension::decode_native_callback_envelope;
use molten::system_extension::encode_native_callback_outcome;

const MAX_CALLBACK_BYTES: u64 = 1_048_576;
const MAX_CALLBACK_ITEMS: usize = 64;
const READ_LIMIT_BYTES: u64 = MAX_CALLBACK_BYTES + 1;
const EFFECT_PORT_ID: &str = "molten.fixture.native.effect";
const EFFECT_PORT_VERSION: &str = "v1";
const EFFECT_OPERATION: &str = "fixture-effect";
const EFFECT_INPUT_SCHEMA: &str = "molten.fixture.native.effect-input.v1";
const EFFECT_OUTPUT_SCHEMA: &str = "molten.fixture.native.effect-output.v1";
const EFFECT_ACCOUNTED_BYTES: u64 = 64;

fn main() {
    if let Err(error) = run() {
        let _ = writeln!(std::io::stderr(), "native extension fixture failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    if std::env::var_os("HOME").is_some() {
        return Err("inherited HOME is not admitted".to_string());
    }
    let mut bytes = Vec::new();
    std::io::stdin()
        .take(READ_LIMIT_BYTES)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("read callback input: {error}"))?;
    let byte_count = u64::try_from(bytes.len()).map_err(|_| "callback input length does not fit u64".to_string())?;
    if byte_count > MAX_CALLBACK_BYTES {
        return Err("callback input exceeds fixture bound".to_string());
    }
    let envelope = decode_native_callback_envelope(&bytes, MAX_CALLBACK_BYTES, MAX_CALLBACK_ITEMS)
        .map_err(|error| error.to_string())?;
    let output_ref = molten::preserves_rail::content_ref_from_bytes(envelope.invocation.event_ref.as_bytes());
    let state_ref = molten::preserves_rail::content_ref_from_bytes(
        format!(
            "{}\0{}\0{}",
            envelope.context.instance_id, envelope.invocation.generation, envelope.invocation.sequence
        )
        .as_bytes(),
    );
    let checkpoint_ref = (envelope.invocation.callback == CallbackKind::Checkpoint).then(|| {
        molten::preserves_rail::content_ref_from_bytes(
            format!("checkpoint\0{}", envelope.invocation.event_ref).as_bytes(),
        )
    });
    let effects = if envelope.invocation.callback == CallbackKind::Request {
        vec![TypedEffectRequest {
            target: EffectTarget::FabricPort(FabricPortKey {
                port_id: EFFECT_PORT_ID.to_string(),
                version: EFFECT_PORT_VERSION.to_string(),
            }),
            operation: EFFECT_OPERATION.to_string(),
            input_schema_ref: EFFECT_INPUT_SCHEMA.to_string(),
            output_schema_ref: EFFECT_OUTPUT_SCHEMA.to_string(),
            request_ref: molten::preserves_rail::content_ref_from_bytes(
                format!("effect\0{}", envelope.invocation.event_ref).as_bytes(),
            ),
            generation: envelope.invocation.generation,
            accounted_bytes: EFFECT_ACCOUNTED_BYTES,
        }]
    } else {
        Vec::new()
    };
    let output = encode_native_callback_outcome(&CallbackOutcome {
        output_refs: vec![output_ref],
        effects,
        state_ref: Some(state_ref),
        checkpoint_ref,
        health: HealthState::Healthy,
    })
    .map_err(|error| error.to_string())?;
    std::io::stdout().write_all(&output).map_err(|error| format!("write callback output: {error}"))?;
    std::io::stdout().flush().map_err(|error| format!("flush callback output: {error}"))
}
