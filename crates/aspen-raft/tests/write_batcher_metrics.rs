//! Unit test: verify write batcher metrics are recorded in Prometheus output.
//!
//! The write batcher's `flush_batch()` emits:
//! - `aspen.write_batcher.batch_size` histogram (number of ops per flush)
//! - `aspen.write_batcher.flush_total` counter (number of flushes)
//! - `aspen.write_batcher.flush_duration_ms` histogram (flush latency)
//! - `aspen.write_batcher.forwarded_total` counter (forwarded writes)
//! - `aspen.write_batcher.batcher_skipped_total` counter (bypassed writes)
//!
//! This test installs a prometheus recorder, emits the same metrics that
//! flush_batch would emit for a known batch size, and verifies the values
//! appear in the rendered prometheus text.
//!
//! Runs in its own process (cargo nextest) so the global recorder is safe.

use std::sync::Arc;

use metrics_exporter_prometheus::PrometheusBuilder;
use metrics_exporter_prometheus::PrometheusHandle;

fn install_test_recorder() -> Arc<PrometheusHandle> {
    let handle = PrometheusBuilder::new().install_recorder().expect("prometheus recorder install");
    Arc::new(handle)
}

/// Parse a prometheus metric value from rendered text.
///
/// For counters: looks for `metric_name{labels...} VALUE` or just `metric_name VALUE`.
/// For histograms: looks for `metric_name_count{labels...} VALUE` for observation count,
/// or `metric_name_sum{labels...} VALUE` for sum.
fn parse_metric(text: &str, line_prefix: &str) -> Option<f64> {
    for line in text.lines() {
        if line.starts_with('#') {
            continue;
        }
        if line.starts_with(line_prefix) || line.contains(line_prefix) {
            let value_str = line.rsplit_once(' ')?.1;
            return value_str.parse::<f64>().ok();
        }
    }
    None
}

#[test]
fn test_write_batcher_flush_metrics_recorded() {
    let handle = install_test_recorder();

    // Simulate what flush_batch does: record batch_size histogram and flush counter.
    // Keep one event per metric so this smoke test remains independent of recorder
    // aggregation/flush timing details.
    metrics::histogram!("aspen.write_batcher.batch_size").record(5.0);
    metrics::counter!("aspen.write_batcher.flush_total").increment(1);
    metrics::histogram!("aspen.write_batcher.flush_duration_ms").record(3.2);

    // Simulate one forwarded write and one batcher-skipped write.
    metrics::counter!("aspen.write_batcher.forwarded_total").increment(1);
    metrics::counter!("aspen.write_batcher.batcher_skipped_total").increment(1);

    let output = handle.render();

    // Verify flush_total counter = 1
    let flush_total = parse_metric(&output, "aspen_write_batcher_flush_total");
    assert_eq!(flush_total, Some(1.0), "expected 1 flush.\nOutput:\n{}", output,);

    // Verify batch_size histogram: 1 observation, sum = 5
    let batch_count = parse_metric(&output, "aspen_write_batcher_batch_size_count");
    assert_eq!(batch_count, Some(1.0), "expected 1 batch_size observation.\nOutput:\n{}", output,);

    let batch_sum = parse_metric(&output, "aspen_write_batcher_batch_size_sum");
    assert_eq!(batch_sum, Some(5.0), "expected batch_size sum = 5.\nOutput:\n{}", output,);

    // Verify flush_duration_ms histogram: 1 observation
    let dur_count = parse_metric(&output, "aspen_write_batcher_flush_duration_ms_count");
    assert_eq!(dur_count, Some(1.0), "expected 1 flush_duration observation.\nOutput:\n{}", output,);

    // Verify forwarded_total counter = 1
    let forwarded = parse_metric(&output, "aspen_write_batcher_forwarded_total");
    assert_eq!(forwarded, Some(1.0), "expected 1 forwarded write.\nOutput:\n{}", output,);

    // Verify batcher_skipped_total counter = 1
    let skipped = parse_metric(&output, "aspen_write_batcher_batcher_skipped_total");
    assert_eq!(skipped, Some(1.0), "expected 1 batcher-skipped write.\nOutput:\n{}", output,);
}
