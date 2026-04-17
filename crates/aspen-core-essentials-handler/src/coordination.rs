//! Native coordination primitive handler.
//!
//! Provides native implementations for coordination primitives that have
//! building blocks in `aspen-coordination`: counters, sequences, locks,
//! and rate limiters.
//!
//! Operations that require the WASM coordination plugin (queue, barrier,
//! semaphore, rwlock, service registry) are NOT claimed by this handler,
//! allowing them to fall through to the WASM plugin handler in the
//! dispatch chain.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use anyhow::anyhow;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::LockSetMemberTokenWire;
use aspen_client_api::LockSetResultResponse;
use aspen_client_api::RateLimiterResultResponse;
use aspen_client_api::coordination::CounterResultResponse;
use aspen_client_api::coordination::LockResultResponse;
use aspen_client_api::coordination::SequenceResultResponse;
use aspen_client_api::coordination::SignedCounterResultResponse;
use aspen_coordination::AtomicCounter;
use aspen_coordination::CounterConfig;
use aspen_coordination::DistributedLock;
use aspen_coordination::DistributedLockSet;
use aspen_coordination::DistributedRateLimiter;
use aspen_coordination::LockConfig;
use aspen_coordination::LockEntry;
use aspen_coordination::LockSetMemberToken;
use aspen_coordination::RateLimiterConfig;
use aspen_coordination::SequenceConfig;
use aspen_coordination::SequenceGenerator;
use aspen_coordination::SignedAtomicCounter;
use aspen_core::KeyValueStore;
use aspen_core::ReadRequest;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;
use async_trait::async_trait;

use crate::ClientProtocolContext;
use crate::RequestHandler;

/// Native handler for distributed coordination primitives.
///
/// Handles counter, sequence, lock, and rate limiter operations natively
/// using the `aspen-coordination` library. Other coordination operations
/// (queue, barrier, semaphore, rwlock, service registry) are NOT claimed
/// here — they fall through to the WASM plugin handler.
pub struct CoordinationHandler;

struct LockSetResponseArgs {
    is_success: bool,
    holder_id: Option<String>,
    member_tokens: Vec<LockSetMemberTokenWire>,
    deadline_ms: Option<u64>,
    blocked_member: Option<String>,
    blocked_holder: Option<String>,
    error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CoordinationRequestFamily {
    Counter,
    SignedCounter,
    Sequence,
    Lock,
    RateLimiter,
}

impl CoordinationHandler {
    /// KV key prefix for coordination counters.
    const COUNTER_PREFIX: &'static str = "__coord:counter:";
    /// KV key prefix for coordination sequences.
    const SEQUENCE_PREFIX: &'static str = "__coord:seq:";
    /// KV key prefix for coordination locks.
    const LOCK_PREFIX: &'static str = "__coord:lock:";
    /// KV key prefix for rate limiters.
    const RATE_LIMITER_PREFIX: &'static str = "__coord:ratelimit:";

    fn request_family(request: &ClientRpcRequest) -> Result<CoordinationRequestFamily> {
        match request {
            ClientRpcRequest::CounterGet { .. }
            | ClientRpcRequest::CounterIncrement { .. }
            | ClientRpcRequest::CounterDecrement { .. }
            | ClientRpcRequest::CounterAdd { .. }
            | ClientRpcRequest::CounterSubtract { .. }
            | ClientRpcRequest::CounterSet { .. }
            | ClientRpcRequest::CounterCompareAndSet { .. } => Ok(CoordinationRequestFamily::Counter),
            ClientRpcRequest::SignedCounterGet { .. } | ClientRpcRequest::SignedCounterAdd { .. } => {
                Ok(CoordinationRequestFamily::SignedCounter)
            }
            ClientRpcRequest::SequenceNext { .. }
            | ClientRpcRequest::SequenceReserve { .. }
            | ClientRpcRequest::SequenceCurrent { .. } => Ok(CoordinationRequestFamily::Sequence),
            ClientRpcRequest::LockAcquire { .. }
            | ClientRpcRequest::LockTryAcquire { .. }
            | ClientRpcRequest::LockRelease { .. }
            | ClientRpcRequest::LockRenew { .. }
            | ClientRpcRequest::LockSetAcquire { .. }
            | ClientRpcRequest::LockSetTryAcquire { .. }
            | ClientRpcRequest::LockSetRelease { .. }
            | ClientRpcRequest::LockSetRenew { .. } => Ok(CoordinationRequestFamily::Lock),
            ClientRpcRequest::RateLimiterTryAcquire { .. }
            | ClientRpcRequest::RateLimiterAcquire { .. }
            | ClientRpcRequest::RateLimiterAvailable { .. }
            | ClientRpcRequest::RateLimiterReset { .. } => Ok(CoordinationRequestFamily::RateLimiter),
            _ => Err(anyhow!("unexpected request in CoordinationHandler")),
        }
    }

    fn counter_response(is_success: bool, value: Option<u64>, error: Option<String>) -> ClientRpcResponse {
        ClientRpcResponse::CounterResult(CounterResultResponse {
            is_success,
            value,
            error,
        })
    }

    fn signed_counter_response(is_success: bool, value: Option<i64>, error: Option<String>) -> ClientRpcResponse {
        ClientRpcResponse::SignedCounterResult(SignedCounterResultResponse {
            is_success,
            value,
            error,
        })
    }

    fn sequence_response(is_success: bool, value: Option<u64>, error: Option<String>) -> ClientRpcResponse {
        ClientRpcResponse::SequenceResult(SequenceResultResponse {
            is_success,
            value,
            error,
        })
    }

    fn lock_response(
        is_success: bool,
        fencing_token: Option<u64>,
        holder_id: Option<String>,
        deadline_ms: Option<u64>,
        error: Option<String>,
    ) -> ClientRpcResponse {
        ClientRpcResponse::LockResult(LockResultResponse {
            is_success,
            fencing_token,
            holder_id,
            deadline_ms,
            error,
        })
    }

    fn lockset_response(args: LockSetResponseArgs) -> ClientRpcResponse {
        ClientRpcResponse::LockSetResult(LockSetResultResponse {
            is_success: args.is_success,
            holder_id: args.holder_id,
            member_tokens: args.member_tokens,
            deadline_ms: args.deadline_ms,
            blocked_member: args.blocked_member,
            blocked_holder: args.blocked_holder,
            error: args.error,
        })
    }

