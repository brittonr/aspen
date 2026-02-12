//! Bolero fuzz targets for Coordination handler.
//!
//! Tests that the Coordination handler doesn't panic on any valid or malformed input.
//! Can run as both property-based tests (cargo test) and fuzz tests (cargo bolero).

#![cfg(feature = "testing")]

use std::sync::Arc;

use aspen_client_api::ClientRpcRequest;
use aspen_testing::DeterministicClusterController;
use aspen_testing::DeterministicKeyValueStore;
use aspen_rpc_handlers::context::test_support::TestContextBuilder;
use aspen_rpc_handlers::handlers::CoordinationHandler;
use aspen_rpc_handlers::registry::RequestHandler;
use aspen_rpc_handlers::test_mocks::MockEndpointProvider;
#[cfg(feature = "sql")]
use aspen_rpc_handlers::test_mocks::mock_sql_executor;
use bolero::check;
use bolero::generator::*;

// Tiger Style: Bounded operation sequences
const MAX_SEQUENCE_LENGTH: usize = 20;

/// Generate a valid key string (alphanumeric, bounded length).
fn gen_key() -> impl ValueGenerator<Output = String> {
    r#gen::<String>()
        .with()
        .len(1usize..64)
        .map_gen(|s: String| s.chars().filter(|c| c.is_alphanumeric()).take(32).collect::<String>())
        .map_gen(|s: String| if s.is_empty() { "key".to_string() } else { s })
}

/// Generate a holder ID string.
fn gen_holder_id() -> impl ValueGenerator<Output = String> {
    r#gen::<u64>().map_gen(|n| format!("holder_{}", n % 1000))
}

/// Generate a bounded u64 for TTLs and timeouts (in milliseconds).
fn gen_ttl_ms() -> impl ValueGenerator<Output = u64> {
    1000u64..=3600000u64 // 1 second to 1 hour in ms
}

/// Generate a bounded i64 for counter deltas.
fn gen_delta() -> impl ValueGenerator<Output = i64> {
    -1000i64..=1000i64
}

/// Generate arbitrary lock operation requests.
fn gen_lock_request() -> impl ValueGenerator<Output = ClientRpcRequest> {
    let key = gen_key();
    let holder_id = gen_holder_id();
    let ttl_ms = gen_ttl_ms();
    let fencing_token = r#gen::<u64>();

    (r#gen::<u8>(), key, holder_id, ttl_ms, fencing_token).map_gen(|(v, k, h, t, ft)| match v % 4 {
        0 => ClientRpcRequest::LockAcquire {
            key: k.clone(),
            holder_id: h.clone(),
            ttl_ms: t,
            timeout_ms: t / 2,
        },
        1 => ClientRpcRequest::LockTryAcquire {
            key: k.clone(),
            holder_id: h.clone(),
            ttl_ms: t,
        },
        2 => ClientRpcRequest::LockRelease {
            key: k.clone(),
            holder_id: h.clone(),
            fencing_token: ft,
        },
        _ => ClientRpcRequest::LockRenew {
            key: k.clone(),
            holder_id: h.clone(),
            fencing_token: ft,
            ttl_ms: t,
        },
    })
}

/// Generate arbitrary counter operation requests.
fn gen_counter_request() -> impl ValueGenerator<Output = ClientRpcRequest> {
    let key = gen_key();
    let amount = r#gen::<i64>().with().bounds(-1000i64..=1000i64);
    let value = r#gen::<u64>().with().bounds(0u64..=1000000u64);
    let expected = r#gen::<u64>().with().bounds(0u64..=1000000u64);

    (r#gen::<u8>(), key, amount, value, expected).map_gen(|(v, k, a, val, exp)| match v % 6 {
        0 => ClientRpcRequest::CounterIncrement { key: k.clone() },
        1 => ClientRpcRequest::CounterDecrement { key: k.clone() },
        2 => ClientRpcRequest::CounterGet { key: k.clone() },
        3 => ClientRpcRequest::CounterCompareAndSet {
            key: k.clone(),
            expected: exp,
            new_value: val,
        },
        4 => ClientRpcRequest::SignedCounterAdd {
            key: k.clone(),
            amount: a,
        },
        _ => ClientRpcRequest::SignedCounterGet { key: k.clone() },
    })
}

