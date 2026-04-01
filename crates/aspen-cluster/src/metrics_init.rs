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

use metrics_exporter_prometheus::PrometheusBuilder;
use metrics_exporter_prometheus::PrometheusHandle;
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
