//! Standalone test for VM executor with echo-worker binary.
//!
//! Run with: cargo run --example test_vm_standalone --features vm-executor

use std::fs;
use std::sync::Arc;
use std::time::Duration;

use aspen_blob::BlobWrite;
use aspen_blob::InMemoryBlobStore;
use aspen_jobs::HyperlightWorker;
use aspen_jobs::Job;
use aspen_jobs::JobSpec;
use aspen_jobs::Worker;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    println!("\n=== Standalone VM Executor Test ===\n");

    // Check if we're on Linux (required for Hyperlight)
    if !cfg!(target_os = "linux") {
        eprintln!("This test requires Linux with KVM support");
        return Ok(());
    }

    // Load the echo-worker binary
    let binary_path = "target/x86_64-unknown-none/release/echo-worker";
    println!("Loading binary from: {}", binary_path);

    let binary = match fs::read(binary_path) {
        Ok(data) => {
            println!("✓ Loaded binary: {} bytes", data.len());
            data
        }
        Err(e) => {
            eprintln!("✗ Failed to load binary: {}", e);
            eprintln!("  Please run: cd examples/vm-jobs/echo-worker && cargo build --release");
            return Ok(());
        }
    };

    // Verify the binary has the required symbols
    println!("\nChecking binary symbols...");
    let output = std::process::Command::new("nm").arg(binary_path).output()?;

    let symbols = String::from_utf8_lossy(&output.stdout);
    let required_symbols = ["execute", "get_result_len", "_start"];
    for symbol in &required_symbols {
        if symbols.contains(symbol) {
            println!("  ✓ Found symbol: {}", symbol);
        } else {
            println!("  ✗ Missing symbol: {}", symbol);
        }
    }

    // Create blob store for the worker
    println!("\nCreating blob store...");
    let blob_store = Arc::new(InMemoryBlobStore::new());
    println!("✓ Created in-memory blob store");

    // Upload binary to blob store
    println!("\nUploading binary to blob store...");
    let add_result = blob_store.add_bytes(&binary).await?;
    let blob_hash = add_result.blob_ref.hash.to_string();
    let blob_size = add_result.blob_ref.size_bytes;
    println!("✓ Binary uploaded to blob store");
    println!("  Hash: {}", blob_hash);
    println!("  Size: {} bytes", blob_size);
    println!("  Was new: {}", add_result.was_new);

    // Create HyperlightWorker with blob store
    println!("\nCreating HyperlightWorker...");
    let worker = match HyperlightWorker::new(blob_store.clone()) {
        Ok(w) => {
            println!("✓ Created HyperlightWorker with blob store");
            println!("  Supported job types: {:?}", w.job_types());
            w
        }
        Err(e) => {
            eprintln!("✗ Failed to create HyperlightWorker: {}", e);
            eprintln!("  This requires KVM support. Check:");
            eprintln!("  - ls -l /dev/kvm");
            eprintln!("  - lsmod | grep kvm");
            return Ok(());
        }
    };

    // Create a job with the blob-stored binary
    println!("\nCreating job with blob-stored binary...");
    let mut spec = JobSpec::with_blob_binary(blob_hash, blob_size, "elf")
        .timeout(Duration::from_secs(5))
        .tag("test-echo");

    // Add input data to the payload
    let input = b"Hello from standalone test!";
    spec.payload["input"] = serde_json::json!(std::str::from_utf8(input)?);
    println!("  Input: {:?}", std::str::from_utf8(input)?);

    let job = Job::from_spec(spec);

    // Execute the job
    println!("\nExecuting job in VM...");
    let start = std::time::Instant::now();
    let result = worker.execute(job).await;
    let elapsed = start.elapsed();

    // Display results
    println!("\n=== Execution Results ===");
    println!("Execution time: {:?}", elapsed);

    match result {
        aspen_jobs::JobResult::Success(output) => {
            println!("Status: SUCCESS");
            println!("Output: {:?}", output.data);

            // Try to extract string output
            if let Some(raw_output) = output.data.get("raw_output") {
                if let Some(output_str) = raw_output.as_str() {
                    println!("Raw output: {}", output_str);
                    if output_str.contains("Hello from standalone test!") {
                        println!("\n✓ Echo worker correctly processed the input!");
                    } else {
                        println!("\n⚠ Output doesn't match expected echo format");
                    }
                }
            } else if let Some(output_str) = output.data.as_str() {
                println!("String output: {}", output_str);
                if output_str.contains("Hello from standalone test!") {
                    println!("\n✓ Echo worker correctly processed the input!");
                } else {
                    println!("\n⚠ Output doesn't match expected echo format");
                }
            }
        }
        aspen_jobs::JobResult::Failure(f) => {
            println!("Status: FAILED");
            println!("Reason: {}", f.reason);
            if f.reason.contains("KVM") || f.reason.contains("virtualization") {
                println!("\nHint: Make sure KVM is enabled:");
                println!("  sudo modprobe kvm");
                println!("  sudo modprobe kvm_intel  # or kvm_amd");
                println!("  sudo chmod 666 /dev/kvm");
            }
        }
        aspen_jobs::JobResult::Cancelled => {
            println!("Status: CANCELLED");
        }
    }

    println!("\n=== Test Complete ===\n");
    Ok(())
}
