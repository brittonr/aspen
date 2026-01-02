//! Example demonstrating job management patterns with the Aspen client SDK.
//!
//! This example shows the API patterns that will be available for:
//! - Submitting jobs with various configurations
//! - Monitoring job status
//! - Managing job workflows
//!
//! NOTE: The job system RPC integration is pending. This demonstrates
//! the SDK patterns that will be available once integrated.
//!
//! Run with:
//! ```bash
//! cargo run --example job_management -- <ticket>
//! ```

use anyhow::Result;
use aspen_client::{AspenClient, AspenClientJobExt, JobPriority};
use serde_json::json;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Job Management Example");

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

    println!("=== Job Management API Patterns ===\n");

    // Example 1: Simple job submission pattern
    println!("Example 1: Simple Job Submission");
    println!("```rust");
    println!("let job_id = client.jobs()");
    println!("    .submit(\"hello_world\", json!({{\"message\": \"Hello!\"}}))");
    println!("    .execute(&client)");
    println!("    .await?;");
    println!("```");

    // Demonstrate the builder (even though it won't execute)
    let builder = client
        .jobs()
        .submit("hello_world", json!({"message": "Hello from Aspen!"}));
    println!("Created job builder: {:?}\n", builder);

    // Example 2: Job with configuration
    println!("Example 2: Configured Job Submission");
    println!("```rust");
    println!("let job_id = client.jobs()");
    println!("    .submit(\"data_processing\", payload)");
    println!("    .with_priority(JobPriority::High)");
    println!("    .with_max_retries(3)");
    println!("    .with_timeout(Duration::from_secs(300))");
    println!("    .add_tag(\"batch\")");
    println!("    .execute(&client)");
    println!("    .await?;");
    println!("```");

    let configured_builder = client
        .jobs()
        .submit("data_processing", json!({"input": "data.csv"}))
        .with_priority(JobPriority::High)
        .with_max_retries(3)
        .with_timeout(Duration::from_secs(300))
        .add_tag("batch");
    println!("Created configured job: {:?}\n", configured_builder);

    // Example 3: Scheduled jobs
    println!("Example 3: Scheduled Job Pattern");
    println!("```rust");
    println!("// Daily backup at 2 AM");
    println!("let job_id = client.jobs()");
    println!("    .submit(\"backup\", payload)");
    println!("    .with_cron_schedule(\"0 2 * * *\")");
    println!("    .execute(&client)");
    println!("    .await?;");
    println!("```\n");

    // Example 4: Job workflows with dependencies
    println!("Example 4: Job Workflow Pattern");
    println!("```rust");
    println!("// ETL pipeline with dependencies");
    println!("let extract = client.jobs()");
    println!("    .submit(\"extract\", json!({{\"source\": \"db\"}}))");
    println!("    .execute(&client).await?;");
    println!("");
    println!("let transform = client.jobs()");
    println!("    .submit(\"transform\", json!({{\"format\": \"parquet\"}}))");
    println!("    .add_dependency(&extract)");
    println!("    .execute(&client).await?;");
    println!("");
    println!("let load = client.jobs()");
    println!("    .submit(\"load\", json!({{\"target\": \"warehouse\"}}))");
    println!("    .add_dependency(&transform)");
    println!("    .execute(&client).await?;");
    println!("```\n");

    // Example 5: Job monitoring patterns
    println!("Example 5: Job Monitoring Patterns");
    println!("```rust");
    println!("// Get job details");
    println!("let job = client.jobs().get(&job_id).await?;");
    println!("");
    println!("// Wait for completion");
    println!("let result = client.jobs()");
    println!("    .wait_for_completion(");
    println!("        job_id,");
    println!("        Duration::from_secs(1),  // poll interval");
    println!("        Some(Duration::from_secs(60))  // timeout");
    println!("    ).await?;");
    println!("");
    println!("// List jobs with filters");
    println!("let jobs = client.jobs()");
    println!("    .list(JobListOptions {{");
    println!("        status: Some(JobStatus::Running),");
    println!("        tags: vec![\"production\".to_string()],");
    println!("        ..Default::default()");
    println!("    }}).await?;");
    println!("```\n");

    // Example 6: Queue statistics
    println!("Example 6: Queue Management");
    println!("```rust");
    println!("// Get queue statistics");
    println!("let stats = client.jobs().get_queue_stats().await?;");
    println!("println!(\"Pending: {{}}\", stats.pending_count);");
    println!("println!(\"Running: {{}}\", stats.running_count);");
    println!("");
    println!("// Get worker status");
    println!("let workers = client.jobs().get_worker_status().await?;");
    println!("for worker in workers {{");
    println!("    println!(\"{{}}: {{}} active\", worker.worker_id, worker.active_jobs);");
    println!("}}");
    println!("```\n");

    println!("=== Current Status ===\n");
    println!("The job management SDK provides a clean async API for:");
    println!("✓ Job submission with builder pattern");
    println!("✓ Priority and retry configuration");
    println!("✓ Scheduling with cron expressions");
    println!("✓ Dependency-based workflows");
    println!("✓ Job monitoring and waiting");
    println!("✓ Queue statistics and worker management");
    println!("");
    println!("Integration Status: The SDK structure is complete.");
    println!("Next step: Wire up the RPC layer when job operations");
    println!("are added to the client-server protocol.\n");

    // The examples above demonstrate the SDK patterns that will be available
    // once the job system RPC integration is complete.
    println!("SDK demonstration complete!");
    println!("\nNote: Actual job execution requires RPC integration.");

    Ok(())
}