/// Generate arbitrary rate limiter requests.
fn gen_rate_limiter_request() -> impl ValueGenerator<Output = ClientRpcRequest> {
    let key = gen_key();
    let tokens = r#gen::<u64>().with().bounds(1u64..=1000u64);
    let capacity = r#gen::<u64>().with().bounds(1u64..=10000u64);
    let refill_rate = r#gen::<f64>().with().bounds(0.1f64..=100.0f64);

    (r#gen::<u8>(), key, tokens, capacity, refill_rate).map_gen(|(v, k, t, c, r)| match v % 3 {
        0 => ClientRpcRequest::RateLimiterTryAcquire {
            key: k.clone(),
            tokens: t,
            capacity: c,
            refill_rate: r,
        },
        1 => ClientRpcRequest::RateLimiterAvailable {
            key: k.clone(),
            capacity: c,
            refill_rate: r,
        },
        _ => ClientRpcRequest::RateLimiterReset {
            key: k.clone(),
            capacity: c,
            refill_rate: r,
        },
    })
}

/// Generate arbitrary semaphore requests.
fn gen_semaphore_request() -> impl ValueGenerator<Output = ClientRpcRequest> {
    let name = gen_key();
    let holder_id = gen_holder_id();
    let permits = r#gen::<u32>().with().bounds(1u32..=100u32);
    let capacity = r#gen::<u32>().with().bounds(10u32..=1000u32);
    let ttl_ms = gen_ttl_ms();
    let timeout_ms = gen_ttl_ms();

    (r#gen::<u8>(), name, holder_id, permits, capacity, ttl_ms, timeout_ms).map_gen(|(v, n, h, p, c, ttl, timeout)| {
        match v % 3 {
            0 => ClientRpcRequest::SemaphoreAcquire {
                name: n.clone(),
                holder_id: h.clone(),
                permits: p,
                capacity: c,
                ttl_ms: ttl,
                timeout_ms: timeout,
            },
            1 => ClientRpcRequest::SemaphoreRelease {
                name: n.clone(),
                holder_id: h.clone(),
                permits: p,
            },
            _ => ClientRpcRequest::SemaphoreStatus { name: n.clone() },
        }
    })
}

/// Generate arbitrary barrier requests.
fn gen_barrier_request() -> impl ValueGenerator<Output = ClientRpcRequest> {
    let name = gen_key();
    let participant_id = gen_holder_id();
    let required_count = r#gen::<u32>().with().bounds(1u32..=10u32);
    let timeout_ms = gen_ttl_ms();

    (r#gen::<u8>(), name, participant_id, required_count, timeout_ms).map_gen(|(v, n, p, c, t)| match v % 3 {
        0 => ClientRpcRequest::BarrierEnter {
            name: n.clone(),
            participant_id: p.clone(),
            required_count: c,
            timeout_ms: t,
        },
        1 => ClientRpcRequest::BarrierStatus { name: n.clone() },
        _ => ClientRpcRequest::BarrierLeave {
            name: n.clone(),
            participant_id: p.clone(),
            timeout_ms: t,
        },
    })
}

/// Test that can_handle correctly identifies coordination requests.
#[test]
fn fuzz_coordination_can_handle() {
    let handler = CoordinationHandler;

    check!().with_generator(gen_lock_request()).for_each(|request| {
        // Lock operations should be handled by CoordinationHandler
        assert!(handler.can_handle(&request));
    });

    check!().with_generator(gen_counter_request()).for_each(|request| {
        // Counter operations should be handled by CoordinationHandler
        assert!(handler.can_handle(&request));
    });

    check!().with_generator(gen_rate_limiter_request()).for_each(|request| {
        // Rate limiter operations should be handled by CoordinationHandler
        assert!(handler.can_handle(&request));
    });

    check!().with_generator(gen_semaphore_request()).for_each(|request| {
        // Semaphore operations should be handled by CoordinationHandler
        assert!(handler.can_handle(&request));
    });

    check!().with_generator(gen_barrier_request()).for_each(|request| {
        // Barrier operations should be handled by CoordinationHandler
        assert!(handler.can_handle(&request));
    });
}

