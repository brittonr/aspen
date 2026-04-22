use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_kv_types::WriteResult;
use tracing::debug;
use tracing::warn;

use super::command_conversion::write_command_to_app_request;
use super::*;
use crate::types::NodeId;
use crate::types::member_endpoint_addr;

struct DirectWriteForwardTarget {
    leader_id: NodeId,
    leader_addr: iroh::EndpointAddr,
}

impl WriteBatcher {
    #[inline]
    fn direct_write_forward_target(
        &self,
        forward_info: &openraft::error::ForwardToLeader<AppTypeConfig>,
    ) -> Option<DirectWriteForwardTarget> {
        let leader_id = forward_info.leader_id?;
        let leader_node = forward_info.leader_node.as_ref()?;
        if NodeId(leader_id.0) == self.node_id() {
            return None;
        }
        let leader_addr = match member_endpoint_addr(leader_node) {
            Ok(leader_addr) => leader_addr,
            Err(error) => {
                warn!(
                    leader_id = leader_id.0,
                    endpoint_id = %leader_node.endpoint_id(),
                    error = %error,
                    "cannot forward direct write because leader membership address is invalid"
                );
                return None;
            }
        };
        Some(DirectWriteForwardTarget {
            leader_id: NodeId(leader_id.0),
            leader_addr,
        })
    }

    /// Write directly to Raft without batching.
    ///
    /// If Raft returns ForwardToLeader (leadership changed), forwards the
    /// write to the leader via the write forwarder. Without a forwarder,
    /// returns NotLeader so callers can retry.
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
            Err(error) => {
                // Forward to leader if this is a leadership change during write.
                if let Some(forward_info) = error.forward_to_leader() {
                    if let Some(forward_target) = self.direct_write_forward_target(forward_info)
                        && let Some(forwarder) = self.write_forwarder()
                    {
                        debug!(
                            leader_id = forward_target.leader_id.0,
                            "forwarding direct write to leader after leadership change"
                        );
                        let request = WriteRequest { command };
                        return forwarder
                            .forward_write(forward_target.leader_id, forward_target.leader_addr, request)
                            .await;
                    }

                    return Err(KeyValueStoreError::NotLeader {
                        leader: forward_info.leader_id.map(|id| id.0),
                        reason: "leadership changed during direct write".to_string(),
                    });
                }

                Err(KeyValueStoreError::Failed {
                    reason: format!("raft error: {}", error),
                })
            }
        }
    }
}
