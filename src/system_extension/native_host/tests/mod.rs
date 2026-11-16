use std::os::fd::AsRawFd;

use super::super::*;
use super::*;
use crate::fabric_durability::*;

const HASH_A: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const HASH_C: &str = "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const HASH_D: &str = "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const GENERATION: u64 = 1;
const CALLBACK_BYTES: u64 = 4_096;
const MAX_ITEMS: u64 = 16;
const PROFILE_LIMIT: u64 = 32;
const OPERATION_BYTE_LIMIT: u64 = 65_536;
const NAMESPACE_BYTE_LIMIT: u64 = 1_048_576;
const PAYLOAD_BYTES: &[u8] = b"native-payload-v2";
const STATE_BYTES: &[u8] = b"native-state-v2";
const OUTPUT_BYTES: &[u8] = b"native-output-v2";
const EFFECT_REQUEST_BYTES: &[u8] = b"native-effect-request-v2";

fn value(bytes: &[u8]) -> NativeCallbackValue {
    NativeCallbackValue {
        value_ref: crate::preserves_rail::content_ref_from_bytes(bytes),
        bytes: bytes.to_vec(),
    }
}

fn invocation() -> CallbackInvocation {
    CallbackInvocation {
        callback: CallbackKind::Request,
        generation: crate::system_extension::Generation::new(GENERATION),
        incarnation: crate::system_extension::Incarnation::new(0),
        sequence: GENERATION,
        event_ref: HASH_A.to_string(),
        payload_ref: Some(value(PAYLOAD_BYTES).value_ref),
        logical_tick: GENERATION,
        deadline_tick: PROFILE_LIMIT,
    }
}

fn context() -> CallbackContext {
    CallbackContext {
        manifest_ref: HASH_A.to_string(),
        executable_ref: HASH_B.to_string(),
        instance_id: "native-fixture".to_string(),
        extension_id: "fixture-extension".to_string(),
        service_id: "fixture-service".to_string(),
        state_ref: Some(value(STATE_BYTES).value_ref),
        policy_refs: vec![HASH_C.to_string()],
        resource_ref: HASH_D.to_string(),
        port_binding_refs: vec![HASH_D.to_string()],
    }
}

fn effect() -> TypedEffectRequest {
    TypedEffectRequest {
        target: EffectTarget::FabricPort(crate::fabric::FabricPortKey {
            port_id: "fixture-port".to_string(),
            version: "v1".to_string(),
        }),
        operation: "fixture-operation".to_string(),
        input_schema_ref: "fixture-input-v1".to_string(),
        output_schema_ref: "fixture-output-v1".to_string(),
        request_ref: value(EFFECT_REQUEST_BYTES).value_ref,
        generation: crate::system_extension::Generation::new(GENERATION),
        incarnation: crate::system_extension::Incarnation::new(0),
        accounted_bytes: u64::try_from(EFFECT_REQUEST_BYTES.len()).expect("effect request length"),
    }
}

fn instance() -> NativeInstanceRecord {
    NativeInstanceRecord {
        schema: NATIVE_INSTANCE_STATE_SCHEMA.to_string(),
        instance_id: "native-fixture".to_string(),
        extension_id: "fixture-extension".to_string(),
        service_id: "fixture-service".to_string(),
        manifest_ref: HASH_A.to_string(),
        executable_ref: HASH_B.to_string(),
        profile_ref: HASH_C.to_string(),
        state_schema_ref: HASH_D.to_string(),
        lifecycle: LifecycleState {
            generation: crate::system_extension::Generation::new(GENERATION),
            incarnation: crate::system_extension::Incarnation::new(0),
            phase: LifecyclePhase::Running,
            restart_attempts: 0,
            health: HealthState::Healthy,
            checkpoint_ref: Some(HASH_A.to_string()),
        },
        usage: ResourceUsage::default(),
        callback_sequence: GENERATION,
        event_sequence: GENERATION,
        state_ref: Some(value(STATE_BYTES).value_ref),
        checkpoint_ref: Some(HASH_A.to_string()),
        unresolved: vec![NativeOperationRecord {
            schema: NATIVE_OPERATION_SCHEMA.to_string(),
            operation_ref: HASH_C.to_string(),
            parent_ref: HASH_D.to_string(),
            kind: NativeOperationKind::Effect,
            generation: crate::system_extension::Generation::new(GENERATION),
            state: NativeOperationState::Unknown,
            terminal_ref: None,
            is_retry_permitted: false,
        }],
        completed_operations: vec![NativeOperationRecord {
            schema: NATIVE_OPERATION_SCHEMA.to_string(),
            operation_ref: native_identity_ref(&["completed-fixture"]),
            parent_ref: HASH_B.to_string(),
            kind: NativeOperationKind::Callback,
            generation: crate::system_extension::Generation::new(GENERATION),
            state: NativeOperationState::Terminal,
            terminal_ref: Some(HASH_C.to_string()),
            is_retry_permitted: false,
        }],
        completed_operation_refs: vec![HASH_B.to_string()],
        evidence_refs: vec![HASH_D.to_string()],
        is_accepting_ingress: true,
    }
}

