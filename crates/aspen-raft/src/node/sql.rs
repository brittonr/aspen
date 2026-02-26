//! SqlQueryExecutor trait implementation for RaftNode.
//!
//! This module is only compiled when the `sql` feature is enabled.

use aspen_core::SqlConsistency;
use aspen_core::SqlQueryError;
use aspen_core::SqlQueryExecutor;
use aspen_core::SqlQueryRequest;
use aspen_core::SqlQueryResult;
use aspen_core::validate_sql_query;
use aspen_core::validate_sql_request;
use aspen_raft_types::READ_INDEX_TIMEOUT;
use async_trait::async_trait;
use openraft::ReadPolicy;
use tracing::instrument;

use super::RaftNode;
use crate::StateMachineVariant;

#[async_trait]
impl SqlQueryExecutor for RaftNode {
    #[instrument(skip(self))]
    async fn execute_sql(&self, request: SqlQueryRequest) -> Result<SqlQueryResult, SqlQueryError> {
        let _permit = self.semaphore().acquire().await.map_err(|_| SqlQueryError::ExecutionFailed {
            reason: "semaphore closed".into(),
        })?;

        // Validate request bounds (Tiger Style)
        validate_sql_request(&request)?;

        // Validate query is read-only
        validate_sql_query(&request.query)?;

        // For linearizable consistency, use ReadIndex protocol
        //
        // Tiger Style: Timeout the linearizer acquisition itself, not just await_ready().
        // The get_read_linearizer() call spawns a background task that waits for heartbeat
        // quorum confirmation. If followers are unreachable, this can hang indefinitely.
        if request.consistency == SqlConsistency::Linearizable {
            let linearizer =
                tokio::time::timeout(READ_INDEX_TIMEOUT, self.raft().get_read_linearizer(ReadPolicy::ReadIndex))
                    .await
                    .map_err(|_| SqlQueryError::Timeout {
                        duration_ms: READ_INDEX_TIMEOUT.as_millis() as u64,
                    })?
                    .map_err(|_err| {
                        let leader_hint = self.raft().metrics().borrow().current_leader.map(|id| id.0);
                        SqlQueryError::NotLeader { leader: leader_hint }
                    })?;

            // Tiger Style: Explicit timeout prevents indefinite hang during network partition
            tokio::time::timeout(READ_INDEX_TIMEOUT, linearizer.await_ready(self.raft()))
                .await
                .map_err(|_| SqlQueryError::Timeout {
                    duration_ms: READ_INDEX_TIMEOUT.as_millis() as u64,
                })?
                .map_err(|_err| {
                    let leader_hint = self.raft().metrics().borrow().current_leader.map(|id| id.0);
                    SqlQueryError::NotLeader { leader: leader_hint }
                })?;
        }

        // Execute query on state machine
        match self.state_machine() {
            StateMachineVariant::InMemory(_) => {
                // In-memory backend doesn't support SQL
                Err(SqlQueryError::NotSupported {
                    backend: "in-memory".into(),
                })
            }
            StateMachineVariant::Redb(sm) => {
                // Execute SQL on Redb state machine via DataFusion
                // Use cached executor for ~400us savings per query
                let executor = self.sql_executor().get_or_init(|| sm.create_sql_executor());
                executor.execute(&request.query, &request.params, request.limit, request.timeout_ms).await
            }
        }
    }
}
