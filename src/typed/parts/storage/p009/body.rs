
fn migration_phase_receipt_value(phase: &str, input: &StepInput<'_>) -> IoValue {
    record("typed-storage-migration-phase-receipt-v1", vec![
        record("phase", vec![string(phase)]),
        record("decision", vec![string("pass")]),
        record("recipe", vec![string(&input.recipe.recipe_ref)]),
        record("source", vec![
            string(&input.source.storage_ref),
            string(&input.source.schema_ref),
            string(&input.source.value_ref),
        ]),
        record("target", vec![
            string(input.storage_ref),
            string(&input.recipe.target_schema_ref),
            string(input.value_ref),
        ]),
        checks_value(&["phase-recorded", "policy-bound", "lineage-bound"]),
    ])
}

fn sorted_unique_strings(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}

pub fn list_receipt_refs(root: &Path) -> Result<Vec<String>> {
    ensure_dirs(root)?;
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let table = read_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    let mut refs = Vec::new();
    for item in table.iter().map_err(index_error)? {
        let (key, _value) = item.map_err(index_error)?;
        push_bounded(&mut refs, key.value().to_string(), MAX_TYPED_STORAGE_RECEIPTS, "typed storage receipt refs")?;
    }
    refs.sort();
    Ok(refs)
}

pub fn read_receipt(root: &Path, receipt_ref: &str) -> Result<Receipt> {
    ensure_dirs(root)?;
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let table = read_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    let Some(bytes) = table.get(receipt_ref).map_err(index_error)? else {
        return Err(MoltenError::invalid_harness(format!("unknown typed storage receipt {receipt_ref}")));
    };
    let value = parse_canonical_bytes(bytes.value())?;
    parse_receipt_value(&value, Some(receipt_ref))
}

pub fn inferred_schema_ref(value: &IoValue) -> Result<String> {
    canonical_hash(&inferred_schema_value(value))
}

pub fn inferred_schema_value(value: &IoValue) -> IoValue {
    let class = match value.value_class() {
        preserves::ValueClass::Atomic(_) => "atomic",
        preserves::ValueClass::Embedded => "embedded",
        preserves::ValueClass::Compound(preserves::CompoundClass::Record) => "record",
        preserves::ValueClass::Compound(preserves::CompoundClass::Sequence) => "sequence",
        preserves::ValueClass::Compound(preserves::CompoundClass::Set) => "set",
        preserves::ValueClass::Compound(preserves::CompoundClass::Dictionary) => "dictionary",
    };
    record("storage-schema-artifact-v1", vec![
        string(crate::preserves_rail::TYPED_STORAGE_SCHEMA_ARTIFACT_SCHEMA),
        record("inference", vec![string("preserves-value-class")]),
        record("class", vec![string(class)]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("canonical-preserves-class"), string("pass")]),
            record("check", vec![string("no-raw-memory-layout"), string("pass")]),
        ])]),
    ])
}

pub fn effect_manifest_value(
    producer_ref: &str,
    namespace: &str,
    schema_ref: &str,
    operations: &[String],
) -> Result<IoValue> {
    require_ref(producer_ref, "storage effect manifest producer ref")?;
    validate_namespace(namespace)?;
    require_ref(schema_ref, "storage effect manifest schema ref")?;
    validate_operations(operations)?;
    Ok(record("storage-effect-manifest-v1", vec![
        string(crate::preserves_rail::TYPED_STORAGE_EFFECT_MANIFEST_SCHEMA),
        record("producer", vec![string(producer_ref)]),
        record("namespace", vec![string(namespace)]),
        record("schema-ref", vec![string(schema_ref)]),
        record("operations", vec![sequence(operations.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("declared-storage-effect"), string("pass")]),
            record("check", vec![string("typed-schema-binding"), string("pass")]),
            record("check", vec![string("handler-profile-required"), string("pass")]),
        ])]),
    ]))
}