    fn rate_limiter_response(
        is_success: bool,
        tokens_remaining: Option<u64>,
        retry_after_ms: Option<u64>,
        error: Option<String>,
    ) -> ClientRpcResponse {
        ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
            is_success,
            tokens_remaining,
            retry_after_ms,
            error,
        })
    }

    fn default_counter_config() -> CounterConfig {
        CounterConfig {
            max_retries: 100,
            retry_delay_ms: 1,
        }
    }

    fn default_sequence_config() -> SequenceConfig {
        SequenceConfig {
            batch_size_ids: 100,
            start_value: 1,
        }
    }

    fn default_lock_config(ttl_ms: u64) -> LockConfig {
        LockConfig {
            ttl_ms,
            acquire_timeout_ms: 10_000,
            initial_backoff_ms: 10,
            max_backoff_ms: 1_000,
        }
    }

    fn lock_config_with_timeout(ttl_ms: u64, acquire_timeout_ms: u64) -> LockConfig {
        LockConfig {
            ttl_ms,
            acquire_timeout_ms,
            initial_backoff_ms: 10,
            max_backoff_ms: 1_000,
        }
    }
}

#[async_trait]
impl RequestHandler for CoordinationHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            // Lock (all four operations implemented natively)
            ClientRpcRequest::LockAcquire { .. }
                | ClientRpcRequest::LockTryAcquire { .. }
                | ClientRpcRequest::LockRelease { .. }
                | ClientRpcRequest::LockRenew { .. }
                | ClientRpcRequest::LockSetAcquire { .. }
                | ClientRpcRequest::LockSetTryAcquire { .. }
                | ClientRpcRequest::LockSetRelease { .. }
                | ClientRpcRequest::LockSetRenew { .. }
                // Counter
                | ClientRpcRequest::CounterGet { .. }
                | ClientRpcRequest::CounterIncrement { .. }
                | ClientRpcRequest::CounterDecrement { .. }
                | ClientRpcRequest::CounterAdd { .. }
                | ClientRpcRequest::CounterSubtract { .. }
                | ClientRpcRequest::CounterSet { .. }
                | ClientRpcRequest::CounterCompareAndSet { .. }
                // Signed counter
                | ClientRpcRequest::SignedCounterGet { .. }
                | ClientRpcRequest::SignedCounterAdd { .. }
                // Sequence
                | ClientRpcRequest::SequenceNext { .. }
                | ClientRpcRequest::SequenceReserve { .. }
                | ClientRpcRequest::SequenceCurrent { .. }
                // Rate limiter (all four operations implemented natively)
                | ClientRpcRequest::RateLimiterTryAcquire { .. }
                | ClientRpcRequest::RateLimiterAcquire { .. }
                | ClientRpcRequest::RateLimiterAvailable { .. }
                | ClientRpcRequest::RateLimiterReset { .. }
        )
    }

    async fn handle(&self, request: ClientRpcRequest, ctx: &ClientProtocolContext) -> Result<ClientRpcResponse> {
        let response = match Self::request_family(&request)? {
            CoordinationRequestFamily::Counter => self.handle_counter_request(request, ctx).await,
            CoordinationRequestFamily::SignedCounter => self.handle_signed_counter_request(request, ctx).await,
            CoordinationRequestFamily::Sequence => self.handle_sequence_request(request, ctx).await,
            CoordinationRequestFamily::Lock => self.handle_lock_request(request, ctx).await,
            CoordinationRequestFamily::RateLimiter => self.handle_rate_limiter_request(request, ctx).await,
        };
        Ok(response)
    }

    fn name(&self) -> &'static str {
        "CoordinationHandler"
    }
}

// =============================================================================
// Helper methods for request dispatch and primitive creation
// =============================================================================

