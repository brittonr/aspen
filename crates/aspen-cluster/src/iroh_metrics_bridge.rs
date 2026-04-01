//! Bridge iroh-metrics counters into the `metrics` crate registry.
//!
//! Iroh uses its own `iroh_metrics` counter system internally. This module
//! periodically samples those counters and emits them as `metrics::gauge!()`
//! calls so they appear in our Prometheus export alongside Aspen's own metrics.
//!
//! # Metrics Emitted
//!
//! All iroh endpoint metrics are bridged under the `iroh.` prefix:
//!
//! - `iroh.socket.send_ipv4` — IPv4 packets sent
//! - `iroh.socket.send_relay` — packets sent via relay
//! - `iroh.socket.recv_data_ipv4` — IPv4 data packets received
//! - `iroh.socket.num_conns_opened` — connections opened
//! - `iroh.socket.paths_direct` — direct (holepunched) paths
//! - `iroh.socket.paths_relay` — relayed paths
//! - `iroh.net_report.reports` — net reports executed
//! - `iroh.portmapper.*` — portmapper metrics
//! - ... and all other iroh endpoint metrics
//!
//! Each counter value is emitted as a gauge (since iroh counters are monotonic
//! but we're sampling point-in-time values, not deltas).

use std::sync::Arc;
use std::time::Duration;

use iroh_metrics::MetricsGroupSet;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::trace;

/// Default sampling interval for the iroh metrics bridge.
const BRIDGE_INTERVAL: Duration = Duration::from_secs(10);

/// Start a background task that samples iroh endpoint metrics and emits them
/// as `metrics::gauge!()` calls.
///
/// Returns a cancellation token to stop the bridge. The task runs until
/// cancelled or the endpoint is dropped.
pub fn spawn_iroh_metrics_bridge(endpoint: Arc<iroh::Endpoint>) -> CancellationToken {
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    tokio::spawn(async move {
        debug!("iroh metrics bridge started (interval={}s)", BRIDGE_INTERVAL.as_secs());

        loop {
            tokio::select! {
                _ = cancel_clone.cancelled() => {
                    debug!("iroh metrics bridge stopped");
                    break;
                }
                _ = tokio::time::sleep(BRIDGE_INTERVAL) => {
                    sample_iroh_metrics(&endpoint);
                }
            }
        }
    });

    cancel
}

/// Sample all iroh endpoint metrics and emit as `metrics::gauge!()`.
fn sample_iroh_metrics(endpoint: &iroh::Endpoint) {
    let endpoint_metrics = endpoint.metrics();
    let mut count = 0u32;

    for (group_name, item) in endpoint_metrics.iter() {
        let metric_name: String = format!("iroh.{}.{}", group_name, item.name());
        let value = item.value().to_f32() as f64;
        // Use the metrics crate's gauge function with a dynamic key.
        // SharedString is Cow<'static, str> which accepts owned String.
        let gauge = metrics::gauge!(metric_name);
        gauge.set(value);
        count = count.saturating_add(1);
    }

    trace!(metrics_sampled = count, "iroh metrics bridge tick");
}
