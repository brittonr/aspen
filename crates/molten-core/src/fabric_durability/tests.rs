use super::*;

const PROFILE_REF: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const VALUE_SCHEMA_REF: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const VALUE_REF: &str = "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const OTHER_VALUE_REF: &str = "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const SNAPSHOT_REF: &str = "blake3:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const ORDERED_REF: &str = "blake3:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
const AUTHORITY_REF: &str = "blake3:1111111111111111111111111111111111111111111111111111111111111111";
const OPERATION_REF: &str = "blake3:2222222222222222222222222222222222222222222222222222222222222222";
const GENERATION: u64 = 1;
const STALE_GENERATION: u64 = 2;
const PROFILE_LIMIT: u64 = 16;
const OPERATION_BYTE_LIMIT: u64 = 128;
const NAMESPACE_BYTE_LIMIT: u64 = 1_024;
const FIRST_SEQUENCE: u64 = 0;
const SECOND_SEQUENCE: u64 = 1;
const EXPIRY_TICK: u64 = 8;
const EXPECTED_ORDERED_ENTRY_COUNT: usize = 2;

fn profile(kind: DurableAdapterKind) -> DurableStateProfile {
    DurableStateProfile {
        schema: DURABLE_STATE_PROFILE_SCHEMA.to_string(),
        profile_id: kind.as_str().to_string(),
        profile_ref: PROFILE_REF.to_string(),
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
    }
}