fn durability_profile() -> DurableProfile {
    durable_profile(&DurableStateProfile {
        schema: DURABLE_STATE_PROFILE_SCHEMA.to_string(),
        profile_id: "native-host-redb-v1".to_string(),
        profile_ref: HASH_A.to_string(),
        adapter_kind: DurableAdapterKind::LiveRedb,
        supported_levels: vec![
            DurabilityLevel::Buffered,
            DurabilityLevel::ProcessLoss,
            DurabilityLevel::MachineLoss,
        ],
        max_namespaces: PROFILE_LIMIT,
        max_log_records: PROFILE_LIMIT,
        max_ordered_entries: PROFILE_LIMIT,
        max_operation_bytes: OPERATION_BYTE_LIMIT,
        max_namespace_bytes: NAMESPACE_BYTE_LIMIT,
        max_batch_operations: PROFILE_LIMIT,
        max_snapshots: PROFILE_LIMIT,
        max_effect_transactions: PROFILE_LIMIT,
        non_claims: REQUIRED_DURABILITY_NON_CLAIMS.to_vec(),
    })
    .expect("native journal durability profile")
}

fn durability_descriptor() -> DurableNamespaceDescriptor {
    DurableNamespaceDescriptor {
        schema: DURABLE_STATE_NAMESPACE_SCHEMA.to_string(),
        profile_ref: HASH_A.to_string(),
        adapter_id: "native-host-redb".to_string(),
        namespace_id: "native-host-instances".to_string(),
        generation: GENERATION,
        value_schema_ref: HASH_B.to_string(),
        atomicity_domain: AtomicityDomain {
            domain_id: "native-host-domain".to_string(),
            adapter_id: "native-host-redb".to_string(),
            namespace_id: "native-host-instances".to_string(),
            generation: GENERATION,
            object_classes: vec![DurableObjectClass::LogRecord],
            max_operations: PROFILE_LIMIT,
            max_bytes: OPERATION_BYTE_LIMIT,
            supported_levels: vec![DurabilityLevel::MachineLoss],
        },
        retention_authority_ref: Some(HASH_C.to_string()),
        quota_bytes: NAMESPACE_BYTE_LIMIT,
    }
}

// r[verify molten.system_extension.native_host.callback_protocol]
#[test]
fn callback_envelope_and_outcome_roundtrip_canonical_bytes() {
    let inputs = CallbackInputs {
        payload: Some(value(PAYLOAD_BYTES)),
        state: Some(value(STATE_BYTES)),
    };
    let envelope = canonical_callback_envelope(&context(), &invocation(), &inputs).expect("callback envelope");
    let decoded = decode_callback_envelope(&envelope.bytes, CALLBACK_BYTES, CALLBACK_BYTES, MAX_ITEMS)
        .expect("decode callback envelope");
    assert_eq!(decoded.invocation, invocation());
    assert_eq!(decoded.context, context());
    assert_eq!(decoded.inputs, inputs);

    let outcome = MaterializedCallbackOutcome {
        outputs: vec![value(OUTPUT_BYTES)],
        effects: vec![MaterializedEffectRequest {
            effect: effect(),
            request: value(EFFECT_REQUEST_BYTES),
        }],
        state: Some(value(STATE_BYTES)),
        checkpoint: None,
        health: HealthState::Healthy,
    };
    let bytes = encode_callback_outcome(&outcome).expect("encode outcome");
    assert_eq!(
        decode_callback_outcome(&bytes, CALLBACK_BYTES, CALLBACK_BYTES, MAX_ITEMS).expect("decode outcome"),
        outcome,
    );
}

const HIGH_GENERATION: u64 = u64::MAX - 1;
const HIGH_INCARNATION: u64 = u64::MAX - 2;

// These records are assembled from the original raw-u64 wire layout, not from
// any encoded output or decoder of the migrated adapter.
fn raw_native_bytes(bytes: &[u8]) -> preserves::IOValue {
    crate::preserves_rail::sequence(
        bytes.iter().map(|byte| crate::preserves_rail::u64_value(u64::from(*byte))).collect(),
    )
}

fn raw_native_value(value: &NativeCallbackValue) -> preserves::IOValue {
    crate::preserves_rail::record("native-callback-value-v2", vec![
        crate::preserves_rail::string(&value.value_ref),
        raw_native_bytes(&value.bytes),
    ])
}

