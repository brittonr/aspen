//! Distributed lock-set primitive.
//!
//! Acquires a bounded set of resources atomically with per-resource fencing
//! tokens. Callers never observe partial acquisition.

use std::sync::Arc;
use std::time::Duration;

use aspen_constants::coordination::MAX_LOCKSET_KEYS;
use aspen_kv_types::BatchCondition;
use aspen_kv_types::BatchOperation;
use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::ReadRequest;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use rand::Rng;
use serde::Deserialize;
use serde::Serialize;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::CoordinationError;
use crate::LockConfig;
use crate::error::CasConflictSnafu;
use crate::error::LockSetHeldSnafu;
use crate::error::LockSetLostSnafu;
use crate::error::LockSetMembershipMismatchSnafu;
use crate::error::TimeoutSnafu;
use crate::types::FencingToken;
use crate::types::LockEntry;
use crate::types::LockSetMemberToken;
use crate::types::now_unix_ms;
use crate::verified::compute_backoff_with_jitter;
use crate::verified::compute_lockset_member_token;
use crate::verified::compute_lockset_takeover_count;
use crate::verified::lockset::LockSetValidationError;
use crate::verified::normalize_lockset_member_tokens;
use crate::verified::normalize_lockset_members;
use crate::verified::should_renew_lockset;
use crate::verified::should_retry_lockset_acquire;

/// Request model for a distributed lock set.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockSetRequest {
    holder_id: String,
    members: Vec<String>,
}

impl LockSetRequest {
    /// Create a validated lock-set request with canonical member ordering.
    pub fn new(holder_id: impl Into<String>, members: Vec<String>) -> Result<Self, CoordinationError> {
        let holder_id = holder_id.into();
        let members = map_lockset_validation_error(normalize_lockset_members(members, MAX_LOCKSET_KEYS))?;
        Ok(Self { holder_id, members })
    }

    /// Canonical member list.
    pub fn members(&self) -> &[String] {
        &self.members
    }

    /// Lock holder identifier.
    pub fn holder_id(&self) -> &str {
        &self.holder_id
    }
}

/// Distributed lock set handle.
pub struct DistributedLockSet<S: KeyValueStore + ?Sized> {
    store: Arc<S>,
    request: LockSetRequest,
    config: LockConfig,
}

