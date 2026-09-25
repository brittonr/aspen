use super::*;

const SINGLE_AFFECTED_ITEM: u64 = 1;
const ADJACENT_PAIR_WIDTH: usize = 2;

// r[impl molten.fabric_durability.durable_log]
pub fn append_log(
    profile: &DurableStateProfile,
    state: &DurableState,
    request: &AppendRequest,
) -> Result<DurableTransition, Vec<DurabilityIssue>> {
    let mut issues = state_issues(profile, state);
    validate_request_scope(state, request, &mut issues);
    validate_level(profile, &state.descriptor.atomicity_domain, request.durability, &mut issues);
    validate_payload(profile, state, &request.value, &request.value_ref, &mut issues);
    match state.next_log_sequence() {
        Ok(expected_sequence) if request.expected_sequence != expected_sequence => {
            issues.push(DurabilityIssue::SequenceMismatch {
                expected: expected_sequence,
                actual: request.expected_sequence,
            });
        }
        Err(issue) => issues.push(issue),
        Ok(_) => {}
    }
    let current_records = state
        .buffered_log
        .len()
        .checked_add(state.durable_log.len())
        .ok_or_else(|| vec![DurabilityIssue::CollectionLimitExceeded])?;
    let actual_records = u64::try_from(current_records)
        .ok()
        .and_then(|count| count.checked_add(SINGLE_AFFECTED_ITEM))
        .ok_or_else(|| vec![DurabilityIssue::CollectionLimitExceeded])?;
    if actual_records > profile.max_log_records {
        issues.push(DurabilityIssue::OperationLimitExceeded {
            actual: actual_records,
            maximum: profile.max_log_records,
        });
    }
    if !issues.is_empty() {
        return Err(issues);
    }

    let value_bytes = u64::try_from(request.value.len()).map_err(|_| vec![DurabilityIssue::CollectionLimitExceeded])?;
    let mut next = state.clone();
    let record = LogRecord {
        sequence: request.expected_sequence,
        value: request.value.clone(),
        value_ref: request.value_ref.clone(),
        durability: request.durability,
    };
    next.buffered_log.push(record);
    next.buffered_bytes = checked_add_bytes(next.buffered_bytes, value_bytes)?;
    let outcome = if request.durability.is_durable() {
        for buffered in &mut next.buffered_log {
            buffered.durability = request.durability;
        }
        next.durable_log.append(&mut next.buffered_log);
        next.durable_bytes = checked_add_bytes(next.durable_bytes, next.buffered_bytes)?;
        next.buffered_bytes = 0;
        MutationOutcome::Durable
    } else {
        MutationOutcome::Buffered
    };
    ensure_quota(&next)?;
    Ok(DurableTransition {
        next,
        outcome,
        operation: "append-log".to_string(),
        affected_items: SINGLE_AFFECTED_ITEM,
        affected_bytes: value_bytes,
        retry_safe: false,
        reconciliation_required: false,
    })
}

// r[impl molten.fabric_durability.durable_log]
pub fn flush_log(
    profile: &DurableStateProfile,
    state: &DurableState,
    generation: u64,
    durability: DurabilityLevel,
) -> Result<DurableTransition, Vec<DurabilityIssue>> {
    let mut issues = state_issues(profile, state);
    validate_generation(state, generation, &mut issues);
    validate_level(profile, &state.descriptor.atomicity_domain, durability, &mut issues);
    if !durability.is_durable() {
        issues.push(DurabilityIssue::UnsupportedDurability(durability));
    }
    if !issues.is_empty() {
        return Err(issues);
    }

    let mut next = state.clone();
    let affected_items =
        u64::try_from(next.buffered_log.len()).map_err(|_| vec![DurabilityIssue::CollectionLimitExceeded])?;
    let affected_bytes = next.buffered_bytes;
    for record in &mut next.buffered_log {
        record.durability = durability;
    }
    next.durable_log.append(&mut next.buffered_log);
    next.durable_bytes = checked_add_bytes(next.durable_bytes, affected_bytes)?;
    next.buffered_bytes = 0;
    Ok(DurableTransition {
        next,
        outcome: MutationOutcome::Durable,
        operation: "flush-log".to_string(),
        affected_items,
        affected_bytes,
        retry_safe: true,
        reconciliation_required: false,
    })
}

