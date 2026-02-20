//! Example demonstrating observability features in the Aspen client SDK.
//!
//! This example shows how to:
//! - Use distributed tracing with W3C Trace Context
//! - Collect and export metrics
//! - Monitor operation performance
//! - Debug distributed workflows
//!
//! Run with:
//! ```bash
//! cargo run --example observability -- <ticket>
//! ```

use std::time::Duration;

use anyhow::Result;
use aspen_client::AspenClient;
use aspen_client::AspenClientJobExt;
use aspen_client::AspenClientObservabilityExt;
use aspen_client::JobPriority;
use aspen_client::SpanStatus;
use aspen_client::TraceContext;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Observability Example\n");

    // Get cluster ticket from command line
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <cluster_ticket>", args[0]);
        std::process::exit(1);
    }
    let ticket = &args[1];

    // Connect to the cluster
    println!("Connecting to Aspen cluster...");
    let client = AspenClient::connect(ticket, Duration::from_secs(10), None).await?;
    println!("Connected successfully\n");

    // Create observability client
    let observability = client.observability();

    // Example 1: Basic span creation
    println!("=== Example 1: Basic Tracing ===\n");
    {
        let mut span = observability.start_span("example_operation", None).await;
        println!("Started span: {}", span.context.span_id);
        println!("Trace ID: {}", span.context.trace_id);

        // Add attributes
        span.set_attribute("component", "example");
        span.set_attribute("user.id", "user123");

        // Simulate work
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Add an event
        let mut event_attrs = std::collections::HashMap::new();
        event_attrs.insert("key".to_string(), "value".to_string());
        span.add_event("checkpoint_reached", event_attrs);

        // Set status and end
        span.set_status(SpanStatus::Ok);
        observability.end_span(span).await;

        println!("Span completed\n");
    }

    // Example 2: Nested spans
    println!("=== Example 2: Nested Spans ===\n");
    {
        let mut parent_span = observability.start_span("parent_operation", None).await;
        println!("Parent span: {}", parent_span.context.span_id);

        for i in 0..3 {
            let mut child_span =
                observability.start_span(format!("child_operation_{}", i), Some(&parent_span.context)).await;
            println!("  Child span {}: {}", i, child_span.context.span_id);

            // Simulate work
            tokio::time::sleep(Duration::from_millis(50)).await;

            child_span.set_status(SpanStatus::Ok);
            observability.end_span(child_span).await;
        }

        parent_span.set_status(SpanStatus::Ok);
        observability.end_span(parent_span).await;
        println!("Nested spans completed\n");
    }

    // Example 3: Automatic span recording
    println!("=== Example 3: Automatic Span Recording ===\n");
    {
        let trace_ctx = TraceContext::new_root();

        // Successful operation
        let result = observability
            .record_operation("successful_operation", Some(&trace_ctx), async {
                tokio::time::sleep(Duration::from_millis(75)).await;
                Ok::<_, anyhow::Error>("Success!")
            })
            .await?;
        println!("Operation result: {}", result);

        // Failed operation (will record error)
        let _ = observability
            .record_operation("failing_operation", Some(&trace_ctx), async {
                tokio::time::sleep(Duration::from_millis(25)).await;
                Err::<String, _>(anyhow::anyhow!("Simulated error"))
            })
            .await
            .unwrap_or_else(|e| {
                println!("Operation failed (as expected): {}", e);
                String::new()
            });

        println!();
    }

    // Example 4: Metrics collection
    println!("=== Example 4: Metrics Collection ===\n");
    {
        let metrics = observability.metrics();

        // Increment counters
        metrics.increment("requests_total", 1).await;
        metrics.increment("requests_total", 1).await;
        metrics.increment("errors_total", 1).await;

        // Set gauges
        metrics.gauge("active_connections", 42.0).await;
        metrics.gauge("memory_usage_mb", 256.5).await;

        // Record histogram values (e.g., latencies)
        for i in 0..100 {
            let latency = 10.0 + (i as f64 * 0.5) + (rand::random::<f64>() * 5.0);
            metrics.histogram("request_duration_ms", latency).await;
        }

        // Get counter values
        let counters = metrics.get_counters().await;
        println!("Counters:");
        for (name, value) in counters {
            println!("  {}: {}", name, value);
        }

        // Get gauge values
        let gauges = metrics.get_gauges().await;
        println!("\nGauges:");
        for (name, value) in gauges {
            println!("  {}: {:.2}", name, value);
        }

        // Get histogram statistics
        if let Some(stats) = metrics.get_histogram_stats("request_duration_ms").await {
            println!("\nHistogram 'request_duration_ms':");
            println!("  Count: {}", stats.count);
            println!("  Mean: {:.2}ms", stats.mean);
            println!("  P50: {:.2}ms", stats.p50);
            println!("  P95: {:.2}ms", stats.p95);
            println!("  P99: {:.2}ms", stats.p99);
        }

        println!();
    }

    // Example 5: Export metrics in Prometheus format
    println!("=== Example 5: Prometheus Export ===\n");
    {
        let metrics = observability.metrics();

        // Add some labeled metrics
        let labeled_metrics =
            MetricsCollector::new().with_label("service", "aspen_client").with_label("environment", "example");

        labeled_metrics.increment("api_calls", 10).await;
        labeled_metrics.gauge("queue_depth", 5.0).await;

        let prometheus_output = labeled_metrics.export_prometheus().await;
        println!("Prometheus format:");
        println!("{}", prometheus_output);
    }

    // Example 6: Distributed workflow tracing
    println!("=== Example 6: Distributed Workflow Tracing ===\n");
    {
        // Start a trace for a complete workflow
        let workflow_trace = TraceContext::new_root();
        println!("Workflow trace ID: {}", workflow_trace.trace_id);
        println!("Traceparent header: {}", workflow_trace.to_traceparent());

        // Stage 1: Data extraction
        let mut extract_span = observability.start_span("workflow.extract", Some(&workflow_trace)).await;
        extract_span.set_attribute("stage", "1");
        extract_span.set_attribute("data_source", "database");
        tokio::time::sleep(Duration::from_millis(100)).await;
        extract_span.set_status(SpanStatus::Ok);
        observability.end_span(extract_span).await;

        // Stage 2: Data transformation
        let mut transform_span = observability.start_span("workflow.transform", Some(&workflow_trace)).await;
        transform_span.set_attribute("stage", "2");
        transform_span.set_attribute("format", "parquet");
        tokio::time::sleep(Duration::from_millis(150)).await;
        transform_span.set_status(SpanStatus::Ok);
        observability.end_span(transform_span).await;

        // Stage 3: Data loading
        let mut load_span = observability.start_span("workflow.load", Some(&workflow_trace)).await;
        load_span.set_attribute("stage", "3");
        load_span.set_attribute("destination", "warehouse");
        tokio::time::sleep(Duration::from_millis(75)).await;
        load_span.set_status(SpanStatus::Ok);
        observability.end_span(load_span).await;

        println!("Workflow completed with trace\n");
    }

    // Example 7: Performance monitoring
    println!("=== Example 7: Performance Monitoring ===\n");
    {
        let metrics = observability.metrics();

        // Simulate monitoring job submission performance
        println!("Monitoring job submission performance...");

        for i in 0..10 {
            let start = std::time::Instant::now();

            // Simulate job submission (would be real in production)
            let _job_builder = client
                .jobs()
                .submit(format!("perf_test_{}", i), json!({"test": true}))
                .with_priority(JobPriority::Normal);

            let duration = start.elapsed().as_millis() as f64;
            metrics.histogram("job_submission_latency_ms", duration).await;

            // Track success/failure
            if i % 5 == 0 {
                metrics.increment("job_submission_errors", 1).await;
            } else {
                metrics.increment("job_submission_success", 1).await;
            }
        }

        // Calculate success rate
        let counters = metrics.get_counters().await;
        let success = counters.get("job_submission_success").copied().unwrap_or(0);
        let errors = counters.get("job_submission_errors").copied().unwrap_or(0);
        let total = success + errors;
        let success_rate = if total > 0 {
            (success as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        println!("Job submission metrics:");
        println!("  Total: {}", total);
        println!("  Success: {}", success);
        println!("  Errors: {}", errors);
        println!("  Success rate: {:.1}%", success_rate);

        if let Some(stats) = metrics.get_histogram_stats("job_submission_latency_ms").await {
            println!("  P50 latency: {:.2}ms", stats.p50);
            println!("  P99 latency: {:.2}ms", stats.p99);
        }

        println!();
    }

    // Example 8: Export all spans
    println!("=== Example 8: Span Export ===\n");
    {
        let all_spans = observability.export_spans().await;
        println!("Total spans collected: {}", all_spans.len());

        for (i, span) in all_spans.iter().take(5).enumerate() {
            println!("Span {}:", i + 1);
            println!("  Operation: {}", span.operation);
            println!("  Trace ID: {}", span.context.trace_id);
            println!("  Span ID: {}", span.context.span_id);
            if let Some(duration) = span.duration() {
                println!("  Duration: {:.2}ms", duration.as_millis());
            }
            println!("  Status: {:?}", span.status);
        }

        // Clear spans after export
        observability.clear_spans().await;
        println!("\nSpans cleared after export");
    }

    println!("\n=== Summary ===");
    println!("The observability features provide:");
    println!("✓ Distributed tracing with W3C Trace Context");
    println!("✓ Nested spans for complex operations");
    println!("✓ Automatic span creation with error tracking");
    println!("✓ Metrics collection (counters, gauges, histograms)");
    println!("✓ Prometheus export format");
    println!("✓ Performance monitoring and percentiles");
    println!("✓ Distributed workflow tracing");
    println!("\nThese features help debug and monitor distributed systems!");

    Ok(())
}

// Helper to create MetricsCollector (since it's not Clone)
use aspen_client::MetricsCollector;