impl<S: KeyValueStore + ?Sized + 'static> DistributedLockSet<S> {
    /// Create a new lock-set handle.
    pub fn new(
        store: Arc<S>,
        members: Vec<String>,
        holder_id: impl Into<String>,
        config: LockConfig,
    ) -> Result<Self, CoordinationError> {
        debug_assert!(config.ttl_ms > 0, "LOCKSET: ttl_ms must be > 0");
        debug_assert!(config.acquire_timeout_ms > 0, "LOCKSET: acquire_timeout_ms must be > 0");
        debug_assert!(config.initial_backoff_ms > 0, "LOCKSET: initial_backoff_ms must be > 0");
        Ok(Self {
            store,
            request: LockSetRequest::new(holder_id, members)?,
            config,
        })
    }

    /// Canonical member list.
    pub fn members(&self) -> &[String] {
        self.request.members()
    }

    /// Holder ID.
    pub fn holder_id(&self) -> &str {
        self.request.holder_id()
    }

    /// Retrying acquire for the full set.
    pub async fn acquire(&self) -> Result<LockSetGuard<S>, CoordinationError> {
        let acquire_deadline_ms = now_unix_ms().saturating_add(self.config.acquire_timeout_ms);
        let mut backoff_ms = self.config.initial_backoff_ms;

        loop {
            match self.try_acquire().await {
                Ok(guard) => return Ok(guard),
                Err(CoordinationError::LockSetHeld { .. }) => {
                    if !should_retry_lockset_acquire(now_unix_ms(), acquire_deadline_ms) {
                        return TimeoutSnafu {
                            operation: format!("lock-set acquisition for {} members", self.request.members.len()),
                        }
                        .fail();
                    }
                    let jitter_seed = rand::rng().random::<u64>();
                    let backoff = compute_backoff_with_jitter(backoff_ms, self.config.max_backoff_ms, jitter_seed);
                    tokio::time::sleep(Duration::from_millis(backoff.sleep_ms)).await;
                    backoff_ms = backoff.next_backoff_ms;
                }
                Err(CoordinationError::CasConflict) => continue,
                Err(error) => return Err(error),
            }
        }
    }

    /// Immediate acquire attempt.
    pub async fn try_acquire(&self) -> Result<LockSetGuard<S>, CoordinationError> {
        debug!(holder_id = %self.request.holder_id, members = ?self.request.members, "lockset acquire attempt");
        let states = self.read_member_states().await?;
        let prepared = self.prepare_acquire(&states)?;
        if !self.apply_conditional_batch(prepared.conditions, prepared.operations).await? {
            metrics::counter!("aspen.lockset.conflicts_total", "reason" => "cas").increment(1);
            metrics::counter!("aspen.lockset.acquire_attempts_total", "outcome" => "conflict").increment(1);
            return Err(self.describe_acquire_conflict().await?);
        }
        if prepared.takeover_count > 0 {
            metrics::counter!("aspen.lockset.takeovers_total").increment(prepared.takeover_count);
            info!(holder_id = %self.request.holder_id, takeovers = prepared.takeover_count, members = ?self.request.members, "lockset takeover after expiry");
        }
        metrics::counter!("aspen.lockset.acquire_attempts_total", "outcome" => "success").increment(1);
        metrics::histogram!("aspen.lockset.member_count").record(self.request.members.len() as f64);
        Ok(self.build_guard(prepared.member_tokens, prepared.deadline_ms))
    }

    /// Renew all members together using the configured TTL.
    pub async fn renew_member_tokens(&self, member_tokens: &[LockSetMemberToken]) -> Result<u64, CoordinationError> {
        let normalized_tokens = self.normalize_guard_tokens(member_tokens)?;
        let states = self.read_member_states().await?;
        let prepared = self.prepare_renew(&states, &normalized_tokens)?;
        if !self.apply_conditional_batch(prepared.conditions, prepared.operations).await? {
            metrics::counter!("aspen.lockset.renew_total", "outcome" => "conflict").increment(1);
            return Err(self.describe_guard_conflict(&normalized_tokens).await?);
        }
        metrics::counter!("aspen.lockset.renew_total", "outcome" => "success").increment(1);
        debug!(holder_id = %self.request.holder_id, members = ?self.request.members, deadline_ms = prepared.deadline_ms, "lockset renewed");
        Ok(prepared.deadline_ms)
    }

    /// Release all members together.
    pub async fn release_member_tokens(&self, member_tokens: &[LockSetMemberToken]) -> Result<(), CoordinationError> {
        let normalized_tokens = self.normalize_guard_tokens(member_tokens)?;
        let states = self.read_member_states().await?;
        let prepared = self.prepare_release(&states, &normalized_tokens)?;
        if !self.apply_conditional_batch(prepared.conditions, prepared.operations).await? {
            metrics::counter!("aspen.lockset.release_total", "outcome" => "conflict").increment(1);
            return Err(self.describe_guard_conflict(&normalized_tokens).await?);
        }
        metrics::counter!("aspen.lockset.release_total", "outcome" => "success").increment(1);
        debug!(holder_id = %self.request.holder_id, members = ?self.request.members, "lockset released");
        Ok(())
    }

    fn normalize_guard_tokens(
        &self,
        member_tokens: &[LockSetMemberToken],
    ) -> Result<Vec<LockSetMemberToken>, CoordinationError> {
        let normalized = map_lockset_validation_error_tokens(normalize_lockset_member_tokens(
            member_tokens.to_vec(),
            MAX_LOCKSET_KEYS,
        ))?;
        let expected_members = self.request.members.clone();
        let actual_members: Vec<String> = normalized.iter().map(|token| token.member.clone()).collect();
        if actual_members != expected_members {
            return LockSetMembershipMismatchSnafu {
                expected_members: expected_members.join(","),
                actual_members: actual_members.join(","),
            }
            .fail();
        }
        Ok(normalized)
    }

    fn build_guard(&self, member_tokens: Vec<LockSetMemberToken>, deadline_ms: u64) -> LockSetGuard<S> {
        LockSetGuard {
            store: Arc::clone(&self.store),
            request: self.request.clone(),
            config: self.config.clone(),
            member_tokens,
            deadline_ms,
            should_release_on_drop: true,
        }
    }

    async fn read_member_states(&self) -> Result<Vec<LockSetMemberState>, CoordinationError> {
        let mut states = Vec::with_capacity(self.request.members.len());
        for member in &self.request.members {
            let state = self.read_member_state(member).await?;
            states.push(state);
        }
        Ok(states)
    }

    async fn read_member_state(&self, member: &str) -> Result<LockSetMemberState, CoordinationError> {
        match self.store.read(ReadRequest::new(member.to_string())).await {
            Ok(result) => {
                let current_json = result.kv.map(|kv| kv.value).unwrap_or_default();
                let entry: LockEntry =
                    serde_json::from_str(&current_json).map_err(|_| CoordinationError::CorruptedData {
                        key: member.to_string(),
                        reason: "invalid JSON".to_string(),
                    })?;
                Ok(LockSetMemberState::from_current(member.to_string(), current_json, entry))
            }
            Err(KeyValueStoreError::NotFound { .. }) => Ok(LockSetMemberState::missing(member.to_string())),
            Err(source) => Err(CoordinationError::Storage { source }),
        }
    }

    fn prepare_acquire(&self, states: &[LockSetMemberState]) -> Result<PreparedGuardWrite, CoordinationError> {
        let now_ms = now_unix_ms();
        let takeover_count =
            compute_lockset_takeover_count(&states.iter().map(|state| state.entry.clone()).collect::<Vec<_>>(), now_ms);
        let mut conditions = Vec::with_capacity(states.len());
        let mut operations = Vec::with_capacity(states.len());
        let mut member_tokens = Vec::with_capacity(states.len());
        let mut deadline_ms = 0_u64;

        for state in states {
            if let Some(entry) = &state.entry
                && !entry.is_expired()
            {
                metrics::counter!("aspen.lockset.conflicts_total", "reason" => "held").increment(1);
                return LockSetHeldSnafu {
                    member: state.member.clone(),
                    holder: entry.holder_id.clone(),
                    deadline_ms: entry.deadline_ms,
                }
                .fail();
            }
            let new_entry = self.build_acquired_entry(state.entry.as_ref(), now_ms);
            deadline_ms = new_entry.deadline_ms;
            conditions.push(state.acquire_condition());
            operations.push(BatchOperation::Set {
                key: state.member.clone(),
                value: serde_json::to_string(&new_entry)?,
            });
            member_tokens
                .push(LockSetMemberToken::new(state.member.clone(), FencingToken::new(new_entry.fencing_token)));
        }

        Ok(PreparedGuardWrite {
            conditions,
            operations,
            deadline_ms,
            member_tokens,
            takeover_count,
        })
    }

    fn prepare_renew(
        &self,
        states: &[LockSetMemberState],
        member_tokens: &[LockSetMemberToken],
    ) -> Result<PreparedGuardWrite, CoordinationError> {
        let now_ms = now_unix_ms();
        let mut conditions = Vec::with_capacity(states.len());
        let mut operations = Vec::with_capacity(states.len());
        let mut deadline_ms = 0_u64;

        for (state, token) in states.iter().zip(member_tokens.iter()) {
            let entry = state.require_matching_entry(token, self.request.holder_id())?;
            if !should_renew_lockset(entry.deadline_ms, now_ms, entry.ttl_ms) && entry.is_expired() {
                return LockSetLostSnafu {
                    member: state.member.clone(),
                    expected_holder: self.request.holder_id.clone(),
                    current_holder: "expired".to_string(),
                }
                .fail();
            }
            let renewed =
                LockEntry::new_at(self.request.holder_id.clone(), entry.fencing_token, self.config.ttl_ms, now_ms);
            deadline_ms = renewed.deadline_ms;
            conditions.push(state.current_value_condition()?);
            operations.push(BatchOperation::Set {
                key: state.member.clone(),
                value: serde_json::to_string(&renewed)?,
            });
        }

        Ok(PreparedGuardWrite::for_update(conditions, operations, deadline_ms))
    }

    fn prepare_release(
        &self,
        states: &[LockSetMemberState],
        member_tokens: &[LockSetMemberToken],
    ) -> Result<PreparedGuardWrite, CoordinationError> {
        let mut conditions = Vec::with_capacity(states.len());
        let mut operations = Vec::with_capacity(states.len());

        for (state, token) in states.iter().zip(member_tokens.iter()) {
            let entry = state.require_matching_entry(token, self.request.holder_id())?;
            conditions.push(state.current_value_condition()?);
            operations.push(BatchOperation::Set {
                key: state.member.clone(),
                value: serde_json::to_string(&entry.released())?,
            });
        }

        Ok(PreparedGuardWrite::for_update(conditions, operations, 0))
    }

    fn build_acquired_entry(&self, current_entry: Option<&LockEntry>, now_ms: u64) -> LockEntry {
        let token = compute_lockset_member_token(current_entry);
        LockEntry::new_at(self.request.holder_id.clone(), token, self.config.ttl_ms, now_ms)
    }

    async fn apply_conditional_batch(
        &self,
        conditions: Vec<BatchCondition>,
        operations: Vec<BatchOperation>,
    ) -> Result<bool, CoordinationError> {
        let request = WriteRequest::from_command(WriteCommand::ConditionalBatch { conditions, operations });
        let result = self.store.write(request).await.map_err(|source| CoordinationError::Storage { source })?;
        Ok(result.conditions_met.unwrap_or(false))
    }

    async fn describe_acquire_conflict(&self) -> Result<CoordinationError, CoordinationError> {
        let states = self.read_member_states().await?;
        for state in states {
            if let Some(entry) = state.entry
                && !entry.is_expired()
            {
                return LockSetHeldSnafu {
                    member: state.member,
                    holder: entry.holder_id,
                    deadline_ms: entry.deadline_ms,
                }
                .fail();
            }
        }
        CasConflictSnafu.fail()
    }

    async fn describe_guard_conflict(
        &self,
        member_tokens: &[LockSetMemberToken],
    ) -> Result<CoordinationError, CoordinationError> {
        let states = self.read_member_states().await?;
        for (state, token) in states.iter().zip(member_tokens.iter()) {
            if state.require_matching_entry(token, self.request.holder_id()).is_err() {
                return Err(state.guard_conflict(token, self.request.holder_id()));
            }
        }
        CasConflictSnafu.fail()
    }
}

