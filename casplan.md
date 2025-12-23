# Plan: Distributed Primitives Built on CAS

This plan details the implementation of high-level distributed coordination primitives built on Aspen's Compare-And-Swap (CAS) operations.

## Overview

With CAS as the foundation, we will implement four production-grade distributed primitives:

1. **Distributed Locks** - Mutual exclusion with fencing tokens and TTL
2. **Atomic Counters** - Race-free increment/decrement operations
3. **Sequence Generator** - Monotonically increasing IDs
4. **Rate Limiter** - Token bucket with distributed state

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     Application Layer                           │
├─────────────────────────────────────────────────────────────────┤
│  DistributedLock  │  AtomicCounter  │  SequenceGen  │  RateLimiter
├─────────────────────────────────────────────────────────────────┤
│                  Coordination Primitives API                     │
│                    src/coordination/mod.rs                       │
├─────────────────────────────────────────────────────────────────┤
│                    KeyValueStore Trait                           │
│              (CAS: CompareAndSwap, CompareAndDelete)             │
├─────────────────────────────────────────────────────────────────┤
│                    Raft Consensus Layer                          │
│                    (Linearizable Operations)                     │
└─────────────────────────────────────────────────────────────────┘
```

## Files to Create/Modify

### New Files

```
src/coordination/
├── mod.rs              # Module exports
├── lock.rs             # DistributedLock implementation
├── counter.rs          # AtomicCounter implementation
├── sequence.rs         # SequenceGenerator implementation
├── rate_limiter.rs     # Distributed rate limiter
├── types.rs            # Shared types (LockEntry, FencingToken, etc.)
└── error.rs            # Coordination-specific errors

tests/
├── coordination_lock_test.rs      # Lock unit tests
├── coordination_counter_test.rs   # Counter unit tests
├── coordination_sequence_test.rs  # Sequence generator tests
├── coordination_rate_limiter_test.rs # Rate limiter tests
├── madsim_coordination_test.rs    # Deterministic simulation tests
└── coordination_proptest.rs       # Property-based tests
```

### Modified Files

```
src/lib.rs              # Add pub mod coordination
src/client_rpc.rs       # Add coordination RPC types
src/protocol_handlers.rs # Add coordination handlers
```

---

## 1. Distributed Lock Implementation

### Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Fencing tokens | Monotonically increasing u64 | Prevents split-brain via token validation |
| TTL mechanism | Stored in lock entry, checked on read | Auto-release on holder crash |
| Lock renewal | Explicit renew() method | Client controls when to extend |
| Fairness | Optional FIFO queue | Trade latency for strict ordering |
| Acquisition | Exponential backoff with jitter | Prevents thundering herd |

### Data Structures

```rust
// src/coordination/types.rs

/// Lock entry stored in KV store as JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockEntry {
    /// Unique identifier of the lock holder.
    pub holder_id: String,
    /// Monotonically increasing token for fencing.
    pub fencing_token: u64,
    /// When the lock was acquired (Unix timestamp ms).
    pub acquired_at_ms: u64,
    /// TTL in milliseconds.
    pub ttl_ms: u64,
    /// Deadline = acquired_at_ms + ttl_ms.
    pub deadline_ms: u64,
}

impl LockEntry {
    /// Check if this lock entry has expired.
    pub fn is_expired(&self) -> bool {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        now_ms > self.deadline_ms
    }
}

/// Fencing token returned on successful lock acquisition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FencingToken(pub u64);

impl FencingToken {
    /// Create from raw value.
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// Get the raw token value.
    pub fn value(&self) -> u64 {
        self.0
    }
}
```

### Lock Interface

```rust
// src/coordination/lock.rs

/// Configuration for distributed lock.
pub struct LockConfig {
    /// Time-to-live for the lock in milliseconds.
    pub ttl_ms: u64,
    /// Maximum time to wait for lock acquisition.
    pub acquire_timeout_ms: u64,
    /// Initial backoff for retry (doubles each attempt).
    pub initial_backoff_ms: u64,
    /// Maximum backoff between retries.
    pub max_backoff_ms: u64,
}

impl Default for LockConfig {
    fn default() -> Self {
        Self {
            ttl_ms: 30_000,           // 30 seconds
            acquire_timeout_ms: 10_000, // 10 seconds
            initial_backoff_ms: 10,    // 10ms initial
            max_backoff_ms: 1_000,     // 1 second max
        }
    }
}