impl CoordinationHandler {
    async fn handle_counter_request(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> ClientRpcResponse {
        if let ClientRpcRequest::CounterGet { key } = request {
            let counter = self.make_counter(&ctx.kv_store, &key);
            return match counter.get().await {
                Ok(value) => Self::counter_response(true, Some(value), None),
                Err(error) => Self::counter_response(false, None, Some(error.to_string())),
            };
        }
        if let ClientRpcRequest::CounterIncrement { key } = request {
            let counter = self.make_counter(&ctx.kv_store, &key);
            return match counter.increment().await {
                Ok(value) => Self::counter_response(true, Some(value), None),
                Err(error) => Self::counter_response(false, None, Some(error.to_string())),
            };
        }
        if let ClientRpcRequest::CounterDecrement { key } = request {
            let counter = self.make_counter(&ctx.kv_store, &key);
            return match counter.decrement().await {
                Ok(value) => Self::counter_response(true, Some(value), None),
                Err(error) => Self::counter_response(false, None, Some(error.to_string())),
            };
        }
        if let ClientRpcRequest::CounterAdd { key, amount } = request {
            let counter = self.make_counter(&ctx.kv_store, &key);
            return match counter.add(amount).await {
                Ok(value) => Self::counter_response(true, Some(value), None),
                Err(error) => Self::counter_response(false, None, Some(error.to_string())),
            };
        }
        if let ClientRpcRequest::CounterSubtract { key, amount } = request {
            let counter = self.make_counter(&ctx.kv_store, &key);
            return match counter.subtract(amount).await {
                Ok(value) => Self::counter_response(true, Some(value), None),
                Err(error) => Self::counter_response(false, None, Some(error.to_string())),
            };
        }
        if let ClientRpcRequest::CounterSet { key, value } = request {
            let counter = self.make_counter(&ctx.kv_store, &key);
            return match counter.set(value).await {
                Ok(()) => Self::counter_response(true, Some(value), None),
                Err(error) => Self::counter_response(false, None, Some(error.to_string())),
            };
        }
        if let ClientRpcRequest::CounterCompareAndSet {
            key,
            expected,
            new_value,
        } = request
        {
            let counter = self.make_counter(&ctx.kv_store, &key);
            return match counter.compare_and_set(expected, new_value).await {
                Ok(swapped) => Self::counter_response(
                    swapped,
                    Some(if swapped { new_value } else { expected }),
                    (!swapped).then(|| "compare-and-set failed: value has changed".to_string()),
                ),
                Err(error) => Self::counter_response(false, None, Some(error.to_string())),
            };
        }

        Self::counter_response(false, None, Some("unexpected counter request".to_string()))
    }

    async fn handle_signed_counter_request(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> ClientRpcResponse {
        if let ClientRpcRequest::SignedCounterGet { key } = request {
            let counter = self.make_signed_counter(&ctx.kv_store, &key);
            return match counter.get().await {
                Ok(value) => Self::signed_counter_response(true, Some(value), None),
                Err(error) => Self::signed_counter_response(false, None, Some(error.to_string())),
            };
        }
        if let ClientRpcRequest::SignedCounterAdd { key, amount } = request {
            let counter = self.make_signed_counter(&ctx.kv_store, &key);
            return match counter.add(amount).await {
                Ok(value) => Self::signed_counter_response(true, Some(value), None),
                Err(error) => Self::signed_counter_response(false, None, Some(error.to_string())),
            };
        }

        Self::signed_counter_response(false, None, Some("unexpected signed counter request".to_string()))
    }

    async fn handle_sequence_request(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> ClientRpcResponse {
        if let ClientRpcRequest::SequenceNext { key } = request {
            let sequence = self.make_sequence(&ctx.kv_store, &key);
            return match sequence.next().await {
                Ok(value) => Self::sequence_response(true, Some(value), None),
                Err(error) => Self::sequence_response(false, None, Some(error.to_string())),
            };
        }
        if let ClientRpcRequest::SequenceReserve { key, count } = request {
            let sequence = self.make_sequence(&ctx.kv_store, &key);
            return match sequence.reserve(count).await {
                Ok(start) => Self::sequence_response(true, Some(start), None),
                Err(error) => Self::sequence_response(false, None, Some(error.to_string())),
            };
        }
        if let ClientRpcRequest::SequenceCurrent { key } = request {
            let sequence = self.make_sequence(&ctx.kv_store, &key);
            return match sequence.current().await {
                Ok(value) => Self::sequence_response(true, Some(value), None),
                Err(error) => Self::sequence_response(false, None, Some(error.to_string())),
            };
        }

        Self::sequence_response(false, None, Some("unexpected sequence request".to_string()))
    }

    async fn handle_lock_request(&self, request: ClientRpcRequest, ctx: &ClientProtocolContext) -> ClientRpcResponse {
        if let ClientRpcRequest::LockAcquire {
            key,
            holder_id,
            ttl_ms,
            timeout_ms,
        } = request
        {
            let config = Self::lock_config_with_timeout(ttl_ms, timeout_ms);
            let lock = DistributedLock::new(
                Arc::clone(&ctx.kv_store),
                format!("{}{key}", Self::LOCK_PREFIX),
                &holder_id,
                config,
            );
            return match lock.acquire().await {
                Ok(guard) => Self::forget_lock_guard(guard, holder_id),
                Err(error) => Self::lock_response(false, None, None, None, Some(error.to_string())),
            };
        }
        if let ClientRpcRequest::LockTryAcquire { key, holder_id, ttl_ms } = request {
            let lock = self.make_lock(&ctx.kv_store, &key, &holder_id, ttl_ms);
            return match lock.try_acquire().await {
                Ok(guard) => Self::forget_lock_guard(guard, holder_id),
                Err(error) => Self::lock_response(false, None, None, None, Some(error.to_string())),
            };
        }
        if let ClientRpcRequest::LockRelease {
            key,
            holder_id,
            fencing_token,
        } = request
        {
            return self.release_lock(ctx, key, holder_id, fencing_token).await;
        }
        if let ClientRpcRequest::LockRenew {
            key,
            holder_id,
            fencing_token,
            ttl_ms,
        } = request
        {
            return self.renew_lock(ctx, key, holder_id, fencing_token, ttl_ms).await;
        }
        if let ClientRpcRequest::LockSetAcquire {
            members,
            holder_id,
            ttl_ms,
            timeout_ms,
        } = request
        {
            return self.acquire_lockset(ctx, members, holder_id, ttl_ms, timeout_ms).await;
        }
        if let ClientRpcRequest::LockSetTryAcquire {
            members,
            holder_id,
            ttl_ms,
        } = request
        {
            return self.try_acquire_lockset(ctx, members, holder_id, ttl_ms).await;
        }
        if let ClientRpcRequest::LockSetRelease {
            holder_id,
            member_tokens,
        } = request
        {
            return self.release_lockset(ctx, holder_id, member_tokens).await;
        }
        if let ClientRpcRequest::LockSetRenew {
            holder_id,
            member_tokens,
            ttl_ms,
        } = request
        {
            return self.renew_lockset(ctx, holder_id, member_tokens, ttl_ms).await;
        }

        Self::lock_response(false, None, None, None, Some("unexpected lock request".to_string()))
    }

    async fn handle_rate_limiter_request(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> ClientRpcResponse {
        if let ClientRpcRequest::RateLimiterTryAcquire {
            key,
            tokens,
            capacity_tokens,
            refill_rate,
        } = request
        {
            let bucket = self.make_rate_limiter(&ctx.kv_store, &key, refill_rate, capacity_tokens);
            return match bucket.try_acquire_n(tokens).await {
                Ok(remaining) => Self::rate_limiter_response(true, Some(remaining), None, None),
                Err(aspen_coordination::RateLimitError::TokensExhausted { retry_after_ms, .. }) => {
                    Self::rate_limiter_response(false, None, Some(retry_after_ms), Some("rate limited".into()))
                }
                Err(error) => Self::rate_limiter_response(false, None, None, Some(error.to_string())),
            };
        }
        if let ClientRpcRequest::RateLimiterAcquire {
            key,
            tokens,
            capacity_tokens,
            refill_rate,
            timeout_ms,
        } = request
        {
            let bucket = self.make_rate_limiter(&ctx.kv_store, &key, refill_rate, capacity_tokens);
            return match bucket.acquire_n(tokens, Duration::from_millis(timeout_ms)).await {
                Ok(remaining) => Self::rate_limiter_response(true, Some(remaining), None, None),
                Err(error) => Self::rate_limiter_response(false, None, None, Some(error.to_string())),
            };
        }
        if let ClientRpcRequest::RateLimiterAvailable {
            key,
            capacity_tokens,
            refill_rate,
        } = request
        {
            let bucket = self.make_rate_limiter(&ctx.kv_store, &key, refill_rate, capacity_tokens);
            return match bucket.available().await {
                Ok(available) => Self::rate_limiter_response(true, Some(available), None, None),
                Err(error) => Self::rate_limiter_response(false, None, None, Some(error.to_string())),
            };
        }
        if let ClientRpcRequest::RateLimiterReset {
            key,
            capacity_tokens,
            refill_rate,
        } = request
        {
            let bucket = self.make_rate_limiter(&ctx.kv_store, &key, refill_rate, capacity_tokens);
            return match bucket.reset().await {
                Ok(()) => Self::rate_limiter_response(true, Some(capacity_tokens), None, None),
                Err(error) => Self::rate_limiter_response(false, None, None, Some(error.to_string())),
            };
        }

        Self::rate_limiter_response(false, None, None, Some("unexpected rate limiter request".to_string()))
    }

    fn forget_lock_guard(
        guard: aspen_coordination::LockGuard<dyn KeyValueStore>,
        holder_id: String,
    ) -> ClientRpcResponse {
        let token = guard.fencing_token();
        let deadline = guard.deadline_ms();
        std::mem::forget(guard);
        Self::lock_response(true, Some(token.value()), Some(holder_id), Some(deadline), None)
    }

    fn forget_lockset_guard(
        guard: aspen_coordination::LockSetGuard<dyn KeyValueStore>,
        holder_id: String,
    ) -> ClientRpcResponse {
        let deadline = guard.deadline_ms();
        let member_tokens = guard
            .member_tokens()
            .iter()
            .map(|token| LockSetMemberTokenWire {
                member: Self::strip_lock_prefix(&token.member),
                fencing_token: token.fencing_token.value(),
            })
            .collect();
        std::mem::forget(guard);
        Self::lockset_response(LockSetResponseArgs {
            is_success: true,
            holder_id: Some(holder_id),
            member_tokens,
            deadline_ms: Some(deadline),
            blocked_member: None,
            blocked_holder: None,
            error: None,
        })
    }

    async fn acquire_lockset(
        &self,
        ctx: &ClientProtocolContext,
        members: Vec<String>,
        holder_id: String,
        ttl_ms: u64,
        timeout_ms: u64,
    ) -> ClientRpcResponse {
        let config = Self::lock_config_with_timeout(ttl_ms, timeout_ms);
        let prefixed_members = Self::prefix_lockset_members(members);
        match DistributedLockSet::new(Arc::clone(&ctx.kv_store), prefixed_members, holder_id.clone(), config) {
            Ok(lockset) => match lockset.acquire().await {
                Ok(guard) => Self::forget_lockset_guard(guard, holder_id),
                Err(error) => Self::lockset_error_response(error),
            },
            Err(error) => Self::lockset_error_response(error),
        }
    }

    async fn try_acquire_lockset(
        &self,
        ctx: &ClientProtocolContext,
        members: Vec<String>,
        holder_id: String,
        ttl_ms: u64,
    ) -> ClientRpcResponse {
        let prefixed_members = Self::prefix_lockset_members(members);
        match DistributedLockSet::new(
            Arc::clone(&ctx.kv_store),
            prefixed_members,
            holder_id.clone(),
            Self::default_lock_config(ttl_ms),
        ) {
            Ok(lockset) => match lockset.try_acquire().await {
                Ok(guard) => Self::forget_lockset_guard(guard, holder_id),
                Err(error) => Self::lockset_error_response(error),
            },
            Err(error) => Self::lockset_error_response(error),
        }
    }

    async fn release_lockset(
        &self,
        ctx: &ClientProtocolContext,
        holder_id: String,
        member_tokens: Vec<LockSetMemberTokenWire>,
    ) -> ClientRpcResponse {
        let coordination_tokens = Self::prefix_member_tokens(member_tokens);
        let members: Vec<String> = coordination_tokens.iter().map(|token| token.member.clone()).collect();
        match DistributedLockSet::new(
            Arc::clone(&ctx.kv_store),
            members,
            holder_id.clone(),
            Self::default_lock_config(30_000),
        ) {
            Ok(lockset) => match lockset.release_member_tokens(&coordination_tokens).await {
                Ok(()) => Self::lockset_response(LockSetResponseArgs {
                    is_success: true,
                    holder_id: Some(holder_id),
                    member_tokens: Vec::new(),
                    deadline_ms: Some(0),
                    blocked_member: None,
                    blocked_holder: None,
                    error: None,
                }),
                Err(error) => Self::lockset_error_response(error),
            },
            Err(error) => Self::lockset_error_response(error),
        }
    }

    async fn renew_lockset(
        &self,
        ctx: &ClientProtocolContext,
        holder_id: String,
        member_tokens: Vec<LockSetMemberTokenWire>,
        ttl_ms: u64,
    ) -> ClientRpcResponse {
        let coordination_tokens = Self::prefix_member_tokens(member_tokens);
        let members: Vec<String> = coordination_tokens.iter().map(|token| token.member.clone()).collect();
        match DistributedLockSet::new(
            Arc::clone(&ctx.kv_store),
            members,
            holder_id.clone(),
            Self::default_lock_config(ttl_ms),
        ) {
            Ok(lockset) => match lockset.renew_member_tokens(&coordination_tokens).await {
                Ok(deadline_ms) => Self::lockset_response(LockSetResponseArgs {
                    is_success: true,
                    holder_id: Some(holder_id),
                    member_tokens: coordination_tokens
                        .iter()
                        .map(|token| LockSetMemberTokenWire {
                            member: Self::strip_lock_prefix(&token.member),
                            fencing_token: token.fencing_token.value(),
                        })
                        .collect(),
                    deadline_ms: Some(deadline_ms),
                    blocked_member: None,
                    blocked_holder: None,
                    error: None,
                }),
                Err(error) => Self::lockset_error_response(error),
            },
            Err(error) => Self::lockset_error_response(error),
        }
    }

    fn lockset_error_response(error: aspen_coordination::CoordinationError) -> ClientRpcResponse {
        match error {
            aspen_coordination::CoordinationError::LockSetHeld {
                member,
                holder,
                deadline_ms,
            } => Self::lockset_response(LockSetResponseArgs {
                is_success: false,
                holder_id: None,
                member_tokens: Vec::new(),
                deadline_ms: Some(deadline_ms),
                blocked_member: Some(Self::strip_lock_prefix(&member)),
                blocked_holder: Some(holder),
                error: Some("lock set blocked".to_string()),
            }),
            other => Self::lockset_response(LockSetResponseArgs {
                is_success: false,
                holder_id: None,
                member_tokens: Vec::new(),
                deadline_ms: None,
                blocked_member: None,
                blocked_holder: None,
                error: Some(other.to_string()),
            }),
        }
    }

    fn prefix_lockset_members(members: Vec<String>) -> Vec<String> {
        members.into_iter().map(|member| format!("{}{}", Self::LOCK_PREFIX, member)).collect()
    }

    fn prefix_member_tokens(member_tokens: Vec<LockSetMemberTokenWire>) -> Vec<LockSetMemberToken> {
        member_tokens
            .into_iter()
            .map(|token| {
                LockSetMemberToken::new(
                    format!("{}{}", Self::LOCK_PREFIX, token.member),
                    aspen_coordination::FencingToken::new(token.fencing_token),
                )
            })
            .collect()
    }

    fn strip_lock_prefix(member: &str) -> String {
        member.strip_prefix(Self::LOCK_PREFIX).unwrap_or(member).to_string()
    }

    async fn release_lock(
        &self,
        ctx: &ClientProtocolContext,
        key: String,
        holder_id: String,
        fencing_token: u64,
    ) -> ClientRpcResponse {
        let lock_key = format!("{}{key}", Self::LOCK_PREFIX);
        match ctx.kv_store.read(ReadRequest::new(lock_key.clone())).await {
            Ok(result) if result.kv.is_some() => {
                let current_json = result.kv.as_ref().map(|kv| kv.value.clone()).unwrap_or_default();
                match serde_json::from_str::<LockEntry>(&current_json) {
                    Ok(entry) if entry.holder_id == holder_id && entry.fencing_token == fencing_token => {
                        let released = entry.released();
                        let released_json = serde_json::to_string(&released).unwrap_or_default();
                        match ctx
                            .kv_store
                            .write(WriteRequest {
                                command: WriteCommand::CompareAndSwap {
                                    key: lock_key,
                                    expected: Some(current_json),
                                    new_value: released_json,
                                },
                            })
                            .await
                        {
                            Ok(_) => Self::lock_response(true, Some(fencing_token), Some(holder_id), Some(0), None),
                            Err(error) => {
                                Self::lock_response(false, None, Some(holder_id), None, Some(error.to_string()))
                            }
                        }
                    }
                    Ok(entry) => {
                        let message = format!(
                            "lock held by '{}' with token {}, not '{}' with token {}",
                            entry.holder_id, entry.fencing_token, holder_id, fencing_token
                        );
                        Self::lock_response(false, None, Some(holder_id), None, Some(message))
                    }
                    Err(error) => Self::lock_response(
                        false,
                        None,
                        Some(holder_id),
                        None,
                        Some(format!("failed to parse lock entry: {error}")),
                    ),
                }
            }
            Ok(_) => Self::lock_response(true, None, Some(holder_id), None, None),
            Err(error) => Self::lock_response(false, None, Some(holder_id), None, Some(error.to_string())),
        }
    }

    async fn renew_lock(
        &self,
        ctx: &ClientProtocolContext,
        key: String,
        holder_id: String,
        fencing_token: u64,
        ttl_ms: u64,
    ) -> ClientRpcResponse {
        let lock_key = format!("{}{key}", Self::LOCK_PREFIX);
        match ctx.kv_store.read(ReadRequest::new(lock_key.clone())).await {
            Ok(result) if result.kv.is_some() => {
                let current_json = result.kv.as_ref().map(|kv| kv.value.clone()).unwrap_or_default();
                match serde_json::from_str::<LockEntry>(&current_json) {
                    Ok(entry)
                        if entry.holder_id == holder_id
                            && entry.fencing_token == fencing_token
                            && !entry.is_expired() =>
                    {
                        let renewed = LockEntry::new(holder_id.clone(), fencing_token, ttl_ms);
                        let renewed_json = serde_json::to_string(&renewed).unwrap_or_default();
                        match ctx
                            .kv_store
                            .write(WriteRequest {
                                command: WriteCommand::CompareAndSwap {
                                    key: lock_key,
                                    expected: Some(current_json),
                                    new_value: renewed_json,
                                },
                            })
                            .await
                        {
                            Ok(_) => Self::lock_response(
                                true,
                                Some(fencing_token),
                                Some(holder_id),
                                Some(renewed.deadline_ms),
                                None,
                            ),
                            Err(error) => Self::lock_response(
                                false,
                                Some(fencing_token),
                                Some(holder_id),
                                None,
                                Some(error.to_string()),
                            ),
                        }
                    }
                    Ok(entry) => {
                        let reason = if entry.is_expired() {
                            "lock expired".to_string()
                        } else {
                            format!(
                                "lock held by '{}' with token {}, not '{holder_id}' with token {fencing_token}",
                                entry.holder_id, entry.fencing_token
                            )
                        };
                        Self::lock_response(false, None, Some(holder_id), None, Some(reason))
                    }
                    Err(error) => Self::lock_response(
                        false,
                        None,
                        Some(holder_id),
                        None,
                        Some(format!("failed to parse lock entry: {error}")),
                    ),
                }
            }
            Ok(_) => Self::lock_response(false, None, Some(holder_id), None, Some("lock not found".to_string())),
            Err(error) => Self::lock_response(false, None, Some(holder_id), None, Some(error.to_string())),
        }
    }

    fn make_counter(&self, store: &Arc<dyn KeyValueStore>, key: &str) -> AtomicCounter<dyn KeyValueStore> {
        let prefixed = format!("{}{key}", Self::COUNTER_PREFIX);
        AtomicCounter::new(Arc::clone(store), prefixed, Self::default_counter_config())
    }

    fn make_signed_counter(&self, store: &Arc<dyn KeyValueStore>, key: &str) -> SignedAtomicCounter<dyn KeyValueStore> {
        let prefixed = format!("{}{key}", Self::COUNTER_PREFIX);
        SignedAtomicCounter::new(Arc::clone(store), prefixed, Self::default_counter_config())
    }

    fn make_sequence(&self, store: &Arc<dyn KeyValueStore>, key: &str) -> SequenceGenerator<dyn KeyValueStore> {
        let prefixed = format!("{}{key}", Self::SEQUENCE_PREFIX);
        SequenceGenerator::new(Arc::clone(store), prefixed, Self::default_sequence_config())
    }

    fn make_lock(
        &self,
        store: &Arc<dyn KeyValueStore>,
        key: &str,
        holder_id: &str,
        ttl_ms: u64,
    ) -> DistributedLock<dyn KeyValueStore> {
        let prefixed = format!("{}{key}", Self::LOCK_PREFIX);
        let config = Self::default_lock_config(ttl_ms);
        DistributedLock::new(Arc::clone(store), prefixed, holder_id, config)
    }

    fn make_rate_limiter(
        &self,
        store: &Arc<dyn KeyValueStore>,
        key: &str,
        refill_rate: f64,
        capacity_tokens: u64,
    ) -> DistributedRateLimiter<dyn KeyValueStore> {
        let prefixed = format!("{}{key}", Self::RATE_LIMITER_PREFIX);
        let config = RateLimiterConfig::new(refill_rate, capacity_tokens);
        DistributedRateLimiter::new(Arc::clone(store), prefixed, config)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aspen_rpc_core::test_support::MockEndpointProvider;
    use aspen_rpc_core::test_support::TestContextBuilder;
    use aspen_testing::DeterministicClusterController;
    use aspen_testing::DeterministicKeyValueStore;

    use super::*;

    fn handler() -> CoordinationHandler {
        CoordinationHandler
    }

    async fn setup_test_context() -> ClientProtocolContext {
        let controller = Arc::new(DeterministicClusterController::new());
        let kv_store = Arc::new(DeterministicKeyValueStore::new());
        let mock_endpoint = Arc::new(MockEndpointProvider::with_seed(777).await);

        TestContextBuilder::new()
            .with_node_id(7)
            .with_controller(controller)
            .with_kv_store(kv_store)
            .with_endpoint_manager(mock_endpoint)
            .with_cookie("coord-test")
            .build()
    }

    // =========================================================================
    // can_handle: operations we implement natively → true
    // =========================================================================

    #[test]
    fn test_can_handle_counter_variants() {
        let h = handler();
        assert!(h.can_handle(&ClientRpcRequest::CounterGet { key: "k".into() }));
        assert!(h.can_handle(&ClientRpcRequest::CounterIncrement { key: "k".into() }));
        assert!(h.can_handle(&ClientRpcRequest::CounterDecrement { key: "k".into() }));
        assert!(h.can_handle(&ClientRpcRequest::CounterAdd {
            key: "k".into(),
            amount: 1,
        }));
        assert!(h.can_handle(&ClientRpcRequest::CounterSubtract {
            key: "k".into(),
            amount: 1,
        }));
        assert!(h.can_handle(&ClientRpcRequest::CounterSet {
            key: "k".into(),
            value: 0,
        }));
        assert!(h.can_handle(&ClientRpcRequest::CounterCompareAndSet {
            key: "k".into(),
            expected: 0,
            new_value: 1,
        }));
    }

    #[test]
    fn test_can_handle_signed_counter_variants() {
        let h = handler();
        assert!(h.can_handle(&ClientRpcRequest::SignedCounterGet { key: "k".into() }));
        assert!(h.can_handle(&ClientRpcRequest::SignedCounterAdd {
            key: "k".into(),
            amount: -1,
        }));
    }

    #[test]
    fn test_can_handle_sequence_variants() {
        let h = handler();
        assert!(h.can_handle(&ClientRpcRequest::SequenceNext { key: "s".into() }));
        assert!(h.can_handle(&ClientRpcRequest::SequenceReserve {
            key: "s".into(),
            count: 10,
        }));
        assert!(h.can_handle(&ClientRpcRequest::SequenceCurrent { key: "s".into() }));
    }

    #[test]
    fn test_can_handle_lock_variants() {
        let h = handler();
        assert!(h.can_handle(&ClientRpcRequest::LockAcquire {
            key: "l".into(),
            holder_id: "h".into(),
            ttl_ms: 1000,
            timeout_ms: 5000,
        }));
        assert!(h.can_handle(&ClientRpcRequest::LockTryAcquire {
            key: "l".into(),
            holder_id: "h".into(),
            ttl_ms: 1000,
        }));
        assert!(h.can_handle(&ClientRpcRequest::LockRelease {
            key: "l".into(),
            holder_id: "h".into(),
            fencing_token: 1,
        }));
        assert!(h.can_handle(&ClientRpcRequest::LockRenew {
            key: "l".into(),
            holder_id: "h".into(),
            fencing_token: 1,
            ttl_ms: 1000,
        }));
        assert!(h.can_handle(&ClientRpcRequest::LockSetAcquire {
            members: vec!["repo:a".into(), "pipeline:42".into()],
            holder_id: "h".into(),
            ttl_ms: 1000,
            timeout_ms: 5000,
        }));
        assert!(h.can_handle(&ClientRpcRequest::LockSetTryAcquire {
            members: vec!["repo:a".into(), "pipeline:42".into()],
            holder_id: "h".into(),
            ttl_ms: 1000,
        }));
        assert!(h.can_handle(&ClientRpcRequest::LockSetRelease {
            holder_id: "h".into(),
            member_tokens: vec![LockSetMemberTokenWire {
                member: "repo:a".into(),
                fencing_token: 1,
            }],
        }));
        assert!(h.can_handle(&ClientRpcRequest::LockSetRenew {
            holder_id: "h".into(),
            member_tokens: vec![LockSetMemberTokenWire {
                member: "repo:a".into(),
                fencing_token: 1,
            }],
            ttl_ms: 1000,
        }));
    }

    #[test]
    fn test_can_handle_ratelimiter_variants() {
        let h = handler();
        assert!(h.can_handle(&ClientRpcRequest::RateLimiterTryAcquire {
            key: "r".into(),
            tokens: 1,
            capacity_tokens: 10,
            refill_rate: 1.0,
        }));
        assert!(h.can_handle(&ClientRpcRequest::RateLimiterAcquire {
            key: "r".into(),
            tokens: 1,
            capacity_tokens: 10,
            refill_rate: 1.0,
            timeout_ms: 5000,
        }));
        assert!(h.can_handle(&ClientRpcRequest::RateLimiterAvailable {
            key: "r".into(),
            capacity_tokens: 10,
            refill_rate: 1.0,
        }));
        assert!(h.can_handle(&ClientRpcRequest::RateLimiterReset {
            key: "r".into(),
            capacity_tokens: 10,
            refill_rate: 1.0,
        }));
    }

    // =========================================================================
    // can_handle: WASM-plugin-only operations → false (fall through to plugin)
    // =========================================================================

    #[test]
    fn test_cannot_handle_barrier_variants() {
        let h = handler();
        assert!(!h.can_handle(&ClientRpcRequest::BarrierEnter {
            name: "b".into(),
            participant_id: "p".into(),
            required_count: 3,
            timeout_ms: 5000,
        }));
    }

    #[test]
    fn test_cannot_handle_semaphore_variants() {
        let h = handler();
        assert!(!h.can_handle(&ClientRpcRequest::SemaphoreStatus { name: "s".into() }));
    }

    #[test]
    fn test_cannot_handle_rwlock_variants() {
        let h = handler();
        assert!(!h.can_handle(&ClientRpcRequest::RWLockStatus { name: "rw".into() }));
    }

    #[test]
    fn test_cannot_handle_queue_variants() {
        let h = handler();
        assert!(!h.can_handle(&ClientRpcRequest::QueueCreate {
            queue_name: "q".into(),
            default_visibility_timeout_ms: None,
            default_ttl_ms: None,
            max_delivery_attempts: None,
        }));
        assert!(!h.can_handle(&ClientRpcRequest::QueueStatus { queue_name: "q".into() }));
    }

    #[test]
    fn test_cannot_handle_service_registry_variants() {
        let h = handler();
        assert!(!h.can_handle(&ClientRpcRequest::ServiceDiscover {
            service_name: "svc".into(),
            healthy_only: true,
            tags: String::new(),
            version_prefix: None,
            limit: None,
        }));
    }

    #[test]
    fn test_cannot_handle_non_coordination() {
        let h = handler();
        assert!(!h.can_handle(&ClientRpcRequest::Ping));
        assert!(!h.can_handle(&ClientRpcRequest::GetHealth));
        assert!(!h.can_handle(&ClientRpcRequest::ReadKey { key: "k".into() }));
    }

    #[test]
    fn test_request_family_routes_native_variants() {
        assert_eq!(
            CoordinationHandler::request_family(&ClientRpcRequest::CounterGet { key: "k".into() })
                .expect("counter route should succeed"),
            CoordinationRequestFamily::Counter
        );
        assert_eq!(
            CoordinationHandler::request_family(&ClientRpcRequest::SignedCounterAdd {
                key: "k".into(),
                amount: -1,
            })
            .expect("signed counter route should succeed"),
            CoordinationRequestFamily::SignedCounter
        );
        assert_eq!(
            CoordinationHandler::request_family(&ClientRpcRequest::SequenceCurrent { key: "s".into() })
                .expect("sequence route should succeed"),
            CoordinationRequestFamily::Sequence
        );
        assert_eq!(
            CoordinationHandler::request_family(&ClientRpcRequest::LockTryAcquire {
                key: "l".into(),
                holder_id: "h".into(),
                ttl_ms: 1000,
            })
            .expect("lock route should succeed"),
            CoordinationRequestFamily::Lock
        );
        assert_eq!(
            CoordinationHandler::request_family(&ClientRpcRequest::RateLimiterAvailable {
                key: "r".into(),
                capacity_tokens: 10,
                refill_rate: 1.0,
            })
            .expect("rate limiter route should succeed"),
            CoordinationRequestFamily::RateLimiter
        );
    }

    #[tokio::test]
    async fn test_handle_rejects_non_coordination_requests_with_error() {
        let ctx = setup_test_context().await;
        let error = handler()
            .handle(ClientRpcRequest::Ping, &ctx)
            .await
            .expect_err("non-coordination request should return an error");
        assert!(error.to_string().contains("unexpected request in CoordinationHandler"));
    }

    #[tokio::test]
    async fn test_handle_counter_increment_then_get() {
        let ctx = setup_test_context().await;
        let result = handler()
            .handle(ClientRpcRequest::CounterIncrement { key: "ctr".into() }, &ctx)
            .await
            .expect("counter increment should succeed");
        match result {
            ClientRpcResponse::CounterResult(response) => {
                assert!(response.is_success);
                assert_eq!(response.value, Some(1));
            }
            other => panic!("expected CounterResult, got {other:?}"),
        }

        let get_result = handler()
            .handle(ClientRpcRequest::CounterGet { key: "ctr".into() }, &ctx)
            .await
            .expect("counter get should succeed");
        match get_result {
            ClientRpcResponse::CounterResult(response) => {
                assert!(response.is_success);
                assert_eq!(response.value, Some(1));
            }
            other => panic!("expected CounterResult, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_handle_sequence_reserve_then_current() {
        let ctx = setup_test_context().await;
        let reserve_result = handler()
            .handle(
                ClientRpcRequest::SequenceReserve {
                    key: "seq".into(),
                    count: 5,
                },
                &ctx,
            )
            .await
            .expect("sequence reserve should succeed");
        match reserve_result {
            ClientRpcResponse::SequenceResult(response) => {
                assert!(response.is_success);
                assert_eq!(response.value, Some(1));
            }
            other => panic!("expected SequenceResult, got {other:?}"),
        }

        let current_result = handler()
            .handle(ClientRpcRequest::SequenceCurrent { key: "seq".into() }, &ctx)
            .await
            .expect("sequence current should succeed");
        match current_result {
            ClientRpcResponse::SequenceResult(response) => {
                assert!(response.is_success);
                assert_eq!(response.value, Some(6));
            }
            other => panic!("expected SequenceResult, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_handle_lock_try_acquire_and_release() {
        let ctx = setup_test_context().await;
        let acquire_result = handler()
            .handle(
                ClientRpcRequest::LockTryAcquire {
                    key: "lock".into(),
                    holder_id: "holder-a".into(),
                    ttl_ms: 5_000,
                },
                &ctx,
            )
            .await
            .expect("lock acquisition should succeed");
        let fencing_token = match acquire_result {
            ClientRpcResponse::LockResult(response) => {
                assert!(response.is_success);
                assert_eq!(response.holder_id.as_deref(), Some("holder-a"));
                response.fencing_token.expect("fencing token")
            }
            other => panic!("expected LockResult, got {other:?}"),
        };

        let release_result = handler()
            .handle(
                ClientRpcRequest::LockRelease {
                    key: "lock".into(),
                    holder_id: "holder-a".into(),
                    fencing_token,
                },
                &ctx,
            )
            .await
            .expect("lock release should succeed");
        match release_result {
            ClientRpcResponse::LockResult(response) => {
                assert!(response.is_success);
                assert_eq!(response.deadline_ms, Some(0));
            }
            other => panic!("expected LockResult, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_handle_lockset_try_acquire_renew_and_release() {
        let ctx = setup_test_context().await;
        let acquire_result = handler()
            .handle(
                ClientRpcRequest::LockSetTryAcquire {
                    members: vec!["pipeline:42".into(), "repo:a".into()],
                    holder_id: "holder-a".into(),
                    ttl_ms: 5_000,
                },
                &ctx,
            )
            .await
            .expect("lockset acquisition should succeed");
        let member_tokens = match acquire_result {
            ClientRpcResponse::LockSetResult(response) => {
                assert!(response.is_success);
                assert_eq!(response.member_tokens.len(), 2);
                assert_eq!(response.member_tokens[0].member, "pipeline:42");
                response.member_tokens
            }
            other => panic!("expected LockSetResult, got {other:?}"),
        };

        let renew_result = handler()
            .handle(
                ClientRpcRequest::LockSetRenew {
                    holder_id: "holder-a".into(),
                    member_tokens: member_tokens.clone(),
                    ttl_ms: 10_000,
                },
                &ctx,
            )
            .await
            .expect("lockset renew should succeed");
        match renew_result {
            ClientRpcResponse::LockSetResult(response) => {
                assert!(response.is_success);
                assert!(response.deadline_ms.unwrap_or(0) > 0);
            }
            other => panic!("expected LockSetResult, got {other:?}"),
        }

        let release_result = handler()
            .handle(
                ClientRpcRequest::LockSetRelease {
                    holder_id: "holder-a".into(),
                    member_tokens,
                },
                &ctx,
            )
            .await
            .expect("lockset release should succeed");
        match release_result {
            ClientRpcResponse::LockSetResult(response) => {
                assert!(response.is_success);
                assert_eq!(response.deadline_ms, Some(0));
            }
            other => panic!("expected LockSetResult, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_handle_lockset_overlap_conflict_has_no_partial_claim() {
        let ctx = setup_test_context().await;
        let first = handler()
            .handle(
                ClientRpcRequest::LockSetTryAcquire {
                    members: vec!["repo:a".into(), "pipeline:42".into()],
                    holder_id: "holder-a".into(),
                    ttl_ms: 5_000,
                },
                &ctx,
            )
            .await
            .expect("first acquire should succeed");
        match first {
            ClientRpcResponse::LockSetResult(response) => assert!(response.is_success),
            other => panic!("expected LockSetResult, got {other:?}"),
        }

        let second = handler()
            .handle(
                ClientRpcRequest::LockSetTryAcquire {
                    members: vec!["repo:a".into(), "deploy:prod".into()],
                    holder_id: "holder-b".into(),
                    ttl_ms: 5_000,
                },
                &ctx,
            )
            .await
            .expect("second acquire should return cleanly");
        match second {
            ClientRpcResponse::LockSetResult(response) => {
                assert!(!response.is_success);
                assert_eq!(response.blocked_member.as_deref(), Some("repo:a"));
            }
            other => panic!("expected LockSetResult, got {other:?}"),
        }
        assert!(ctx.kv_store.read(ReadRequest::new("__coord:lock:deploy:prod")).await.is_err());
    }

    #[tokio::test]
    async fn test_handle_lockset_expiry_takeover() {
        let ctx = setup_test_context().await;
        let first = handler()
            .handle(
                ClientRpcRequest::LockSetTryAcquire {
                    members: vec!["repo:a".into(), "pipeline:42".into()],
                    holder_id: "holder-a".into(),
                    ttl_ms: 50,
                },
                &ctx,
            )
            .await
            .expect("first acquire should succeed");
        let first_tokens = match first {
            ClientRpcResponse::LockSetResult(response) => response.member_tokens,
            other => panic!("expected LockSetResult, got {other:?}"),
        };
        tokio::time::sleep(Duration::from_millis(100)).await;

        let second = handler()
            .handle(
                ClientRpcRequest::LockSetTryAcquire {
                    members: vec!["repo:a".into(), "pipeline:42".into()],
                    holder_id: "holder-b".into(),
                    ttl_ms: 50,
                },
                &ctx,
            )
            .await
            .expect("second acquire should succeed after expiry");
        match second {
            ClientRpcResponse::LockSetResult(response) => {
                assert!(response.is_success);
                let old_repo = first_tokens.iter().find(|token| token.member == "repo:a").unwrap().fencing_token;
                let new_repo =
                    response.member_tokens.iter().find(|token| token.member == "repo:a").unwrap().fencing_token;
                assert!(new_repo > old_repo);
            }
            other => panic!("expected LockSetResult, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_handle_lockset_compound_operation_scenario() {
        let ctx = setup_test_context().await;
        let response = handler()
            .handle(
                ClientRpcRequest::LockSetAcquire {
                    members: vec!["repo:alpha".into(), "pipeline:42".into(), "deploy:prod".into()],
                    holder_id: "deploy-worker".into(),
                    ttl_ms: 5_000,
                    timeout_ms: 5_000,
                },
                &ctx,
            )
            .await
            .expect("compound lockset acquire should succeed");
        match response {
            ClientRpcResponse::LockSetResult(result) => {
                assert!(result.is_success);
                assert_eq!(result.member_tokens.iter().map(|token| token.member.as_str()).collect::<Vec<_>>(), vec![
                    "deploy:prod",
                    "pipeline:42",
                    "repo:alpha"
                ]);
            }
            other => panic!("expected LockSetResult, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_handle_rate_limiter_available_and_try_acquire() {
        let ctx = setup_test_context().await;
        let available_result = handler()
            .handle(
                ClientRpcRequest::RateLimiterAvailable {
                    key: "limit".into(),
                    capacity_tokens: 3,
                    refill_rate: 1.0,
                },
                &ctx,
            )
            .await
            .expect("available should succeed");
        match available_result {
            ClientRpcResponse::RateLimiterResult(response) => {
                assert!(response.is_success);
                assert_eq!(response.tokens_remaining, Some(3));
            }
            other => panic!("expected RateLimiterResult, got {other:?}"),
        }

        let try_result = handler()
            .handle(
                ClientRpcRequest::RateLimiterTryAcquire {
                    key: "limit".into(),
                    tokens: 2,
                    capacity_tokens: 3,
                    refill_rate: 1.0,
                },
                &ctx,
            )
            .await
            .expect("try acquire should succeed");
        match try_result {
            ClientRpcResponse::RateLimiterResult(response) => {
                assert!(response.is_success);
                assert_eq!(response.tokens_remaining, Some(1));
            }
            other => panic!("expected RateLimiterResult, got {other:?}"),
        }
    }
}