fn raw_native_optional(value: Option<&NativeCallbackValue>) -> preserves::IOValue {
    value.map_or_else(
        || crate::preserves_rail::record("none", vec![]),
        |value| crate::preserves_rail::record("some", vec![raw_native_value(value)]),
    )
}

fn raw_native_ref_optional(value: Option<&str>) -> preserves::IOValue {
    value.map_or_else(
        || crate::preserves_rail::record("none", vec![]),
        |value| crate::preserves_rail::record("some", vec![crate::preserves_rail::string(value)]),
    )
}

fn raw_native_refs(values: &[String]) -> preserves::IOValue {
    crate::preserves_rail::sequence(values.iter().map(crate::preserves_rail::string).collect())
}

fn raw_native_operation(operation: &NativeOperationRecord) -> preserves::IOValue {
    let string = |value: &str| crate::preserves_rail::string(value);
    crate::preserves_rail::record("native-instance-operation-v2", vec![
        string(&operation.schema),
        string(&operation.operation_ref),
        string(&operation.parent_ref),
        string(operation.kind.as_str()),
        crate::preserves_rail::u64_value(1),
        string(operation.state.as_str()),
        raw_native_ref_optional(operation.terminal_ref.as_deref()),
        crate::preserves_rail::bool_value(operation.is_retry_permitted),
    ])
}

// r[verify molten.system_extension.lifecycle]
// r[verify molten.system_extension.native_host.callback_protocol]
// r[verify molten.system_extension.native_host.durability]
#[test]
fn full_width_native_envelope_and_durable_state_keep_raw_u64_bytes_and_refs() {
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
        raw_native_optional(inputs.payload.as_ref()),
        number(high_invocation.logical_tick),
        number(high_invocation.deadline_tick),
        raw_native_optional(inputs.state.as_ref()),
        raw_native_refs(&context.policy_refs),
        string(&context.resource_ref),
        raw_native_refs(&context.port_binding_refs),
        string(NATIVE_FRAMING),
    ]);
    assert_eq!(envelope.bytes, crate::preserves_rail::canonical_bytes(&raw_envelope).unwrap());
    assert_eq!(envelope.envelope_ref, crate::preserves_rail::canonical_hash(&raw_envelope).unwrap());
    let decoded = decode_callback_envelope(&envelope.bytes, CALLBACK_BYTES, CALLBACK_BYTES, MAX_ITEMS).unwrap();
    assert_eq!(decoded.invocation, high_invocation);

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
        raw_native_ref_optional(Some(HASH_A)),
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
        raw_native_ref_optional(high_instance.state_ref.as_deref()),
        raw_native_ref_optional(Some(HASH_A)),
        crate::preserves_rail::sequence(high_instance.unresolved.iter().map(raw_native_operation).collect()),
        crate::preserves_rail::sequence(high_instance.completed_operations.iter().map(raw_native_operation).collect()),
        raw_native_refs(&high_instance.completed_operation_refs),
        raw_native_refs(&high_instance.evidence_refs),
        crate::preserves_rail::bool_value(true),
    ]);
    assert_eq!(canonical.bytes, crate::preserves_rail::canonical_bytes(&raw_state).unwrap());
    assert_eq!(canonical.record_ref, crate::preserves_rail::canonical_hash(&raw_state).unwrap());
    assert_eq!(decode_instance_record(&canonical.bytes).unwrap(), high_instance);
}

// r[verify molten.system_extension.native_host.callback_protocol]
#[test]
fn full_width_native_effect_keeps_raw_u64_outcome_bytes_and_ref() {
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
        raw_native_value(&effect.request),
        number(HIGH_GENERATION),
        number(HIGH_INCARNATION),
        number(EFFECT_REQUEST_BYTES.len() as u64),
        string(NATIVE_CALLBACK_OUTCOME_SCHEMA),
    ]);
    let raw = crate::preserves_rail::record("native-callback-outcome-v3", vec![
        string(NATIVE_CALLBACK_OUTCOME_SCHEMA),
        crate::preserves_rail::sequence(vec![]),
        crate::preserves_rail::sequence(vec![raw_effect]),
        raw_native_optional(None),
        raw_native_optional(None),
        string("healthy"),
    ]);
    assert_eq!(bytes, crate::preserves_rail::canonical_bytes(&raw).unwrap());
    let parsed = crate::preserves_rail::strict_canonical_decode(&bytes).unwrap();
    assert_eq!(
        crate::preserves_rail::canonical_hash(&parsed.value).unwrap(),
        crate::preserves_rail::canonical_hash(&raw).unwrap()
    );
}