/// A distributed mutex lock.
pub struct DistributedLock {
    store: Arc<dyn KeyValueStore>,
    key: String,
    holder_id: String,
    config: LockConfig,
}

impl DistributedLock {
    /// Create a new distributed lock handle.
    pub fn new(
        store: Arc<dyn KeyValueStore>,
        key: impl Into<String>,
        holder_id: impl Into<String>,
        config: LockConfig,
    ) -> Self;

    /// Attempt to acquire the lock.
    ///
    /// Returns the fencing token on success.
    /// Retries with exponential backoff until timeout.
    pub async fn acquire(&self) -> Result<LockGuard, CoordinationError>;

    /// Try to acquire the lock without blocking.
    ///
    /// Returns immediately with success or failure.
    pub async fn try_acquire(&self) -> Result<LockGuard, CoordinationError>;

    /// Extend the lock's TTL.
    ///
    /// Must be called before the lock expires to prevent release.
    /// Returns error if lock was lost (another holder acquired it).
    pub async fn renew(&self, guard: &LockGuard) -> Result<(), CoordinationError>;
}

/// RAII guard that releases the lock on drop.
pub struct LockGuard {
    store: Arc<dyn KeyValueStore>,
    key: String,
    fencing_token: FencingToken,
    entry_json: String, // For CAS release
}

impl LockGuard {
    /// Get the fencing token.
    ///
    /// Include this token in all protected operations.
    pub fn fencing_token(&self) -> FencingToken {
        self.fencing_token
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        // Spawn release task (best-effort, lock will expire anyway)
        let store = self.store.clone();
        let key = self.key.clone();
        let expected = self.entry_json.clone();
        tokio::spawn(async move {
            let _ = store.write(WriteRequest {
                command: WriteCommand::CompareAndDelete { key, expected },
            }).await;
        });
    }
}
```

### Lock Implementation Algorithm

```rust
// Acquisition algorithm with exponential backoff
async fn acquire(&self) -> Result<LockGuard, CoordinationError> {
    let deadline = Instant::now() + Duration::from_millis(self.config.acquire_timeout_ms);
    let mut backoff_ms = self.config.initial_backoff_ms;
    let mut rng = rand::thread_rng();

    loop {
        // Try to acquire
        match self.try_acquire_once().await {
            Ok(guard) => return Ok(guard),
            Err(CoordinationError::LockHeld { holder, deadline_ms }) => {
                // Lock is held, check timeout
                if Instant::now() >= deadline {
                    return Err(CoordinationError::Timeout {
                        operation: "lock acquisition".to_string(),
                    });
                }

                // Wait with jitter
                let jitter = rng.gen_range(0..backoff_ms / 2);
                let sleep_ms = backoff_ms + jitter;
                tokio::time::sleep(Duration::from_millis(sleep_ms)).await;

                // Exponential backoff
                backoff_ms = (backoff_ms * 2).min(self.config.max_backoff_ms);
            }
            Err(e) => return Err(e),
        }
    }
}

async fn try_acquire_once(&self) -> Result<LockGuard, CoordinationError> {
    // Read current lock state
    let current = self.read_lock_entry().await?;

    // Check if lock is free or expired
    let (expected, new_token) = match current {
        Some(entry) if !entry.is_expired() => {
            // Lock held by someone else
            return Err(CoordinationError::LockHeld {
                holder: entry.holder_id,
                deadline_ms: entry.deadline_ms,
            });
        }
        Some(entry) => {
            // Lock expired, we can take it
            (Some(serde_json::to_string(&entry)?), entry.fencing_token + 1)
        }
        None => {
            // Lock doesn't exist
            (None, 1)
        }
    };

    // Create new lock entry
    let now_ms = now_unix_ms();
    let new_entry = LockEntry {
        holder_id: self.holder_id.clone(),
        fencing_token: new_token,
        acquired_at_ms: now_ms,
        ttl_ms: self.config.ttl_ms,
        deadline_ms: now_ms + self.config.ttl_ms,
    };
    let new_json = serde_json::to_string(&new_entry)?;

    // Atomic CAS
    match self.store.write(WriteRequest {
        command: WriteCommand::CompareAndSwap {
            key: self.key.clone(),
            expected,
            new_value: new_json.clone(),
        },
    }).await {
        Ok(_) => Ok(LockGuard {
            store: self.store.clone(),
            key: self.key.clone(),
            fencing_token: FencingToken(new_token),
            entry_json: new_json,
        }),
        Err(KeyValueStoreError::CompareAndSwapFailed { actual, .. }) => {
            // Someone else got it
            if let Some(json) = actual {
                let entry: LockEntry = serde_json::from_str(&json)?;
                Err(CoordinationError::LockHeld {
                    holder: entry.holder_id,
                    deadline_ms: entry.deadline_ms,
                })
            } else {
                // Deleted between our read and CAS, retry
                Err(CoordinationError::Retry)
            }
        }
        Err(e) => Err(CoordinationError::Storage(e)),
    }
}
```

### Fencing Token Validation Pattern

```rust
/// Trait for services that accept fencing tokens.
pub trait FenceAware {
    /// Validate a fencing token before executing an operation.
    ///
    /// Returns error if the token is stale (lower than current).
    fn validate_fence(&self, token: FencingToken) -> Result<(), FenceError>;
}