// r[impl molten.fabric_durability.durable_log]
pub fn truncate_log(
    profile: &DurableStateProfile,
    state: &DurableState,
    generation: u64,
    retain_from_sequence: u64,
    authority_ref: Option<&str>,
) -> Result<DurableTransition, Vec<DurabilityIssue>> {
    let mut issues = state_issues(profile, state);
    validate_generation(state, generation, &mut issues);
    if state.descriptor.retention_authority_ref.as_deref() != authority_ref {
        issues.push(DurabilityIssue::RetentionAuthorityRequired);
    }
    if !issues.is_empty() {
        return Err(issues);
    }

    let mut next = state.clone();
    let before = next.durable_log.len();
    let removed_bytes = next.durable_log.iter().filter(|record| record.sequence < retain_from_sequence).try_fold(
        0u64,
        |total, record| {
            let bytes =
                u64::try_from(record.value.len()).map_err(|_| vec![DurabilityIssue::CollectionLimitExceeded])?;
            checked_add_bytes(total, bytes)
        },
    )?;
    next.durable_log.retain(|record| record.sequence >= retain_from_sequence);
    next.durable_bytes = next.durable_bytes.checked_sub(removed_bytes).ok_or_else(|| {
        vec![DurabilityIssue::ByteLimitExceeded {
            actual: removed_bytes,
            maximum: state.durable_bytes,
        }]
    })?;
    let affected_items = u64::try_from(before.saturating_sub(next.durable_log.len()))
        .map_err(|_| vec![DurabilityIssue::CollectionLimitExceeded])?;
    Ok(DurableTransition {
        next,
        outcome: MutationOutcome::Durable,
        operation: "truncate-log".to_string(),
        affected_items,
        affected_bytes: removed_bytes,
        retry_safe: true,
        reconciliation_required: false,
    })
}

// r[impl molten.fabric_durability.durable_log]
pub fn read_log(state: &DurableState, sequence: u64) -> Option<&LogRecord> {
    state.durable_log.iter().chain(state.buffered_log.iter()).find(|record| record.sequence == sequence)
}

// r[impl molten.fabric_durability.durable_log]
pub fn scan_log(state: &DurableState, start_sequence: u64, limit: u64) -> Result<LogScanPage, DurabilityIssue> {
    if limit == 0 {
        return Err(DurabilityIssue::ZeroLimit("log-scan-limit"));
    }
    let limit = usize::try_from(limit).map_err(|_| DurabilityIssue::CollectionLimitExceeded)?;
    if limit > MAX_DURABILITY_COLLECTION_ITEMS {
        return Err(DurabilityIssue::CollectionLimitExceeded);
    }
    let mut records = Vec::with_capacity(limit.min(state.durable_log.len().saturating_add(state.buffered_log.len())));
    let mut continuation = None;
    for record in state
        .durable_log
        .iter()
        .chain(state.buffered_log.iter())
        .filter(|record| record.sequence >= start_sequence)
    {
        if records.len() == limit {
            continuation = Some(record.sequence);
            break;
        }
        records.push(record.clone());
    }
    Ok(LogScanPage { records, continuation })
}

pub fn log_tail(state: &DurableState) -> Option<&LogRecord> {
    state.buffered_log.last().or_else(|| state.durable_log.last())
}

// r[impl molten.fabric_durability.ordered_store]
pub fn read_ordered<'a>(state: &'a DurableState, key: &[u8]) -> Option<&'a VersionedValue> {
    state.ordered.get(key)
}

// r[impl molten.fabric_durability.ordered_store]
// r[impl molten.fabric_durability.atomic_batch]
pub fn apply_atomic_batch(
    profile: &DurableStateProfile,
    state: &DurableState,
    request: &AtomicBatchRequest,
) -> Result<DurableTransition, Vec<DurabilityIssue>> {
    let mut issues = state_issues(profile, state);
    validate_batch_domain(state, request, &mut issues);
    validate_level(profile, &request.domain, request.durability, &mut issues);
    let operation_count =
        u64::try_from(request.mutations.len()).map_err(|_| vec![DurabilityIssue::CollectionLimitExceeded])?;
    if operation_count == 0 {
        issues.push(DurabilityIssue::EmptyField("batch-mutations"));
    }
    let maximum_operations = request.domain.max_operations.min(profile.max_batch_operations);
    if operation_count > maximum_operations {
        issues.push(DurabilityIssue::OperationLimitExceeded {
            actual: operation_count,
            maximum: maximum_operations,
        });
    }
    let batch_bytes = match validate_mutations(profile, state, &request.mutations, &mut issues) {
        Ok(bytes) => bytes,
        Err(issue) => {
            issues.push(issue);
            return Err(issues);
        }
    };
    let maximum_bytes = request.domain.max_bytes.min(profile.max_operation_bytes);
    if batch_bytes > maximum_bytes {
        issues.push(DurabilityIssue::ByteLimitExceeded {
            actual: batch_bytes,
            maximum: maximum_bytes,
        });
    }
    if !issues.is_empty() {
        return Err(issues);
    }

    let mut next = state.clone();
    let durable_bytes_before = next.durable_bytes;
    for mutation in &request.mutations {
        apply_ordered_mutation(&mut next, mutation)?;
    }
    let ordered_entry_count =
        u64::try_from(next.ordered.len()).map_err(|_| vec![DurabilityIssue::CollectionLimitExceeded])?;
    if ordered_entry_count > profile.max_ordered_entries {
        return Err(vec![DurabilityIssue::OperationLimitExceeded {
            actual: ordered_entry_count,
            maximum: profile.max_ordered_entries,
        }]);
    }
    next.durable_bytes = ordered_bytes(&next)?
        .checked_add(log_bytes(&next.durable_log)?)
        .ok_or_else(|| vec![DurabilityIssue::CollectionLimitExceeded])?;
    ensure_quota(&next)?;
    let changed_bytes = next.durable_bytes.abs_diff(durable_bytes_before);
    Ok(DurableTransition {
        next,
        outcome: if request.durability.is_durable() {
            MutationOutcome::Durable
        } else {
            MutationOutcome::Buffered
        },
        operation: "atomic-ordered-batch".to_string(),
        affected_items: operation_count,
        affected_bytes: changed_bytes,
        retry_safe: false,
        reconciliation_required: false,
    })
}

