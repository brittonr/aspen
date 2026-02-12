//! Distributed counting semaphore for limiting concurrent access.
//!
//! A semaphore maintains a set of permits that can be acquired by clients.
//! Each permit holder has a TTL for automatic release on crash recovery.
//!
//! The semaphore state is stored as a JSON object in the key-value store.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use anyhow::bail;
use aspen_core::KeyValueStore;
use aspen_core::KeyValueStoreError;
use aspen_core::ReadRequest;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;
use aspen_core::constants::MAX_SEMAPHORE_HOLDERS;
use serde::Deserialize;
use serde::Serialize;
use tracing::debug;

use crate::error::CoordinationError;
use crate::types::now_unix_ms;
use crate::verified;

/// Semaphore state stored in the key-value store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemaphoreState {
    /// Semaphore name.
    pub name: String,
    /// Maximum permits (capacity).
    pub capacity: u32,
    /// Current permit holders.
    pub holders: Vec<SemaphoreHolder>,
}

/// A holder of semaphore permits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemaphoreHolder {
    /// Unique holder identifier.
    pub holder_id: String,
    /// Number of permits held.
    pub permits: u32,
    /// Time acquired (ms since epoch).
    pub acquired_at_ms: u64,
    /// Deadline for automatic release (ms since epoch).
    pub deadline_ms: u64,
}

impl SemaphoreState {
    /// Calculate available permits, accounting for expired holders.
    fn available_permits(&self) -> u32 {
        let now = now_unix_ms();
        crate::verified::calculate_available_permits(
            self.capacity,
            self.holders.iter().map(|h| (h.permits, h.deadline_ms)),
            now,
        )
    }

    /// Remove expired holders.
    fn cleanup_expired(&mut self) {
        let now = now_unix_ms();
        self.holders.retain(|h| !crate::verified::is_holder_expired(h.deadline_ms, now));
    }

    /// Find holder by ID.
    fn find_holder(&self, holder_id: &str) -> Option<&SemaphoreHolder> {
        self.holders.iter().find(|h| h.holder_id == holder_id)
    }

    /// Find holder by ID (mutable).
    fn find_holder_mut(&mut self, holder_id: &str) -> Option<&mut SemaphoreHolder> {
        self.holders.iter_mut().find(|h| h.holder_id == holder_id)
    }
}

/// Manager for distributed semaphore operations.
pub struct SemaphoreManager<S: KeyValueStore + ?Sized> {
    store: Arc<S>,
}

