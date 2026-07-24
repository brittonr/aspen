use super::*;

const PROFILE_DECLARATION_REF: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const VALUE_SCHEMA_REF: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const VALUE_REF: &str = "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const SNAPSHOT_REF: &str = "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const ORDERED_REF: &str = "blake3:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const AUTHORITY_REF: &str = "blake3:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
const OPERATION_REF: &str = "blake3:1111111111111111111111111111111111111111111111111111111111111111";
const GENERATION: u64 = 1;
const STALE_GENERATION: u64 = 2;
const PROFILE_LIMIT: u64 = 32;
const OPERATION_BYTE_LIMIT: u64 = 1_024;
const NAMESPACE_BYTE_LIMIT: u64 = 16_384;
const FIRST_SEQUENCE: u64 = 0;
const EXPECTED_DURABILITY_PORT_COUNT: usize = 4;
const SIMULATED_DELAY_TICKS: u64 = 3;

pub(crate) fn profile(kind: DurableAdapterKind) -> CanonicalDurableProfile {
    canonical_durable_profile(&DurableStateProfile {
        schema: DURABLE_STATE_PROFILE_SCHEMA.to_string(),
        profile_id: kind.as_str().to_string(),
        profile_ref: PROFILE_DECLARATION_REF.to_string(),
        adapter_kind: kind,
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
    .expect("canonical durability profile")
}

pub(crate) fn descriptor() -> DurableNamespaceDescriptor {
    DurableNamespaceDescriptor {
        schema: DURABLE_STATE_NAMESPACE_SCHEMA.to_string(),
        profile_ref: PROFILE_DECLARATION_REF.to_string(),
        adapter_id: "adapter-a".to_string(),
        namespace_id: "namespace-a".to_string(),
        generation: GENERATION,
        value_schema_ref: VALUE_SCHEMA_REF.to_string(),
        atomicity_domain: AtomicityDomain {
            domain_id: "domain-a".to_string(),
            adapter_id: "adapter-a".to_string(),
            namespace_id: "namespace-a".to_string(),
            generation: GENERATION,
            object_classes: vec![
                DurableObjectClass::LogRecord,
                DurableObjectClass::OrderedValue,
                DurableObjectClass::Snapshot,
                DurableObjectClass::Checkpoint,
                DurableObjectClass::EffectTransaction,
            ],
            max_operations: PROFILE_LIMIT,
            max_bytes: OPERATION_BYTE_LIMIT,
            supported_levels: vec![
                DurabilityLevel::Buffered,
                DurabilityLevel::ProcessLoss,
                DurabilityLevel::MachineLoss,
            ],
        },
        retention_authority_ref: Some(AUTHORITY_REF.to_string()),
        quota_bytes: NAMESPACE_BYTE_LIMIT,
    }
}

fn append_request(durability: DurabilityLevel) -> AppendRequest {
    AppendRequest {
        adapter_id: "adapter-a".to_string(),
        namespace_id: "namespace-a".to_string(),
        generation: GENERATION,
        expected_sequence: FIRST_SEQUENCE,
        value: b"durable-record".to_vec(),
        value_ref: VALUE_REF.to_string(),
        durability,
    }
}

fn batch_request() -> AtomicBatchRequest {
    AtomicBatchRequest {
        domain: descriptor().atomicity_domain,
        generation: GENERATION,
        mutations: vec![OrderedMutation::Put {
            key: b"key-a".to_vec(),
            value: b"value-a".to_vec(),
            value_ref: VALUE_REF.to_string(),
            precondition: ValuePrecondition::Missing,
        }],
        durability: DurabilityLevel::ProcessLoss,
    }
}

// r[verify molten.fabric_durability.port_contracts]
// r[verify molten.fabric_durability.non_claims]
#[test]
fn canonical_profile_and_ports_are_stable_and_hide_backend_handles() {
    let profile = profile(DurableAdapterKind::LiveRedb);
    let descriptors = fabric_durability_port_descriptors(&profile);
    let text = crate::preserves_rail::to_text(&profile.value).expect("profile text");

    assert_eq!(descriptors.len(), EXPECTED_DURABILITY_PORT_COUNT);
    assert!(profile.profile_ref.starts_with("blake3:"));
    assert!(text.contains("does-not-prove-replication"));
    assert!(!text.contains("redb::"));
    assert!(!text.contains("std::fs"));
    assert!(
        descriptors
            .iter()
            .all(|descriptor| descriptor.class == crate::fabric::FabricPortClass::DurableState)
    );
}

// r[verify molten.fabric_durability.live_sim_parity]
// r[verify molten.fabric_durability.final_validation]
#[test]
fn redb_adapter_persists_log_ordered_snapshot_and_effect_inventory_across_restart() {
    let root = crate::test_support::process_workspace("fabric-durability-redb").expect("workspace");
    let live = profile(DurableAdapterKind::LiveRedb);
    let mut adapter = RedbDurableStateAdapter::open(&root, live.clone(), descriptor()).expect("open adapter");
    let append = adapter.append(&append_request(DurabilityLevel::ProcessLoss)).expect("append");
    assert_eq!(append.outcome, MutationOutcome::Durable);
    adapter.apply_batch(&batch_request()).expect("ordered batch");

    let snapshot_bytes = b"snapshot-bytes";
    let snapshot_content_ref = format!("blake3:{}", blake3::hash(snapshot_bytes).to_hex());
    adapter
        .create_snapshot(
            &SnapshotRequest {
                kind: SnapshotKind::Checkpoint,
                generation: GENERATION,
                snapshot_ref: SNAPSHOT_REF.to_string(),
                content_ref: snapshot_content_ref,
                ordered_state_ref: ORDERED_REF.to_string(),
                covered_log_sequence: Some(FIRST_SEQUENCE),
                durability: DurabilityLevel::ProcessLoss,
            },
            snapshot_bytes,
        )
        .expect("snapshot");
    adapter
        .apply_effect(&EffectTransactionCommand::Reserve {
            transaction_id: "effect-a".to_string(),
            generation: GENERATION,
            operation_ref: OPERATION_REF.to_string(),
            expires_at_tick: None,
            profile: EffectTransactionProfile {
                durable_reservation: true,
                exclusive: true,
                expiring: false,
                idempotent_commit: true,
                compensating_abort: false,
            },
        })
        .expect("reserve effect");
    drop(adapter);

    let reopened = RedbDurableStateAdapter::open(&root, live, descriptor()).expect("reopen adapter");
    assert_eq!(reopened.state().durable_log.len(), 1);
    assert_eq!(reopened.state().ordered.get(b"key-a".as_slice()).expect("ordered value").value, b"value-a");
    assert!(reopened.state().snapshots.contains_key(SNAPSHOT_REF));
    let restore = reopened.restore_snapshot(SNAPSHOT_REF, GENERATION).expect("restore plan");
    assert_eq!(restore.snapshot.kind, SnapshotKind::Checkpoint);
    assert_eq!(restore.restored_state_ref, ORDERED_REF);
    assert_eq!(reopened.state().effects.get("effect-a").expect("effect").phase, EffectTransactionPhase::Reserved);
    let status = reopened.status().expect("status");
    assert_eq!(status.unresolved_effects, 1);
    let rendered = crate::preserves_rail::to_text(&status.value).expect("status text");
    assert!(!rendered.contains("value-a"));
    assert!(!rendered.contains("key-a"));
}

// r[verify molten.fabric_durability.snapshot_recovery]
// r[verify molten.fabric_durability.final_validation]
#[test]
fn redb_adapter_rejects_snapshot_bytes_that_do_not_match_content_identity() {
    let root = crate::test_support::process_workspace("fabric-durability-bad-snapshot").expect("workspace");
    let mut adapter = RedbDurableStateAdapter::open(&root, profile(DurableAdapterKind::LiveRedb), descriptor())
        .expect("open adapter");
    let request = SnapshotRequest {
        kind: SnapshotKind::Snapshot,
        generation: GENERATION,
        snapshot_ref: SNAPSHOT_REF.to_string(),
        content_ref: VALUE_REF.to_string(),
        ordered_state_ref: ORDERED_REF.to_string(),
        covered_log_sequence: None,
        durability: DurabilityLevel::ProcessLoss,
    };
    let error = adapter.create_snapshot(&request, b"tampered").expect_err("snapshot mismatch must deny");
    assert!(error.to_string().contains("content ref mismatch"));
    assert!(adapter.state().snapshots.is_empty());
}

// r[verify molten.fabric_durability.live_sim_parity]
// r[verify molten.fabric_durability.uncertain_outcomes]
#[test]
fn deterministic_adapter_models_precommit_and_postcommit_failures_without_implicit_retry() {
    let mut adapter =
        SimulatedDurableStateAdapter::new(profile(DurableAdapterKind::DeterministicSimulation), descriptor())
            .expect("simulation adapter");
    let denied = adapter
        .append(&append_request(DurabilityLevel::ProcessLoss), Some(&SimulatedDurabilityFault::CrashBeforeMutation))
        .expect("precommit failure receipt");
    assert_eq!(denied.outcome, MutationOutcome::FailedBeforeMutation);
    assert!(adapter.state().durable_log.is_empty());

    let uncertain = adapter
        .append(
            &append_request(DurabilityLevel::ProcessLoss),
            Some(&SimulatedDurabilityFault::ResponseLostAfterCommit),
        )
        .expect("postcommit response loss");
    assert_eq!(uncertain.outcome, MutationOutcome::Uncertain);
    assert!(uncertain.value.clone().collect_simple_record("fabric-durability-transition-v1", None).is_some());
    assert_eq!(adapter.state().durable_log.len(), 1);
    adapter
        .inject_fault(&SimulatedDurabilityFault::DelayCompletion {
            ticks: SIMULATED_DELAY_TICKS,
        })
        .expect("modeled latency");
    assert_eq!(adapter.simulated_ticks(), SIMULATED_DELAY_TICKS);
}

// r[verify molten.fabric_durability.snapshot_recovery]
#[test]
fn deterministic_adapter_corruption_requires_quarantine_and_stale_generation_denies() {
    let simulation = profile(DurableAdapterKind::DeterministicSimulation);
    let mut state = DurableState::empty(descriptor());
    state.snapshots.insert(SNAPSHOT_REF.to_string(), SnapshotRecord {
        kind: SnapshotKind::Checkpoint,
        snapshot_ref: SNAPSHOT_REF.to_string(),
        content_ref: VALUE_REF.to_string(),
        source_namespace: "namespace-a".to_string(),
        source_generation: GENERATION,
        value_schema_ref: VALUE_SCHEMA_REF.to_string(),
        covered_log_sequence: None,
        ordered_state_ref: ORDERED_REF.to_string(),
        durability: DurabilityLevel::ProcessLoss,
        corrupted: true,
    });
    let decision = evaluate_recovery(&state, &RecoveryInventory {
        active_generation: STALE_GENERATION,
        expected_schema_ref: VALUE_SCHEMA_REF.to_string(),
        permit_repair: false,
        permit_quarantine: true,
    });
    let canonical = canonical_recovery_decision(&simulation, &state, decision).expect("recovery evidence");
    assert_eq!(canonical.decision.disposition, RecoveryDisposition::QuarantineRequired);
    let text = crate::preserves_rail::to_text(&canonical.value).expect("recovery text");
    assert!(text.contains("SnapshotCorrupt"));
    assert!(text.contains("StaleGeneration"));
}

// r[verify molten.fabric_durability.port_contracts]
// r[verify molten.fabric_durability.final_validation]
#[test]
fn registered_effect_port_routes_only_known_commands_to_the_exact_bound_profile() {
    use crate::fabric::FabricPortRequirement;
    use crate::fabric::resolve_canonical_fabric_port_binding;
    use crate::system_extension::EffectTarget;
    use crate::system_extension::FabricEffectPort;
    use crate::system_extension::TypedEffectRequest;

    let simulation = profile(DurableAdapterKind::DeterministicSimulation);
    let port_descriptor = fabric_durability_port_descriptors(&simulation)
        .into_iter()
        .find(|descriptor| descriptor.port_id == FABRIC_DURABLE_LOG_PORT_ID)
        .expect("durable log descriptor");
    let binding =
        resolve_canonical_fabric_port_binding(std::slice::from_ref(&port_descriptor), &FabricPortRequirement {
            port_id: port_descriptor.port_id.clone(),
            version: port_descriptor.version.clone(),
            class: port_descriptor.class,
            operation_classes: port_descriptor.operation_classes.clone(),
            input_schema_refs: port_descriptor.input_schema_refs.clone(),
            output_schema_refs: port_descriptor.output_schema_refs.clone(),
            allowed_authorities: port_descriptor.authority_requirements.clone(),
            available_resources: port_descriptor.resource_requirements.clone(),
            expected_determinism: port_descriptor.determinism,
            expected_replay: port_descriptor.replay,
            expected_profile: port_descriptor.implementation_profile.clone(),
        })
        .expect("canonical durability binding");
    let adapter = SimulatedDurableStateAdapter::new(simulation, descriptor()).expect("simulation adapter");
    let mut port = RegisteredDurableEffectPort::new(adapter);
    port.register(OPERATION_REF.to_string(), DurablePortCommand::Append(append_request(DurabilityLevel::ProcessLoss)))
        .expect("register durable request");
    let effect = TypedEffectRequest {
        target: EffectTarget::FabricPort(binding.binding.key.clone()),
        operation: "append".to_string(),
        input_schema_ref: DURABLE_STATE_OPERATION_SCHEMA.to_string(),
        output_schema_ref: DURABLE_STATE_OUTCOME_SCHEMA.to_string(),
        request_ref: OPERATION_REF.to_string(),
        generation: GENERATION,
        accounted_bytes: 1,
    };
    let output = port.route(&binding, &effect).expect("route durable effect");
    assert!(output.output_ref.starts_with("blake3:"));
    assert_eq!(port.adapter().state().durable_log.len(), 1);

    let mut unknown = effect;
    unknown.request_ref = VALUE_REF.to_string();
    assert!(port.route(&binding, &unknown).is_err());
}

// r[verify molten.fabric_durability.port_contracts]
#[test]
fn extension_context_denies_stale_generation_unbound_port_and_profile_substitution() {
    let simulation = profile(DurableAdapterKind::DeterministicSimulation);
    let context = ExtensionDurabilityContext::from_test_snapshot("service-a", GENERATION, &simulation, vec![
        FABRIC_DURABLE_LOG_PORT_ID.to_string(),
    ]);
    assert!(context.admit_operation(&simulation, FABRIC_DURABLE_LOG_PORT_ID, "service-a", GENERATION, 1).is_ok());
    assert!(
        context
            .admit_operation(&simulation, FABRIC_DURABLE_LOG_PORT_ID, "service-a", STALE_GENERATION, 1)
            .is_err()
    );
    assert!(
        context
            .admit_operation(&simulation, FABRIC_ORDERED_STORE_PORT_ID, "service-a", GENERATION, 1)
            .is_err()
    );
    let live = profile(DurableAdapterKind::LiveRedb);
    assert!(context.admit_operation(&live, FABRIC_DURABLE_LOG_PORT_ID, "service-a", GENERATION, 1).is_err());
}
