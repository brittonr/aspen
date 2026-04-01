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
    // We simulate 3 flushes with batch sizes 5, 10, and 15.
    let batch_sizes = [5u64, 10, 15];
    for &batch_size in &batch_sizes {
        metrics::histogram!("aspen.write_batcher.batch_size").record(batch_size as f64);
        metrics::counter!("aspen.write_batcher.flush_total").increment(1);
        // Simulate flush duration
        metrics::histogram!("aspen.write_batcher.flush_duration_ms").record(3.2);
    }

    // Simulate 2 forwarded writes and 1 batcher-skipped write.
    metrics::counter!("aspen.write_batcher.forwarded_total").increment(1);
    metrics::counter!("aspen.write_batcher.forwarded_total").increment(1);
    metrics::counter!("aspen.write_batcher.batcher_skipped_total").increment(1);

    let output = handle.render();

    // Verify flush_total counter = 3
    let flush_total = parse_metric(&output, "aspen_write_batcher_flush_total");
    assert_eq!(flush_total, Some(3.0), "expected 3 flushes.\nOutput:\n{}", output,);

    // Verify batch_size histogram: 3 observations (count), sum = 5+10+15 = 30
    let batch_count = parse_metric(&output, "aspen_write_batcher_batch_size_count");
    assert_eq!(batch_count, Some(3.0), "expected 3 batch_size observations.\nOutput:\n{}", output,);

    let batch_sum = parse_metric(&output, "aspen_write_batcher_batch_size_sum");
    assert_eq!(batch_sum, Some(30.0), "expected batch_size sum = 30.\nOutput:\n{}", output,);

    // Verify flush_duration_ms histogram: 3 observations
    let dur_count = parse_metric(&output, "aspen_write_batcher_flush_duration_ms_count");
    assert_eq!(dur_count, Some(3.0), "expected 3 flush_duration observations.\nOutput:\n{}", output,);

    // Verify forwarded_total counter = 2
    let forwarded = parse_metric(&output, "aspen_write_batcher_forwarded_total");
    assert_eq!(forwarded, Some(2.0), "expected 2 forwarded writes.\nOutput:\n{}", output,);

    // Verify batcher_skipped_total counter = 1
    let skipped = parse_metric(&output, "aspen_write_batcher_batcher_skipped_total");
    assert_eq!(skipped, Some(1.0), "expected 1 batcher-skipped write.\nOutput:\n{}", output,);
}
