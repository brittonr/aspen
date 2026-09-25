
fn load_state(database: &redb::Database, descriptor: DurableNamespaceDescriptor) -> crate::error::Result<DurableState> {
    let mut state = DurableState::empty(descriptor);
    let read = database.begin_read().map_err(adapter_error)?;
    {
        let table = read.open_table(LOG_TABLE).map_err(adapter_error)?;
        for item in table.iter().map_err(adapter_error)? {
            let (sequence, bytes) = item.map_err(adapter_error)?;
            let record = decode_log_record(sequence.value(), bytes.value())?;
            state.durable_bytes = checked_adapter_add(state.durable_bytes, byte_count(record.value.len())?)?;
            state.durable_log.push(record);
        }
    }
    {
        let table = read.open_table(ORDERED_TABLE).map_err(adapter_error)?;
        for item in table.iter().map_err(adapter_error)? {
            let (key, bytes) = item.map_err(adapter_error)?;
            let value = decode_versioned_value(bytes.value())?;
            state.durable_bytes = checked_adapter_add(state.durable_bytes, byte_count(key.value().len())?)?;
            state.durable_bytes = checked_adapter_add(state.durable_bytes, byte_count(value.value.len())?)?;
            state.ordered.insert(key.value().to_vec(), value);
        }
    }
    {
        let table = read.open_table(SNAPSHOT_TABLE).map_err(adapter_error)?;
        for item in table.iter().map_err(adapter_error)? {
            let (snapshot_ref, bytes) = item.map_err(adapter_error)?;
            let snapshot = decode_snapshot(snapshot_ref.value(), bytes.value())?;
            state.snapshots.insert(snapshot.snapshot_ref.clone(), snapshot);
        }
    }
    {
        let table = read.open_table(EFFECT_TABLE).map_err(adapter_error)?;
        for item in table.iter().map_err(adapter_error)? {
            let (transaction_id, bytes) = item.map_err(adapter_error)?;
            let effect = decode_effect(transaction_id.value(), bytes.value())?;
            state.effects.insert(effect.transaction_id.clone(), effect);
        }
    }
    Ok(state)
}

fn persist_log(database: &redb::Database, records: &[LogRecord]) -> crate::error::Result<()> {
    let write = database.begin_write().map_err(adapter_error)?;
    {
        let mut table = write.open_table(LOG_TABLE).map_err(adapter_error)?;
        for record in records {
            let bytes = encode_log_record(record)?;
            table.insert(record.sequence, bytes.as_slice()).map_err(adapter_error)?;
        }
    }
    write.commit().map_err(adapter_error)
}

fn replace_log(database: &redb::Database, records: &[LogRecord]) -> crate::error::Result<()> {
    let write = database.begin_write().map_err(adapter_error)?;
    {
        let mut table = write.open_table(LOG_TABLE).map_err(adapter_error)?;
        let keys = table
            .iter()
            .map_err(adapter_error)?
            .map(|item| item.map(|(key, _value)| key.value()).map_err(adapter_error))
            .collect::<crate::error::Result<Vec<_>>>()?;
        for key in keys {
            table.remove(key).map_err(adapter_error)?;
        }
        for record in records {
            let bytes = encode_log_record(record)?;
            table.insert(record.sequence, bytes.as_slice()).map_err(adapter_error)?;
        }
    }
    write.commit().map_err(adapter_error)
}

fn persist_ordered_batch(
    database: &redb::Database,
    request: &AtomicBatchRequest,
    next: &DurableState,
) -> crate::error::Result<()> {
    let write = database.begin_write().map_err(adapter_error)?;
    {
        let mut table = write.open_table(ORDERED_TABLE).map_err(adapter_error)?;
        for mutation in &request.mutations {
            match mutation {
                OrderedMutation::Put { key, .. } => {
                    let value = next.ordered.get(key).ok_or_else(|| {
                        crate::error::MoltenError::invalid_harness("admitted ordered mutation produced no value")
                    })?;
                    let bytes = encode_versioned_value(value)?;
                    table.insert(key.as_slice(), bytes.as_slice()).map_err(adapter_error)?;
                }
                OrderedMutation::Delete { key, .. } => {
                    table.remove(key.as_slice()).map_err(adapter_error)?;
                }
            }
        }
    }
    write.commit().map_err(adapter_error)
}

