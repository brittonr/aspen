use super::super::HealthState;
use super::super::LifecyclePhase;
use super::super::ResourceUsage;
// r[impl molten.system_extension.native_host.validation]
use super::*;

const HASH_A: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const HASH_C: &str = "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const HASH_D: &str = "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const HASH_E: &str = "blake3:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const HASH_F: &str = "blake3:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
const GENERATION: u64 = 1;
const STALE_GENERATION: u64 = 2;
const CALLBACK_BYTES: u64 = 128;
const MAX_CALLBACK_BYTES: u64 = 4_096;
const MAX_DIAGNOSTIC_BYTES: u64 = 1_024;
const MAX_INSTANCES: usize = 4;
const MAX_OPERATIONS: usize = 8;
const MAX_BINDINGS: usize = 8;
const MAX_POLICIES: usize = 4;

fn profile() -> NativeHostProfile {
    NativeHostProfile {
        schema: NATIVE_HOST_PROFILE_SCHEMA.to_string(),
        profile_id: "native-host-local-pilot-v1".to_string(),
        profile_ref: HASH_A.to_string(),
        execution_profile_ref: HASH_B.to_string(),
        transport_profile_ref: HASH_C.to_string(),
        alpn: NATIVE_ALPN.to_string(),
        framing: NATIVE_FRAMING.to_string(),
        max_callback_input_bytes: MAX_CALLBACK_BYTES,
        max_callback_output_bytes: MAX_CALLBACK_BYTES,
        max_diagnostic_bytes: MAX_DIAGNOSTIC_BYTES,
        max_instances: MAX_INSTANCES,
        max_unresolved_operations: MAX_OPERATIONS,
        max_port_bindings: MAX_BINDINGS,
        max_policy_refs: MAX_POLICIES,
        is_local_live_pilot: true,
        non_claims: REQUIRED_NATIVE_HOST_NON_CLAIMS.to_vec(),
    }
}

fn executable() -> NativeExecutableEvidence {
    NativeExecutableEvidence {
        schema: NATIVE_EXECUTABLE_EVIDENCE_SCHEMA.to_string(),
        executable_ref: HASH_A.to_string(),
        executable_bytes_ref: HASH_B.to_string(),
        artifact_kind_ref: HASH_C.to_string(),
        target_ref: HASH_D.to_string(),
        dependency_closure_ref: HASH_E.to_string(),
        materialization_ref: HASH_F.to_string(),
        provenance_ref: HASH_A.to_string(),
        source_gate_ref: HASH_B.to_string(),
        policy_ref: HASH_C.to_string(),
        authority_ref: HASH_D.to_string(),
        resource_ref: HASH_E.to_string(),
        execution_profile_ref: HASH_B.to_string(),
        manifest_ref: HASH_F.to_string(),
        state_schema_ref: HASH_A.to_string(),
        port_binding_refs: vec![HASH_B.to_string()],
    }
}

fn instance() -> NativeInstanceRecord {
    NativeInstanceRecord {
        schema: NATIVE_INSTANCE_STATE_SCHEMA.to_string(),
        instance_id: "fixture-instance".to_string(),
        extension_id: "fixture-extension".to_string(),
        service_id: "fixture-service".to_string(),
        manifest_ref: HASH_F.to_string(),
        executable_ref: HASH_A.to_string(),
        profile_ref: HASH_A.to_string(),
        state_schema_ref: HASH_A.to_string(),
        lifecycle: super::super::LifecycleState {
            generation: GENERATION,
            phase: LifecyclePhase::Running,
            restart_attempts: 0,
            health: HealthState::Healthy,
            checkpoint_ref: Some(HASH_B.to_string()),
        },
        usage: ResourceUsage::default(),
        callback_sequence: 0,
        event_sequence: 0,
        checkpoint_ref: Some(HASH_B.to_string()),
        unresolved: Vec::new(),
        completed_operations: Vec::new(),
        completed_operation_refs: Vec::new(),
        evidence_refs: vec![HASH_C.to_string()],
        is_accepting_ingress: true,
    }
}

fn operation(kind: NativeOperationKind) -> NativeOperationRecord {
    NativeOperationRecord {
        schema: NATIVE_OPERATION_SCHEMA.to_string(),
        operation_ref: HASH_D.to_string(),
        parent_ref: HASH_E.to_string(),
        kind,
        generation: GENERATION,
        state: NativeOperationState::IntentCommitted,
        terminal_ref: None,
        is_retry_permitted: false,
    }
}

