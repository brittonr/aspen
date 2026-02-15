use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteResult;

use super::command_conversion::write_command_to_app_request;
use super::*;

impl WriteBatcher {
    /// Write directly to Raft without batching.
    pub(super) async fn write_direct(&self, command: WriteCommand) -> Result<WriteResult, KeyValueStoreError> {
        let app_request = write_command_to_app_request(&command);

        let result = self.raft.client_write(app_request).await;

        match result {
            Ok(_resp) => Ok(WriteResult {
                command: Some(command),
                batch_applied: None,
                conditions_met: None,
                failed_condition_index: None,
                lease_id: None,
                ttl_seconds: None,
                keys_deleted: None,
                succeeded: None,
                txn_results: None,
                header_revision: None,
                occ_conflict: None,
                conflict_key: None,
                conflict_expected_version: None,
                conflict_actual_version: None,
            }),
            Err(e) => Err(KeyValueStoreError::Failed {
                reason: format!("raft error: {}", e),
            }),
        }
    }
}
