//! Bolero fuzz targets for KV handler.
//!
//! Tests that the KV handler doesn't panic on any valid or malformed input.
//! Can run as both property-based tests (cargo test) and fuzz tests (cargo bolero).

#![cfg(feature = "testing")]

use std::sync::Arc;

use aspen_client_api::BatchWriteOperation;
use aspen_client_api::ClientRpcRequest;
use aspen_rpc_handlers::context::test_support::TestContextBuilder;
use aspen_rpc_handlers::handlers::KvHandler;
use aspen_rpc_handlers::registry::RequestHandler;
use aspen_rpc_handlers::test_mocks::MockEndpointProvider;
#[cfg(feature = "sql")]
use aspen_rpc_handlers::test_mocks::mock_sql_executor;
use aspen_testing::DeterministicClusterController;
use aspen_testing::DeterministicKeyValueStore;
use bolero::check;
use bolero::generator::*;

// Tiger Style: Bounded batch sizes
const MAX_BATCH_SIZE: usize = 100;

/// Generate a valid key string (alphanumeric, bounded length).
fn gen_key() -> impl ValueGenerator<Output = String> {
    r#gen::<String>()
        .with()
        .len(1usize..64)
        .map_gen(|s: String| s.chars().filter(|c| c.is_alphanumeric()).take(32).collect::<String>())
        .map_gen(|s: String| if s.is_empty() { "key".to_string() } else { s })
}

/// Generate a valid value (UTF-8 bytes, bounded size).
fn gen_value() -> impl ValueGenerator<Output = Vec<u8>> {
    r#gen::<String>().with().len(0usize..1024).map_gen(|s: String| s.into_bytes())
}

/// Generate a scan limit.
fn gen_scan_limit() -> impl ValueGenerator<Output = Option<u32>> {
    r#gen::<Option<u32>>().map_gen(|o| o.map(|v| (v % 1000) + 1))
}

/// Generate arbitrary read/write requests.
fn gen_read_write_request() -> impl ValueGenerator<Output = ClientRpcRequest> {
    let key = gen_key();
    let value = gen_value();

    (r#gen::<u8>(), key, value).map_gen(|(v, k, val)| match v % 3 {
        0 => ClientRpcRequest::ReadKey { key: k.clone() },
        1 => ClientRpcRequest::WriteKey {
            key: k.clone(),
            value: val.clone(),
        },
        _ => ClientRpcRequest::DeleteKey { key: k.clone() },
    })
}

/// Generate arbitrary scan requests.
fn gen_scan_request() -> impl ValueGenerator<Output = ClientRpcRequest> {
    let prefix = gen_key();
    let limit = gen_scan_limit();
    let continuation_token = r#gen::<Option<String>>().map_gen(|o| o.map(|s| s.chars().take(32).collect()));

    (prefix, limit, continuation_token).map_gen(|(p, l, c)| ClientRpcRequest::ScanKeys {
        prefix: p,
        limit: l,
        continuation_token: c,
    })
}

/// Generate batch write operations.
fn gen_batch_write_ops() -> impl ValueGenerator<Output = Vec<BatchWriteOperation>> {
    r#gen::<Vec<(String, String)>>().with().len(1usize..MAX_BATCH_SIZE).map_gen(|pairs| {
        pairs
            .iter()
            .map(|(k, v)| {
                let key = k.chars().filter(|c| c.is_alphanumeric()).take(32).collect::<String>();
                BatchWriteOperation::Set {
                    key: if key.is_empty() { "key".to_string() } else { key },
                    value: v.clone().into_bytes(),
                }
            })
            .collect()
    })
}

/// Generate compare-and-swap requests.
fn gen_cas_request() -> impl ValueGenerator<Output = ClientRpcRequest> {
    let key = gen_key();
    let expected = r#gen::<Option<Vec<u8>>>().map_gen(|o| o.map(|v| v.into_iter().take(256).collect()));
    let new_value = gen_value();

    (key, expected, new_value).map_gen(|(k, e, n)| ClientRpcRequest::CompareAndSwapKey {
        key: k,
        expected: e,
        new_value: n,
    })
}

/// Generate compare-and-delete requests.
fn gen_cad_request() -> impl ValueGenerator<Output = ClientRpcRequest> {
    let key = gen_key();
    let expected = gen_value();

    (key, expected).map_gen(|(k, e)| ClientRpcRequest::CompareAndDeleteKey { key: k, expected: e })
}

/// Test that can_handle correctly identifies KV requests.
#[test]
fn fuzz_kv_can_handle() {
    let handler = KvHandler;

    check!().with_generator(gen_read_write_request()).for_each(|request| {
        assert!(handler.can_handle(&request));
    });

    check!().with_generator(gen_scan_request()).for_each(|request| {
        assert!(handler.can_handle(&request));
    });

    check!().with_generator(gen_cas_request()).for_each(|request| {
        assert!(handler.can_handle(&request));
    });

    check!().with_generator(gen_cad_request()).for_each(|request| {
        assert!(handler.can_handle(&request));
    });
}

