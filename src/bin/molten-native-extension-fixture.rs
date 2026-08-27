use std::io::Read;
use std::io::Write;

use molten::fabric::FabricPortKey;
use molten::system_extension::CallbackKind;
use molten::system_extension::EffectTarget;
use molten::system_extension::HealthState;
use molten::system_extension::NativeCallbackValue;
use molten::system_extension::NativeMaterializedCallbackOutcome;
use molten::system_extension::NativeMaterializedEffectRequest;
use molten::system_extension::TypedEffectRequest;
use molten::system_extension::decode_native_callback_envelope;
use molten::system_extension::encode_native_callback_outcome;

const MAX_CALLBACK_BYTES: u64 = 1_048_576;
const MAX_VALUE_BYTES: u64 = 262_144;
const MAX_CALLBACK_ITEMS: u64 = 64;
const READ_LIMIT_BYTES: u64 = MAX_CALLBACK_BYTES + 1;
const EFFECT_PORT_ID: &str = "molten.fixture.native.effect";
const EFFECT_PORT_VERSION: &str = "v1";
const EFFECT_OPERATION: &str = "fixture-effect";
const EFFECT_INPUT_SCHEMA: &str = "molten.fixture.native.effect-input.v1";
const EFFECT_OUTPUT_SCHEMA: &str = "molten.fixture.native.effect-output.v1";

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
    let envelope = decode_native_callback_envelope(&bytes, MAX_CALLBACK_BYTES, MAX_VALUE_BYTES, MAX_CALLBACK_ITEMS)
        .map_err(|error| error.to_string())?;
    let output_bytes = envelope
        .inputs
        .payload
        .as_ref()
        .map_or_else(|| envelope.invocation.event_ref.as_bytes().to_vec(), |value| value.bytes.clone());
    let output = value(output_bytes);
    let prior_state = envelope.inputs.state.as_ref().map_or(&[][..], |value| value.bytes.as_slice());
    let state_bytes = [
        envelope.context.instance_id.as_bytes(),
        &envelope.invocation.generation.to_le_bytes(),
        &envelope.invocation.sequence.to_le_bytes(),
        prior_state,
    ]
    .concat();
    let state = value(state_bytes);
    let checkpoint = (envelope.invocation.callback == CallbackKind::Checkpoint).then(|| state.clone());
    let effects = if envelope.invocation.callback == CallbackKind::Request {
        let request = value(format!("effect\0{}", envelope.invocation.event_ref).into_bytes());
        let accounted_bytes =
            u64::try_from(request.bytes.len()).map_err(|_| "effect request byte count does not fit u64".to_string())?;
        vec![NativeMaterializedEffectRequest {
            effect: TypedEffectRequest {
                target: EffectTarget::FabricPort(FabricPortKey {
                    port_id: EFFECT_PORT_ID.to_string(),
                    version: EFFECT_PORT_VERSION.to_string(),
                }),
                operation: EFFECT_OPERATION.to_string(),
                input_schema_ref: EFFECT_INPUT_SCHEMA.to_string(),
                output_schema_ref: EFFECT_OUTPUT_SCHEMA.to_string(),
                request_ref: request.value_ref.clone(),
                generation: envelope.invocation.generation,
                accounted_bytes,
            },
            request,
        }]
    } else {
        Vec::new()
    };
    let output = encode_native_callback_outcome(&NativeMaterializedCallbackOutcome {
        outputs: vec![output],
        effects,
        state: Some(state),
        checkpoint,
        health: HealthState::Healthy,
    })
    .map_err(|error| error.to_string())?;
    std::io::stdout().write_all(&output).map_err(|error| format!("write callback output: {error}"))?;
    std::io::stdout().flush().map_err(|error| format!("flush callback output: {error}"))
}

fn value(bytes: Vec<u8>) -> NativeCallbackValue {
    NativeCallbackValue {
        value_ref: molten::preserves_rail::content_ref_from_bytes(&bytes),
        bytes,
    }
}
