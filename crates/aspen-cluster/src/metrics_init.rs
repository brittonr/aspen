//! Metrics registry initialization.
//!
//! Installs the `metrics-exporter-prometheus` recorder at node startup and
//! returns a [`PrometheusHandle`] for rendering Prometheus text format on demand.
//!
//! The `metrics` crate is a facade — call-sites use `metrics::counter!()`,
//! `metrics::gauge!()`, `metrics::histogram!()` and the recorder captures them.
//! When no recorder is installed (e.g., in tests), calls are no-ops.
//!
//! # Architecture
//!
//! ```text
//! metrics::counter!("aspen.rpc.requests_total", ...)
//!        │
//!        ▼
//! PrometheusBuilder recorder (installed once at startup)
//!        │
//!        ▼
//! PrometheusHandle::render() → Prometheus text format
//! ```

use std::sync::Arc;
use std::time::Duration;

use metrics_exporter_prometheus::PrometheusBuilder;
use metrics_exporter_prometheus::PrometheusHandle;
use tracing::debug;
use tracing::info;
use tracing::warn;

/// Install the Prometheus metrics recorder and return a handle for rendering.
///
/// This must be called exactly once per process, early in startup (before any
/// `metrics::*!()` calls). A second call will log a warning and return `None`.
///
/// The returned [`PrometheusHandle`] is cheaply cloneable and can be stored in
/// [`ClientProtocolContext`] for the `GetMetrics` handler to call `render()`.
pub fn install_prometheus_recorder() -> Option<Arc<PrometheusHandle>> {
    match PrometheusBuilder::new().install_recorder() {
        Ok(handle) => {
            info!("prometheus metrics recorder installed");
            Some(Arc::new(handle))
        }
        Err(e) => {
            warn!(error = %e, "failed to install prometheus recorder (already installed?)");
            None
        }
    }
}

/// Install a Prometheus recorder and configure OTLP metrics export.
///
/// The Prometheus recorder is installed as the global `metrics` recorder
/// (for `GetMetrics` RPC rendering). A separate OpenTelemetry
/// `PeriodicReader` pushes metrics to the OTLP endpoint independently.
///
/// # Arguments
///
/// * `otlp_endpoint` - gRPC endpoint, e.g. `http://localhost:4317`
/// * `node_id` - Node identifier for resource attributes
/// * `cluster_cookie` - Cluster cookie for resource attributes
#[cfg(feature = "otlp")]
pub fn install_prometheus_and_otlp_recorder(
    otlp_endpoint: &str,
    node_id: u64,
    cluster_cookie: &str,
) -> Option<Arc<PrometheusHandle>> {
    use opentelemetry::KeyValue;
    use opentelemetry_sdk::Resource;

    // Build the OTLP metrics pipeline using the high-level builder.
    let exporter = match opentelemetry_otlp::MetricExporter::builder().with_tonic().build() {
        Ok(e) => e,
        Err(e) => {
            warn!(error = %e, "failed to build OTLP metric exporter, falling back to prometheus-only");
            return install_prometheus_recorder();
        }
    };

    let resource = Resource::builder()
        .with_attributes([
            KeyValue::new("service.name", "aspen-node"),
            KeyValue::new("service.instance.id", node_id.to_string()),
            KeyValue::new("aspen.cluster.cookie", cluster_cookie.to_string()),
        ])
        .build();

    // Set the OTLP endpoint via env var (OTEL_EXPORTER_OTLP_ENDPOINT).
    // The opentelemetry-otlp crate reads this automatically.
    // SAFETY: Called once at startup before any threads use this env var.
    unsafe {
        std::env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", otlp_endpoint);
    }

    let reader = opentelemetry_sdk::metrics::PeriodicReader::builder(exporter)
        .with_interval(std::time::Duration::from_secs(10))
        .build();

    let _provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(resource)
        .build();

    // Install the Prometheus recorder as the global `metrics` recorder.
    // The OTel PeriodicReader pushes to OTLP independently — it reads from
    // the MeterProvider, not from the `metrics` crate. To bridge the two,
    // use `metrics-exporter-opentelemetry` as the global recorder in a
    // future iteration. For now, OTLP receives OTel SDK-native metrics
    // while Prometheus receives `metrics` crate metrics.
    let handle = install_prometheus_recorder();
    if handle.is_some() {
        info!(endpoint = otlp_endpoint, "OTLP metric exporter configured alongside prometheus");
    }
    handle
}

/// Interval between periodic network gauge emissions.
const NETWORK_GAUGE_INTERVAL: Duration = Duration::from_secs(10);

/// Spawn a background task that periodically emits network connection gauges.
///
/// Polls the connection pool every 10 seconds and sets gauges for:
/// - `aspen.network.connections` (by state: healthy/degraded/failed)
/// - `aspen.network.active_streams`
///
/// Accepts any `RaftConnectionPool<T>` via `Arc`. Returns a `JoinHandle`
/// the caller can abort on shutdown.
pub fn spawn_network_gauge_task<T>(
    pool: Arc<aspen_raft::connection_pool::RaftConnectionPool<T>>,
) -> tokio::task::JoinHandle<()>
where T: aspen_core::NetworkTransport<Endpoint = iroh::Endpoint, Address = iroh::EndpointAddr> + 'static {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(NETWORK_GAUGE_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;
            let m = pool.metrics().await;

            metrics::gauge!("aspen.network.connections", "state" => "healthy").set(m.healthy_connections as f64);
            metrics::gauge!("aspen.network.connections", "state" => "degraded").set(m.degraded_connections as f64);
            metrics::gauge!("aspen.network.connections", "state" => "failed").set(m.failed_connections as f64);
            metrics::gauge!("aspen.network.active_streams").set(m.total_active_streams as f64);

            debug!(
                healthy = m.healthy_connections,
                degraded = m.degraded_connections,
                failed = m.failed_connections,
                streams = m.total_active_streams,
                "network gauges updated",
            );
        }
    })
}
