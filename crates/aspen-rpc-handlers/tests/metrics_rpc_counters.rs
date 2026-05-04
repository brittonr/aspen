//! Integration test: verify RPC request counters appear in Prometheus output.
//!
//! Installs a Prometheus recorder, dispatches requests through HandlerRegistry,
//! then queries GetMetrics and parses the output to verify counter values.
//!
//! This test runs in its own process (cargo nextest), so installing the global
//! metrics recorder is safe.

#![cfg(feature = "testing")]

use std::sync::Arc;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_core::EndpointProvider;
use aspen_rpc_core::RequestHandler;
use aspen_rpc_core::test_support::MockEndpointProvider;
use aspen_rpc_core::test_support::TestContextBuilder;
use aspen_rpc_handlers::HandlerRegistry;
use aspen_rpc_handlers::context::ClientProtocolContext;
use metrics_exporter_prometheus::PrometheusBuilder;

/// Install the global prometheus recorder. Returns the handle for rendering.
///
/// Must be called exactly once per process (enforced by the `metrics` crate).
fn install_test_recorder() -> Arc<metrics_exporter_prometheus::PrometheusHandle> {
    let handle = PrometheusBuilder::new()
        .install_recorder()
        .expect("prometheus recorder should install (one test per process)");
    Arc::new(handle)
}

/// Build a test context with a prometheus handle wired in.
async fn build_ctx_with_metrics(handle: Arc<metrics_exporter_prometheus::PrometheusHandle>) -> ClientProtocolContext {
    let mock_endpoint = Arc::new(MockEndpointProvider::with_seed(9999).await) as Arc<dyn EndpointProvider>;
    let mut ctx = TestContextBuilder::new()
        .with_node_id(1)
        .with_endpoint_manager(mock_endpoint)
        .with_cookie("metrics-test")
        .build();
    ctx.prometheus_handle = Some(handle);
    ctx
}

/// Build a HandlerRegistry with a mock Read/Write handler plus CoreHandler.
///
/// The mock handler accepts ReadKey and WriteKey and returns stub responses
/// so that dispatch() succeeds and the counter/histogram in the dispatch loop
/// records the operation.
fn build_registry_with_mock_kv(_ctx: &ClientProtocolContext) -> HandlerRegistry {
    let registry = HandlerRegistry::empty();

    let core: Arc<dyn RequestHandler> = Arc::new(aspen_core_essentials_handler::CoreHandler);
    let mock_kv: Arc<dyn RequestHandler> = Arc::new(MockKvHandler);

    // CoreHandler at priority 200, MockKvHandler at priority 110.
    registry.add_handlers(vec![(core, 200), (mock_kv, 110)]);
    registry
}

/// Mock handler that accepts ReadKey / WriteKey and returns stub responses.
struct MockKvHandler;

#[async_trait::async_trait]
impl RequestHandler for MockKvHandler {
    fn name(&self) -> &'static str {
        "MockKvHandler"
    }

    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(request, ClientRpcRequest::ReadKey { .. } | ClientRpcRequest::WriteKey { .. })
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        _ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::ReadKey { .. } => {
                Ok(ClientRpcResponse::ReadResult(aspen_client_api::ReadResultResponse {
                    value: None,
                    was_found: false,
                    error: None,
                }))
            }
            ClientRpcRequest::WriteKey { .. } => {
                Ok(ClientRpcResponse::WriteResult(aspen_client_api::WriteResultResponse {
                    is_success: true,
                    error: None,
                }))
            }
            _ => Err(anyhow::anyhow!("unexpected request")),
        }
    }
}

/// Parse a Prometheus counter value from rendered text.
///
/// Looks for lines like:
///   `aspen_rpc_requests_total{operation="ReadKey"} 10`
///
/// Returns `None` if not found.
fn parse_counter(text: &str, metric_name: &str, label_key: &str, label_val: &str) -> Option<f64> {
    for line in text.lines() {
        if line.starts_with('#') {
            continue;
        }
        // Match: metric_name{...label_key="label_val"...} VALUE
        let expected_label = format!("{}=\"{}\"", label_key, label_val);
        if line.contains(metric_name) && line.contains(&expected_label) {
            // Value is the last whitespace-separated token
            let value_str = line.rsplit_once(' ')?.1;
            return value_str.parse::<f64>().ok();
        }
    }
    None
}

#[tokio::test]
async fn test_rpc_request_counters_in_prometheus_output() {
    let handle = install_test_recorder();
    let ctx = build_ctx_with_metrics(handle).await;
    let registry = build_registry_with_mock_kv(&ctx);

    // Dispatch 10 ReadKey requests.
    for i in 0..10 {
        let req = ClientRpcRequest::ReadKey {
            key: format!("key-{}", i),
        };
        let resp = registry.dispatch(req, &ctx, 0, None).await.expect("dispatch ReadKey");
        assert!(matches!(resp, ClientRpcResponse::ReadResult(_)));
    }

    // Dispatch 5 WriteKey requests.
    for i in 0..5 {
        let req = ClientRpcRequest::WriteKey {
            key: format!("key-{}", i),
            value: vec![i as u8],
        };
        let resp = registry.dispatch(req, &ctx, 0, None).await.expect("dispatch WriteKey");
        assert!(matches!(resp, ClientRpcResponse::WriteResult(_)));
    }

    // Query GetMetrics to get prometheus output.
    let metrics_resp =
        registry.dispatch(ClientRpcRequest::GetMetrics, &ctx, 0, None).await.expect("dispatch GetMetrics");

    let prometheus_text = match metrics_resp {
        ClientRpcResponse::Metrics(m) => m.prometheus_text,
        other => panic!("expected Metrics response, got {:?}", other),
    };

    // Verify counter values.
    // The metrics crate converts dots to underscores: "aspen.rpc.requests_total"
    // becomes "aspen_rpc_requests_total".
    let read_count = parse_counter(&prometheus_text, "aspen_rpc_requests_total", "operation", "ReadKey");
    let write_count = parse_counter(&prometheus_text, "aspen_rpc_requests_total", "operation", "WriteKey");
    // GetMetrics itself is also counted (we dispatched it once).
    let metrics_count = parse_counter(&prometheus_text, "aspen_rpc_requests_total", "operation", "GetMetrics");

    assert_eq!(
        read_count,
        Some(10.0),
        "expected 10 ReadKey requests in prometheus output.\nFull output:\n{}",
        prometheus_text,
    );
    assert_eq!(
        write_count,
        Some(5.0),
        "expected 5 WriteKey requests in prometheus output.\nFull output:\n{}",
        prometheus_text,
    );
    assert!(
        metrics_count.is_some() && metrics_count.unwrap() >= 1.0,
        "expected at least 1 GetMetrics request in prometheus output.\nFull output:\n{}",
        prometheus_text,
    );

    // Verify duration histograms exist for the operations.
    assert!(
        prometheus_text.contains("aspen_rpc_duration_ms"),
        "expected duration histogram in prometheus output",
    );
}