/// Example: Protected storage that validates fencing tokens.
pub struct FencedStorage<S> {
    inner: S,
    /// Current highest token seen per lock key.
    tokens: DashMap<String, u64>,
}

impl<S> FencedStorage<S> {
    pub fn execute<T>(
        &self,
        lock_key: &str,
        token: FencingToken,
        operation: impl FnOnce(&S) -> T,
    ) -> Result<T, FenceError> {
        // Check and update token
        let current = self.tokens.entry(lock_key.to_string())
            .or_insert(0);

        if token.value() < *current {
            return Err(FenceError::StaleToken {
                presented: token.value(),
                current: *current,
            });
        }

        *current = token.value();
        drop(current); // Release lock before operation

        Ok(operation(&self.inner))
    }
}
```

---

## 2. Atomic Counter Implementation

### Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Storage format | JSON with u64 value | Simple, human-readable |
| Overflow handling | Saturating arithmetic | Prevents panic on overflow |
| Negative values | Signed i64 option | Some use cases need negative |
| Batch increment | Local aggregation | Reduces Raft round-trips |
| Contention strategy | Immediate retry with jitter | Fast convergence |

### Counter Interface

```rust
// src/coordination/counter.rs

/// Configuration for atomic counter.
pub struct CounterConfig {
    /// Maximum retries on CAS failure.
    pub max_retries: u32,
    /// Base delay between retries.
    pub retry_delay_ms: u64,
}

impl Default for CounterConfig {
    fn default() -> Self {
        Self {
            max_retries: 100,      // Many retries for high contention
            retry_delay_ms: 1,     // Very short delay
        }
    }
}

/// Unsigned atomic counter.
pub struct AtomicCounter {
    store: Arc<dyn KeyValueStore>,
    key: String,
    config: CounterConfig,
}

impl AtomicCounter {
    /// Create a new atomic counter.
    pub fn new(
        store: Arc<dyn KeyValueStore>,
        key: impl Into<String>,
        config: CounterConfig,
    ) -> Self;

    /// Get the current counter value.
    pub async fn get(&self) -> Result<u64, CoordinationError>;

    /// Increment the counter by 1 and return the new value.
    pub async fn increment(&self) -> Result<u64, CoordinationError>;

    /// Increment by a specific amount and return the new value.
    pub async fn add(&self, amount: u64) -> Result<u64, CoordinationError>;

    /// Decrement the counter by 1 (saturating at 0).
    pub async fn decrement(&self) -> Result<u64, CoordinationError>;

    /// Subtract amount (saturating at 0).
    pub async fn subtract(&self, amount: u64) -> Result<u64, CoordinationError>;

    /// Reset counter to zero.
    pub async fn reset(&self) -> Result<(), CoordinationError>;

    /// Compare-and-set: atomically set value if current equals expected.
    pub async fn compare_and_set(
        &self,
        expected: u64,
        new_value: u64,
    ) -> Result<bool, CoordinationError>;
}

/// Signed atomic counter (allows negative values).
pub struct SignedAtomicCounter {
    store: Arc<dyn KeyValueStore>,
    key: String,
    config: CounterConfig,
}

