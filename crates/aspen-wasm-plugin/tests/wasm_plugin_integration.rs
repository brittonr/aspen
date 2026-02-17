//! Integration tests for the WASM plugin handler pipeline.
//!
//! These tests load the pre-built echo plugin WASM binary, dispatch real
//! `ClientRpcRequest` messages through the guest's `handle_request` export,
//! and verify correctness of the full load-register-dispatch path.
//!
//! All tests require `/dev/kvm` for the hyperlight-wasm micro-VM sandbox.

#![cfg(feature = "testing")]

use std::sync::Arc;

use aspen_blob::InMemoryBlobStore;
use aspen_blob::prelude::*;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_core::KeyValueStore;
use aspen_kv_types::WriteRequest;
use aspen_rpc_core::RequestHandler;
use aspen_rpc_core::test_support::MockEndpointProvider;
use aspen_rpc_core::test_support::TestContextBuilder;
use aspen_testing::DeterministicClusterController;
use aspen_testing::DeterministicKeyValueStore;
use aspen_wasm_plugin::test_support::PluginHostContext;
use aspen_wasm_plugin::test_support::load_wasm_handler;

const ECHO_PLUGIN_NAME: &str = "echo-plugin";
const PLUGIN_MEMORY: u64 = 4 * 1024 * 1024; // 4 MiB

/// Load the echo plugin WASM binary from the build output directory.
///
/// Expects the binary at `target/wasm32-unknown-unknown/release/aspen_echo_plugin.wasm`
/// relative to the workspace root. Panics with an actionable error if not found.
fn load_echo_plugin_wasm() -> Vec<u8> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    // CARGO_MANIFEST_DIR points to crates/aspen-wasm-plugin; workspace root is two levels up
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("could not determine workspace root");
    let wasm_path = workspace_root
        .join("target")
        .join("wasm32-unknown-unknown")
        .join("release")
        .join("aspen_echo_plugin.wasm");

    std::fs::read(&wasm_path).unwrap_or_else(|e| {
        panic!(
            "failed to read echo plugin WASM binary at {}: {e}\n\
             Build it first with: cargo build --package aspen-echo-plugin \
             --target wasm32-unknown-unknown --release",
            wasm_path.display()
        )
    })
}

/// Create a `PluginHostContext` with deterministic in-memory implementations.
fn create_test_host_context(
    kv_store: Arc<dyn aspen_core::KeyValueStore>,
    blob_store: Arc<dyn BlobStore>,
    controller: Arc<dyn aspen_traits::ClusterController>,
) -> Arc<PluginHostContext> {
    Arc::new(PluginHostContext::new(
        kv_store,
        blob_store,
        controller,
        1, // node_id
        ECHO_PLUGIN_NAME.to_string(),
    ))
}

/// Create a `WasmPluginHandler` from the echo plugin WASM binary.
fn create_echo_handler(ctx: Arc<PluginHostContext>) -> aspen_wasm_plugin::WasmPluginHandler {
    let wasm_bytes = load_echo_plugin_wasm();
    load_wasm_handler(&wasm_bytes, ECHO_PLUGIN_NAME, ctx, PLUGIN_MEMORY).expect("failed to load echo plugin")
}