impl<S: KeyValueStore + ?Sized + 'static> SemaphoreManager<S> {
    /// Create a new semaphore manager.
    pub fn new(store: Arc<S>) -> Self {
        Self { store }
    }

    /// Acquire permits, blocking until available or timeout.
    ///
    /// Returns (permits_acquired, available_after) on success.
    pub async fn acquire(
        &self,
        name: &str,
        holder_id: &str,
        permits: u32,
        capacity: u32,
        ttl_ms: u64,
        timeout: Option<Duration>,
    ) -> Result<(u32, u32)> {
        let deadline = timeout.map(|t| std::time::Instant::now() + t);

        loop {
            // Check timeout
            if let Some(d) = deadline
                && std::time::Instant::now() >= d
            {
                bail!("semaphore acquire timeout");
            }

            match self.try_acquire(name, holder_id, permits, capacity, ttl_ms).await? {
                Some(result) => return Ok(result),
                None => {
                    // Wait and retry
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            }
        }
    }

    /// Try to acquire permits without blocking.
    ///
    /// Returns Some((permits_acquired, available_after)) on success, None if no permits available.
    pub async fn try_acquire(
        &self,
        name: &str,
        holder_id: &str,
        permits: u32,
        capacity: u32,
        ttl_ms: u64,
    ) -> Result<Option<(u32, u32)>> {
        let key = verified::semaphore_key(name);

        loop {
            // Read current state
            let current = self.read_state(&key).await?;

            match current {
                None => {
                    // Create new semaphore
                    if permits > capacity {
                        bail!("requested permits {} exceeds capacity {}", permits, capacity);
                    }

                    let now = now_unix_ms();
                    let state = SemaphoreState {
                        name: name.to_string(),
                        capacity,
                        holders: vec![SemaphoreHolder {
                            holder_id: holder_id.to_string(),
                            permits,
                            acquired_at_ms: now,
                            deadline_ms: crate::verified::compute_holder_deadline(now, ttl_ms),
                        }],
                    };
                    let new_json = serde_json::to_string(&state)?;

                    match self
                        .store
                        .write(WriteRequest {
                            command: WriteCommand::CompareAndSwap {
                                key: key.clone(),
                                expected: None,
                                new_value: new_json,
                            },
                        })
                        .await
                    {
                        Ok(_) => {
                            debug!(name, holder_id, permits, "semaphore created");
                            return Ok(Some((permits, capacity - permits)));
                        }
                        Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => {
                            continue;
                        }
                        Err(e) => bail!("semaphore CAS failed: {}", e),
                    }
                }
                Some(mut state) => {
                    // Cleanup expired holders
                    state.cleanup_expired();

                    // Check if already holding permits
                    if let Some(holder) = state.find_holder(holder_id) {
                        // Refresh TTL and return current permits
                        let mut new_state = state.clone();
                        if let Some(h) = new_state.find_holder_mut(holder_id) {
                            let now = now_unix_ms();
                            h.deadline_ms = crate::verified::compute_holder_deadline(now, ttl_ms);
                        }

                        let old_json = serde_json::to_string(&state)?;
                        let new_json = serde_json::to_string(&new_state)?;

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
                                let available = new_state.available_permits();
                                return Ok(Some((holder.permits, available)));
                            }
                            Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => {
                                continue;
                            }
                            Err(e) => bail!("semaphore CAS failed: {}", e),
                        }
                    }

                    // Check if enough permits available
                    let available = state.available_permits();
                    if available < permits {
                        return Ok(None);
                    }

                    // Tiger Style: Enforce holder limit to prevent resource exhaustion
                    let active_holders = state.holders.len();
                    if active_holders >= MAX_SEMAPHORE_HOLDERS as usize {
                        return Err(CoordinationError::TooManySemaphoreHolders {
                            name: name.to_string(),
                            count: active_holders as u32,
                            max: MAX_SEMAPHORE_HOLDERS,
                        }
                        .into());
                    }

                    // Acquire permits
                    let now = now_unix_ms();
                    let mut new_state = state.clone();
                    new_state.holders.push(SemaphoreHolder {
                        holder_id: holder_id.to_string(),
                        permits,
                        acquired_at_ms: now,
                        deadline_ms: crate::verified::compute_holder_deadline(now, ttl_ms),
                    });

                    let old_json = serde_json::to_string(&state)?;
                    let new_json = serde_json::to_string(&new_state)?;

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
                            let new_available = new_state.available_permits();
                            debug!(name, holder_id, permits, available = new_available, "acquired permits");
                            return Ok(Some((permits, new_available)));
                        }
                        Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => {
                            continue;
                        }
                        Err(e) => bail!("semaphore CAS failed: {}", e),
                    }
                }
            }
        }
    }

    /// Release permits back to the semaphore.
    ///
    /// If permits = 0, releases all permits held by the holder.
    /// Returns the number of available permits after release.
    pub async fn release(&self, name: &str, holder_id: &str, permits: u32) -> Result<u32> {
        let key = verified::semaphore_key(name);

        loop {
            let current = self.read_state(&key).await?;

            match current {
                None => {
                    // No semaphore, nothing to release
                    return Ok(0);
                }
                Some(mut state) => {
                    state.cleanup_expired();

                    // Find holder
                    if state.find_holder(holder_id).is_none() {
                        // Not holding any permits
                        return Ok(state.available_permits());
                    }

                    let mut new_state = state.clone();

                    if permits == 0 {
                        // Release all permits
                        new_state.holders.retain(|h| h.holder_id != holder_id);
                    } else {
                        // Release specific number of permits
                        if let Some(holder) = new_state.find_holder_mut(holder_id) {
                            if permits >= holder.permits {
                                new_state.holders.retain(|h| h.holder_id != holder_id);
                            } else {
                                holder.permits -= permits;
                            }
                        }
                    }

                    let old_json = serde_json::to_string(&state)?;
                    let new_json = serde_json::to_string(&new_state)?;

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
                            let available = new_state.available_permits();
                            debug!(name, holder_id, available, "released permits");
                            return Ok(available);
                        }
                        Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => {
                            continue;
                        }
                        Err(e) => bail!("semaphore CAS failed: {}", e),
                    }
                }
            }
        }
    }

    /// Get semaphore status.
    ///
    /// Returns (available, capacity).
    pub async fn status(&self, name: &str) -> Result<(u32, u32)> {
        let key = verified::semaphore_key(name);

        match self.read_state(&key).await? {
            Some(mut state) => {
                state.cleanup_expired();
                Ok((state.available_permits(), state.capacity))
            }
            None => Ok((0, 0)),
        }
    }

    /// Read semaphore state from the store.
    async fn read_state(&self, key: &str) -> Result<Option<SemaphoreState>> {
        match self.store.read(ReadRequest::new(key.to_string())).await {
            Ok(result) => {
                let value = result.kv.map(|kv| kv.value).unwrap_or_default();
                if value.is_empty() {
                    Ok(None)
                } else {
                    let state: SemaphoreState = serde_json::from_str(&value)?;
                    Ok(Some(state))
                }
            }
            Err(KeyValueStoreError::NotFound { .. }) => Ok(None),
            Err(e) => bail!("semaphore read failed: {}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use aspen_testing::DeterministicKeyValueStore;

    use super::*;

    #[tokio::test]
    async fn test_semaphore_acquire_release() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = SemaphoreManager::new(store);

        // Acquire 2 of 3 permits
        let (acquired, available) = manager.try_acquire("test", "h1", 2, 3, 60000).await.unwrap().unwrap();

        assert_eq!(acquired, 2);
        assert_eq!(available, 1);

        // Release
        let available = manager.release("test", "h1", 0).await.unwrap();
        assert_eq!(available, 3);
    }

    #[tokio::test]
    async fn test_semaphore_capacity_limit() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = SemaphoreManager::new(store);

        // Acquire all permits
        let (acquired, available) = manager.try_acquire("test", "h1", 3, 3, 60000).await.unwrap().unwrap();

        assert_eq!(acquired, 3);
        assert_eq!(available, 0);

        // Try to acquire more - should fail
        let result = manager.try_acquire("test", "h2", 1, 3, 60000).await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_semaphore_status() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = SemaphoreManager::new(store);

        // No semaphore yet
        let (available, capacity) = manager.status("test").await.unwrap();
        assert_eq!(available, 0);
        assert_eq!(capacity, 0);

        // Create semaphore
        manager.try_acquire("test", "h1", 2, 5, 60000).await.unwrap();

        let (available, capacity) = manager.status("test").await.unwrap();
        assert_eq!(available, 3);
        assert_eq!(capacity, 5);
    }

    #[tokio::test]
    async fn test_semaphore_multiple_holders() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = SemaphoreManager::new(store);

        // First holder gets 2
        manager.try_acquire("test", "h1", 2, 5, 60000).await.unwrap().unwrap();

        // Second holder gets 2
        let (acquired, available) = manager.try_acquire("test", "h2", 2, 5, 60000).await.unwrap().unwrap();

        assert_eq!(acquired, 2);
        assert_eq!(available, 1);

        // Third holder can only get 1
        let (acquired, available) = manager.try_acquire("test", "h3", 1, 5, 60000).await.unwrap().unwrap();

        assert_eq!(acquired, 1);
        assert_eq!(available, 0);
    }

    #[tokio::test]
    async fn test_semaphore_holder_limit_enforced() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = SemaphoreManager::new(store);

        // Use a very large capacity so permits aren't the limiting factor
        let capacity = MAX_SEMAPHORE_HOLDERS * 2;

        // Acquire MAX_SEMAPHORE_HOLDERS holders, each holding 1 permit
        for i in 0..MAX_SEMAPHORE_HOLDERS {
            let result = manager.try_acquire("test", &format!("holder{}", i), 1, capacity, 60000).await.unwrap();
            assert!(result.is_some(), "holder {} should acquire permit", i);
        }

        // One more holder should fail with TooManySemaphoreHolders error
        let result = manager.try_acquire("test", "one_too_many", 1, capacity, 60000).await;
        assert!(result.is_err(), "should fail when exceeding holder limit");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("too many holders"), "error should mention 'too many holders', got: {}", err_msg);
    }

    #[tokio::test]
    async fn test_semaphore_concurrent_acquisition() {
        use tokio::task::JoinSet;

        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = Arc::new(SemaphoreManager::new(store));

        let mut tasks = JoinSet::new();
        let num_holders = 10;
        let capacity = 100; // Enough capacity for all

        // Spawn concurrent acquires
        for i in 0..num_holders {
            let mgr = Arc::clone(&manager);
            let holder_id = format!("holder{}", i);
            tasks.spawn(async move { mgr.try_acquire("concurrent_test", &holder_id, 1, capacity, 60000).await });
        }

        // Wait for all to complete
        let mut success_count = 0;
        while let Some(result) = tasks.join_next().await {
            let inner = result.unwrap();
            if inner.is_ok() && inner.unwrap().is_some() {
                success_count += 1;
            }
        }

        assert_eq!(success_count, num_holders, "all holders should acquire permits");

        // Verify status
        let (available, cap) = manager.status("concurrent_test").await.unwrap();
        assert_eq!(cap, capacity);
        assert_eq!(available, capacity - num_holders);
    }
}