impl SignedAtomicCounter {
    pub async fn get(&self) -> Result<i64, CoordinationError>;
    pub async fn add(&self, amount: i64) -> Result<i64, CoordinationError>;
    // ... similar methods with i64
}
```

### Counter Implementation

```rust
impl AtomicCounter {
    pub async fn add(&self, amount: u64) -> Result<u64, CoordinationError> {
        let mut attempt = 0;
        let mut rng = rand::thread_rng();

        loop {
            // Read current value
            let current = match self.store.read(ReadRequest {
                key: self.key.clone(),
            }).await {
                Ok(result) => result.value.parse::<u64>()
                    .map_err(|_| CoordinationError::CorruptedData {
                        key: self.key.clone(),
                    })?,
                Err(KeyValueStoreError::NotFound { .. }) => 0,
                Err(e) => return Err(CoordinationError::Storage(e)),
            };

            // Calculate new value (saturating to prevent overflow)
            let new_value = current.saturating_add(amount);

            // Try CAS
            match self.store.write(WriteRequest {
                command: WriteCommand::CompareAndSwap {
                    key: self.key.clone(),
                    expected: if current == 0 { None } else { Some(current.to_string()) },
                    new_value: new_value.to_string(),
                },
            }).await {
                Ok(_) => return Ok(new_value),
                Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => {
                    // Contention - retry
                    attempt += 1;
                    if attempt >= self.config.max_retries {
                        return Err(CoordinationError::MaxRetriesExceeded {
                            operation: "counter increment".to_string(),
                            attempts: attempt,
                        });
                    }

                    // Jittered delay to reduce contention
                    let jitter = rng.gen_range(0..self.config.retry_delay_ms);
                    tokio::time::sleep(Duration::from_millis(jitter)).await;
                }
                Err(e) => return Err(CoordinationError::Storage(e)),
            }
        }
    }
}
```

### Batched Counter for High Throughput

```rust
/// Buffered counter that batches local increments.
///
/// Accumulates increments locally and flushes to storage periodically.
/// Trade-off: lower latency per increment, but may lose unflushed counts on crash.
pub struct BufferedCounter {
    /// Underlying atomic counter.
    counter: AtomicCounter,
    /// Local accumulator.
    local: AtomicU64,
    /// Flush threshold.
    flush_threshold: u64,
    /// Flush interval.
    flush_interval: Duration,
    /// Background flusher handle.
    flusher: Option<JoinHandle<()>>,
}

impl BufferedCounter {
    /// Increment locally (fast, no network).
    pub fn increment(&self) {
        let prev = self.local.fetch_add(1, Ordering::Relaxed);
        if prev + 1 >= self.flush_threshold {
            self.trigger_flush();
        }
    }

    /// Flush accumulated count to storage.
    pub async fn flush(&self) -> Result<u64, CoordinationError> {
        let to_flush = self.local.swap(0, Ordering::AcqRel);
        if to_flush > 0 {
            self.counter.add(to_flush).await
        } else {
            self.counter.get().await
        }
    }

    /// Get accurate count (flushes first).
    pub async fn get_accurate(&self) -> Result<u64, CoordinationError> {
        self.flush().await
    }

    /// Get approximate count (may not include recent local increments).
    pub async fn get_approximate(&self) -> Result<u64, CoordinationError> {
        let stored = self.counter.get().await?;
        let local = self.local.load(Ordering::Relaxed);
        Ok(stored + local)
    }
}
```

---

## 3. Sequence Generator Implementation

### Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Uniqueness | Cluster-wide via Raft | Linearizable guarantees uniqueness |
| Performance | Batch reservation | Reduces Raft round-trips |
| Format | Plain u64 | Simple, sortable |
| Overflow | Error on u64::MAX | Explicit failure better than wrap |

### Sequence Generator Interface

```rust
// src/coordination/sequence.rs

/// Configuration for sequence generator.
pub struct SequenceConfig {
    /// Number of IDs to reserve in each batch.
    pub batch_size: u64,
    /// Start value for new sequences.
    pub start_value: u64,
}

impl Default for SequenceConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            start_value: 1,
        }
    }
}

/// Distributed sequence number generator.
///
/// Generates globally unique, monotonically increasing IDs.
/// Uses batch reservation for performance.
pub struct SequenceGenerator {
    store: Arc<dyn KeyValueStore>,
    key: String,
    config: SequenceConfig,
    /// Local batch state.
    state: Mutex<SequenceState>,
}

struct SequenceState {
    /// Next ID to return.
    next: u64,
    /// End of current batch (exclusive).
    batch_end: u64,
}

impl SequenceGenerator {
    /// Create a new sequence generator.
    pub fn new(
        store: Arc<dyn KeyValueStore>,
        key: impl Into<String>,
        config: SequenceConfig,
    ) -> Self;