// r[impl molten.fabric_durability.ordered_store]
pub fn scan_ordered(state: &DurableState, request: &OrderedScanRequest) -> Result<OrderedScanPage, DurabilityIssue> {
    if request.limit == 0 {
        return Err(DurabilityIssue::ZeroLimit("scan-limit"));
    }
    if let (Some(start), Some(end)) = (&request.start_inclusive, &request.end_exclusive)
        && start >= end
    {
        return Err(DurabilityIssue::KeyRangeInvalid);
    }
    let limit = usize::try_from(request.limit).map_err(|_| DurabilityIssue::CollectionLimitExceeded)?;
    if limit > MAX_DURABILITY_COLLECTION_ITEMS {
        return Err(DurabilityIssue::CollectionLimitExceeded);
    }
    let start = request.start_inclusive.as_ref().map_or(std::ops::Bound::Unbounded, std::ops::Bound::Included);
    let end = request.end_exclusive.as_ref().map_or(std::ops::Bound::Unbounded, std::ops::Bound::Excluded);
    let mut entries = Vec::with_capacity(limit.min(state.ordered.len()));
    let mut continuation = None;
    for (key, value) in state.ordered.range::<Vec<u8>, _>((start, end)) {
        if entries.len() == limit {
            continuation = Some(key.clone());
            break;
        }
        entries.push((key.clone(), value.clone()));
    }
    Ok(OrderedScanPage { entries, continuation })
}

// r[impl molten.fabric_durability.snapshot_recovery]
pub fn create_snapshot(
    profile: &DurableStateProfile,
    state: &DurableState,
    request: &SnapshotRequest,
) -> Result<DurableTransition, Vec<DurabilityIssue>> {
    let mut issues = state_issues(profile, state);
    validate_generation(state, request.generation, &mut issues);
    validate_level(profile, &state.descriptor.atomicity_domain, request.durability, &mut issues);
    for (label, content_ref) in [
        ("snapshot-ref", request.snapshot_ref.as_str()),
        ("snapshot-content-ref", request.content_ref.as_str()),
        ("ordered-state-ref", request.ordered_state_ref.as_str()),
    ] {
        if !crate::fabric::valid_blake3_ref(content_ref) {
            issues.push(DurabilityIssue::MalformedContentRef(label));
        }
    }
    let next_snapshot_count = u64::try_from(state.snapshots.len())
        .ok()
        .and_then(|count| count.checked_add(SINGLE_AFFECTED_ITEM))
        .ok_or_else(|| vec![DurabilityIssue::CollectionLimitExceeded])?;
    if next_snapshot_count > profile.max_snapshots {
        issues.push(DurabilityIssue::SnapshotLimitExceeded);
    }
    if state.snapshots.contains_key(&request.snapshot_ref) {
        issues.push(DurabilityIssue::DuplicateValue("snapshot-ref"));
    }
    if let Some(sequence) = request.covered_log_sequence
        && !state.durable_log.iter().any(|record| record.sequence == sequence)
    {
        issues.push(DurabilityIssue::SequenceMismatch {
            expected: state.durable_log.last().map_or(0, |record| record.sequence),
            actual: sequence,
        });
    }
    if !issues.is_empty() {
        return Err(issues);
    }

    let mut next = state.clone();
    next.snapshots.insert(request.snapshot_ref.clone(), SnapshotRecord {
        kind: request.kind,
        snapshot_ref: request.snapshot_ref.clone(),
        content_ref: request.content_ref.clone(),
        source_namespace: state.descriptor.namespace_id.clone(),
        source_generation: request.generation,
        value_schema_ref: state.descriptor.value_schema_ref.clone(),
        covered_log_sequence: request.covered_log_sequence,
        ordered_state_ref: request.ordered_state_ref.clone(),
        durability: request.durability,
        corrupted: false,
    });
    Ok(DurableTransition {
        next,
        outcome: if request.durability.is_durable() {
            MutationOutcome::Durable
        } else {
            MutationOutcome::Buffered
        },
        operation: "create-snapshot".to_string(),
        affected_items: SINGLE_AFFECTED_ITEM,
        affected_bytes: 0,
        retry_safe: false,
        reconciliation_required: false,
    })
}