fn persist_snapshot(
    root: &crate::local_store::DurableStoreRoot,
    database: &redb::Database,
    request: &SnapshotRequest,
    bytes: &[u8],
    next: &DurableState,
) -> crate::error::Result<()> {
    let snapshot = next.snapshots.get(&request.snapshot_ref).ok_or_else(|| {
        crate::error::MoltenError::invalid_harness("admitted snapshot transition produced no snapshot")
    })?;
    let relative = format!("{SNAPSHOT_DIRECTORY}/{}.bin", snapshot_file_stem(&request.content_ref)?);
    root.root().write(&crate::local_store::LocalStorePath::parse(&relative)?, bytes)?;
    let encoded = encode_snapshot(snapshot)?;
    let write = database.begin_write().map_err(adapter_error)?;
    {
        let mut table = write.open_table(SNAPSHOT_TABLE).map_err(adapter_error)?;
        table.insert(request.snapshot_ref.as_str(), encoded.as_slice()).map_err(adapter_error)?;
    }
    write.commit().map_err(adapter_error)
}

fn persist_effect(
    database: &redb::Database,
    command: &EffectTransactionCommand,
    next: &DurableState,
) -> crate::error::Result<()> {
    let transaction_id = effect_transaction_id(command);
    let effect = next.effects.get(transaction_id).ok_or_else(|| {
        crate::error::MoltenError::invalid_harness("admitted effect transition produced no effect state")
    })?;
    let encoded = encode_effect(effect)?;
    let write = database.begin_write().map_err(adapter_error)?;
    {
        let mut table = write.open_table(EFFECT_TABLE).map_err(adapter_error)?;
        table.insert(transaction_id, encoded.as_slice()).map_err(adapter_error)?;
    }
    write.commit().map_err(adapter_error)
}

fn effect_transaction_id(command: &EffectTransactionCommand) -> &str {
    match command {
        EffectTransactionCommand::Reserve { transaction_id, .. }
        | EffectTransactionCommand::Commit { transaction_id, .. }
        | EffectTransactionCommand::Abort { transaction_id, .. }
        | EffectTransactionCommand::Expire { transaction_id, .. }
        | EffectTransactionCommand::MarkUncertain { transaction_id, .. }
        | EffectTransactionCommand::Reconcile { transaction_id, .. } => transaction_id,
    }
}

fn encode_log_record(record: &LogRecord) -> crate::error::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    push_byte(&mut bytes, encode_level(record.durability));
    push_blob(&mut bytes, record.value_ref.as_bytes())?;
    push_blob(&mut bytes, &record.value)?;
    Ok(bytes)
}

fn decode_log_record(sequence: u64, bytes: &[u8]) -> crate::error::Result<LogRecord> {
    let mut cursor = ByteCursor::new(bytes);
    let durability = decode_level(cursor.take_byte()?)?;
    let value_ref = cursor.take_string()?;
    let value = cursor.take_blob()?.to_vec();
    cursor.finish()?;
    Ok(LogRecord {
        sequence,
        value,
        value_ref,
        durability,
    })
}

fn encode_versioned_value(value: &VersionedValue) -> crate::error::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    push_u64(&mut bytes, value.version);
    push_blob(&mut bytes, value.value_ref.as_bytes())?;
    push_blob(&mut bytes, &value.value)?;
    Ok(bytes)
}

fn decode_versioned_value(bytes: &[u8]) -> crate::error::Result<VersionedValue> {
    let mut cursor = ByteCursor::new(bytes);
    let version = cursor.take_u64()?;
    let value_ref = cursor.take_string()?;
    let value = cursor.take_blob()?.to_vec();
    cursor.finish()?;
    Ok(VersionedValue {
        value,
        value_ref,
        version,
    })
}

