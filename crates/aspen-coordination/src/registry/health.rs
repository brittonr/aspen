//! Health and heartbeat operations for service instances.

use anyhow::Result;
use anyhow::bail;
use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use tracing::debug;

use super::ServiceRegistry;
use super::types::HealthStatus;
use super::types::ServiceInstance;
use super::types::ServiceInstanceMetadata;
use crate::types::now_unix_ms;

impl<S: KeyValueStore + ?Sized + 'static> ServiceRegistry<S> {
    /// Send heartbeat to renew instance TTL.
    ///
    /// Returns (new_deadline_ms, health_status) on success.
    pub async fn heartbeat(
        &self,
        service_name: &str,
        instance_id: &str,
        fencing_token: u64,
    ) -> Result<(u64, HealthStatus)> {
        let key = Self::instance_key(service_name, instance_id);
        let now = now_unix_ms();

        loop {
            let existing = self.read_json::<ServiceInstance>(&key).await?;

            match existing {
                None => bail!("instance not found: {}:{}", service_name, instance_id),
                Some(mut inst) => {
                    if inst.fencing_token != fencing_token {
                        bail!("fencing token mismatch: expected {}, got {}", inst.fencing_token, fencing_token);
                    }

                    let old_json = serde_json::to_string(&inst)?;

                    // Update heartbeat and deadline
                    inst.last_heartbeat_ms = now;
                    inst.deadline_ms =
                        crate::verified::compute_heartbeat_deadline(now, inst.ttl_ms, inst.lease_id.is_some());

                    let new_json = serde_json::to_string(&inst)?;

                    match self
                        .store
                        .write(WriteRequest {
                            command: WriteCommand::CompareAndSwap {
                                key: key.clone(),
                                expected: Some(old_json),
                                new_value: new_json,
                            },
                        })
                        .await
                    {
                        Ok(_) => {
                            debug!(service_name, instance_id, new_deadline = inst.deadline_ms, "heartbeat sent");
                            return Ok((inst.deadline_ms, inst.health_status));
                        }
                        Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => continue,
                        Err(e) => bail!("failed to send heartbeat: {}", e),
                    }
                }
            }
        }
    }

    /// Update health status of an instance.
    pub async fn update_health(
        &self,
        service_name: &str,
        instance_id: &str,
        fencing_token: u64,
        status: HealthStatus,
    ) -> Result<()> {
        let key = Self::instance_key(service_name, instance_id);

        loop {
            let existing = self.read_json::<ServiceInstance>(&key).await?;

            match existing {
                None => bail!("instance not found: {}:{}", service_name, instance_id),
                Some(mut inst) => {
                    if inst.fencing_token != fencing_token {
                        bail!("fencing token mismatch: expected {}, got {}", inst.fencing_token, fencing_token);
                    }

                    let old_json = serde_json::to_string(&inst)?;
                    inst.health_status = status;
                    let new_json = serde_json::to_string(&inst)?;

                    match self
                        .store
                        .write(WriteRequest {
                            command: WriteCommand::CompareAndSwap {
                                key: key.clone(),
                                expected: Some(old_json),
                                new_value: new_json,
                            },
                        })
                        .await
                    {
                        Ok(_) => {
                            debug!(service_name, instance_id, status = status.as_str(), "health status updated");
                            return Ok(());
                        }
                        Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => continue,
                        Err(e) => bail!("failed to update health status: {}", e),
                    }
                }
            }
        }
    }

    /// Update instance metadata.
    pub async fn update_metadata(
        &self,
        service_name: &str,
        instance_id: &str,
        fencing_token: u64,
        metadata: ServiceInstanceMetadata,
    ) -> Result<()> {
        let key = Self::instance_key(service_name, instance_id);

        loop {
            let existing = self.read_json::<ServiceInstance>(&key).await?;

            match existing {
                None => bail!("instance not found: {}:{}", service_name, instance_id),
                Some(mut inst) => {
                    if inst.fencing_token != fencing_token {
                        bail!("fencing token mismatch: expected {}, got {}", inst.fencing_token, fencing_token);
                    }

                    let old_json = serde_json::to_string(&inst)?;
                    inst.metadata = metadata.clone();
                    let new_json = serde_json::to_string(&inst)?;

                    match self
                        .store
                        .write(WriteRequest {
                            command: WriteCommand::CompareAndSwap {
                                key: key.clone(),
                                expected: Some(old_json),
                                new_value: new_json,
                            },
                        })
                        .await
                    {
                        Ok(_) => {
                            debug!(service_name, instance_id, "metadata updated");
                            return Ok(());
                        }
                        Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => continue,
                        Err(e) => bail!("failed to update metadata: {}", e),
                    }
                }
            }
        }
    }
}