/// Guard for an acquired lock set.
pub struct LockSetGuard<S: KeyValueStore + ?Sized + 'static> {
    store: Arc<S>,
    request: LockSetRequest,
    config: LockConfig,
    member_tokens: Vec<LockSetMemberToken>,
    deadline_ms: u64,
    should_release_on_drop: bool,
}

impl<S: KeyValueStore + ?Sized + 'static> LockSetGuard<S> {
    /// Canonical member-token list.
    pub fn member_tokens(&self) -> &[LockSetMemberToken] {
        &self.member_tokens
    }

    /// Canonical member names.
    pub fn members(&self) -> &[String] {
        self.request.members()
    }

    /// Holder ID.
    pub fn holder_id(&self) -> &str {
        self.request.holder_id()
    }

    /// Shared deadline for the full set.
    pub fn deadline_ms(&self) -> u64 {
        self.deadline_ms
    }

    /// Return the fencing token for one member.
    pub fn fencing_token_for(&self, member: &str) -> Option<FencingToken> {
        self.member_tokens.iter().find(|token| token.member == member).map(|token| token.fencing_token)
    }

    /// Renew the full set using the original configuration TTL.
    pub async fn renew(&mut self) -> Result<(), CoordinationError> {
        let lockset = DistributedLockSet {
            store: Arc::clone(&self.store),
            request: self.request.clone(),
            config: self.config.clone(),
        };
        self.deadline_ms = lockset.renew_member_tokens(&self.member_tokens).await?;
        Ok(())
    }

    /// Release the full set.
    pub async fn release(mut self) -> Result<(), CoordinationError> {
        let result = self.release_impl().await;
        if result.is_ok() {
            self.should_release_on_drop = false;
        }
        result
    }

    async fn release_impl(&self) -> Result<(), CoordinationError> {
        let lockset = DistributedLockSet {
            store: Arc::clone(&self.store),
            request: self.request.clone(),
            config: self.config.clone(),
        };
        lockset.release_member_tokens(&self.member_tokens).await
    }
}

