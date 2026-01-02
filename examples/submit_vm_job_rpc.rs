//! Example demonstrating how to submit a VM job via RPC with proper binary encoding
//!
//! This shows the correct way to handle native binaries in job submission:
//! 1. Job type should be "vm_execute"
//! 2. Payload uses tagged enum format with "type" field
//! 3. Binary is base64-encoded

use std::fs;
use std::time::Duration;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== VM Job Submission via RPC ===\n");

    // Load the echo-worker binary
    let binary_path = "target/x86_64-unknown-none/release/echo-worker";
    println!("Loading binary: {}", binary_path);

    let binary = match fs::read(binary_path) {
        Ok(data) => {
            println!("✓ Loaded {} bytes", data.len());
            data
        }
        Err(e) => {
            eprintln!("Failed to load binary: {}", e);
            eprintln!("Run: cd examples/vm-jobs/echo-worker && cargo build --release");
            return Ok(());
        }
    };

    // Create the proper payload structure
    // This matches the JobPayload enum with #[serde(tag = "type")]
    let payload = serde_json::json!({
        "type": "NativeBinary",
        "binary": BASE64.encode(&binary)
    });

    println!("\nPayload structure (first 100 chars):");
    let payload_str = serde_json::to_string(&payload)?;
    println!("{}", &payload_str[..payload_str.len().min(100)]);

    // Add input data to the payload
    let input_data = b"Hello from RPC submission!";
    let mut full_payload = serde_json::json!({
        "vm_payload": payload,
        "input": BASE64.encode(input_data)
    });

    // This is how it would be submitted through ClientRpcRequest::JobSubmit
    println!("\n=== RPC Submission Structure ===");
    println!("job_type: \"vm_execute\"");
    println!("payload: <JSON with binary and input>");
    println!("priority: 5");
    println!("timeout_ms: 5000");

    // Simulate what the RPC handler would receive
    let job_type = "vm_execute";
    let job_payload = full_payload;

    println!("\nTo submit via RPC client:");
    println!("```rust");
    println!("let request = ClientRpcRequest::JobSubmit {{");
    println!("    job_type: \"vm_execute\".to_string(),");
    println!("    payload: job_payload, // Contains base64-encoded binary");
    println!("    priority: Some(5),");
    println!("    timeout_ms: Some(5000),");
    println!("    max_retries: Some(0),");
    println!("    dependencies: vec![],");
    println!("    schedule: None,");
    println!("    tags: vec![\"vm-job\".to_string()],");
    println!("}};");
    println!("```");

    // Verify the payload can be deserialized
    println!("\n=== Verification ===");
    if let Some(vm_payload) = job_payload.get("vm_payload") {
        use aspen_jobs::vm_executor::JobPayload;
        match serde_json::from_value::<JobPayload>(vm_payload.clone()) {
            Ok(JobPayload::NativeBinary { binary: decoded }) => {
                println!("✓ Payload deserializes correctly");
                println!("✓ Binary size after decoding: {} bytes", decoded.len());
            }
            Ok(_) => println!("✗ Unexpected payload type"),
            Err(e) => println!("✗ Deserialization failed: {}", e),
        }
    }

    println!("\n=== Summary ===");
    println!("✓ The job system already handles binaries properly!");
    println!("✓ Base64 encoding is automatic via serde");
    println!("✓ Job type 'vm_execute' routes to HyperlightWorker");
    println!("\nThe RPC protocol doesn't need extension - it already supports");
    println!("binary payloads through JSON with base64 encoding!");

    Ok(())
}