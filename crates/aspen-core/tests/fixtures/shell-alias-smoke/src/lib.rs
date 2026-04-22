pub fn storage_table_marker() -> usize {
    core::mem::size_of_val(&aspen_core::storage::SM_KV_TABLE)
}
