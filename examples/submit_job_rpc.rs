//! Example of submitting a job via RPC to an Aspen cluster.

use anyhow::Result;
use aspen_client::AspenClient;
use aspen_client_rpc::{ClientRpcRequest, ClientRpcResponse};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Connecting to local Aspen node...");
    
    // Connect to local node (assuming it's running on default port 7701)
    let client = AspenClient::connect("http://127.0.0.1:7701").await?;
    
    println!("Submitting test job...");
    
    // Submit a test job
    let request = ClientRpcRequest::JobSubmit {
        job_type: "test_job".to_string(),
        payload: json!({
            "message": "Hello from RPC test",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }),
        priority: Some(1),
        timeout_ms: Some(60000),
        max_retries: Some(3),
        retry_delay_ms: Some(1000),
        schedule: None,
        tags: vec!["test".to_string()],
    };
    
    match client.send(request).await {
        Ok(ClientRpcResponse::JobSubmitResult(result)) => {
            if result.success {
                println!("✓ Job submitted successfully!");
                println!("  Job ID: {}", result.job_id.unwrap_or_else(|| "unknown".to_string()));
            } else {
                println!("✗ Job submission failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()));
            }
        }
        Ok(other) => {
            println!("✗ Unexpected response: {:?}", other);
        }
        Err(e) => {
            println!("✗ Error submitting job: {}", e);
        }
    }
    
    // Try to get queue stats
    println!("\nGetting queue statistics...");
    let stats_request = ClientRpcRequest::JobQueueStats;
    
    match client.send(stats_request).await {
        Ok(ClientRpcResponse::JobQueueStatsResult(stats)) => {
            println!("✓ Queue statistics:");
            println!("  Pending: {}", stats.pending_count);
            println!("  Running: {}", stats.running_count);
            println!("  Completed: {}", stats.completed_count);
            println!("  Failed: {}", stats.failed_count);
        }
        Ok(other) => {
            println!("✗ Unexpected response: {:?}", other);
        }
        Err(e) => {
            println!("✗ Error getting stats: {}", e);
        }
    }
    
    Ok(())
}