fn descriptor() -> DurableNamespaceDescriptor {
    DurableNamespaceDescriptor {
        schema: DURABLE_STATE_NAMESPACE_SCHEMA.to_string(),
        profile_ref: PROFILE_REF.to_string(),
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

fn state() -> DurableState {
    DurableState::empty(descriptor())
}

fn append_request(sequence: u64, durability: DurabilityLevel, value_ref: &str) -> AppendRequest {
    AppendRequest {
        adapter_id: "adapter-a".to_string(),
        namespace_id: "namespace-a".to_string(),
        generation: GENERATION,
        expected_sequence: sequence,
        value: format!("record-{sequence}").into_bytes(),
        value_ref: value_ref.to_string(),
        durability,
    }
}

// r[verify molten.fabric_durability.port_contracts]
#[test]
fn profile_and_namespace_require_complete_bounded_contracts() {
    let live = profile(DurableAdapterKind::LiveRedb);
    assert!(validate_namespace_descriptor(&live, &descriptor()).is_ok());

    let mut invalid = live;
    invalid.non_claims.pop();
    invalid.max_batch_operations = 0;
    let issues = validate_durable_profile(&invalid).expect_err("invalid profile must deny");
    assert!(issues.iter().any(|issue| matches!(issue, DurabilityIssue::MissingNonClaim(_))));
    assert!(issues.contains(&DurabilityIssue::ZeroLimit("max-batch-operations")));
}

// r[verify molten.fabric_durability.durable_log]
// r[verify molten.fabric_durability.live_sim_parity]
#[test]
fn buffered_append_requires_flush_and_simulated_crash_loses_only_buffered_records() {
    let profile = profile(DurableAdapterKind::DeterministicSimulation);
    let buffered =
        append_log(&profile, &state(), &append_request(FIRST_SEQUENCE, DurabilityLevel::Buffered, VALUE_REF))
            .expect("buffered append");
    assert_eq!(buffered.outcome, MutationOutcome::Buffered);
    assert_eq!(buffered.next.buffered_log.len(), 1);
    assert!(buffered.next.durable_log.is_empty());

    let crashed = simulate_process_crash(&buffered.next);
    assert!(crashed.next.buffered_log.is_empty());
    assert!(crashed.next.durable_log.is_empty());
    assert!(crashed.reconciliation_required);

    let flushed =
        flush_log(&profile, &buffered.next, GENERATION, DurabilityLevel::ProcessLoss).expect("flush buffered record");
    let after_crash = simulate_process_crash(&flushed.next);
    assert_eq!(after_crash.next.durable_log.len(), 1);
    assert_eq!(after_crash.next.durable_log[0].sequence, FIRST_SEQUENCE);
}

// r[verify molten.fabric_durability.durable_log]
#[test]
fn log_rejects_stale_generation_sequence_drift_and_unauthorized_truncation() {
    let profile = profile(DurableAdapterKind::DeterministicSimulation);
    let first =
        append_log(&profile, &state(), &append_request(FIRST_SEQUENCE, DurabilityLevel::ProcessLoss, VALUE_REF))
            .expect("first append");
    let mut stale = append_request(FIRST_SEQUENCE, DurabilityLevel::ProcessLoss, OTHER_VALUE_REF);
    stale.generation = STALE_GENERATION;
    let issues = append_log(&profile, &first.next, &stale).expect_err("stale and duplicate sequence must deny");
    assert!(issues.iter().any(|issue| matches!(issue, DurabilityIssue::GenerationMismatch { .. })));
    assert!(issues.iter().any(|issue| matches!(issue, DurabilityIssue::SequenceMismatch { .. })));
    assert!(truncate_log(&profile, &first.next, GENERATION, SECOND_SEQUENCE, None).is_err());
    assert!(truncate_log(&profile, &first.next, GENERATION, SECOND_SEQUENCE, Some(AUTHORITY_REF)).is_ok());
}

// r[verify molten.fabric_durability.ordered_store]
// r[verify molten.fabric_durability.atomic_batch]
#[test]
fn ordered_batch_is_atomic_ordered_and_preconditioned() {
    let profile = profile(DurableAdapterKind::DeterministicSimulation);
    let request = AtomicBatchRequest {
        domain: descriptor().atomicity_domain,
        generation: GENERATION,
        mutations: vec![
            OrderedMutation::Put {
                key: b"b".to_vec(),
                value: b"second".to_vec(),
                value_ref: OTHER_VALUE_REF.to_string(),
                precondition: ValuePrecondition::Missing,
            },
            OrderedMutation::Put {
                key: b"a".to_vec(),
                value: b"first".to_vec(),
                value_ref: VALUE_REF.to_string(),
                precondition: ValuePrecondition::Missing,
            },
        ],
        durability: DurabilityLevel::ProcessLoss,
    };
    let transition = apply_atomic_batch(&profile, &state(), &request).expect("atomic batch");
    let page = scan_ordered(&transition.next, &OrderedScanRequest {
        start_inclusive: None,
        end_exclusive: None,
        limit: PROFILE_LIMIT,
    })
    .expect("ordered scan");
    assert_eq!(page.entries.iter().map(|entry| entry.0.clone()).collect::<Vec<_>>(), vec![
        b"a".to_vec(),
        b"b".to_vec()
    ]);

    let mut denied = request;
    denied.mutations[0] = OrderedMutation::Delete {
        key: b"b".to_vec(),
        precondition: ValuePrecondition::Version(STALE_GENERATION),
    };
    assert!(apply_atomic_batch(&profile, &transition.next, &denied).is_err());
    assert_eq!(transition.next.ordered.len(), EXPECTED_ORDERED_ENTRY_COUNT);
}

// r[verify molten.fabric_durability.atomic_batch]
#[test]
fn batch_rejects_cross_adapter_domain_before_mutation() {
    let profile = profile(DurableAdapterKind::DeterministicSimulation);
    let mut domain = descriptor().atomicity_domain;
    domain.adapter_id = "adapter-b".to_string();
    let request = AtomicBatchRequest {
        domain,
        generation: GENERATION,
        mutations: vec![OrderedMutation::Put {
            key: b"key".to_vec(),
            value: b"value".to_vec(),
            value_ref: VALUE_REF.to_string(),
            precondition: ValuePrecondition::Any,
        }],
        durability: DurabilityLevel::ProcessLoss,
    };
    let issues = apply_atomic_batch(&profile, &state(), &request).expect_err("cross-adapter batch must deny");
    assert!(issues.contains(&DurabilityIssue::CrossAdapterBatch));
    assert!(state().ordered.is_empty());
}

// r[verify molten.fabric_durability.snapshot_recovery]
#[test]
fn snapshot_recovery_admits_valid_state_and_quarantines_corruption() {
    let profile = profile(DurableAdapterKind::DeterministicSimulation);
    let logged =
        append_log(&profile, &state(), &append_request(FIRST_SEQUENCE, DurabilityLevel::ProcessLoss, VALUE_REF))
            .expect("durable log");
    let snapshotted = create_snapshot(&profile, &logged.next, &SnapshotRequest {
        kind: SnapshotKind::Checkpoint,
        generation: GENERATION,
        snapshot_ref: SNAPSHOT_REF.to_string(),
        content_ref: VALUE_REF.to_string(),
        ordered_state_ref: ORDERED_REF.to_string(),
        covered_log_sequence: Some(FIRST_SEQUENCE),
        durability: DurabilityLevel::ProcessLoss,
    })
    .expect("snapshot");
    let inventory = RecoveryInventory {
        active_generation: GENERATION,
        expected_schema_ref: VALUE_SCHEMA_REF.to_string(),
        permit_repair: false,
        permit_quarantine: true,
    };
    assert_eq!(evaluate_recovery(&snapshotted.next, &inventory).disposition, RecoveryDisposition::Admit);
    let corrupt = mark_snapshot_corrupt(&snapshotted.next, SNAPSHOT_REF).expect("mark corruption");
    assert_eq!(evaluate_recovery(&corrupt, &inventory).disposition, RecoveryDisposition::QuarantineRequired);
}

// r[verify molten.fabric_durability.effect_transaction]
// r[verify molten.fabric_durability.uncertain_outcomes]
#[test]
fn effect_transactions_are_idempotent_and_uncertainty_requires_reconciliation() {
    let profile = profile(DurableAdapterKind::DeterministicSimulation);
    let reserved = apply_effect_transaction(&profile, &state(), &EffectTransactionCommand::Reserve {
        transaction_id: "effect-a".to_string(),
        generation: GENERATION,
        operation_ref: OPERATION_REF.to_string(),
        expires_at_tick: Some(EXPIRY_TICK),
        profile: EffectTransactionProfile {
            durable_reservation: true,
            exclusive: true,
            expiring: true,
            idempotent_commit: true,
            compensating_abort: false,
        },
    })
    .expect("reserve effect");
    let uncertain = apply_effect_transaction(&profile, &reserved.next, &EffectTransactionCommand::MarkUncertain {
        transaction_id: "effect-a".to_string(),
        generation: GENERATION,
    })
    .expect("mark uncertain");
    assert_eq!(uncertain.outcome, MutationOutcome::Uncertain);
    assert!(uncertain.reconciliation_required);
    assert!(
        apply_effect_transaction(&profile, &uncertain.next, &EffectTransactionCommand::Commit {
            transaction_id: "effect-a".to_string(),
            generation: GENERATION,
        })
        .is_err()
    );
    let reconciled = apply_effect_transaction(&profile, &uncertain.next, &EffectTransactionCommand::Reconcile {
        transaction_id: "effect-a".to_string(),
        generation: GENERATION,
        committed: true,
    })
    .expect("reconcile effect");
    let duplicate = apply_effect_transaction(&profile, &reconciled.next, &EffectTransactionCommand::Reconcile {
        transaction_id: "effect-a".to_string(),
        generation: GENERATION,
        committed: true,
    })
    .expect("idempotent terminal replay");
    assert_eq!(duplicate.outcome, MutationOutcome::DuplicateTerminal);
}

// r[verify molten.fabric_durability.final_validation]
#[test]
fn recovery_denies_stale_generation_schema_drift_and_unresolved_effects() {
    let profile = profile(DurableAdapterKind::DeterministicSimulation);
    let reserved = apply_effect_transaction(&profile, &state(), &EffectTransactionCommand::Reserve {
        transaction_id: "effect-b".to_string(),
        generation: GENERATION,
        operation_ref: OPERATION_REF.to_string(),
        expires_at_tick: None,
        profile: EffectTransactionProfile {
            durable_reservation: true,
            exclusive: false,
            expiring: false,
            idempotent_commit: false,
            compensating_abort: true,
        },
    })
    .expect("reserve effect");
    let decision = evaluate_recovery(&reserved.next, &RecoveryInventory {
        active_generation: STALE_GENERATION,
        expected_schema_ref: OTHER_VALUE_REF.to_string(),
        permit_repair: false,
        permit_quarantine: false,
    });
    assert_eq!(decision.disposition, RecoveryDisposition::Deny);
    assert!(decision.unresolved_effect_count > 0);
    assert!(decision.diagnostics.iter().any(|issue| matches!(issue, DurabilityIssue::StaleGeneration { .. })));
    assert!(decision.diagnostics.iter().any(|issue| matches!(issue, DurabilityIssue::UnresolvedEffect(_))));
}