fn ingress() -> NativeIngressEnvelope {
    NativeIngressEnvelope {
        schema: NATIVE_INGRESS_SCHEMA.to_string(),
        request_ref: HASH_A.to_string(),
        endpoint_ref: HASH_B.to_string(),
        peer_ref: HASH_C.to_string(),
        service_id: "fixture-service".to_string(),
        manifest_ref: HASH_F.to_string(),
        generation: GENERATION,
        authority_ref: HASH_D.to_string(),
        policy_ref: HASH_E.to_string(),
        resource_ref: HASH_F.to_string(),
        transport_profile_ref: HASH_C.to_string(),
        alpn: NATIVE_ALPN.to_string(),
        framing: NATIVE_FRAMING.to_string(),
        payload_ref: HASH_A.to_string(),
        accounted_bytes: CALLBACK_BYTES,
    }
}

// r[verify molten.system_extension.native_host.callback_protocol]
// r[verify molten.system_extension.native_host.execution]
// r[verify molten.system_extension.native_host.durability]
// r[verify molten.system_extension.native_host.effects]
// r[verify molten.system_extension.native_host.neutrality]
// r[verify molten.system_extension.native_host.validation]
#[test]
fn contract_surface_binds_protocol_execution_state_effects_and_neutral_identity() {
    assert_eq!(NATIVE_CALLBACK_ENVELOPE_SCHEMA, "molten.system-extension.native-callback-envelope.v1");
    assert_eq!(NATIVE_FRAMING, "preserves-packed-single-frame-v1");
    assert_eq!(NATIVE_INSTANCE_STATE_SCHEMA, "molten.system-extension.native-instance-state.v1");
    assert_eq!(NativeOperationKind::Effect.as_str(), "effect");
    assert!(!NATIVE_HOST_PROFILE_SCHEMA.contains("kiln"));
}

// r[verify molten.system_extension.native_host.profile]
// r[verify molten.system_extension.native_host.nonclaims]
#[test]
fn exact_local_pilot_profile_passes_and_missing_nonclaim_denies() {
    let admitted = admit_native_host_profile(&profile()).expect("native host profile");
    assert_eq!(admitted.profile.alpn, NATIVE_ALPN);
    assert_eq!(admitted.profile.non_claims, REQUIRED_NATIVE_HOST_NON_CLAIMS);

    let mut invalid = profile();
    invalid.non_claims.pop();
    let issues = admit_native_host_profile(&invalid).expect_err("missing non-claim must deny");
    assert!(issues.iter().any(|issue| matches!(issue, NativeHostIssue::MissingNonClaim(_))));
}

// r[verify molten.system_extension.native_host.executable]
#[test]
fn complete_executable_evidence_passes_and_possession_alone_denies() {
    let profile = admit_native_host_profile(&profile()).expect("native host profile");
    let admitted = admit_native_executable(&profile, &executable()).expect("native executable");
    assert_eq!(admitted.executable.execution_profile_ref, HASH_B);

    let mut possession = executable();
    possession.provenance_ref.clear();
    possession.policy_ref.clear();
    possession.authority_ref.clear();
    let issues = admit_native_executable(&profile, &possession).expect_err("possession alone must deny");
    assert!(issues.len() >= 3);
}

// r[verify molten.system_extension.native_host.intent]
// r[verify molten.system_extension.native_host.recovery]
#[test]
fn intent_precedes_start_and_unknown_never_enables_retry() {
    let profile = admit_native_host_profile(&profile()).expect("native host profile");
    let with_intent = commit_native_operation_intent(&profile, &instance(), operation(NativeOperationKind::Callback))
        .expect("callback intent");
    let started =
        observe_native_operation(&with_intent, HASH_D, NativeOperationState::Started, None).expect("start observation");
    let unknown =
        observe_native_operation(&started, HASH_D, NativeOperationState::Unknown, None).expect("unknown observation");
    let inventory = classify_native_recovery(&unknown);
    assert_eq!(inventory[0].class, NativeRecoveryClass::Unknown);
    assert!(!inventory[0].is_retry_permitted);
    assert!(commit_native_operation_intent(&profile, &unknown, operation(NativeOperationKind::Callback)).is_err());
}

