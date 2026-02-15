//! Job monitoring and observability system.
//!
//! This module provides comprehensive monitoring, tracing, and metrics collection
//! for distributed job execution across the cluster.
//!
//! ## Features
//!
//! - Distributed tracing with OpenTelemetry compatibility
//! - Metrics aggregation with Prometheus export
//! - Job execution history and audit logs
//! - Performance profiling and bottleneck detection
//! - Real-time monitoring dashboards
//! - Alerting and anomaly detection
//!
//! ## Tiger Style
//!
//! - Bounded metrics storage (MAX_METRICS_PER_NODE = 10000)
//! - Fixed retention periods for historical data
//! - Fail-fast on monitoring errors (don't block job execution)
//! - Efficient sampling for high-throughput scenarios

mod audit;
mod metrics;
mod profiling;
mod service;
mod trace;

pub use audit::AuditAction;
pub use audit::AuditFilter;
pub use audit::AuditLogEntry;
pub use audit::AuditResult;
pub use metrics::AggregatedMetrics;
pub use metrics::JobMetrics;
pub use profiling::Bottleneck;
pub use profiling::BottleneckType;
pub use profiling::JobProfile;
pub use service::JobMonitoringService;
pub use trace::SpanEvent;
pub use trace::SpanLink;
pub use trace::SpanStatus;
pub use trace::TraceContext;
pub use trace::TraceFilter;
pub use trace::TraceSpan;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use chrono::Utc;

    use super::*;

    #[test]
    fn test_trace_context() {
        let root = TraceContext::new_root();
        assert!(root.parent_span_id.is_none());

        let child = root.child();
        assert_eq!(child.trace_id, root.trace_id);
        assert_eq!(child.parent_span_id, Some(root.span_id));
    }

    #[tokio::test]
    async fn test_monitoring_service() {
        let store = Arc::new(aspen_testing::DeterministicKeyValueStore::new());
        let service = JobMonitoringService::new(store);

        // Start a span
        let trace_ctx = TraceContext::new_root();
        let span_id = service.start_span(&trace_ctx, "test_op", "test_service").await;

        // Add event
        let event = SpanEvent {
            name: "checkpoint".to_string(),
            timestamp: Utc::now(),
            attributes: HashMap::new(),
        };
        service.add_span_event(&span_id, event).await.unwrap();

        // End span
        service.end_span(&span_id, SpanStatus::Ok).await.unwrap();

        // Query traces
        let traces = service.query_traces(TraceFilter::default()).await.unwrap();
        assert_eq!(traces.len(), 1);
    }
}