// r[verify molten.system_extension.native_host.callback_protocol]
#[test]
fn malformed_oversized_trailing_and_ambient_outcomes_deny() {
    let outcome = MaterializedCallbackOutcome {
        outputs: Vec::new(),
        effects: vec![MaterializedEffectRequest {
            effect: effect(),
            request: value(EFFECT_REQUEST_BYTES),
        }],
        state: Some(value(STATE_BYTES)),
        checkpoint: None,
        health: HealthState::Healthy,
    };
    let mut bytes = encode_callback_outcome(&outcome).expect("encode outcome");
    bytes.push(0);
    assert!(decode_callback_outcome(&bytes, CALLBACK_BYTES, CALLBACK_BYTES, MAX_ITEMS).is_err());
    let maximum = u64::try_from(bytes.len() - 1).expect("fixture output length");
    assert!(decode_callback_outcome(&bytes, maximum, CALLBACK_BYTES, MAX_ITEMS).is_err());

    let mut ambient = outcome;
    ambient.effects[0].effect.target = EffectTarget::Ambient(AmbientEffect::Process);
    let ambient_bytes = encode_callback_outcome(&ambient).expect("encode ambient outcome");
    assert!(decode_callback_outcome(&ambient_bytes, CALLBACK_BYTES, CALLBACK_BYTES, MAX_ITEMS).is_err());
}

// r[verify molten.system_extension.native_host.value_materialization]
// r[verify molten.system_extension.native_host.value_publication]
// r[verify molten.system_extension.native_host.value_validation]
#[test]
fn value_port_verifies_identity_bounds_rejection_and_uncertainty() {
    let exact = value(PAYLOAD_BYTES);
    let mut port = InMemoryCallbackValuePort::from_values([PAYLOAD_BYTES.to_vec()]);
    assert_eq!(port.materialize(&exact.value_ref, CALLBACK_BYTES).expect("materialize exact value"), exact,);
    assert!(port.materialize(HASH_A, CALLBACK_BYTES).is_err());
    assert!(port.materialize(&exact.value_ref, 1).is_err());

    let mut corrupt = exact.clone();
    corrupt.bytes.push(0);
    assert!(admit_callback_value(&corrupt, CALLBACK_BYTES).is_err());

    let output = value(OUTPUT_BYTES);
    port.fail_next_publication(ValuePortFailureKind::RejectedBeforeAcceptance);
    let rejected = port.publish(&output, CALLBACK_BYTES).expect_err("publication rejection");
    assert!(!rejected.may_have_published());
    assert!(!port.contains(&output.value_ref));

    port.fail_next_publication(ValuePortFailureKind::UnknownAfterAcceptance);
    let unknown = port.publish(&output, CALLBACK_BYTES).expect_err("uncertain publication");
    assert!(unknown.may_have_published());
    assert!(port.contains(&output.value_ref));
}

// r[verify molten.system_extension.native_host.value_protocol]
// r[verify molten.system_extension.native_host.value_validation]
#[test]
fn reference_only_and_substituted_v2_values_deny() {
    let missing_payload = CallbackInputs {
        payload: None,
        state: Some(value(STATE_BYTES)),
    };
    assert!(canonical_callback_envelope(&context(), &invocation(), &missing_payload).is_err());

    let mut substituted = value(OUTPUT_BYTES);
    substituted.value_ref = value(PAYLOAD_BYTES).value_ref;
    let outcome = MaterializedCallbackOutcome {
        outputs: vec![substituted],
        effects: Vec::new(),
        state: Some(value(STATE_BYTES)),
        checkpoint: None,
        health: HealthState::Healthy,
    };
    assert!(encode_callback_outcome(&outcome).is_err());
}

// r[verify molten.system_extension.native_host.semantic_state]
// r[verify molten.system_extension.native_host.durability]
// r[verify molten.fabric_durability.redb_restore_profile_admission]
#[test]
fn memory_and_redb_journals_roundtrip_exact_instance_across_restart() {
    let mut memory = InMemory::default();
    memory.save_instance(&instance()).expect("memory save");
    assert_eq!(memory.latest_instance("native-fixture").expect("memory latest"), Some(instance()));

    let temporary = cap_tempfile::tempdir(cap_tempfile::ambient_authority()).expect("temporary capability root");
    let descriptor_path = std::path::PathBuf::from(format!("/proc/self/fd/{}", temporary.as_raw_fd()));
    let host_path = std::fs::read_link(descriptor_path).expect("temporary host path");
    {
        let adapter = RedbDurableStateAdapter::open(&host_path, durability_profile(), durability_descriptor())
            .expect("open native journal");
        let mut journal = Durable::new(adapter);
        journal.save_instance(&instance()).expect("durable save");
    }
    let adapter = RedbDurableStateAdapter::open(&host_path, durability_profile(), durability_descriptor())
        .expect("reopen native journal");
    let journal = Durable::new(adapter);
    assert_eq!(journal.latest_instance("native-fixture").expect("durable latest"), Some(instance()));
    assert_eq!(journal.history("native-fixture").expect("durable history"), vec![instance()]);
}