// r[verify molten.system_extension.native_host.effect_completion]
#[test]
fn linked_effect_completion_passes_and_stale_or_duplicate_completion_denies() {
    let profile = admit_native_host_profile(&profile()).expect("native host profile");
    let pending = commit_native_operation_intent(&profile, &instance(), operation(NativeOperationKind::Effect))
        .expect("effect intent");
    let input = NativeEffectCompletionInput {
        completion_ref: HASH_C.to_string(),
        effect_ref: HASH_E.to_string(),
        operation_ref: HASH_D.to_string(),
        port_binding_ref: HASH_B.to_string(),
        generation: GENERATION,
    };
    let plan = admit_native_effect_completion(&pending, &input).expect("linked completion");
    assert_eq!(plan.payload_ref, HASH_C);

    let mut stale = input.clone();
    stale.generation = STALE_GENERATION;
    assert!(admit_native_effect_completion(&pending, &stale).is_err());
    let terminal = observe_native_operation(&pending, HASH_D, NativeOperationState::Terminal, Some(HASH_C.to_string()))
        .expect("terminal effect");
    assert!(admit_native_effect_completion(&terminal, &input).is_ok());
    let consumed = consume_native_effect_completion(&terminal, &input).expect("consume completion");
    assert!(admit_native_effect_completion(&consumed, &input).is_err());
}

// r[verify molten.system_extension.native_host.ingress]
#[test]
fn ingress_binds_transport_and_service_acceptance_separately() {
    let profile = admit_native_host_profile(&profile()).expect("native host profile");
    let admitted = admit_native_ingress(&profile, &instance(), &ingress()).expect("ingress admission");
    assert!(admitted.acknowledgement_ref.starts_with("blake3:"));

    let mut stale = ingress();
    stale.generation = STALE_GENERATION;
    let issues = admit_native_ingress(&profile, &instance(), &stale).expect_err("stale ingress must deny");
    assert!(issues.iter().any(|issue| matches!(issue, NativeHostIssue::StaleGeneration { .. })));
}

// r[verify molten.system_extension.native_host.recovery]
#[test]
fn startup_inventory_classifies_every_unresolved_operation_state_without_retry() {
    let mut candidate = instance();
    candidate.unresolved = [
        ("not-started", NativeOperationState::IntentCommitted, GENERATION),
        ("running", NativeOperationState::Started, GENERATION),
        ("terminal", NativeOperationState::Terminal, GENERATION),
        ("unknown", NativeOperationState::Unknown, GENERATION),
        ("stale", NativeOperationState::Started, STALE_GENERATION),
    ]
    .into_iter()
    .map(|(label, state, generation)| NativeOperationRecord {
        schema: NATIVE_OPERATION_SCHEMA.to_string(),
        operation_ref: native_identity_ref(&[label]),
        parent_ref: HASH_E.to_string(),
        kind: NativeOperationKind::Callback,
        generation,
        state,
        terminal_ref: (state == NativeOperationState::Terminal).then(|| HASH_C.to_string()),
        is_retry_permitted: false,
    })
    .collect();
    let classes = classify_native_recovery(&candidate)
        .into_iter()
        .map(|row| (row.class, row.is_retry_permitted))
        .collect::<Vec<_>>();
    assert!(classes.contains(&(NativeRecoveryClass::NotStarted, false)));
    assert!(classes.contains(&(NativeRecoveryClass::RunningObserved, false)));
    assert!(classes.contains(&(NativeRecoveryClass::Terminal, false)));
    assert!(classes.contains(&(NativeRecoveryClass::Unknown, false)));
    assert!(classes.contains(&(NativeRecoveryClass::Stale, false)));
}

#[test]
fn recovery_admission_rejects_profile_checkpoint_and_state_schema_drift() {
    let profile = admit_native_host_profile(&profile()).expect("native host profile");
    let executable = admit_native_executable(&profile, &executable()).expect("native executable");
    let mut invalid = instance();
    invalid.profile_ref = HASH_F.to_string();
    invalid.state_schema_ref = HASH_F.to_string();
    invalid.checkpoint_ref = None;
    let issues =
        admit_native_instance_recovery(&profile, &executable, &invalid).expect_err("incompatible recovery must deny");
    assert!(issues.len() >= 3);
}

// r[verify molten.system_extension.native_host.operator]
#[test]
fn removal_requires_terminal_idle_instance_without_unresolved_work() {
    let profile = admit_native_host_profile(&profile()).expect("native host profile");
    let pending = commit_native_operation_intent(&profile, &instance(), operation(NativeOperationKind::Effect))
        .expect("effect intent");
    assert!(admit_native_removal(&pending).is_err());

    let mut removable = instance();
    removable.lifecycle.phase = LifecyclePhase::Stopped;
    removable.is_accepting_ingress = false;
    let plan = admit_native_removal(&removable).expect("removal admission");
    assert_eq!(plan.instance_id, "fixture-instance");
}
