//! Simple example to test job submission to Aspen cluster.
//!
//! Usage:
//! ```bash
//! cargo run --example submit_job -- <cluster_ticket>
//! ```

use anyhow::Result;
use aspen_client::{AspenClient, AspenClientJobExt, JobPriority};
use serde_json::json;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

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
    println!("Connected successfully!");

    // Submit a simple test job
    println!("\nSubmitting test job...");
    let job_id = client
        .jobs()
        .submit(
            "test_job",
            json!({
                "message": "Hello from Aspen job system!",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "test_data": {
                    "value": 42,
                    "items": ["a", "b", "c"]
                }
            }),
        )
        .with_priority(JobPriority::Normal)
        .with_max_retries(3)
        .with_timeout(Duration::from_secs(60))
        .add_tag("test")
        .add_tag("example")
        .execute(&client)
        .await?;

    println!("Job submitted successfully!");
    println!("Job ID: {}", job_id);

    // Check job status
    println!("\nChecking job status...");
    if let Some(job) = client.jobs().get(&job_id).await? {
        println!("Job Status: {}", job.status);
        println!("Job Type: {}", job.job_type);
        println!("Priority: {}", job.priority);
        println!("Tags: {:?}", job.tags);
        println!("Progress: {}%", job.progress);
    }

    // Get queue statistics
    println!("\nQueue Statistics:");
    let stats = client.jobs().get_queue_stats().await?;
    println!("  Pending: {}", stats.pending_count);
    println!("  Running: {}", stats.running_count);
    println!("  Completed: {}", stats.completed_count);
    println!("  Failed: {}", stats.failed_count);

    // Wait a bit for job to process
    println!("\nWaiting for job to complete (max 10 seconds)...");
    match tokio::time::timeout(
        Duration::from_secs(10),
        client.jobs().wait_for_completion(&job_id, Duration::from_secs(1), Some(Duration::from_secs(10))),
    )
    .await
    {
        Ok(Ok(completed_job)) => {
            println!("Job completed!");
            println!("Final status: {}", completed_job.status);
            if let Some(result) = completed_job.result {
                println!("Result: {}", serde_json::to_string_pretty(&result)?);
            }
        }
        Ok(Err(e)) => {
            println!("Error waiting for job: {}", e);
        }
        Err(_) => {
            println!("Timeout waiting for job completion");

            // Check final status
            if let Some(job) = client.jobs().get(&job_id).await? {
                println!("Current status: {}", job.status);
            }
        }
    }

    println!("\nDone!");
    Ok(())
}