impl<S: KeyValueStore + ?Sized + 'static> Drop for LockSetGuard<S> {
    fn drop(&mut self) {
        if !self.should_release_on_drop {
            return;
        }

        let store = Arc::clone(&self.store);
        let request = self.request.clone();
        let config = self.config.clone();
        let member_tokens = self.member_tokens.clone();

        tokio::spawn(async move {
            let lockset = DistributedLockSet { store, request, config };
            if let Err(error) = lockset.release_member_tokens(&member_tokens).await {
                warn!(error = %error, members = ?lockset.request.members, "lockset release on drop failed");
            }
        });
    }
}

#[derive(Debug, Clone)]
struct LockSetMemberState {
    member: String,
    current_json: Option<String>,
    entry: Option<LockEntry>,
}

impl LockSetMemberState {
    fn missing(member: String) -> Self {
        Self {
            member,
            current_json: None,
            entry: None,
        }
    }

    fn from_current(member: String, current_json: String, entry: LockEntry) -> Self {
        Self {
            member,
            current_json: Some(current_json),
            entry: Some(entry),
        }
    }

    fn acquire_condition(&self) -> BatchCondition {
        match &self.current_json {
            Some(current_json) => BatchCondition::ValueEquals {
                key: self.member.clone(),
                expected: current_json.clone(),
            },
            None => BatchCondition::KeyNotExists {
                key: self.member.clone(),
            },
        }
    }