// r[impl molten.fabric_durability.snapshot_recovery]
pub fn plan_snapshot_restore(
    state: &DurableState,
    snapshot_ref: &str,
    target_generation: u64,
    expected_content_ref: &str,
) -> Result<SnapshotRestorePlan, Vec<DurabilityIssue>> {
    let Some(snapshot) = state.snapshots.get(snapshot_ref) else {
        return Err(vec![DurabilityIssue::SnapshotNotFound]);
    };
    let mut issues = Vec::new();
    if snapshot.corrupted || snapshot.content_ref != expected_content_ref {
        issues.push(DurabilityIssue::SnapshotCorrupt);
    }
    if snapshot.value_schema_ref != state.descriptor.value_schema_ref {
        issues.push(DurabilityIssue::SnapshotSchemaMismatch);
    }
    let is_current_or_next = state
        .descriptor
        .generation
        .checked_add(SINGLE_AFFECTED_ITEM)
        .is_some_and(|next| target_generation == state.descriptor.generation || target_generation == next);
    if !is_current_or_next {
        issues.push(DurabilityIssue::GenerationMismatch {
            expected: state.descriptor.generation,
            actual: target_generation,
        });
    }
    if !snapshot.durability.is_durable() {
        issues.push(DurabilityIssue::UnsupportedDurability(snapshot.durability));
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    Ok(SnapshotRestorePlan {
        snapshot: snapshot.clone(),
        target_generation,
        restored_state_ref: snapshot.ordered_state_ref.clone(),
    })
}

// r[impl molten.fabric_durability.effect_transaction]
// r[impl molten.fabric_durability.uncertain_outcomes]
pub fn apply_effect_transaction(
    profile: &DurableStateProfile,
    state: &DurableState,
    command: &EffectTransactionCommand,
) -> Result<DurableTransition, Vec<DurabilityIssue>> {
    let mut issues = state_issues(profile, state);
    let generation = effect_generation(command);
    validate_generation(state, generation, &mut issues);
    if !issues.is_empty() {
        return Err(issues);
    }

    match command {
        EffectTransactionCommand::Reserve {
            transaction_id,
            generation,
            operation_ref,
            expires_at_tick,
            profile: effect_profile,
        } => reserve_effect(
            state,
            ReserveEffect {
                transaction_id,
                generation: *generation,
                operation_ref,
                expires_at_tick: *expires_at_tick,
                effect_profile,
            },
            profile,
        ),
        EffectTransactionCommand::Commit { transaction_id, .. } => {
            transition_effect(state, transaction_id, EffectAction::Commit, None)
        }
        EffectTransactionCommand::Abort { transaction_id, .. } => {
            transition_effect(state, transaction_id, EffectAction::Abort, None)
        }
        EffectTransactionCommand::Expire {
            transaction_id,
            observed_tick,
            ..
        } => transition_effect(state, transaction_id, EffectAction::Expire, Some(*observed_tick)),
        EffectTransactionCommand::MarkUncertain { transaction_id, .. } => {
            transition_effect(state, transaction_id, EffectAction::MarkUncertain, None)
        }
        EffectTransactionCommand::Reconcile {
            transaction_id,
            committed,
            ..
        } => transition_effect(state, transaction_id, EffectAction::Reconcile(*committed), None),
    }
}

// r[impl molten.fabric_durability.snapshot_recovery]
pub fn evaluate_recovery(state: &DurableState, inventory: &RecoveryInventory) -> RecoveryDecision {
    let mut diagnostics = Vec::new();
    if state.descriptor.generation != inventory.active_generation {
        push_recovery_diagnostic(&mut diagnostics, || DurabilityIssue::StaleGeneration {
            active: inventory.active_generation,
            requested: state.descriptor.generation,
        });
    }
    if state.descriptor.value_schema_ref != inventory.expected_schema_ref {
        push_recovery_diagnostic(&mut diagnostics, || DurabilityIssue::SnapshotSchemaMismatch);
    }
    collect_log_gap_diagnostics(&state.durable_log, &mut diagnostics);
    for snapshot in state.snapshots.values() {
        if diagnostics.len() > MAX_DURABILITY_COLLECTION_ITEMS {
            break;
        }
        if snapshot.corrupted {
            push_recovery_diagnostic(&mut diagnostics, || DurabilityIssue::SnapshotCorrupt);
        }
        if snapshot.value_schema_ref != inventory.expected_schema_ref {
            push_recovery_diagnostic(&mut diagnostics, || DurabilityIssue::SnapshotSchemaMismatch);
        }
    }
    for effect in state.effects.values() {
        if diagnostics.len() > MAX_DURABILITY_COLLECTION_ITEMS {
            break;
        }
        if matches!(effect.phase, EffectTransactionPhase::Reserved | EffectTransactionPhase::Uncertain) {
            if effect.transaction_id.len() > MAX_DURABILITY_TEXT_BYTES {
                push_recovery_diagnostic(&mut diagnostics, || DurabilityIssue::CollectionLimitExceeded);
            } else {
                push_recovery_diagnostic(&mut diagnostics, || {
                    DurabilityIssue::UnresolvedEffect(effect.transaction_id.clone())
                });
            }
        }
    }
    let disposition = recovery_disposition(&diagnostics, inventory);
    RecoveryDecision {
        disposition,
        diagnostics,
        durable_log_tail: state.durable_log.last().map(|record| record.sequence),
        snapshot_count: u64::try_from(state.snapshots.len()).unwrap_or(u64::MAX),
        unresolved_effect_count: u64::try_from(
            state
                .effects
                .values()
                .filter(|effect| {
                    matches!(effect.phase, EffectTransactionPhase::Reserved | EffectTransactionPhase::Uncertain)
                })
                .count(),
        )
        .unwrap_or(u64::MAX),
    }
}

// r[impl molten.fabric_durability.live_sim_parity]
pub fn simulate_process_crash(state: &DurableState) -> DurableTransition {
    let mut next = state.clone();
    let affected_items = u64::try_from(next.buffered_log.len()).unwrap_or(u64::MAX);
    let affected_bytes = next.buffered_bytes;
    next.buffered_log.clear();
    next.buffered_bytes = 0;
    DurableTransition {
        next,
        outcome: MutationOutcome::FailedAfterPossibleMutation,
        operation: "simulated-process-crash".to_string(),
        affected_items,
        affected_bytes,
        retry_safe: false,
        reconciliation_required: affected_items > 0,
    }
}

pub fn mark_snapshot_corrupt(state: &DurableState, snapshot_ref: &str) -> Result<DurableState, DurabilityIssue> {
    let mut next = state.clone();
    let snapshot = next.snapshots.get_mut(snapshot_ref).ok_or(DurabilityIssue::SnapshotNotFound)?;
    snapshot.corrupted = true;
    Ok(next)
}

fn state_issues(profile: &DurableStateProfile, state: &DurableState) -> Vec<DurabilityIssue> {
    validate_namespace_descriptor(profile, &state.descriptor).err().unwrap_or_default()
}

fn validate_request_scope(state: &DurableState, request: &AppendRequest, issues: &mut Vec<DurabilityIssue>) {
    if request.adapter_id != state.descriptor.adapter_id {
        issues.push(DurabilityIssue::AdapterMismatch);
    }
    if request.namespace_id != state.descriptor.namespace_id {
        issues.push(DurabilityIssue::NamespaceMismatch);
    }
    validate_generation(state, request.generation, issues);
}

fn validate_generation(state: &DurableState, generation: u64, issues: &mut Vec<DurabilityIssue>) {
    if generation != state.descriptor.generation {
        issues.push(DurabilityIssue::GenerationMismatch {
            expected: state.descriptor.generation,
            actual: generation,
        });
    }
}

fn validate_level(
    profile: &DurableStateProfile,
    domain: &AtomicityDomain,
    level: DurabilityLevel,
    issues: &mut Vec<DurabilityIssue>,
) {
    if !profile.supported_levels.contains(&level) || !domain.supported_levels.contains(&level) {
        issues.push(DurabilityIssue::UnsupportedDurability(level));
    }
}

fn validate_payload(
    profile: &DurableStateProfile,
    state: &DurableState,
    value: &[u8],
    value_ref: &str,
    issues: &mut Vec<DurabilityIssue>,
) {
    if value.is_empty() {
        issues.push(DurabilityIssue::EmptyValue);
    }
    if !crate::fabric::valid_blake3_ref(value_ref) {
        issues.push(DurabilityIssue::MalformedContentRef("value-ref"));
    }
    let Ok(value_bytes) = u64::try_from(value.len()) else {
        issues.push(DurabilityIssue::CollectionLimitExceeded);
        return;
    };
    if value_bytes > profile.max_operation_bytes {
        issues.push(DurabilityIssue::ByteLimitExceeded {
            actual: value_bytes,
            maximum: profile.max_operation_bytes,
        });
    }
    let projected = state
        .buffered_bytes
        .checked_add(state.durable_bytes)
        .and_then(|total| total.checked_add(value_bytes));
    if let Some(projected) = projected {
        let maximum = state.descriptor.quota_bytes.min(profile.max_namespace_bytes);
        if projected > maximum {
            issues.push(DurabilityIssue::NamespaceQuotaExceeded {
                actual: projected,
                maximum,
            });
        }
    } else {
        issues.push(DurabilityIssue::CollectionLimitExceeded);
    }
}

fn validate_batch_domain(state: &DurableState, request: &AtomicBatchRequest, issues: &mut Vec<DurabilityIssue>) {
    if request.domain.adapter_id != state.descriptor.adapter_id {
        issues.push(DurabilityIssue::CrossAdapterBatch);
    }
    if request.domain.namespace_id != state.descriptor.namespace_id {
        issues.push(DurabilityIssue::CrossNamespaceBatch);
    }
    if request.domain != state.descriptor.atomicity_domain {
        issues.push(DurabilityIssue::AtomicityDomainMismatch);
    }
    validate_generation(state, request.generation, issues);
    if !request.domain.object_classes.contains(&DurableObjectClass::OrderedValue) {
        issues.push(DurabilityIssue::AtomicityDomainMismatch);
    }
}

fn validate_mutations(
    profile: &DurableStateProfile,
    state: &DurableState,
    mutations: &[OrderedMutation],
    issues: &mut Vec<DurabilityIssue>,
) -> Result<u64, DurabilityIssue> {
    let mut bytes = 0u64;
    for mutation in mutations {
        let (key, put, precondition) = match mutation {
            OrderedMutation::Put {
                key,
                value,
                value_ref,
                precondition,
            } => (key, Some((value.as_slice(), value_ref.as_str())), precondition),
            OrderedMutation::Delete { key, precondition } => (key, None, precondition),
        };
        if key.is_empty() {
            issues.push(DurabilityIssue::EmptyKey);
        }
        let key_bytes = u64::try_from(key.len()).map_err(|_| DurabilityIssue::CollectionLimitExceeded)?;
        bytes = bytes.checked_add(key_bytes).ok_or(DurabilityIssue::CollectionLimitExceeded)?;
        if let Some((value, value_ref)) = put {
            validate_payload(profile, state, value, value_ref, issues);
            let value_bytes = u64::try_from(value.len()).map_err(|_| DurabilityIssue::CollectionLimitExceeded)?;
            bytes = bytes.checked_add(value_bytes).ok_or(DurabilityIssue::CollectionLimitExceeded)?;
        }
        if !precondition_matches(state.ordered.get(key), precondition) {
            issues.push(DurabilityIssue::PreconditionFailed);
        }
    }
    Ok(bytes)
}

fn precondition_matches(current: Option<&VersionedValue>, precondition: &ValuePrecondition) -> bool {
    match precondition {
        ValuePrecondition::Any => true,
        ValuePrecondition::Missing => current.is_none(),
        ValuePrecondition::Version(expected) => current.is_some_and(|value| value.version == *expected),
        ValuePrecondition::ValueRef(expected) => current.is_some_and(|value| value.value_ref == *expected),
    }
}

fn apply_ordered_mutation(state: &mut DurableState, mutation: &OrderedMutation) -> Result<(), Vec<DurabilityIssue>> {
    match mutation {
        OrderedMutation::Put {
            key, value, value_ref, ..
        } => {
            let version = state
                .ordered
                .get(key)
                .map_or(Ok(SINGLE_AFFECTED_ITEM), |current| {
                    current.version.checked_add(SINGLE_AFFECTED_ITEM).ok_or(DurabilityIssue::VersionOverflow)
                })
                .map_err(|issue| vec![issue])?;
            state.ordered.insert(key.clone(), VersionedValue {
                value: value.clone(),
                value_ref: value_ref.clone(),
                version,
            });
        }
        OrderedMutation::Delete { key, .. } => {
            state.ordered.remove(key);
        }
    }
    Ok(())
}

struct ReserveEffect<'a> {
    transaction_id: &'a str,
    generation: u64,
    operation_ref: &'a str,
    expires_at_tick: Option<u64>,
    effect_profile: &'a EffectTransactionProfile,
}

