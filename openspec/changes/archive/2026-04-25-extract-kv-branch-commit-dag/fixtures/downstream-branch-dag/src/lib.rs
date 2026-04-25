use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;

use aspen_commit_dag::CommitStore;
use aspen_commit_dag::MutationType;
use aspen_commit_dag::verified::hash::GENESIS_HASH;
use aspen_kv_branch::BranchOverlay;
use aspen_kv_types::BatchOperation;
use aspen_kv_types::DeleteRequest;
use aspen_kv_types::DeleteResult;
use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::KeyValueWithRevision;
use aspen_kv_types::ReadRequest;
use aspen_kv_types::ReadResult;
use aspen_kv_types::ScanRequest;
use aspen_kv_types::ScanResult;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteOp;
use aspen_kv_types::WriteRequest;
use aspen_kv_types::WriteResult;
use aspen_traits::KeyValueStore;
use async_trait::async_trait;

const INITIAL_REVISION: u64 = 1;
const NEXT_REVISION_INCREMENT: u64 = 1;
const TEST_RAFT_REVISION: u64 = 7;
const TEST_TIMESTAMP_MS: u64 = 1_234;
const SCAN_LIMIT_DEFAULT: usize = 64;

#[derive(Default)]
struct MemoryStore {
    state: Mutex<MemoryState>,
}

#[derive(Default)]
struct MemoryState {
    values: BTreeMap<String, KeyValueWithRevision>,
    next_revision: u64,
}

impl MemoryStore {
    fn apply_set(state: &mut MemoryState, key: String, value: String) {
        let revision = state.next_revision.max(INITIAL_REVISION);
        state.next_revision = revision.saturating_add(NEXT_REVISION_INCREMENT);
        let create_revision = state.values.get(&key).map(|entry| entry.create_revision).unwrap_or(revision);
        let version = state
            .values
            .get(&key)
            .map(|entry| entry.version.saturating_add(NEXT_REVISION_INCREMENT))
            .unwrap_or(INITIAL_REVISION);
        state.values.insert(key.clone(), KeyValueWithRevision {
            key,
            value,
            version,
            create_revision,
            mod_revision: revision,
        });
    }

    fn apply_delete(state: &mut MemoryState, key: &str) {
        state.values.remove(key);
    }

    fn apply_batch_operation(state: &mut MemoryState, operation: BatchOperation) {
        match operation {
            BatchOperation::Set { key, value } => Self::apply_set(state, key, value),
            BatchOperation::Delete { key } => Self::apply_delete(state, &key),
        }
    }

    fn apply_write_op(state: &mut MemoryState, operation: WriteOp) {
        match operation {
            WriteOp::Set { key, value } => Self::apply_set(state, key, value),
            WriteOp::Delete { key } => Self::apply_delete(state, &key),
        }
    }

    fn success_result(command: WriteCommand, batch_applied: Option<u32>) -> WriteResult {
        WriteResult {
            command: Some(command),
            batch_applied,
            conditions_met: None,
            failed_condition_index: None,
            lease_id: None,
            ttl_seconds: None,
            keys_deleted: None,
            succeeded: Some(true),
            txn_results: None,
            header_revision: None,
            occ_conflict: None,
            conflict_key: None,
            conflict_expected_version: None,
            conflict_actual_version: None,
        }
    }
}

#[async_trait]
impl KeyValueStore for MemoryStore {
    async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError> {
        let mut state = self.state.lock().map_err(|_| KeyValueStoreError::Failed {
            reason: "poisoned fixture lock".into(),
        })?;
        match request.command {
            WriteCommand::Set { key, value } => {
                let command = WriteCommand::Set {
                    key: key.clone(),
                    value: value.clone(),
                };
                Self::apply_set(&mut state, key, value);
                Ok(Self::success_result(command, None))
            }
            WriteCommand::Batch { operations } => {
                let applied = operations.len() as u32;
                for operation in operations.clone() {
                    Self::apply_batch_operation(&mut state, operation);
                }
                Ok(Self::success_result(WriteCommand::Batch { operations }, Some(applied)))
            }
            WriteCommand::OptimisticTransaction { read_set, write_set } => {
                for operation in write_set.clone() {
                    Self::apply_write_op(&mut state, operation);
                }
                Ok(Self::success_result(WriteCommand::OptimisticTransaction { read_set, write_set }, None))
            }
            other => Err(KeyValueStoreError::Failed {
                reason: format!("unsupported fixture write command: {other:?}"),
            }),
        }
    }

    async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError> {
        let state = self.state.lock().map_err(|_| KeyValueStoreError::Failed {
            reason: "poisoned fixture lock".into(),
        })?;
        Ok(ReadResult {
            kv: state.values.get(&request.key).cloned(),
        })
    }

    async fn delete(&self, request: DeleteRequest) -> Result<DeleteResult, KeyValueStoreError> {
        let mut state = self.state.lock().map_err(|_| KeyValueStoreError::Failed {
            reason: "poisoned fixture lock".into(),
        })?;
        Self::apply_delete(&mut state, &request.key);
        Ok(DeleteResult {
            key: request.key,
            is_deleted: true,
        })
    }

    async fn scan(&self, request: ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
        let state = self.state.lock().map_err(|_| KeyValueStoreError::Failed {
            reason: "poisoned fixture lock".into(),
        })?;
        let limit = request.limit_results.map(|value| value as usize).unwrap_or(SCAN_LIMIT_DEFAULT);
        let entries: Vec<_> = state
            .values
            .iter()
            .filter(|(key, _)| key.starts_with(&request.prefix))
            .take(limit)
            .map(|(_, value)| value.clone())
            .collect();
        let result_count = entries.len() as u32;
        Ok(ScanResult {
            entries,
            result_count,
            is_truncated: false,
            continuation_token: None,
        })
    }
}

#[tokio::test]
async fn downstream_uses_branch_overlay_and_commit_dag() -> Result<(), Box<dyn std::error::Error>> {
    let store = Arc::new(MemoryStore::default());
    let branch = BranchOverlay::new("fixture", store.clone());

    branch.write(WriteRequest::set("key", "value")).await?;
    let read = branch.read(ReadRequest::new("key")).await?;
    assert_eq!(read.kv.as_ref().map(|entry| entry.value.as_str()), Some("value"));

    let result = branch.commit().await?;
    let commit_id = result.commit_id.ok_or("commit-dag feature did not produce commit id")?;
    let loaded = CommitStore::load_commit(&commit_id, store.as_ref()).await?;

    assert_eq!(loaded.branch_id, "fixture");
    assert_eq!(loaded.parent, None);
    assert_eq!(loaded.mutations, vec![("key".to_string(), MutationType::Set("value".to_string()))]);
    assert_ne!(loaded.id, GENESIS_HASH);
    Ok(())
}

#[test]
fn downstream_uses_direct_commit_hash_api() {
    let mutations = vec![("direct".to_string(), MutationType::Set("value".to_string()))];
    let mutations_hash = aspen_commit_dag::compute_mutations_hash(&mutations);
    let commit_id = aspen_commit_dag::compute_commit_id(
        &None,
        "direct-fixture",
        &mutations_hash,
        TEST_RAFT_REVISION,
        TEST_TIMESTAMP_MS,
    );

    assert_ne!(commit_id, GENESIS_HASH);
}