    fn current_value_condition(&self) -> Result<BatchCondition, CoordinationError> {
        let expected = self.current_json.clone().ok_or_else(|| CoordinationError::CorruptedData {
            key: self.member.clone(),
            reason: "expected current value for guarded member".to_string(),
        })?;
        Ok(BatchCondition::ValueEquals {
            key: self.member.clone(),
            expected,
        })
    }

    fn require_matching_entry<'a>(
        &'a self,
        token: &LockSetMemberToken,
        expected_holder: &str,
    ) -> Result<&'a LockEntry, CoordinationError> {
        let entry = self.entry.as_ref().ok_or_else(|| CoordinationError::LockSetLost {
            member: self.member.clone(),
            expected_holder: expected_holder.to_string(),
            current_holder: "missing".to_string(),
        })?;
        if entry.holder_id != expected_holder || entry.fencing_token != token.fencing_token.value() {
            return Err(self.guard_conflict(token, expected_holder));
        }
        Ok(entry)
    }

    fn guard_conflict(&self, token: &LockSetMemberToken, expected_holder: &str) -> CoordinationError {
        let current_holder = match &self.entry {
            Some(entry) if entry.holder_id == expected_holder => {
                format!("{} (token {})", entry.holder_id, entry.fencing_token)
            }
            Some(entry) => entry.holder_id.clone(),
            None => "missing".to_string(),
        };
        CoordinationError::LockSetLost {
            member: token.member.clone(),
            expected_holder: expected_holder.to_string(),
            current_holder,
        }
    }
}