/// Create a minimal `ClientProtocolContext` for the `RequestHandler::handle` signature.
async fn create_dummy_protocol_context() -> aspen_rpc_core::ClientProtocolContext {
    let mock_ep = Arc::new(MockEndpointProvider::new().await);
    TestContextBuilder::new()
        .with_endpoint_manager(mock_ep as Arc<dyn aspen_core::EndpointProvider>)
        .build()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires KVM (/dev/kvm)"]
async fn wasm_plugin_ping_returns_pong() {
    let kv = DeterministicKeyValueStore::new();
    let blobs: Arc<dyn BlobStore> = Arc::new(InMemoryBlobStore::new());
    let ctrl = DeterministicClusterController::new();
    let ctx = create_test_host_context(kv, blobs, ctrl);
    let handler = create_echo_handler(ctx);
    let proto_ctx = create_dummy_protocol_context().await;

    let response = handler.handle(ClientRpcRequest::Ping, &proto_ctx).await.expect("handle Ping should succeed");

    assert!(matches!(response, ClientRpcResponse::Pong), "expected Pong, got {response:?}");
}

#[tokio::test]
#[ignore = "requires KVM (/dev/kvm)"]
async fn wasm_plugin_read_key_found() {
    let kv = DeterministicKeyValueStore::new();
    let blobs: Arc<dyn BlobStore> = Arc::new(InMemoryBlobStore::new());
    let ctrl = DeterministicClusterController::new();

    // Pre-populate the KV store
    KeyValueStore::write(kv.as_ref(), WriteRequest::set("hello", "world"))
        .await
        .expect("write should succeed");

    let ctx = create_test_host_context(kv, blobs, ctrl);
    let handler = create_echo_handler(ctx);
    let proto_ctx = create_dummy_protocol_context().await;

    let response = handler
        .handle(
            ClientRpcRequest::ReadKey {
                key: "hello".to_string(),
            },
            &proto_ctx,
        )
        .await
        .expect("handle ReadKey should succeed");

    match response {
        ClientRpcResponse::ReadResult(result) => {
            assert!(result.was_found, "key should be found");
            assert_eq!(result.value.as_deref(), Some(b"world".as_slice()), "value should match what was written");
        }
        other => panic!("expected ReadResult, got {other:?}"),
    }
}

#[tokio::test]
#[ignore = "requires KVM (/dev/kvm)"]
async fn wasm_plugin_read_key_not_found() {
    let kv = DeterministicKeyValueStore::new();
    let blobs: Arc<dyn BlobStore> = Arc::new(InMemoryBlobStore::new());
    let ctrl = DeterministicClusterController::new();
    let ctx = create_test_host_context(kv, blobs, ctrl);
    let handler = create_echo_handler(ctx);
    let proto_ctx = create_dummy_protocol_context().await;

    let response = handler
        .handle(
            ClientRpcRequest::ReadKey {
                key: "nonexistent".to_string(),
            },
            &proto_ctx,
        )
        .await
        .expect("handle ReadKey should succeed");

    match response {
        ClientRpcResponse::ReadResult(result) => {
            assert!(!result.was_found, "key should not be found");
            assert!(result.value.is_none(), "value should be None for missing key");
        }
        other => panic!("expected ReadResult, got {other:?}"),
    }
}

#[tokio::test]
#[ignore = "requires KVM (/dev/kvm)"]
async fn wasm_plugin_can_handle_routing() {
    let kv = DeterministicKeyValueStore::new();
    let blobs: Arc<dyn BlobStore> = Arc::new(InMemoryBlobStore::new());
    let ctrl = DeterministicClusterController::new();
    let ctx = create_test_host_context(kv, blobs, ctrl);
    let handler = create_echo_handler(ctx);

    // Echo plugin handles Ping and ReadKey
    assert!(handler.can_handle(&ClientRpcRequest::Ping), "should handle Ping");
    assert!(handler.can_handle(&ClientRpcRequest::ReadKey { key: "k".to_string() }), "should handle ReadKey");

    // Echo plugin does NOT handle WriteKey or GetClusterState
    assert!(
        !handler.can_handle(&ClientRpcRequest::WriteKey {
            key: "k".to_string(),
            value: b"v".to_vec(),
        }),
        "should not handle WriteKey"
    );
    assert!(!handler.can_handle(&ClientRpcRequest::GetClusterState), "should not handle GetClusterState");
}

#[tokio::test]
#[ignore = "requires KVM (/dev/kvm)"]
async fn wasm_plugin_name_matches() {
    let kv = DeterministicKeyValueStore::new();
    let blobs: Arc<dyn BlobStore> = Arc::new(InMemoryBlobStore::new());
    let ctrl = DeterministicClusterController::new();
    let ctx = create_test_host_context(kv, blobs, ctrl);
    let handler = create_echo_handler(ctx);

    assert_eq!(handler.name(), ECHO_PLUGIN_NAME);
}

#[tokio::test]
#[ignore = "requires KVM (/dev/kvm)"]
async fn wasm_plugin_unhandled_request_dispatched() {
    let kv = DeterministicKeyValueStore::new();
    let blobs: Arc<dyn BlobStore> = Arc::new(InMemoryBlobStore::new());
    let ctrl = DeterministicClusterController::new();
    let ctx = create_test_host_context(kv, blobs, ctrl);
    let handler = create_echo_handler(ctx);
    let proto_ctx = create_dummy_protocol_context().await;

    // Send WriteKey through handle() directly (bypassing can_handle)
    let response = handler
        .handle(
            ClientRpcRequest::WriteKey {
                key: "k".to_string(),
                value: b"v".to_vec(),
            },
            &proto_ctx,
        )
        .await
        .expect("handle should succeed even for unhandled request");

    match response {
        ClientRpcResponse::Error(err) => {
            assert_eq!(err.code, "UNHANDLED", "error code should be UNHANDLED");
        }
        other => panic!("expected Error response, got {other:?}"),
    }
}

#[test]
#[ignore = "requires KVM (/dev/kvm)"]
fn wasm_plugin_invalid_wasm_fails() {
    let kv = DeterministicKeyValueStore::new();
    let blobs: Arc<dyn BlobStore> = Arc::new(InMemoryBlobStore::new());
    let ctrl = DeterministicClusterController::new();
    let ctx = create_test_host_context(kv, blobs, ctrl);

    // Truncated/invalid WASM binary
    let bad_wasm = b"\x00asm\x01\x00\x00\x00garbage";
    let result = load_wasm_handler(bad_wasm, ECHO_PLUGIN_NAME, ctx, PLUGIN_MEMORY);

    assert!(result.is_err(), "loading invalid WASM should fail");
}

#[test]
#[ignore = "requires KVM (/dev/kvm)"]
fn wasm_plugin_name_mismatch_fails() {
    let kv = DeterministicKeyValueStore::new();
    let blobs: Arc<dyn BlobStore> = Arc::new(InMemoryBlobStore::new());
    let ctrl = DeterministicClusterController::new();
    let ctx = create_test_host_context(kv, blobs, ctrl);

    let wasm_bytes = load_echo_plugin_wasm();
    // Pass wrong expected name -- guest will report "echo-plugin"
    let result = load_wasm_handler(&wasm_bytes, "wrong-name", ctx, PLUGIN_MEMORY);

    match result {
        Err(e) => {
            let err_msg = e.to_string();
            assert!(err_msg.contains("mismatch"), "error should mention mismatch: {err_msg}");
        }
        Ok(_) => panic!("expected name mismatch error, but load succeeded"),
    }
}
