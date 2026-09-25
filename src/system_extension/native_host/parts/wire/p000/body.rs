
const CALLBACK_ENVELOPE_RECORD: &str = "native-callback-envelope-v2";
const CALLBACK_OUTCOME_RECORD: &str = "native-callback-outcome-v2";
const EFFECT_RECORD: &str = "native-callback-effect-v2";
const PORT_TARGET_RECORD: &str = "native-callback-port-target-v2";
const VALUE_RECORD: &str = "native-callback-value-v2";
const NONE_RECORD: &str = "none";
const SOME_RECORD: &str = "some";
const ENVELOPE_FIELD_COUNT: u64 = 18;
const OUTCOME_FIELD_COUNT: u64 = 6;
const EFFECT_FIELD_COUNT: u64 = 8;
const PORT_TARGET_FIELD_COUNT: u64 = 2;
const VALUE_FIELD_COUNT: u64 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeCallbackContext {
    pub manifest_ref: String,
    pub executable_ref: String,
    pub instance_id: String,
    pub extension_id: String,
    pub service_id: String,
    pub state_ref: Option<String>,
    pub policy_refs: Vec<String>,
    pub resource_ref: String,
    pub port_binding_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeCallbackInputs {
    pub payload: Option<NativeCallbackValue>,
    pub state: Option<NativeCallbackValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeMaterializedEffectRequest {
    pub effect: TypedEffectRequest,
    pub request: NativeCallbackValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeMaterializedCallbackOutcome {
    pub outputs: Vec<NativeCallbackValue>,
    pub effects: Vec<NativeMaterializedEffectRequest>,
    pub state: Option<NativeCallbackValue>,
    pub checkpoint: Option<NativeCallbackValue>,
    pub health: HealthState,
}

impl NativeMaterializedCallbackOutcome {
    pub fn project(&self) -> CallbackOutcome {
        CallbackOutcome {
            output_refs: self.outputs.iter().map(|value| value.value_ref.clone()).collect(),
            effects: self.effects.iter().map(|effect| effect.effect.clone()).collect(),
            state_ref: self.state.as_ref().map(|value| value.value_ref.clone()),
            checkpoint_ref: self.checkpoint.as_ref().map(|value| value.value_ref.clone()),
            health: self.health,
        }
    }

    pub fn values(&self) -> impl Iterator<Item = &NativeCallbackValue> {
        self.outputs
            .iter()
            .chain(self.effects.iter().map(|effect| &effect.request))
            .chain(self.state.iter())
            .chain(self.checkpoint.iter())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalNativeCallbackEnvelope {
    pub envelope_ref: String,
    pub invocation: CallbackInvocation,
    pub context: NativeCallbackContext,
    pub inputs: NativeCallbackInputs,
    pub value: preserves::IOValue,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedNativeCallbackEnvelope {
    pub invocation: CallbackInvocation,
    pub context: NativeCallbackContext,
    pub inputs: NativeCallbackInputs,
}

// r[impl molten.system_extension.native_host.value_protocol]
// r[impl molten.system_extension.native_host.value_materialization]
pub fn canonical_native_callback_envelope(
    context: &NativeCallbackContext,
    invocation: &CallbackInvocation,
    inputs: &NativeCallbackInputs,
) -> crate::error::Result<CanonicalNativeCallbackEnvelope> {
    require_input_links(context, invocation, inputs)?;
    let value = crate::preserves_rail::record(CALLBACK_ENVELOPE_RECORD, vec![
        crate::preserves_rail::string(NATIVE_CALLBACK_ENVELOPE_SCHEMA),
        crate::preserves_rail::string(&context.manifest_ref),
        crate::preserves_rail::string(&context.executable_ref),
        crate::preserves_rail::string(&context.instance_id),
        crate::preserves_rail::string(&context.extension_id),
        crate::preserves_rail::string(&context.service_id),
        crate::preserves_rail::u64_value(invocation.generation),
        crate::preserves_rail::string(invocation.callback.as_str()),
        crate::preserves_rail::u64_value(invocation.sequence),
        crate::preserves_rail::string(&invocation.event_ref),
        optional_value(inputs.payload.as_ref()),
        crate::preserves_rail::u64_value(invocation.logical_tick),
        crate::preserves_rail::u64_value(invocation.deadline_tick),
        optional_value(inputs.state.as_ref()),
        ref_sequence(&context.policy_refs),
        crate::preserves_rail::string(&context.resource_ref),
        ref_sequence(&context.port_binding_refs),
        crate::preserves_rail::string(NATIVE_FRAMING),
    ]);
    let envelope_ref = crate::preserves_rail::canonical_hash(&value)?;
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    Ok(CanonicalNativeCallbackEnvelope {
        envelope_ref,
        invocation: invocation.clone(),
        context: context.clone(),
        inputs: inputs.clone(),
        value,
        bytes,
    })
}

// r[impl molten.system_extension.native_host.value_protocol]
// r[impl molten.system_extension.native_host.value_materialization]
pub fn decode_native_callback_envelope(
    bytes: &[u8],
    maximum_bytes: u64,
    maximum_value_bytes: u64,
    maximum_items: u64,
) -> crate::error::Result<DecodedNativeCallbackEnvelope> {
    require_byte_bound(bytes, maximum_bytes, "native callback input")?;
    let decoded = crate::preserves_rail::strict_canonical_decode(bytes)?;
    let fields =
        crate::preserves_rail::simple_record_fields(&decoded.value, CALLBACK_ENVELOPE_RECORD, ENVELOPE_FIELD_COUNT)?;
    let schema = crate::preserves_rail::required_string_field(&fields[0], "callback envelope schema")?;
    if schema != NATIVE_CALLBACK_ENVELOPE_SCHEMA {
        return Err(crate::error::MoltenError::invalid_harness("native callback envelope schema mismatch"));
    }
    let callback_name = crate::preserves_rail::required_string_field(&fields[7], "callback kind")?;
    let callback = CallbackKind::parse(&callback_name)
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("native callback kind is unsupported"))?;
    let framing = crate::preserves_rail::required_string_field(&fields[17], "callback framing")?;
    if framing != NATIVE_FRAMING {
        return Err(crate::error::MoltenError::invalid_harness("native callback framing mismatch"));
    }
    let payload = parse_optional_value(&fields[10], "callback payload", maximum_value_bytes)?;
    let state = parse_optional_value(&fields[13], "callback state", maximum_value_bytes)?;
    require_item_bound(payload.iter().count() + state.iter().count(), maximum_items, "callback input values")?;
    let invocation = CallbackInvocation {
        callback,
        generation: required_u64(&fields[6], "callback generation")?,
        sequence: required_u64(&fields[8], "callback sequence")?,
        event_ref: crate::preserves_rail::required_content_ref_string(&fields[9], "callback event ref")?,
        payload_ref: payload.as_ref().map(|value| value.value_ref.clone()),
        logical_tick: required_u64(&fields[11], "callback logical tick")?,
        deadline_tick: required_u64(&fields[12], "callback deadline tick")?,
    };
    let context = NativeCallbackContext {
        manifest_ref: crate::preserves_rail::required_content_ref_string(&fields[1], "callback manifest ref")?,
        executable_ref: crate::preserves_rail::required_content_ref_string(&fields[2], "callback executable ref")?,
        instance_id: crate::preserves_rail::required_string_field(&fields[3], "callback instance id")?,
        extension_id: crate::preserves_rail::required_string_field(&fields[4], "callback extension id")?,
        service_id: crate::preserves_rail::required_string_field(&fields[5], "callback service id")?,
        state_ref: state.as_ref().map(|value| value.value_ref.clone()),
        policy_refs: parse_ref_sequence(&fields[14], "callback policy refs", maximum_items)?,
        resource_ref: crate::preserves_rail::required_content_ref_string(&fields[15], "callback resource ref")?,
        port_binding_refs: parse_ref_sequence(&fields[16], "callback port refs", maximum_items)?,
    };
    let inputs = NativeCallbackInputs { payload, state };
    require_input_links(&context, &invocation, &inputs)?;
    Ok(DecodedNativeCallbackEnvelope {
        invocation,
        context,
        inputs,
    })
}

// r[impl molten.system_extension.native_host.value_protocol]
// r[impl molten.system_extension.native_host.value_publication]
pub fn encode_native_callback_outcome(outcome: &NativeMaterializedCallbackOutcome) -> crate::error::Result<Vec<u8>> {
    for value in outcome.values() {
        super::admit_native_callback_value(value, u64::MAX).map_err(value_error)?;
    }
    let value = crate::preserves_rail::record(CALLBACK_OUTCOME_RECORD, vec![
        crate::preserves_rail::string(NATIVE_CALLBACK_OUTCOME_SCHEMA),
        crate::preserves_rail::sequence(outcome.outputs.iter().map(value_value).collect()),
        crate::preserves_rail::sequence(outcome.effects.iter().map(effect_value).collect()),
        optional_value(outcome.state.as_ref()),
        optional_value(outcome.checkpoint.as_ref()),
        crate::preserves_rail::string(outcome.health.as_str()),
    ]);
    crate::preserves_rail::canonical_bytes(&value)
}

// r[impl molten.system_extension.native_host.value_protocol]
// r[impl molten.system_extension.native_host.value_publication]
pub fn decode_native_callback_outcome(
    bytes: &[u8],
    maximum_bytes: u64,
    maximum_value_bytes: u64,
    maximum_items: u64,
) -> crate::error::Result<NativeMaterializedCallbackOutcome> {
    require_byte_bound(bytes, maximum_bytes, "native callback output")?;
    let decoded = crate::preserves_rail::strict_canonical_decode(bytes)?;
    let fields =
        crate::preserves_rail::simple_record_fields(&decoded.value, CALLBACK_OUTCOME_RECORD, OUTCOME_FIELD_COUNT)?;
    let schema = crate::preserves_rail::required_string_field(&fields[0], "callback outcome schema")?;
    if schema != NATIVE_CALLBACK_OUTCOME_SCHEMA {
        return Err(crate::error::MoltenError::invalid_harness("native callback outcome schema mismatch"));
    }
    let output_values = crate::preserves_rail::required_sequence_field(&fields[1], "callback outputs")?;
    let effect_values = crate::preserves_rail::required_sequence_field(&fields[2], "callback effects")?;
    let state = parse_optional_value(&fields[3], "callback state", maximum_value_bytes)?;
    let checkpoint = parse_optional_value(&fields[4], "callback checkpoint", maximum_value_bytes)?;
    let value_count = output_values
        .len()
        .checked_add(effect_values.len())
        .and_then(|count| count.checked_add(state.iter().count()))
        .and_then(|count| count.checked_add(checkpoint.iter().count()))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("native callback value count overflow"))?;
    require_item_bound(value_count, maximum_items, "callback output values")?;
    let outputs = output_values
        .iter()
        .map(|value| parse_value(value, "callback output", maximum_value_bytes))
        .collect::<crate::error::Result<Vec<_>>>()?;
    let effects = effect_values
        .iter()
        .map(|value| parse_effect(value, maximum_value_bytes))
        .collect::<crate::error::Result<Vec<_>>>()?;
    Ok(NativeMaterializedCallbackOutcome {
        outputs,
        effects,
        state,
        checkpoint,
        health: parse_health(&crate::preserves_rail::required_string_field(&fields[5], "callback health")?)?,
    })
}

fn effect_value(effect: &NativeMaterializedEffectRequest) -> preserves::IOValue {
    let target = match &effect.effect.target {
        EffectTarget::FabricPort(key) => crate::preserves_rail::record(PORT_TARGET_RECORD, vec![
            crate::preserves_rail::string(&key.port_id),
            crate::preserves_rail::string(&key.version),
        ]),
        EffectTarget::Ambient(ambient) => {
            crate::preserves_rail::record("native-callback-ambient-target-v2", vec![crate::preserves_rail::string(
                ambient.as_str(),
            )])
        }
    };
    crate::preserves_rail::record(EFFECT_RECORD, vec![
        target,
        crate::preserves_rail::string(&effect.effect.operation),
        crate::preserves_rail::string(&effect.effect.input_schema_ref),
        crate::preserves_rail::string(&effect.effect.output_schema_ref),
        value_value(&effect.request),
        crate::preserves_rail::u64_value(effect.effect.generation),
        crate::preserves_rail::u64_value(effect.effect.accounted_bytes),
        crate::preserves_rail::string(NATIVE_CALLBACK_OUTCOME_SCHEMA),
    ])
}