fn encode_snapshot(snapshot: &SnapshotRecord) -> crate::error::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    push_byte(&mut bytes, encode_snapshot_kind(snapshot.kind));
    push_blob(&mut bytes, snapshot.content_ref.as_bytes())?;
    push_blob(&mut bytes, snapshot.source_namespace.as_bytes())?;
    push_u64(&mut bytes, snapshot.source_generation);
    push_blob(&mut bytes, snapshot.value_schema_ref.as_bytes())?;
    push_optional_u64(&mut bytes, snapshot.covered_log_sequence);
    push_blob(&mut bytes, snapshot.ordered_state_ref.as_bytes())?;
    push_byte(&mut bytes, encode_level(snapshot.durability));
    push_byte(&mut bytes, u8::from(snapshot.corrupted));
    Ok(bytes)
}

fn decode_snapshot(snapshot_ref: &str, bytes: &[u8]) -> crate::error::Result<SnapshotRecord> {
    let mut cursor = ByteCursor::new(bytes);
    let kind = decode_snapshot_kind(cursor.take_byte()?)?;
    let content_ref = cursor.take_string()?;
    let source_namespace = cursor.take_string()?;
    let source_generation = cursor.take_u64()?;
    let value_schema_ref = cursor.take_string()?;
    let covered_log_sequence = cursor.take_optional_u64()?;
    let ordered_state_ref = cursor.take_string()?;
    let durability = decode_level(cursor.take_byte()?)?;
    let is_corrupted = decode_bool(cursor.take_byte()?)?;
    cursor.finish()?;
    Ok(SnapshotRecord {
        kind,
        snapshot_ref: snapshot_ref.to_string(),
        content_ref,
        source_namespace,
        source_generation,
        value_schema_ref,
        covered_log_sequence,
        ordered_state_ref,
        durability,
        corrupted: is_corrupted,
    })
}

fn encode_effect(effect: &EffectTransactionState) -> crate::error::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    push_u64(&mut bytes, effect.generation);
    push_blob(&mut bytes, effect.operation_ref.as_bytes())?;
    push_byte(&mut bytes, encode_phase(effect.phase));
    push_optional_u64(&mut bytes, effect.expires_at_tick);
    for flag in [
        effect.profile.durable_reservation,
        effect.profile.exclusive,
        effect.profile.expiring,
        effect.profile.idempotent_commit,
        effect.profile.compensating_abort,
    ] {
        push_byte(&mut bytes, u8::from(flag));
    }
    Ok(bytes)
}

fn decode_effect(transaction_id: &str, bytes: &[u8]) -> crate::error::Result<EffectTransactionState> {
    let mut cursor = ByteCursor::new(bytes);
    let generation = cursor.take_u64()?;
    let operation_ref = cursor.take_string()?;
    let phase = decode_phase(cursor.take_byte()?)?;
    let expires_at_tick = cursor.take_optional_u64()?;
    let is_durable_reservation = decode_bool(cursor.take_byte()?)?;
    let is_exclusive = decode_bool(cursor.take_byte()?)?;
    let is_expiring = decode_bool(cursor.take_byte()?)?;
    let is_idempotent_commit = decode_bool(cursor.take_byte()?)?;
    let is_compensating_abort = decode_bool(cursor.take_byte()?)?;
    cursor.finish()?;
    Ok(EffectTransactionState {
        transaction_id: transaction_id.to_string(),
        generation,
        operation_ref,
        phase,
        expires_at_tick,
        profile: EffectTransactionProfile {
            durable_reservation: is_durable_reservation,
            exclusive: is_exclusive,
            expiring: is_expiring,
            idempotent_commit: is_idempotent_commit,
            compensating_abort: is_compensating_abort,
        },
    })
}

fn encode_level(level: DurabilityLevel) -> u8 {
    match level {
        DurabilityLevel::Buffered => LEVEL_BUFFERED,
        DurabilityLevel::ProcessLoss => LEVEL_PROCESS_LOSS,
        DurabilityLevel::MachineLoss => LEVEL_MACHINE_LOSS,
    }
}
