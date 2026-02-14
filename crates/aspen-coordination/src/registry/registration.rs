//! Service registration and deregistration operations.

use anyhow::Result;
use anyhow::bail;
use aspen_constants::coordination::DEFAULT_SERVICE_TTL_MS;
use aspen_constants::coordination::MAX_SERVICE_TTL_MS;
use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use tracing::debug;

use super::ServiceRegistry;
use super::types::HealthStatus;
use super::types::RegisterOptions;
use super::types::ServiceInstance;
use super::types::ServiceInstanceMetadata;
use crate::types::now_unix_ms;

impl<S: KeyValueStore + ?Sized + 'static> ServiceRegistry<S> {
    /// Register a service instance.
    ///
    /// Returns (fencing_token, deadline_ms) on success.
    /// If instance already exists, updates it and returns new token.
    pub async fn register(
        &self,
        service_name: &str,
        instance_id: &str,
        address: &str,
        metadata: ServiceInstanceMetadata,
        options: RegisterOptions,
    ) -> Result<(u64, u64)> {
        let now = now_unix_ms();
        let ttl_ms = options.ttl_ms.unwrap_or(DEFAULT_SERVICE_TTL_MS).min(MAX_SERVICE_TTL_MS);
        let is_lease_based = options.lease_id.is_some();
        let deadline_ms = crate::verified::compute_heartbeat_deadline(now, ttl_ms, is_lease_based);

        let key = Self::instance_key(service_name, instance_id);

        loop {
            // Read existing instance if any
            let existing = self.read_json::<ServiceInstance>(&key).await?;

            let (fencing_token, registered_at_ms) = match &existing {
                Some(inst) => (crate::verified::compute_next_instance_token(inst.fencing_token), inst.registered_at_ms),
                None => (1, now),
            };

            let instance = ServiceInstance {
                instance_id: instance_id.to_string(),
                service_name: service_name.to_string(),
                address: address.to_string(),
                health_status: options.initial_status.unwrap_or(HealthStatus::Healthy),
                metadata: metadata.clone(),
                registered_at_ms,
                last_heartbeat_ms: now,
                deadline_ms,
                ttl_ms,
                lease_id: options.lease_id,
                fencing_token,
            };

            let new_json = serde_json::to_string(&instance)?;

            let command = match &existing {
                Some(old) => {
                    let old_json = serde_json::to_string(old)?;
                    WriteCommand::CompareAndSwap {
                        key: key.clone(),
                        expected: Some(old_json),
                        new_value: new_json,
                    }
                }
                None => WriteCommand::Set {
                    key: key.clone(),
                    value: new_json,
                },
            };

            match self.store.write(WriteRequest { command }).await {
                Ok(_) => {
                    debug!(service_name, instance_id, fencing_token, deadline_ms, "instance registered");
                    return Ok((fencing_token, deadline_ms));
                }
                Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => {
                    // Retry on CAS failure
                    continue;
                }
                Err(e) => bail!("failed to register instance: {}", e),
            }
        }
    }

    /// Deregister a service instance.
    ///
    /// Returns true if instance was found and removed.
    pub async fn deregister(&self, service_name: &str, instance_id: &str, fencing_token: u64) -> Result<bool> {
        let key = Self::instance_key(service_name, instance_id);

        loop {
            let existing = self.read_json::<ServiceInstance>(&key).await?;

            match existing {
                None => return Ok(false),
                Some(inst) => {
                    if inst.fencing_token != fencing_token {
                        bail!("fencing token mismatch: expected {}, got {}", inst.fencing_token, fencing_token);
                    }

                    let old_json = serde_json::to_string(&inst)?;

                    match self
                        .store
                        .write(WriteRequest {
                            command: WriteCommand::CompareAndSwap {
                                key: key.clone(),
                                expected: Some(old_json),
                                new_value: String::new(), // Empty = delete
                            },
                        })
                        .await
                    {
                        Ok(_) => {
                            debug!(service_name, instance_id, "instance deregistered");
                            return Ok(true);
                        }
                        Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => continue,
                        Err(e) => bail!("failed to deregister instance: {}", e),
                    }
                }
            }
        }
    }
}