/// Test that unrelated requests are not handled by CoordinationHandler.
#[test]
fn fuzz_coordination_rejects_unrelated() {
    let handler = CoordinationHandler;

    // KV requests should not be handled
    check!().with_generator(gen_key()).for_each(|key| {
        let request = ClientRpcRequest::ReadKey { key: key.clone() };
        assert!(!handler.can_handle(&request));
    });

    // Core requests should not be handled
    assert!(!handler.can_handle(&ClientRpcRequest::Ping));
    assert!(!handler.can_handle(&ClientRpcRequest::GetHealth));
    assert!(!handler.can_handle(&ClientRpcRequest::GetRaftMetrics));
}

/// Test lock operations don't panic on any input.
#[test]
fn fuzz_lock_operations_no_panic() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    check!().with_generator(gen_lock_request()).for_each(|request| {
        rt.block_on(async {
            let controller = Arc::new(DeterministicClusterController::new());
            let kv_store = Arc::new(DeterministicKeyValueStore::new());
            let mock_endpoint = Arc::new(MockEndpointProvider::with_seed(12345).await);
            let builder = TestContextBuilder::new()
                .with_node_id(1)
                .with_controller(controller)
                .with_kv_store(kv_store)
                .with_endpoint_manager(mock_endpoint)
                .with_cookie("test_cluster");
            #[cfg(feature = "sql")]
            let builder = builder.with_sql_executor(mock_sql_executor());
            let ctx = builder.build();

            let handler = CoordinationHandler;
            // Handler should never panic, even on edge cases
            let _ = handler.handle(request.clone(), &ctx).await;
        });
    });
}

/// Test counter operations don't panic on any input.
#[test]
fn fuzz_counter_operations_no_panic() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    check!().with_generator(gen_counter_request()).for_each(|request| {
        rt.block_on(async {
            let controller = Arc::new(DeterministicClusterController::new());
            let kv_store = Arc::new(DeterministicKeyValueStore::new());
            let mock_endpoint = Arc::new(MockEndpointProvider::with_seed(12345).await);
            let builder = TestContextBuilder::new()
                .with_node_id(1)
                .with_controller(controller)
                .with_kv_store(kv_store)
                .with_endpoint_manager(mock_endpoint)
                .with_cookie("test_cluster");
            #[cfg(feature = "sql")]
            let builder = builder.with_sql_executor(mock_sql_executor());
            let ctx = builder.build();

            let handler = CoordinationHandler;
            let _ = handler.handle(request.clone(), &ctx).await;
        });
    });
}

/// Test rate limiter operations don't panic on any input.
#[test]
fn fuzz_rate_limiter_operations_no_panic() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    check!().with_generator(gen_rate_limiter_request()).for_each(|request| {
        rt.block_on(async {
            let controller = Arc::new(DeterministicClusterController::new());
            let kv_store = Arc::new(DeterministicKeyValueStore::new());
            let mock_endpoint = Arc::new(MockEndpointProvider::with_seed(12345).await);
            let builder = TestContextBuilder::new()
                .with_node_id(1)
                .with_controller(controller)
                .with_kv_store(kv_store)
                .with_endpoint_manager(mock_endpoint)
                .with_cookie("test_cluster");
            #[cfg(feature = "sql")]
            let builder = builder.with_sql_executor(mock_sql_executor());
            let ctx = builder.build();

            let handler = CoordinationHandler;
            let _ = handler.handle(request.clone(), &ctx).await;
        });
    });
}

