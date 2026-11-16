use super::*;

const HIGH_GENERATION: u64 = u64::MAX - 1;
const HIGH_INCARNATION: u64 = u64::MAX - 2;

// These records are assembled from the original raw-u64 wire layout, not from
// any encoded output or decoder of the migrated adapter.
fn raw_bytes(bytes: &[u8]) -> preserves::IOValue {
    crate::preserves_rail::sequence(
        bytes.iter().map(|byte| crate::preserves_rail::u64_value(u64::from(*byte))).collect(),
    )
}

fn raw_value(value: &NativeCallbackValue) -> preserves::IOValue {
    crate::preserves_rail::record("native-callback-value-v2", vec![
        crate::preserves_rail::string(&value.value_ref),
        raw_bytes(&value.bytes),
    ])
}

fn raw_optional(value: Option<&NativeCallbackValue>) -> preserves::IOValue {
    value.map_or_else(
        || crate::preserves_rail::record("none", vec![]),
        |value| crate::preserves_rail::record("some", vec![raw_value(value)]),
    )
}

fn raw_ref_optional(value: Option<&str>) -> preserves::IOValue {
    value.map_or_else(
        || crate::preserves_rail::record("none", vec![]),
        |value| crate::preserves_rail::record("some", vec![crate::preserves_rail::string(value)]),
    )
}

fn raw_refs(values: &[String]) -> preserves::IOValue {
    crate::preserves_rail::sequence(values.iter().map(crate::preserves_rail::string).collect())
}

fn raw_operation(operation: &NativeOperationRecord) -> preserves::IOValue {
    let string = |value: &str| crate::preserves_rail::string(value);
    crate::preserves_rail::record("native-instance-operation-v2", vec![
        string(&operation.schema),
        string(&operation.operation_ref),
        string(&operation.parent_ref),
        string(operation.kind.as_str()),
        crate::preserves_rail::u64_value(1),
        string(operation.state.as_str()),
        raw_ref_optional(operation.terminal_ref.as_deref()),
        crate::preserves_rail::bool_value(operation.is_retry_permitted),
    ])
}

// r[verify molten.system_extension.lifecycle]
// r[verify molten.system_extension.native_host.callback_protocol]
// r[verify molten.system_extension.native_host.durability]
#[test]
fn full_width_envelope_keeps_raw_u64_bytes_and_ref() {
    let mut high_invocation = invocation();
    high_invocation.generation = crate::system_extension::Generation::new(HIGH_GENERATION);
    high_invocation.incarnation = crate::system_extension::Incarnation::new(HIGH_INCARNATION);
    let inputs = CallbackInputs {
        payload: Some(value(PAYLOAD_BYTES)),
        state: Some(value(STATE_BYTES)),
    };
    let context = context();
    let envelope = canonical_callback_envelope(&context, &high_invocation, &inputs).expect("full-width envelope");
    let string = |value: &str| crate::preserves_rail::string(value);
    let number = crate::preserves_rail::u64_value;
    let raw_envelope = crate::preserves_rail::record("native-callback-envelope-v3", vec![
        string(NATIVE_CALLBACK_ENVELOPE_SCHEMA),
        string(&context.manifest_ref),
        string(&context.executable_ref),
        string(&context.instance_id),
        string(&context.extension_id),
        string(&context.service_id),
        number(HIGH_GENERATION),
        number(HIGH_INCARNATION),
        string("request"),
        number(1),
        string(&high_invocation.event_ref),
        raw_optional(inputs.payload.as_ref()),
        number(high_invocation.logical_tick),
        number(high_invocation.deadline_tick),
        raw_optional(inputs.state.as_ref()),
        raw_refs(&context.policy_refs),
        string(&context.resource_ref),
        raw_refs(&context.port_binding_refs),
        string(NATIVE_FRAMING),
    ]);
    assert_eq!(envelope.bytes, crate::preserves_rail::canonical_bytes(&raw_envelope).unwrap());
    assert_eq!(envelope.envelope_ref, crate::preserves_rail::canonical_hash(&raw_envelope).unwrap());
    let decoded = decode_callback_envelope(&envelope.bytes, CALLBACK_BYTES, CALLBACK_BYTES, MAX_ITEMS).unwrap();
    assert_eq!(decoded.invocation, high_invocation);
}