struct PreparedGuardWrite {
    conditions: Vec<BatchCondition>,
    operations: Vec<BatchOperation>,
    deadline_ms: u64,
    member_tokens: Vec<LockSetMemberToken>,
    takeover_count: u64,
}

impl PreparedGuardWrite {
    fn for_update(conditions: Vec<BatchCondition>, operations: Vec<BatchOperation>, deadline_ms: u64) -> Self {
        Self {
            conditions,
            operations,
            deadline_ms,
            member_tokens: Vec::new(),
            takeover_count: 0,
        }
    }
}

fn map_lockset_validation_error(
    result: Result<Vec<String>, LockSetValidationError>,
) -> Result<Vec<String>, CoordinationError> {
    result.map_err(lockset_validation_error_to_coordination)
}

fn map_lockset_validation_error_tokens(
    result: Result<Vec<LockSetMemberToken>, LockSetValidationError>,
) -> Result<Vec<LockSetMemberToken>, CoordinationError> {
    result.map_err(lockset_validation_error_to_coordination)
}

fn lockset_validation_error_to_coordination(error: LockSetValidationError) -> CoordinationError {
    match error {
        LockSetValidationError::EmptySet => CoordinationError::EmptyLockSet,
        LockSetValidationError::TooManyMembers { count, max } => CoordinationError::LockSetTooLarge { count, max },
        LockSetValidationError::DuplicateMember { member } => CoordinationError::DuplicateLockSetMember { member },
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::AtomicU32;
    use std::sync::atomic::Ordering;

    use aspen_kv_types::DeleteRequest;
    use aspen_kv_types::DeleteResult;
    use aspen_kv_types::KeyValueStoreError;
    use aspen_kv_types::ReadRequest;
    use aspen_kv_types::ReadResult;
    use aspen_kv_types::ScanRequest;
    use aspen_kv_types::ScanResult;
    use aspen_kv_types::WriteRequest;
    use aspen_kv_types::WriteResult;
    use aspen_testing::DeterministicKeyValueStore;
    use async_trait::async_trait;

    use super::*;

    struct CountingStore {
        inner: Arc<DeterministicKeyValueStore>,
        write_count: AtomicU32,
    }

    impl CountingStore {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                inner: DeterministicKeyValueStore::new(),
                write_count: AtomicU32::new(0),
            })
        }

        fn write_count(&self) -> u32 {
            self.write_count.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl KeyValueStore for CountingStore {
        async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError> {
            self.write_count.fetch_add(1, Ordering::SeqCst);
            self.inner.write(request).await
        }

        async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError> {
            self.inner.read(request).await
        }

        async fn delete(&self, request: DeleteRequest) -> Result<DeleteResult, KeyValueStoreError> {
            self.inner.delete(request).await
        }

        async fn scan(&self, request: ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
            self.inner.scan(request).await
        }
    }

    fn test_lockset(
        store: Arc<DeterministicKeyValueStore>,
        holder_id: &str,
        members: &[&str],
    ) -> DistributedLockSet<DeterministicKeyValueStore> {
        DistributedLockSet::new(
            store,
            members.iter().map(|member| member.to_string()).collect(),
            holder_id,
            LockConfig {
                ttl_ms: 50,
                acquire_timeout_ms: 50,
                initial_backoff_ms: 1,
                max_backoff_ms: 5,
            },
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_lockset_canonical_request_model() {
        let request = LockSetRequest::new("holder-a", vec![
            "pipeline:42".to_string(),
            "repo:a".to_string(),
            "deploy:prod".to_string(),
        ])
        .unwrap();
        assert_eq!(request.members(), &["deploy:prod", "pipeline:42", "repo:a"]);
    }

    #[tokio::test]
    async fn test_lockset_try_acquire_release() {
        let store = DeterministicKeyValueStore::new();
        let lockset = test_lockset(store, "holder-a", &["repo:a", "pipeline:42"]);

        let guard = lockset.try_acquire().await.unwrap();
        assert_eq!(guard.members(), &["pipeline:42", "repo:a"]);
        assert!(guard.fencing_token_for("repo:a").is_some());
        guard.release().await.unwrap();
    }

    #[tokio::test]
    async fn test_lockset_contention_is_all_or_nothing() {
        let store = DeterministicKeyValueStore::new();
        let first = test_lockset(store.clone(), "holder-a", &["repo:a", "pipeline:42"]);
        let second = test_lockset(store.clone(), "holder-b", &["repo:a", "deploy:prod"]);

        let _guard = first.try_acquire().await.unwrap();
        let result = second.try_acquire().await;
        assert!(matches!(result, Err(CoordinationError::LockSetHeld { member, .. }) if member == "repo:a"));
        assert!(store.read(ReadRequest::new("deploy:prod".to_string())).await.is_err());
    }

    #[tokio::test]
    async fn test_lockset_reacquire_increments_overlapping_member_token() {
        let store = DeterministicKeyValueStore::new();
        let first = test_lockset(store.clone(), "holder-a", &["repo:a", "pipeline:42"]);
        let second = test_lockset(store, "holder-b", &["repo:a", "deploy:prod"]);

        let guard_a = first.try_acquire().await.unwrap();
        let token_a = guard_a.fencing_token_for("repo:a").unwrap().value();
        guard_a.release().await.unwrap();

        let guard_b = second.try_acquire().await.unwrap();
        let token_b = guard_b.fencing_token_for("repo:a").unwrap().value();
        assert!(token_b > token_a);
    }

    #[tokio::test]
    async fn test_lockset_expiry_takeover() {
        let store = DeterministicKeyValueStore::new();
        let first = test_lockset(store.clone(), "holder-a", &["repo:a", "pipeline:42"]);
        let second = test_lockset(store, "holder-b", &["repo:a", "pipeline:42"]);

        let _guard = first.try_acquire().await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;

        let guard = second.try_acquire().await.unwrap();
        assert!(guard.fencing_token_for("repo:a").unwrap().value() > 1);
    }

    #[tokio::test]
    async fn test_lockset_renew_keeps_tokens() {
        let store = DeterministicKeyValueStore::new();
        let lockset = test_lockset(store, "holder-a", &["repo:a", "pipeline:42"]);

        let mut guard = lockset.try_acquire().await.unwrap();
        let repo_token = guard.fencing_token_for("repo:a").unwrap();
        guard.renew().await.unwrap();
        assert_eq!(guard.fencing_token_for("repo:a"), Some(repo_token));
        assert!(guard.deadline_ms() > 0);
    }

    #[tokio::test]
    async fn test_lockset_explicit_release_disarms_drop() {
        let store = CountingStore::new();
        let lockset = DistributedLockSet::new(
            store.clone(),
            vec!["repo:a".to_string(), "pipeline:42".to_string()],
            "holder-a",
            LockConfig {
                ttl_ms: 50,
                acquire_timeout_ms: 50,
                initial_backoff_ms: 1,
                max_backoff_ms: 5,
            },
        )
        .unwrap();

        let guard = lockset.try_acquire().await.unwrap();
        assert_eq!(store.write_count(), 1);
        guard.release().await.unwrap();
        tokio::time::sleep(Duration::from_millis(25)).await;
        assert_eq!(store.write_count(), 2);
    }
}