    /// Get the next sequence number.
    ///
    /// Fast path: returns from local batch.
    /// Slow path: reserves new batch from cluster.
    pub async fn next(&self) -> Result<u64, CoordinationError>;

    /// Reserve a range of N sequence numbers.
    ///
    /// Returns the start of the range (inclusive).
    /// Caller gets [start, start + count).
    pub async fn reserve(&self, count: u64) -> Result<u64, CoordinationError>;

    /// Get current sequence value without incrementing.
    pub async fn current(&self) -> Result<u64, CoordinationError>;
}
```

### Sequence Generator Implementation

```rust
impl SequenceGenerator {
    pub async fn next(&self) -> Result<u64, CoordinationError> {
        // Fast path: check local batch
        {
            let mut state = self.state.lock().await;
            if state.next < state.batch_end {
                let id = state.next;
                state.next += 1;
                return Ok(id);
            }
        }

        // Slow path: reserve new batch
        let batch_start = self.reserve_batch().await?;

        // Update local state
        let mut state = self.state.lock().await;
        state.next = batch_start + 1; // +1 because we return batch_start
        state.batch_end = batch_start + self.config.batch_size;

        Ok(batch_start)
    }

    async fn reserve_batch(&self) -> Result<u64, CoordinationError> {
        self.reserve(self.config.batch_size).await
    }

    pub async fn reserve(&self, count: u64) -> Result<u64, CoordinationError> {
        loop {
            // Read current sequence value
            let current = match self.store.read(ReadRequest {
                key: self.key.clone(),
            }).await {
                Ok(result) => result.value.parse::<u64>()
                    .map_err(|_| CoordinationError::CorruptedData {
                        key: self.key.clone(),
                    })?,
                Err(KeyValueStoreError::NotFound { .. }) => self.config.start_value,
                Err(e) => return Err(CoordinationError::Storage(e)),
            };

            // Check for overflow
            let new_value = current.checked_add(count)
                .ok_or(CoordinationError::SequenceExhausted {
                    key: self.key.clone(),
                })?;

            // Reserve range with CAS
            let expected = if current == self.config.start_value {
                None
            } else {
                Some(current.to_string())
            };

            match self.store.write(WriteRequest {
                command: WriteCommand::CompareAndSwap {
                    key: self.key.clone(),
                    expected,
                    new_value: new_value.to_string(),
                },
            }).await {
                Ok(_) => return Ok(current), // Return start of reserved range
                Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => {
                    // Contention, retry immediately
                    continue;
                }
                Err(e) => return Err(CoordinationError::Storage(e)),
            }
        }
    }
}
```

---

## 4. Distributed Rate Limiter Implementation

### Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Algorithm | Token bucket | Industry standard, allows bursts |
| State storage | Cluster-wide KV | Survives node failures |
| Precision | Millisecond | Good enough for most use cases |
| Clock sync | Relative time deltas | Avoids clock drift issues |

### Rate Limiter Interface

```rust
// src/coordination/rate_limiter.rs

/// Configuration for distributed rate limiter.
pub struct RateLimiterConfig {
    /// Maximum tokens (burst capacity).
    pub capacity: u64,
    /// Tokens added per second.
    pub refill_rate: f64,
    /// Initial tokens (defaults to capacity).
    pub initial_tokens: Option<u64>,
}

/// Distributed token bucket rate limiter.
pub struct DistributedRateLimiter {
    store: Arc<dyn KeyValueStore>,
    key: String,
    config: RateLimiterConfig,
}

/// Stored state of the rate limiter.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BucketState {
    /// Current token count (can be fractional internally).
    tokens: f64,
    /// Last update timestamp (Unix ms).
    last_update_ms: u64,
    /// Capacity for validation.
    capacity: u64,
    /// Refill rate for validation.
    refill_rate: f64,
}

impl DistributedRateLimiter {
    /// Create a new rate limiter.
    pub fn new(
        store: Arc<dyn KeyValueStore>,
        key: impl Into<String>,
        config: RateLimiterConfig,
    ) -> Self;

    /// Try to consume one token.
    ///
    /// Returns `Ok(remaining)` if allowed, `Err` if rate limited.
    pub async fn try_acquire(&self) -> Result<u64, RateLimitError>;

    /// Try to consume N tokens.
    pub async fn try_acquire_n(&self, n: u64) -> Result<u64, RateLimitError>;