// r[verify molten.system_extension.lifecycle]
// r[verify molten.system_extension.native_host.durability]
#[test]
fn full_width_durable_state_keeps_raw_u64_bytes_and_ref() {
    let string = |value: &str| crate::preserves_rail::string(value);
    let number = crate::preserves_rail::u64_value;

    let mut high_instance = instance();
    high_instance.lifecycle.generation = crate::system_extension::Generation::new(HIGH_GENERATION);
    high_instance.lifecycle.incarnation = crate::system_extension::Incarnation::new(HIGH_INCARNATION);
    let canonical = canonical_instance_record(&high_instance).expect("full-width durable state");
    let lifecycle = crate::preserves_rail::record("native-instance-lifecycle-v3", vec![
        number(HIGH_GENERATION),
        number(HIGH_INCARNATION),
        string("running"),
        number(0),
        string("healthy"),
        raw_ref_optional(Some(HASH_A)),
    ]);
    let usage = crate::preserves_rail::record("native-instance-usage-v2", vec![
        number(0),
        number(0),
        number(0),
        number(0),
        number(0),
        number(0),
    ]);
    let raw_state = crate::preserves_rail::record("native-instance-state-v3", vec![
        string(NATIVE_INSTANCE_STATE_SCHEMA),
        string("native-fixture"),
        string("fixture-extension"),
        string("fixture-service"),
        string(HASH_A),
        string(HASH_B),
        string(HASH_C),
        string(HASH_D),
        lifecycle,
        usage,
        number(1),
        number(1),
        raw_ref_optional(high_instance.state_ref.as_deref()),
        raw_ref_optional(Some(HASH_A)),
        crate::preserves_rail::sequence(high_instance.unresolved.iter().map(raw_operation).collect()),
        crate::preserves_rail::sequence(high_instance.completed_operations.iter().map(raw_operation).collect()),
        raw_refs(&high_instance.completed_operation_refs),
        raw_refs(&high_instance.evidence_refs),
        crate::preserves_rail::bool_value(true),
    ]);
    assert_eq!(canonical.bytes, crate::preserves_rail::canonical_bytes(&raw_state).unwrap());
    assert_eq!(canonical.record_ref, crate::preserves_rail::canonical_hash(&raw_state).unwrap());
    assert_eq!(decode_instance_record(&canonical.bytes).unwrap(), high_instance);
}

// r[verify molten.system_extension.native_host.callback_protocol]
#[test]
fn full_width_effect_keeps_raw_u64_outcome_bytes_and_ref() {
    let mut request = effect();
    request.generation = crate::system_extension::Generation::new(HIGH_GENERATION);
    request.incarnation = crate::system_extension::Incarnation::new(HIGH_INCARNATION);
    let outcome = MaterializedCallbackOutcome {
        outputs: Vec::new(),
        effects: vec![MaterializedEffectRequest {
            effect: request,
            request: value(EFFECT_REQUEST_BYTES),
        }],
        state: None,
        checkpoint: None,
        health: HealthState::Healthy,
    };
    let bytes = encode_callback_outcome(&outcome).expect("full-width effect wire bytes");
    assert_eq!(decode_callback_outcome(&bytes, CALLBACK_BYTES, CALLBACK_BYTES, MAX_ITEMS).unwrap(), outcome);
    let string = |value: &str| crate::preserves_rail::string(value);
    let number = crate::preserves_rail::u64_value;
    let effect = &outcome.effects[0];
    let raw_effect = crate::preserves_rail::record("native-callback-effect-v3", vec![
        crate::preserves_rail::record("native-callback-port-target-v2", vec![string("fixture-port"), string("v1")]),
        string("fixture-operation"),
        string("fixture-input-v1"),
        string("fixture-output-v1"),
        raw_value(&effect.request),
        number(HIGH_GENERATION),
        number(HIGH_INCARNATION),
        number(EFFECT_REQUEST_BYTES.len() as u64),
        string(NATIVE_CALLBACK_OUTCOME_SCHEMA),
    ]);
    let raw = crate::preserves_rail::record("native-callback-outcome-v3", vec![
        string(NATIVE_CALLBACK_OUTCOME_SCHEMA),
        crate::preserves_rail::sequence(vec![]),
        crate::preserves_rail::sequence(vec![raw_effect]),
        raw_optional(None),
        raw_optional(None),
        string("healthy"),
    ]);
    assert_eq!(bytes, crate::preserves_rail::canonical_bytes(&raw).unwrap());
    let parsed = crate::preserves_rail::strict_canonical_decode(&bytes).unwrap();
    assert_eq!(
        crate::preserves_rail::canonical_hash(&parsed.value).unwrap(),
        crate::preserves_rail::canonical_hash(&raw).unwrap()
    );
}