/// Test semaphore operations don't panic on any input.
#[test]
fn fuzz_semaphore_operations_no_panic() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    check!().with_generator(gen_semaphore_request()).for_each(|request| {
        rt.block_on(async {
            let controller = Arc::new(DeterministicClusterController::new());
            let kv_store = Arc::new(DeterministicKeyValueStore::new());
            let mock_endpoint = Arc::new(MockEndpointProvider::with_seed(12345).await);
            let builder = TestContextBuilder::new()
                .with_node_id(1)
                .with_controller(controller)
                .with_kv_store(kv_store)
                .with_endpoint_manager(mock_endpoint)
                .with_cookie("test_cluster");
            #[cfg(feature = "sql")]
            let builder = builder.with_sql_executor(mock_sql_executor());
            let ctx = builder.build();

            let handler = CoordinationHandler;
            let _ = handler.handle(request.clone(), &ctx).await;
        });
    });
}

/// Test barrier operations don't panic on any input.
#[test]
fn fuzz_barrier_operations_no_panic() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    check!().with_generator(gen_barrier_request()).for_each(|request| {
        rt.block_on(async {
            let controller = Arc::new(DeterministicClusterController::new());
            let kv_store = Arc::new(DeterministicKeyValueStore::new());
            let mock_endpoint = Arc::new(MockEndpointProvider::with_seed(12345).await);
            let builder = TestContextBuilder::new()
                .with_node_id(1)
                .with_controller(controller)
                .with_kv_store(kv_store)
                .with_endpoint_manager(mock_endpoint)
                .with_cookie("test_cluster");
            #[cfg(feature = "sql")]
            let builder = builder.with_sql_executor(mock_sql_executor());
            let ctx = builder.build();

            let handler = CoordinationHandler;
            let _ = handler.handle(request.clone(), &ctx).await;
        });
    });
}

/// Mixed operation sequences that exercise state transitions.
#[test]
fn fuzz_mixed_operation_sequences() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Generate a sequence of mixed operations
    let sequence_gen = r#gen::<Vec<u8>>().with().len(1usize..MAX_SEQUENCE_LENGTH).map_gen(|bytes| {
        bytes
            .iter()
            .map(|b| match b % 5 {
                0 => ClientRpcRequest::LockAcquire {
                    key: "test_key".to_string(),
                    holder_id: "test_holder".to_string(),
                    ttl_ms: 30000,
                    timeout_ms: 5000,
                },
                1 => ClientRpcRequest::CounterIncrement {
                    key: "test_counter".to_string(),
                },
                2 => ClientRpcRequest::RateLimiterTryAcquire {
                    key: "test_limiter".to_string(),
                    tokens: 1,
                    capacity: 10,
                    refill_rate: 1.0,
                },
                3 => ClientRpcRequest::SemaphoreAcquire {
                    name: "test_sem".to_string(),
                    holder_id: "holder_1".to_string(),
                    permits: 1,
                    capacity: 5,
                    ttl_ms: 30000,
                    timeout_ms: 5000,
                },
                _ => ClientRpcRequest::BarrierStatus {
                    name: "test_barrier".to_string(),
                },
            })
            .collect::<Vec<_>>()
    });

    check!().with_generator(sequence_gen).for_each(|operations| {
        rt.block_on(async {
            let controller = Arc::new(DeterministicClusterController::new());
            let kv_store = Arc::new(DeterministicKeyValueStore::new());
            let mock_endpoint = Arc::new(MockEndpointProvider::with_seed(12345).await);
            let builder = TestContextBuilder::new()
                .with_node_id(1)
                .with_controller(controller)
                .with_kv_store(kv_store)
                .with_endpoint_manager(mock_endpoint)
                .with_cookie("test_cluster");
            #[cfg(feature = "sql")]
            let builder = builder.with_sql_executor(mock_sql_executor());
            let ctx = builder.build();

            let handler = CoordinationHandler;

            // Execute operations in sequence - should never panic
            for op in operations {
                let _ = handler.handle(op.clone(), &ctx).await;
            }
        });
    });
}