fn reserve_effect(
    state: &DurableState,
    request: ReserveEffect<'_>,
    profile: &DurableStateProfile,
) -> Result<DurableTransition, Vec<DurabilityIssue>> {
    let mut issues = Vec::new();
    if request.transaction_id.is_empty() {
        issues.push(DurabilityIssue::EmptyField("effect-transaction-id"));
    }
    if !crate::fabric::valid_blake3_ref(request.operation_ref) {
        issues.push(DurabilityIssue::MalformedContentRef("effect-operation-ref"));
    }
    if state.effects.contains_key(request.transaction_id) {
        issues.push(DurabilityIssue::EffectAlreadyExists);
    }
    let projected = u64::try_from(state.effects.len())
        .ok()
        .and_then(|count| count.checked_add(SINGLE_AFFECTED_ITEM))
        .ok_or_else(|| vec![DurabilityIssue::CollectionLimitExceeded])?;
    if projected > profile.max_effect_transactions {
        issues.push(DurabilityIssue::EffectLimitExceeded);
    }
    if request.effect_profile.expiring != request.expires_at_tick.is_some() {
        issues.push(DurabilityIssue::MalformedField("effect-expiry"));
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    let mut next = state.clone();
    next.effects.insert(request.transaction_id.to_string(), EffectTransactionState {
        transaction_id: request.transaction_id.to_string(),
        generation: request.generation,
        operation_ref: request.operation_ref.to_string(),
        phase: EffectTransactionPhase::Reserved,
        expires_at_tick: request.expires_at_tick,
        profile: request.effect_profile.clone(),
    });
    Ok(effect_transition(next, "effect-reserve", MutationOutcome::Durable, false))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EffectAction {
    Commit,
    Abort,
    Expire,
    MarkUncertain,
    Reconcile(bool),
}

fn transition_effect(
    state: &DurableState,
    transaction_id: &str,
    action: EffectAction,
    observed_tick: Option<u64>,
) -> Result<DurableTransition, Vec<DurabilityIssue>> {
    let Some(current) = state.effects.get(transaction_id) else {
        return Err(vec![DurabilityIssue::EffectNotFound]);
    };
    if current.phase.is_terminal() {
        let is_duplicate_action = matches!(
            (current.phase, action),
            (EffectTransactionPhase::Committed, EffectAction::Commit)
                | (EffectTransactionPhase::Aborted, EffectAction::Abort)
                | (EffectTransactionPhase::Expired, EffectAction::Expire)
                | (EffectTransactionPhase::ReconciledCommitted, EffectAction::Reconcile(true))
                | (EffectTransactionPhase::ReconciledAborted, EffectAction::Reconcile(false))
        );
        if is_duplicate_action && current.profile.idempotent_commit {
            return Ok(effect_transition(
                state.clone(),
                "effect-duplicate-terminal",
                MutationOutcome::DuplicateTerminal,
                false,
            ));
        }
        return Err(vec![DurabilityIssue::EffectTerminal(current.phase)]);
    }

    let next_phase = match action {
        EffectAction::Commit if current.phase == EffectTransactionPhase::Reserved => EffectTransactionPhase::Committed,
        EffectAction::Abort if current.phase == EffectTransactionPhase::Reserved => EffectTransactionPhase::Aborted,
        EffectAction::Expire if current.phase == EffectTransactionPhase::Reserved => {
            let Some(expires_at) = current.expires_at_tick else {
                return Err(vec![DurabilityIssue::EffectNotExpired]);
            };
            if observed_tick.is_none_or(|tick| tick < expires_at) {
                return Err(vec![DurabilityIssue::EffectNotExpired]);
            }
            EffectTransactionPhase::Expired
        }
        EffectAction::MarkUncertain if current.phase == EffectTransactionPhase::Reserved => {
            EffectTransactionPhase::Uncertain
        }
        EffectAction::Reconcile(committed) if current.phase == EffectTransactionPhase::Uncertain => {
            if committed {
                EffectTransactionPhase::ReconciledCommitted
            } else {
                EffectTransactionPhase::ReconciledAborted
            }
        }
        _ => return Err(vec![DurabilityIssue::EffectReconciliationRequired]),
    };
    let mut next = state.clone();
    let effect = next.effects.get_mut(transaction_id).ok_or_else(|| vec![DurabilityIssue::EffectNotFound])?;
    effect.phase = next_phase;
    let is_uncertain = next_phase == EffectTransactionPhase::Uncertain;
    Ok(effect_transition(
        next,
        match action {
            EffectAction::Commit => "effect-commit",
            EffectAction::Abort => "effect-abort",
            EffectAction::Expire => "effect-expire",
            EffectAction::MarkUncertain => "effect-uncertain",
            EffectAction::Reconcile(_) => "effect-reconcile",
        },
        if is_uncertain {
            MutationOutcome::Uncertain
        } else {
            MutationOutcome::Durable
        },
        is_uncertain,
    ))
}

fn effect_transition(
    next: DurableState,
    operation: &str,
    outcome: MutationOutcome,
    reconciliation_required: bool,
) -> DurableTransition {
    DurableTransition {
        next,
        outcome,
        operation: operation.to_string(),
        affected_items: SINGLE_AFFECTED_ITEM,
        affected_bytes: 0,
        retry_safe: outcome == MutationOutcome::DuplicateTerminal,
        reconciliation_required,
    }
}

const fn effect_generation(command: &EffectTransactionCommand) -> u64 {
    match command {
        EffectTransactionCommand::Reserve { generation, .. }
        | EffectTransactionCommand::Commit { generation, .. }
        | EffectTransactionCommand::Abort { generation, .. }
        | EffectTransactionCommand::Expire { generation, .. }
        | EffectTransactionCommand::MarkUncertain { generation, .. }
        | EffectTransactionCommand::Reconcile { generation, .. } => *generation,
    }
}

fn collect_log_gap_diagnostics(log: &[LogRecord], diagnostics: &mut Vec<DurabilityIssue>) {
    for pair in log.windows(ADJACENT_PAIR_WIDTH) {
        if diagnostics.len() > MAX_DURABILITY_COLLECTION_ITEMS {
            break;
        }
        let Some(expected) = pair[0].sequence.checked_add(SINGLE_AFFECTED_ITEM) else {
            push_recovery_diagnostic(diagnostics, || DurabilityIssue::SequenceOverflow);
            continue;
        };
        if pair[1].sequence != expected {
            push_recovery_diagnostic(diagnostics, || DurabilityIssue::LogGap {
                expected,
                actual: pair[1].sequence,
            });
        }
    }
}

fn push_recovery_diagnostic(diagnostics: &mut Vec<DurabilityIssue>, issue: impl FnOnce() -> DurabilityIssue) {
    if diagnostics.len() < MAX_DURABILITY_COLLECTION_ITEMS {
        diagnostics.push(issue());
    } else if diagnostics.len() == MAX_DURABILITY_COLLECTION_ITEMS {
        diagnostics.push(DurabilityIssue::CollectionLimitExceeded);
    }
}

fn recovery_disposition(diagnostics: &[DurabilityIssue], inventory: &RecoveryInventory) -> RecoveryDisposition {
    if diagnostics.is_empty() {
        return RecoveryDisposition::Admit;
    }
    if diagnostics.contains(&DurabilityIssue::CollectionLimitExceeded) {
        return RecoveryDisposition::Deny;
    }
    let has_corruption = diagnostics.iter().any(|issue| {
        matches!(
            issue,
            DurabilityIssue::SnapshotCorrupt | DurabilityIssue::LogGap { .. } | DurabilityIssue::SequenceOverflow
        )
    });
    let has_uncertain = diagnostics.iter().any(|issue| matches!(issue, DurabilityIssue::UnresolvedEffect(_)));
    if has_corruption && inventory.permit_quarantine {
        RecoveryDisposition::QuarantineRequired
    } else if has_uncertain && inventory.permit_repair {
        RecoveryDisposition::RepairRequired
    } else {
        RecoveryDisposition::Deny
    }
}

fn checked_add_bytes(left: u64, right: u64) -> Result<u64, Vec<DurabilityIssue>> {
    left.checked_add(right).ok_or_else(|| vec![DurabilityIssue::CollectionLimitExceeded])
}

fn ensure_quota(state: &DurableState) -> Result<(), Vec<DurabilityIssue>> {
    let total = checked_add_bytes(state.buffered_bytes, state.durable_bytes)?;
    if total > state.descriptor.quota_bytes {
        return Err(vec![DurabilityIssue::NamespaceQuotaExceeded {
            actual: total,
            maximum: state.descriptor.quota_bytes,
        }]);
    }
    Ok(())
}

fn ordered_bytes(state: &DurableState) -> Result<u64, Vec<DurabilityIssue>> {
    state.ordered.iter().try_fold(0u64, |total, (key, value)| {
        let key_bytes = u64::try_from(key.len()).map_err(|_| vec![DurabilityIssue::CollectionLimitExceeded])?;
        let value_bytes =
            u64::try_from(value.value.len()).map_err(|_| vec![DurabilityIssue::CollectionLimitExceeded])?;
        checked_add_bytes(checked_add_bytes(total, key_bytes)?, value_bytes)
    })
}

fn log_bytes(log: &[LogRecord]) -> Result<u64, Vec<DurabilityIssue>> {
    log.iter().try_fold(0u64, |total, record| {
        let bytes = u64::try_from(record.value.len()).map_err(|_| vec![DurabilityIssue::CollectionLimitExceeded])?;
        checked_add_bytes(total, bytes)
    })
}