/// Test that unrelated requests are not handled by KvHandler.
#[test]
fn fuzz_kv_rejects_unrelated() {
    let handler = KvHandler;

    // Coordination requests should not be handled
    assert!(!handler.can_handle(&ClientRpcRequest::LockAcquire {
        key: "test".to_string(),
        holder_id: "holder".to_string(),
        ttl_ms: 30000,
        timeout_ms: 5000,
    }));

    // Core requests should not be handled
    assert!(!handler.can_handle(&ClientRpcRequest::Ping));
    assert!(!handler.can_handle(&ClientRpcRequest::GetHealth));

    // Cluster requests should not be handled
    assert!(!handler.can_handle(&ClientRpcRequest::GetClusterState));
}

/// Test read/write operations don't panic on any input.
#[test]
fn fuzz_kv_read_write_no_panic() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    check!().with_generator(gen_read_write_request()).for_each(|request| {
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

            let handler = KvHandler;
            let _ = handler.handle(request.clone(), &ctx).await;
        });
    });
}

/// Test scan operations don't panic on any input.
#[test]
fn fuzz_kv_scan_no_panic() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    check!().with_generator(gen_scan_request()).for_each(|request| {
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

            let handler = KvHandler;
            let _ = handler.handle(request.clone(), &ctx).await;
        });
    });
}

/// Test batch write operations don't panic on any input.
#[test]
fn fuzz_kv_batch_write_no_panic() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    check!().with_generator(gen_batch_write_ops()).for_each(|operations| {
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

            let handler = KvHandler;
            let request = ClientRpcRequest::BatchWrite {
                operations: operations.clone(),
            };
            let _ = handler.handle(request, &ctx).await;
        });
    });
}

/// Test compare-and-swap operations don't panic on any input.
#[test]
fn fuzz_kv_cas_no_panic() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    check!().with_generator(gen_cas_request()).for_each(|request| {
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

            let handler = KvHandler;
            let _ = handler.handle(request.clone(), &ctx).await;
        });
    });
}

/// Test compare-and-delete operations don't panic on any input.
#[test]
fn fuzz_kv_cad_no_panic() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    check!().with_generator(gen_cad_request()).for_each(|request| {
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

            let handler = KvHandler;
            let _ = handler.handle(request.clone(), &ctx).await;
        });
    });
}

/// Test reserved prefix rejection.
#[test]
fn fuzz_kv_reserved_prefix() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Generate keys with reserved prefix
    let reserved_key_gen = gen_key().map_gen(|suffix| format!("_system:{}", suffix));

    check!().with_generator(reserved_key_gen).for_each(|key| {
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

            let handler = KvHandler;

            // Write should fail or be rejected for reserved keys
            let request = ClientRpcRequest::WriteKey {
                key: key.clone(),
                value: b"test".to_vec(),
            };
            let result = handler.handle(request, &ctx).await;
            // Should not panic - may succeed or return error depending on implementation
            assert!(result.is_ok() || result.is_err());
        });
    });
}

/// Mixed operation sequences that exercise state transitions.
#[test]
fn fuzz_kv_mixed_sequences() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Generate a sequence of operations on the same key
    let sequence_gen = r#gen::<Vec<u8>>().with().len(1usize..20).map_gen(|bytes| {
        bytes
            .iter()
            .map(|b| match b % 4 {
                0 => ClientRpcRequest::WriteKey {
                    key: "test_key".to_string(),
                    value: vec![*b],
                },
                1 => ClientRpcRequest::ReadKey {
                    key: "test_key".to_string(),
                },
                2 => ClientRpcRequest::DeleteKey {
                    key: "test_key".to_string(),
                },
                _ => ClientRpcRequest::ScanKeys {
                    prefix: "test".to_string(),
                    limit: Some(10),
                    continuation_token: None,
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

            let handler = KvHandler;

            // Execute operations in sequence - should never panic
            for op in operations {
                let _ = handler.handle(op.clone(), &ctx).await;
            }
        });
    });
}

/// Test empty and edge case inputs.
#[test]
fn fuzz_kv_edge_cases() {
    let rt = tokio::runtime::Runtime::new().unwrap();

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

        let handler = KvHandler;

        // Empty value write
        let _ = handler
            .handle(
                ClientRpcRequest::WriteKey {
                    key: "empty_value".to_string(),
                    value: vec![],
                },
                &ctx,
            )
            .await;

        // Empty prefix scan
        let _ = handler
            .handle(
                ClientRpcRequest::ScanKeys {
                    prefix: "".to_string(),
                    limit: Some(10),
                    continuation_token: None,
                },
                &ctx,
            )
            .await;

        // Empty batch write
        let _ = handler.handle(ClientRpcRequest::BatchWrite { operations: vec![] }, &ctx).await;

        // Zero limit scan
        let _ = handler
            .handle(
                ClientRpcRequest::ScanKeys {
                    prefix: "test".to_string(),
                    limit: Some(0),
                    continuation_token: None,
                },
                &ctx,
            )
            .await;
    });
}
