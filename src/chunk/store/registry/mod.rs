//! Redb index of manifests, chunk availability, pins, partial fetches, and receipts.

pub(super) mod survey;
pub(super) mod update;

use redb::ReadableTable;

const INDEX_FILE: &str = "chunk-index.redb";
const INDEX_MANIFESTS: redb::TableDefinition<'static, &str, &[u8]> =
    redb::TableDefinition::new("chunk_store_manifests_v1");
const INDEX_CHUNKS: redb::TableDefinition<'static, &str, &[u8]> = redb::TableDefinition::new("chunk_store_chunks_v1");
const INDEX_AVAILABILITY: redb::TableDefinition<'static, &str, &str> =
    redb::TableDefinition::new("chunk_store_availability_v1");
const INDEX_PINS: redb::TableDefinition<'static, &str, &str> = redb::TableDefinition::new("chunk_store_pins_v1");
const INDEX_PARTIAL_FETCHES: redb::TableDefinition<'static, &str, &[u8]> =
    redb::TableDefinition::new("chunk_store_partial_fetches_v1");
pub(super) const INDEX_RECEIPTS: redb::TableDefinition<'static, &str, &[u8]> =
    redb::TableDefinition::new("chunk_store_receipts_v1");

/// Records a denial receipt and fails when `manifest` declares a transform this store cannot serve
/// yet.
pub(super) fn deny_unsupported_transform(
    root: &crate::blocks::CapabilityChunkRoot,
    operation: &str,
    manifest: &crate::blocks::ChunkManifest,
    chunk_refs: &[String],
) -> crate::error::Result<()> {
    let Some(message) = crate::blocks::wire::policy::unsupported_transform_message(manifest) else {
        return Ok(());
    };
    let receipt_value = crate::blocks::wire::outcome::denial_receipt_value(
        operation,
        Some(&manifest.manifest_ref),
        chunk_refs,
        &message,
        vec![("transform-mode", "fail"), ("deny-unsupported-transform", "pass")],
    );
    receipt(root, &receipt_value)?;
    Err(crate::error::Failure::invalid_harness(message))
}

pub(super) fn receipt(
    root: &crate::blocks::CapabilityChunkRoot,
    receipt_value: &preserves::IOValue,
) -> crate::error::Result<()> {
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    receipt_in_tx(&write_txn, receipt_value)?;
    write_txn.commit().map_err(index_error)
}

fn receipt_in_tx(write_txn: &redb::WriteTransaction, receipt_value: &preserves::IOValue) -> crate::error::Result<()> {
    let parsed = crate::blocks::parse_receipt_value(receipt_value, None)?;
    let receipt_bytes = crate::preserves_rail::canonical_bytes(receipt_value)?;
    let mut receipts = write_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    receipts.insert(parsed.receipt_ref.as_str(), receipt_bytes.as_slice()).map_err(index_error)?;
    Ok(())
}

pub(super) fn ensure_index_tables(root: &crate::blocks::CapabilityChunkRoot) -> crate::error::Result<redb::Database> {
    let database_file = root.root().open_database_file(&crate::blocks::layout::path(INDEX_FILE)?)?;
    let db = redb::Database::builder().create_file(database_file).map_err(index_error)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    {
        write_txn.open_table(INDEX_MANIFESTS).map_err(index_error)?;
        write_txn.open_table(INDEX_CHUNKS).map_err(index_error)?;
        write_txn.open_table(INDEX_AVAILABILITY).map_err(index_error)?;
        write_txn.open_table(INDEX_PINS).map_err(index_error)?;
        write_txn.open_table(INDEX_PARTIAL_FETCHES).map_err(index_error)?;
        write_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    }
    write_txn.commit().map_err(index_error)?;
    Ok(db)
}

fn clear_index_tables_in_tx(write_txn: &redb::WriteTransaction) -> crate::error::Result<()> {
    {
        let mut table = write_txn.open_table(INDEX_MANIFESTS).map_err(index_error)?;
        let keys = table_keys(&table)?;
        for key in keys {
            table.remove(key.as_str()).map_err(index_error)?;
        }
    }
    {
        let mut table = write_txn.open_table(INDEX_CHUNKS).map_err(index_error)?;
        let keys = table_keys(&table)?;
        for key in keys {
            table.remove(key.as_str()).map_err(index_error)?;
        }
    }
    {
        let mut table = write_txn.open_table(INDEX_AVAILABILITY).map_err(index_error)?;
        let keys = str_table_keys(&table)?;
        for key in keys {
            table.remove(key.as_str()).map_err(index_error)?;
        }
    }
    {
        let mut table = write_txn.open_table(INDEX_PINS).map_err(index_error)?;
        let keys = str_table_keys(&table)?;
        for key in keys {
            table.remove(key.as_str()).map_err(index_error)?;
        }
    }
    {
        let mut table = write_txn.open_table(INDEX_PARTIAL_FETCHES).map_err(index_error)?;
        let keys = table_keys(&table)?;
        for key in keys {
            table.remove(key.as_str()).map_err(index_error)?;
        }
    }
    Ok(())
}

fn table_keys(table: &redb::Table<'_, &str, &[u8]>) -> crate::error::Result<Vec<String>> {
    table
        .iter()
        .map_err(index_error)?
        .map(|item| item.map(|(key, _value)| key.value().to_string()).map_err(index_error))
        .collect()
}

fn str_table_keys(table: &redb::Table<'_, &str, &str>) -> crate::error::Result<Vec<String>> {
    table
        .iter()
        .map_err(index_error)?
        .map(|item| item.map(|(key, _value)| key.value().to_string()).map_err(index_error))
        .collect()
}

fn pin_key(kind: &str, reference: &str) -> String {
    format!("{kind}:{reference}")
}

pub(super) fn index_error(error: impl std::fmt::Display) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(format!("chunk store redb index error: {error}"))
}
