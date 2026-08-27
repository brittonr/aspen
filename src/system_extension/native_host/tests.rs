use std::os::fd::AsRawFd;
use std::path::PathBuf;

use super::super::*;
use super::*;
use crate::fabric_durability::*;

const HASH_A: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const HASH_C: &str = "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const HASH_D: &str = "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const GENERATION: u64 = 1;
const CALLBACK_BYTES: u64 = 4_096;
const MAX_ITEMS: usize = 16;
const PROFILE_LIMIT: u64 = 32;
const OPERATION_BYTE_LIMIT: u64 = 65_536;
const NAMESPACE_BYTE_LIMIT: u64 = 1_048_576;

fn invocation() -> CallbackInvocation {
    CallbackInvocation {
        callback: CallbackKind::Request,
        generation: GENERATION,
        sequence: GENERATION,
        event_ref: HASH_A.to_string(),
        payload_ref: Some(HASH_B.to_string()),
        logical_tick: GENERATION,
        deadline_tick: PROFILE_LIMIT,
    }
}

fn context() -> NativeCallbackContext {
    NativeCallbackContext {
        manifest_ref: HASH_A.to_string(),
        executable_ref: HASH_B.to_string(),
        instance_id: "native-fixture".to_string(),
        extension_id: "fixture-extension".to_string(),
        service_id: "fixture-service".to_string(),
        state_ref: Some(HASH_C.to_string()),
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
        request_ref: HASH_D.to_string(),
        generation: GENERATION,
        accounted_bytes: PROFILE_LIMIT,
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
            generation: GENERATION,
            phase: LifecyclePhase::Running,
            restart_attempts: 0,
            health: HealthState::Healthy,
            checkpoint_ref: Some(HASH_A.to_string()),
        },
        usage: ResourceUsage::default(),
        callback_sequence: GENERATION,
        event_sequence: GENERATION,
        checkpoint_ref: Some(HASH_A.to_string()),
        unresolved: vec![NativeOperationRecord {
            schema: NATIVE_OPERATION_SCHEMA.to_string(),
            operation_ref: HASH_C.to_string(),
            parent_ref: HASH_D.to_string(),
            kind: NativeOperationKind::Effect,
            generation: GENERATION,
            state: NativeOperationState::Unknown,
            terminal_ref: None,
            is_retry_permitted: false,
        }],
        completed_operations: vec![NativeOperationRecord {
            schema: NATIVE_OPERATION_SCHEMA.to_string(),
            operation_ref: native_identity_ref(&["completed-fixture"]),
            parent_ref: HASH_B.to_string(),
            kind: NativeOperationKind::Callback,
            generation: GENERATION,
            state: NativeOperationState::Terminal,
            terminal_ref: Some(HASH_C.to_string()),
            is_retry_permitted: false,
        }],
        completed_operation_refs: vec![HASH_B.to_string()],
        evidence_refs: vec![HASH_D.to_string()],
        is_accepting_ingress: true,
    }
}

fn durability_profile() -> CanonicalDurableProfile {
    canonical_durable_profile(&DurableStateProfile {
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
    let envelope = canonical_native_callback_envelope(&context(), &invocation()).expect("callback envelope");
    let decoded =
        decode_native_callback_envelope(&envelope.bytes, CALLBACK_BYTES, MAX_ITEMS).expect("decode callback envelope");
    assert_eq!(decoded.invocation, invocation());
    assert_eq!(decoded.context, context());

    let outcome = CallbackOutcome {
        output_refs: vec![HASH_A.to_string()],
        effects: vec![effect()],
        state_ref: Some(HASH_B.to_string()),
        checkpoint_ref: None,
        health: HealthState::Healthy,
    };
    let bytes = encode_native_callback_outcome(&outcome).expect("encode outcome");
    assert_eq!(decode_native_callback_outcome(&bytes, CALLBACK_BYTES, MAX_ITEMS).expect("decode outcome"), outcome);
}

// r[verify molten.system_extension.native_host.callback_protocol]
#[test]
fn malformed_oversized_trailing_and_ambient_outcomes_deny() {
    let outcome = CallbackOutcome {
        output_refs: Vec::new(),
        effects: vec![effect()],
        state_ref: Some(HASH_B.to_string()),
        checkpoint_ref: None,
        health: HealthState::Healthy,
    };
    let mut bytes = encode_native_callback_outcome(&outcome).expect("encode outcome");
    bytes.push(0);
    assert!(decode_native_callback_outcome(&bytes, CALLBACK_BYTES, MAX_ITEMS).is_err());
    let maximum = u64::try_from(bytes.len() - 1).expect("fixture output length");
    assert!(decode_native_callback_outcome(&bytes, maximum, MAX_ITEMS).is_err());

    let mut ambient = outcome;
    ambient.effects[0].target = EffectTarget::Ambient(AmbientEffect::Process);
    let ambient_bytes = encode_native_callback_outcome(&ambient).expect("encode ambient outcome");
    assert!(decode_native_callback_outcome(&ambient_bytes, CALLBACK_BYTES, MAX_ITEMS).is_err());
}

// r[verify molten.system_extension.native_host.durability]
#[test]
fn memory_and_redb_journals_roundtrip_exact_instance_across_restart() {
    let mut memory = InMemoryNativeHostJournal::default();
    memory.save_instance(&instance()).expect("memory save");
    assert_eq!(memory.latest_instance("native-fixture").expect("memory latest"), Some(instance()));

    let temporary = cap_tempfile::tempdir(cap_tempfile::ambient_authority()).expect("temporary capability root");
    let descriptor_path = PathBuf::from(format!("/proc/self/fd/{}", temporary.as_raw_fd()));
    let host_path = std::fs::read_link(descriptor_path).expect("temporary host path");
    {
        let adapter = RedbDurableStateAdapter::open(&host_path, durability_profile(), durability_descriptor())
            .expect("open native journal");
        let mut journal = DurableNativeHostJournal::new(adapter);
        journal.save_instance(&instance()).expect("durable save");
    }
    let adapter = RedbDurableStateAdapter::open(&host_path, durability_profile(), durability_descriptor())
        .expect("reopen native journal");
    let journal = DurableNativeHostJournal::new(adapter);
    assert_eq!(journal.latest_instance("native-fixture").expect("durable latest"), Some(instance()));
}