    /// Block until a token is available (with timeout).
    pub async fn acquire(&self, timeout: Duration) -> Result<u64, RateLimitError>;

    /// Get current available tokens without consuming.
    pub async fn available(&self) -> Result<u64, CoordinationError>;

    /// Reset the bucket to full capacity.
    pub async fn reset(&self) -> Result<(), CoordinationError>;
}

/// Error when rate limited.
#[derive(Debug)]
pub struct RateLimitError {
    /// Tokens requested.
    pub requested: u64,
    /// Tokens available.
    pub available: u64,
    /// Estimated wait time in milliseconds.
    pub retry_after_ms: u64,
}
```

### Rate Limiter Implementation

```rust
impl DistributedRateLimiter {
    pub async fn try_acquire_n(&self, n: u64) -> Result<u64, RateLimitError> {
        loop {
            // Read current state
            let now_ms = now_unix_ms();
            let current = self.read_state().await?;

            // Calculate replenished tokens
            let elapsed_ms = now_ms.saturating_sub(current.last_update_ms);
            let elapsed_secs = elapsed_ms as f64 / 1000.0;
            let replenished = elapsed_secs * current.refill_rate;
            let available = (current.tokens + replenished).min(current.capacity as f64);

            // Check if we can acquire
            if (n as f64) > available {
                let deficit = (n as f64) - available;
                let wait_secs = deficit / current.refill_rate;
                return Err(RateLimitError {
                    requested: n,
                    available: available as u64,
                    retry_after_ms: (wait_secs * 1000.0) as u64,
                });
            }

            // Prepare new state
            let new_state = BucketState {
                tokens: available - (n as f64),
                last_update_ms: now_ms,
                capacity: current.capacity,
                refill_rate: current.refill_rate,
            };

            // Atomic update
            match self.cas_state(&current, &new_state).await {
                Ok(_) => return Ok(new_state.tokens as u64),
                Err(CoordinationError::CasFailed { .. }) => {
                    // Contention, retry immediately
                    continue;
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    pub async fn acquire(&self, timeout: Duration) -> Result<u64, RateLimitError> {
        let deadline = Instant::now() + timeout;

        loop {
            match self.try_acquire().await {
                Ok(remaining) => return Ok(remaining),
                Err(e) => {
                    let wait = Duration::from_millis(e.retry_after_ms.min(100));
                    if Instant::now() + wait > deadline {
                        return Err(e);
                    }
                    tokio::time::sleep(wait).await;
                }
            }
        }
    }
}
```

---

## 5. Error Types

```rust
// src/coordination/error.rs

use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum CoordinationError {
    #[snafu(display("lock held by {holder} until {deadline_ms}"))]
    LockHeld {
        holder: String,
        deadline_ms: u64,
    },

    #[snafu(display("lock lost: held by {current_holder}, not {expected_holder}"))]
    LockLost {
        expected_holder: String,
        current_holder: String,
    },

    #[snafu(display("operation timed out: {operation}"))]
    Timeout {
        operation: String,
    },

    #[snafu(display("max retries exceeded for {operation}: {attempts} attempts"))]
    MaxRetriesExceeded {
        operation: String,
        attempts: u32,
    },

    #[snafu(display("sequence exhausted for key {key}"))]
    SequenceExhausted {
        key: String,
    },

    #[snafu(display("corrupted data in key {key}"))]
    CorruptedData {
        key: String,
    },

    #[snafu(display("CAS failed, retry needed"))]
    CasFailed,

    #[snafu(display("storage error: {source}"))]
    Storage {
        source: KeyValueStoreError,
    },

    #[snafu(display("serialization error: {source}"))]
    Serialization {
        source: serde_json::Error,
    },
}

#[derive(Debug, Snafu)]
pub enum FenceError {
    #[snafu(display("stale fencing token: presented {presented}, current {current}"))]
    StaleToken {
        presented: u64,
        current: u64,
    },
}
```

---

## 6. Client RPC Additions

```rust
// Additions to src/client_rpc.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientRpcRequest {
    // ... existing variants ...

    // Lock operations
    LockAcquire {
        key: String,
        holder_id: String,
        ttl_ms: u64,
        timeout_ms: u64,
    },
    LockTryAcquire {
        key: String,
        holder_id: String,
        ttl_ms: u64,
    },
    LockRelease {
        key: String,
        fencing_token: u64,
    },
    LockRenew {
        key: String,
        fencing_token: u64,
        ttl_ms: u64,
    },

    // Counter operations
    CounterGet { key: String },
    CounterIncrement { key: String, amount: u64 },
    CounterDecrement { key: String, amount: u64 },
    CounterReset { key: String },

    // Sequence operations
    SequenceNext { key: String },
    SequenceReserve { key: String, count: u64 },

    // Rate limiter operations
    RateLimitTryAcquire { key: String, tokens: u64 },
    RateLimitReset { key: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientRpcResponse {
    // ... existing variants ...

    LockAcquired { fencing_token: u64 },
    LockReleased,
    LockRenewed,
    LockFailed { holder: String, retry_after_ms: u64 },

    CounterValue { value: u64 },
    CounterUpdated { new_value: u64 },

    SequenceValue { value: u64 },
    SequenceRange { start: u64, count: u64 },

    RateLimitAllowed { remaining: u64 },
    RateLimitDenied { retry_after_ms: u64 },
}
```

---

## 7. Testing Strategy

### Unit Tests

```rust
// tests/coordination_lock_test.rs

#[tokio::test]
async fn test_lock_acquire_release() {
    let store = DeterministicKeyValueStore::new();
    let lock = DistributedLock::new(
        Arc::new(store),
        "test_lock",
        "holder_1",
        LockConfig::default(),
    );

    let guard = lock.try_acquire().await.unwrap();
    assert!(guard.fencing_token().value() > 0);
    drop(guard);
}

#[tokio::test]
async fn test_lock_contention() {
    let store = Arc::new(DeterministicKeyValueStore::new());

    let lock1 = DistributedLock::new(store.clone(), "test_lock", "h1", Default::default());
    let lock2 = DistributedLock::new(store.clone(), "test_lock", "h2", Default::default());

    let _guard1 = lock1.try_acquire().await.unwrap();
    let result = lock2.try_acquire().await;
    assert!(matches!(result, Err(CoordinationError::LockHeld { .. })));
}

#[tokio::test]
async fn test_lock_fencing_token_increases() {
    let store = Arc::new(DeterministicKeyValueStore::new());
    let lock = DistributedLock::new(store, "test_lock", "h1", Default::default());

    let guard1 = lock.try_acquire().await.unwrap();
    let token1 = guard1.fencing_token();
    drop(guard1);

    let guard2 = lock.try_acquire().await.unwrap();
    let token2 = guard2.fencing_token();

    assert!(token2.value() > token1.value());
}

// tests/coordination_counter_test.rs

#[tokio::test]
async fn test_counter_increment() {
    let store = Arc::new(DeterministicKeyValueStore::new());
    let counter = AtomicCounter::new(store, "test_counter", Default::default());

    assert_eq!(counter.get().await.unwrap(), 0);
    assert_eq!(counter.increment().await.unwrap(), 1);
    assert_eq!(counter.increment().await.unwrap(), 2);
    assert_eq!(counter.get().await.unwrap(), 2);
}

#[tokio::test]
async fn test_counter_concurrent_increments() {
    let store = Arc::new(DeterministicKeyValueStore::new());
    let counter = Arc::new(AtomicCounter::new(store, "test_counter", Default::default()));

    let handles: Vec<_> = (0..100)
        .map(|_| {
            let c = counter.clone();
            tokio::spawn(async move { c.increment().await })
        })
        .collect();

    for h in handles {
        h.await.unwrap().unwrap();
    }

    assert_eq!(counter.get().await.unwrap(), 100);
}
```

### Property-Based Tests

```rust
// tests/coordination_proptest.rs

proptest! {
    #[test]
    fn counter_increments_are_monotonic(ops in vec(1u64..100, 1..50)) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let store = Arc::new(DeterministicKeyValueStore::new());
            let counter = AtomicCounter::new(store, "test", Default::default());

            let mut prev = 0;
            for amount in ops {
                let new = counter.add(amount).await.unwrap();
                prop_assert!(new > prev);
                prev = new;
            }
            Ok(())
        })?;
    }

    #[test]
    fn sequence_numbers_are_unique(batch_sizes in vec(1u64..100, 1..20)) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let store = Arc::new(DeterministicKeyValueStore::new());
            let seq = SequenceGenerator::new(store, "test", Default::default());

            let mut seen = HashSet::new();
            for _ in batch_sizes {
                let id = seq.next().await.unwrap();
                prop_assert!(!seen.contains(&id), "Duplicate ID: {}", id);
                seen.insert(id);
            }
            Ok(())
        })?;
    }
}
```

### Simulation Tests (madsim)

```rust
// tests/madsim_coordination_test.rs

