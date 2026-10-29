#[path = "../../../../../examples/component-extension/src/codec.rs"]
mod codec;
#[path = "../../../../../examples/component-extension/src/identity.rs"]
mod identity;
#[path = "../../../../../examples/component-extension/src/invocation.rs"]
mod invocation;
#[path = "../../../../../examples/component-extension/src/packed.rs"]
mod packed;

#[test]
fn values_interoperate_at_integer_and_hash_boundaries() {
    // r[verify molten.wasm_component_adoption.consumers]
    // r[verify molten.wasm_component_adoption.validation]
    for size in [0, 64, 1024, 1025, 2048] {
        let mut bytes = Vec::with_capacity(size);
        for index in 0..size {
            bytes.push(if index % 32 < 16 {
                b'a' + (index % 26) as u8
            } else {
                [0, 1, 127, 128, 255][index % 5]
            });
        }
        let value = crate::system_extension::NativeCallbackValue {
            value_ref: crate::preserves_rail::content_ref_from_bytes(&bytes),
            bytes: bytes.clone(),
        };
        let expected = crate::system_extension::MaterializedCallbackOutcome {
            outputs: vec![value.clone()],
            effects: Vec::with_capacity(0),
            state: Some(value),
            checkpoint: None,
            health: crate::system_extension::HealthState::Healthy,
        };
        let guest = codec::outcome(Some(&bytes), Some(&bytes), None);
        let admitted = crate::system_extension::decode_callback_outcome(&guest, 65_536, 65_536, 2)
            .expect("actual guest output follows native value protocol");
        assert_eq!(admitted, expected);

        let host = crate::system_extension::encode_callback_outcome(&expected).expect("native outcome encoding");
        let fields = codec::record::<6>(&host, "native-callback-outcome-v3").expect("guest reads native outcome");
        let mut position = 0;
        let decoded =
            codec::admitted_value(fields[3], &mut position).expect("guest reads product-admitted native value");
        assert_eq!(position, fields[3].len());
        assert_eq!(decoded.as_deref(), Some(bytes.as_slice()));
    }
}

#[test]
fn rejects_non_byte_or_noncanonical_value_members() {
    // r[verify molten.wasm_component_adoption.consumers]
    // r[verify molten.wasm_component_adoption.validation]
    for field in [
        &[0xb2, 0][..],
        &[0xb5, 0xb0, 1, 255, 0x84],
        &[0xb5, 0xb0, 2, 1, 0, 0x84],
        &[0xb5, 0xb0, 1, 0, 0x84],
        &[0xb5, 0xb0, 0x80, 0, 0x84],
    ] {
        let mut value = Vec::with_capacity(128);
        codec::start_record(&mut value, "some");
        codec::start_record(&mut value, "native-callback-value-v2");
        codec::write_atom(&mut value, 0xb1, &identity::reference(&[]));
        value.extend_from_slice(field);
        value.extend_from_slice(&[0x84, 0x84]);
        assert!(codec::admitted_value(&value, &mut 0).is_err());
    }
}

#[test]
fn grouped_ascii_rejects_corruption_at_every_integer_position() {
    // r[verify molten.wasm_component_adoption.validation]
    let mut encoded = Vec::with_capacity(256);
    codec::start_record(&mut encoded, "some");
    codec::write_value(&mut encoded, b"abcdefgh");
    encoded.push(0x84);
    let start = encoded.len() - 3 - 8 * 3;
    for index in 0..8 {
        for (field, invalid) in [(0, 0xb1), (1, 0), (2, 0), (2, 128)] {
            let mut corrupt = encoded.clone();
            corrupt[start + index * 3 + field] = invalid;
            assert!(codec::admitted_value(&corrupt, &mut 0).is_err());
        }
    }
}

#[test]
fn packed_nesting_accepts_the_limit_and_rejects_excess_depth() {
    // r[verify molten.wasm_component_adoption.validation]
    for levels in [32, 33] {
        let mut value = Vec::with_capacity(128);
        codec::start_record(&mut value, "depth");
        value.extend(std::iter::repeat_n(0xb5, levels));
        value.push(0x80);
        value.extend(std::iter::repeat_n(0x84, levels + 1));
        assert_eq!(codec::record::<1>(&value, "depth").is_ok(), levels == 32);
    }
}

#[test]
fn distinct_output_and_state_keep_their_own_identity() {
    // r[verify molten.wasm_component_adoption.consumers]
    let guest = codec::outcome(Some(b"output"), Some(b"state"), None);
    let admitted = crate::system_extension::decode_callback_outcome(&guest, 65_536, 65_536, 2).expect("native outcome");
    assert_eq!(admitted.outputs[0].bytes, b"output");
    assert_eq!(admitted.state.expect("state").bytes, b"state");
    assert!(crate::system_extension::decode_callback_outcome(&guest, 65_536, 65_536, 1).is_err());
}

fn callback_envelope(callback: crate::system_extension::CallbackKind) -> Vec<u8> {
    let materialize = |bytes: &[u8]| crate::system_extension::NativeCallbackValue {
        value_ref: crate::preserves_rail::content_ref_from_bytes(bytes),
        bytes: bytes.to_vec(),
    };
    let payload = materialize(b"payload");
    let state = materialize(b"state");
    let reference = crate::preserves_rail::content_ref_from_bytes(b"context");
    let context = crate::system_extension::CallbackContext {
        manifest_ref: reference.clone(),
        executable_ref: reference.clone(),
        instance_id: "projection-instance".to_string(),
        extension_id: "projection-extension".to_string(),
        service_id: "projection-service".to_string(),
        state_ref: Some(state.value_ref.clone()),
        policy_refs: vec![reference.clone()],
        resource_ref: reference.clone(),
        port_binding_refs: vec![reference.clone()],
    };
    let invocation = crate::system_extension::CallbackInvocation {
        callback,
        generation: 1,
        incarnation: 0,
        sequence: 1,
        event_ref: reference,
        payload_ref: Some(payload.value_ref.clone()),
        logical_tick: 1,
        deadline_tick: 32,
    };
    let inputs = crate::system_extension::CallbackInputs {
        payload: Some(payload),
        state: Some(state),
    };
    crate::system_extension::canonical_callback_envelope(&context, &invocation, &inputs)
        .expect("canonical product envelope")
        .bytes
}

#[test]
fn callback_projection_selects_payload_state_or_no_value() {
    // r[verify molten.wasm_component_adoption.consumers]
    // r[verify molten.wasm_component_adoption.validation]
    for (callback, expected) in [
        (crate::system_extension::CallbackKind::Request, Some(b"payload".as_slice())),
        (crate::system_extension::CallbackKind::Recover, Some(b"payload".as_slice())),
        (crate::system_extension::CallbackKind::Checkpoint, Some(b"state".as_slice())),
        (crate::system_extension::CallbackKind::Health, None),
    ] {
        let mut bytes = callback_envelope(callback);
        let projected = invocation::decode(&bytes).expect("project admitted envelope");
        assert_eq!(projected.callback, callback.as_str().as_bytes());
        assert_eq!(projected.value.as_deref(), expected);
        bytes.push(0);
        assert!(invocation::decode(&bytes).is_err());
        bytes.pop();
        bytes[0] = 0x80;
        assert!(invocation::decode(&bytes).is_err());
    }
}
