//! Example demonstrating distributed coordination primitives with the Aspen client SDK.
//!
//! NOTE: This example demonstrates the patterns that will be available once the
//! coordination module is fully integrated with the AspenClient. Currently, the
//! coordination primitives require the CoordinationRpc trait implementation.
//!
//! Run with:
//! ```bash
//! cargo run --example coordination -- <ticket>
//! ```

use anyhow::Result;
use aspen_client::AspenClient;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging (would use tracing_subscriber in production)
    println!("Coordination Example");

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
    println!("Connected successfully");

    // NOTE: The following examples demonstrate the patterns that will be available
    // once the coordination module is fully integrated with AspenClient.
    // Currently, these would require implementing the CoordinationRpc trait.

    println!("\n=== Coordination Patterns (To Be Integrated) ===\n");

    println!("Example 1: Distributed Lock Pattern");
    println!("  - Would use: LockClient::new(client, \"resource_lock\")");
    println!("  - Acquire lock with timeout");
    println!("  - Perform critical section work");
    println!("  - Auto-release on guard drop");

    println!("\nExample 2: Semaphore Pattern");
    println!("  - Would use: SemaphoreClient::new(client, \"worker_pool\", 3)");
    println!("  - Acquire multiple permits");
    println!("  - Control concurrent access to resources");
    println!("  - Release permits when done");

    println!("\nExample 3: Barrier Pattern");
    println!("  - Would use: BarrierClient::new(client, \"sync_point\", 3)");
    println!("  - Synchronize multiple workers");
    println!("  - All workers wait until threshold reached");
    println!("  - Continue together after synchronization");

    println!("\nExample 4: Rate Limiter Pattern");
    println!("  - Would use: RateLimiterClient::new(client, \"api_calls\", 10, Duration::from_secs(60))");
    println!("  - Enforce rate limits (e.g., 10 requests per minute)");
    println!("  - Get remaining quota");
    println!("  - Handle retry-after for blocked requests");

    println!("\nExample 5: Service Discovery Pattern");
    println!("  - Would use: ServiceClient::new(client, \"web_api\")");
    println!("  - Register service instances with metadata");
    println!("  - Discover healthy instances");
    println!("  - Automatic health checks and heartbeats");

    println!("\nExample 6: Lease Management Pattern");
    println!("  - Would use: LeaseClient::new(client)");
    println!("  - Grant time-limited leases");
    println!("  - Automatic keepalive for active leases");
    println!("  - Revoke leases when done");

    println!("\nExample 7: Distributed Counter Pattern");
    println!("  - Would use: CounterClient::new(client, \"page_views\")");
    println!("  - Atomic increment/decrement");
    println!("  - Get current value");
    println!("  - Reset counter");

    println!("\nExample 8: Queue Operations Pattern");
    println!("  - Would use: QueueClient::new(client, \"task_queue\")");
    println!("  - Enqueue/dequeue with visibility timeout");
    println!("  - Dead letter queue support");
    println!("  - Priority queues and deduplication");

    println!("\n=== Current Status ===\n");

    // The coordination primitives are currently available but require
    // implementing the CoordinationRpc trait. The next phase will provide
    // direct integration with AspenClient for easier usage.

    println!("\n=== Summary ===");
    println!("The coordination module provides powerful distributed primitives:");
    println!("- Locks, semaphores, barriers for synchronization");
    println!("- Rate limiters for resource management");
    println!("- Service discovery with health checks");
    println!("- Leases for time-limited access");
    println!("- Counters and queues for distributed state");
    println!("\nThese will be fully integrated with AspenClient in the next phase.");

    Ok(())
}