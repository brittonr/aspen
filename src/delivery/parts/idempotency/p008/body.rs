fn open_publication_database(root: &crate::local_store::DeliveryStoreRoot) -> Result<redb::Database> {
    let path = crate::local_store::Locator::parse(STORE_FILE)?;
    let is_existing = root.root().entry_kind_optional(&path)?.is_some();
    let file = if is_existing {
        root.root().open_database_file(&path)?
    } else {
        root.root().create_database_file(&path)?
    };
    if is_existing && file.metadata().map_err(MoltenError::from)?.len() == 0 {
        return Err(MoltenError::invalid_harness("existing publication database is empty; reconciliation is required"));
    }
    let mut builder = redb::Database::builder();
    // Redb also invokes repair callbacks while initializing a newly created empty file.
    // Only our no-replace acquisition authorizes that initialization; existing repair aborts.
    if is_existing {
        builder.set_repair_callback(|session| session.abort());
    }
    let db = builder.create_file(file).map_err(store_error)?;
    let write = db.begin_write().map_err(store_error)?;
    {
        write.open_table(STORE_WINDOWS).map_err(store_error)?;
        write.open_table(STORE_ENTRIES).map_err(store_error)?;
        write.open_table(STORE_RECEIPTS).map_err(store_error)?;
        write.open_table(STORE_PINS).map_err(store_error)?;
        write.open_table(STORE_PUBLICATIONS).map_err(store_error)?;
        write.open_table(STORE_PUBLICATION_HEALTH).map_err(store_error)?;
    }
    write.commit().map_err(store_error)?;
    Ok(db)
}

fn read_publication_health(db: &redb::Database) -> Result<transactional_reconciliation_core::QuarantineStatus> {
    let read = db.begin_read().map_err(store_error)?;
    let table = read.open_table(STORE_PUBLICATION_HEALTH).map_err(store_error)?;
    let Some(value) = table.get(PUBLICATION_HEALTH_KEY).map_err(store_error)? else {
        return Ok(transactional_reconciliation_core::QuarantineStatus::Clear);
    };
    let reason = if value.value() == b"commit-outcome-unknown" {
        transactional_reconciliation_core::QuarantineReason::CommitOutcomeUnknown
    } else {
        transactional_reconciliation_core::QuarantineReason::Corrupt
    };
    Ok(transactional_reconciliation_core::QuarantineStatus::Quarantined(reason))
}

fn publication_key(operation_ref: &str, payload_ref: &str) -> String {
    format!("{payload_ref}/{operation_ref}")
}

fn publication_intent_bytes(operation_ref: &str, payload_ref: &str, metadata: &IoValue) -> Result<Vec<u8>> {
    require_ref(operation_ref, "publication operation ref")?;
    require_ref(payload_ref, "publication payload ref")?;
    let value = record("delivery-publication-intent-v1", vec![
        string("molten.delivery.publication-intent.v1"),
        record("operation", vec![string(operation_ref)]),
        record("payload", vec![string(payload_ref)]),
        record("metadata", vec![metadata.clone()]),
    ]);
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    if bytes.len() > MAX_PUBLICATION_RECORD_BYTES {
        return Err(MoltenError::invalid_harness("publication intent exceeds bounded record size"));
    }
    Ok(bytes)
}

fn decode_publication_intent(bytes: &[u8]) -> Result<PublicationIntent> {
    if bytes.len() > MAX_PUBLICATION_RECORD_BYTES {
        return Err(MoltenError::invalid_harness("publication intent exceeds bounded record size"));
    }
    let value = crate::preserves_rail::parse_canonical_bytes(bytes)?;
    let fields = value.collect_simple_record("delivery-publication-intent-v1", Some(4))
        .ok_or_else(|| MoltenError::invalid_harness("expected delivery publication intent"))?;
    require_schema(&fields[0], "molten.delivery.publication-intent.v1", "delivery publication intent")?;
    let metadata_value = crate::preserves_rail::value_to_iovalue(&fields[3]);
    let metadata_fields = metadata_value.collect_simple_record("metadata", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected publication metadata field"))?;
    let metadata = crate::preserves_rail::value_to_iovalue(&metadata_fields[0]);
    Ok(PublicationIntent {
        operation_ref: record_ref(&fields[1], "operation")?,
        payload_ref: record_ref(&fields[2], "payload")?,
        metadata,
    })
}

// r[impl molten.audit_f03.recover_publication]
fn insert_publication_intent(write: &redb::WriteTransaction, entry: &DedupEntry, metadata: &IoValue) -> Result<()> {
    let bytes = publication_intent_bytes(&entry.operation_ref, &entry.payload_ref, metadata)?;
    let key = publication_key(&entry.operation_ref, &entry.payload_ref);
    let mut table = write.open_table(STORE_PUBLICATIONS).map_err(store_error)?;
    if table.get(key.as_str()).map_err(store_error)?.is_some() {
        return Err(MoltenError::invalid_harness("first dedup reservation conflicts with existing publication intent"));
    }
    table.insert(key.as_str(), bytes.as_slice()).map_err(store_error)?;
    Ok(())
}

// r[impl molten.audit_f03.preserve_state]
fn update_publication_intent(
    write: &redb::WriteTransaction,
    intent: &PublicationIntent,
    metadata: &IoValue,
) -> Result<()> {
    let key = publication_key(&intent.operation_ref, &intent.payload_ref);
    let mut table = write.open_table(STORE_PUBLICATIONS).map_err(store_error)?;
    {
        let bytes = table.get(key.as_str()).map_err(store_error)?
            .ok_or_else(|| MoltenError::invalid_harness("publication intent disappeared before transition"))?;
        let prior = decode_publication_intent(bytes.value())?;
        if prior.operation_ref != intent.operation_ref || prior.payload_ref != intent.payload_ref || prior.metadata != intent.metadata {
            return Err(MoltenError::invalid_harness("publication intent changed before transition"));
        }
    }
    let bytes = publication_intent_bytes(&intent.operation_ref, &intent.payload_ref, metadata)?;
    table.insert(key.as_str(), bytes.as_slice()).map_err(store_error)?;
    Ok(())
}

fn read_publications_for_payload(db: &redb::Database, payload_ref: &str) -> Result<Vec<PublicationIntent>> {
    let read = db.begin_read().map_err(store_error)?;
    let table = read.open_table(STORE_PUBLICATIONS).map_err(store_error)?;
    let first = format!("{payload_ref}/");
    let end = format!("{payload_ref}0");
    let mut intents = Vec::new();
    for item in table.range(first.as_str()..end.as_str()).map_err(store_error)? {
        let (key, bytes) = item.map_err(store_error)?;
        if intents.len() >= MAX_REFS {
            return Err(MoltenError::invalid_harness("too many publication intents for one control request"));
        }
        let intent = decode_publication_intent(bytes.value())?;
        if intent.payload_ref != payload_ref || key.value() != publication_key(&intent.operation_ref, payload_ref) {
            return Err(MoltenError::invalid_harness("publication index conflicts with canonical identity"));
        }
        intents.push(intent);
    }
    Ok(intents)
}