#[madsim::test]
async fn test_lock_survives_leader_failover() {
    let cluster = TestCluster::new(3).await;
    cluster.init().await.unwrap();

    let lock = DistributedLock::new(
        cluster.client(0),
        "test_lock",
        "holder_1",
        Default::default(),
    );

    let guard = lock.acquire().await.unwrap();
    let token = guard.fencing_token();

    // Kill leader
    cluster.kill_leader().await;

    // Wait for election
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Lock should still be valid
    let lock2 = DistributedLock::new(
        cluster.client(1),
        "test_lock",
        "holder_2",
        Default::default(),
    );
    let result = lock2.try_acquire().await;
    assert!(matches!(result, Err(CoordinationError::LockHeld { .. })));
}

#[madsim::test]
async fn test_counter_linearizable_under_partition() {
    let cluster = TestCluster::new(5).await;
    cluster.init().await.unwrap();

    let counter = AtomicCounter::new(cluster.client(0), "cnt", Default::default());

    // Increment from majority partition
    for _ in 0..10 {
        counter.increment().await.unwrap();
    }

    // Create partition
    cluster.partition(vec![0, 1, 2], vec![3, 4]).await;

    // Minority should not be able to increment (no quorum)
    let minority_counter = AtomicCounter::new(cluster.client(3), "cnt", Default::default());
    let result = minority_counter.increment().await;
    assert!(result.is_err());

    // Majority can continue
    counter.increment().await.unwrap();
    assert_eq!(counter.get().await.unwrap(), 11);
}
```

---

## 8. Implementation Order

### Phase 1: Core Types and Lock (3-4 hours)

1. Create `src/coordination/mod.rs` with module structure
2. Create `src/coordination/types.rs` with `LockEntry`, `FencingToken`
3. Create `src/coordination/error.rs` with error types
4. Implement `DistributedLock` in `src/coordination/lock.rs`
5. Write unit tests for lock

### Phase 2: Counter and Sequence (2-3 hours)

1. Implement `AtomicCounter` in `src/coordination/counter.rs`
2. Implement `SequenceGenerator` in `src/coordination/sequence.rs`
3. Write unit tests for both

### Phase 3: Rate Limiter (2 hours)

1. Implement `DistributedRateLimiter` in `src/coordination/rate_limiter.rs`
2. Write unit tests

### Phase 4: Client RPC Integration (2 hours)

1. Add RPC types to `src/client_rpc.rs`
2. Add handlers to `src/protocol_handlers.rs`
3. Integration tests

### Phase 5: Simulation and Property Tests (2-3 hours)

1. madsim tests for fault tolerance
2. proptest for invariants
3. Chaos tests

---

## 9. Success Criteria

- [ ] All existing tests pass
- [ ] Lock acquire/release works correctly
- [ ] Lock fencing tokens are monotonically increasing
- [ ] Lock TTL expires and allows re-acquisition
- [ ] Counter increments are linearizable
- [ ] Sequence numbers are globally unique
- [ ] Rate limiter correctly limits requests
- [ ] All primitives survive leader failover
- [ ] Property tests verify invariants
- [ ] Simulation tests verify behavior under partitions
- [ ] `cargo clippy` clean
- [ ] `cargo fmt` clean

---

## References

- [Martin Kleppmann: How to do distributed locking](https://martin.kleppmann.com/2016/02/08/how-to-do-distributed-locking.html)
- [Fencing Tokens: Preventing Split Brain Operations](https://www.systemoverflow.com/learn/distributed-primitives/distributed-locks/fencing-tokens-preventing-split-brain-operations)
- [etcd Distributed Lock Implementation](https://etcd.io/docs/v3.5/learning/why/)
- [Redis Distributed Locks](https://redis.io/docs/latest/develop/clients/patterns/distributed-locks/)
- [Azure Cosmos DB Distributed Counter Pattern](https://devblogs.microsoft.com/cosmosdb/azure-cosmos-db-design-patterns-part-3-distributed-counter/)
- [Distributed Counter System Design](https://systemdesign.one/distributed-counter-system-design/)
- [Tokio Semaphore Documentation](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html)
