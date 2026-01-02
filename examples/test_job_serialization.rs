//! Test to verify how VM job payloads are serialized
//!
//! This demonstrates the proper way to handle native binaries in the job system:
//! - Binaries are automatically base64-encoded in the JSON payload
//! - Job type should be "vm_execute"
//! - The payload follows the JobPayload enum structure

use aspen_jobs::vm_executor::JobPayload;
use aspen_jobs::JobSpec;

fn main() -> anyhow::Result<()> {
    println!("=== Testing VM Job Payload Serialization ===\n");

    // Create a small test binary
    let test_binary = vec![0x7f, 0x45, 0x4c, 0x46]; // ELF magic number
    println!("Test binary: {:?} ({} bytes)", test_binary, test_binary.len());

    // Method 1: Using JobPayload directly (to understand the structure)
    println!("\n1. JobPayload Structure:");
    let payload = JobPayload::native_binary(test_binary.clone());
    let json_value = serde_json::to_value(&payload)?;
    let json_str = serde_json::to_string_pretty(&json_value)?;
    println!("Serialized JobPayload:\n{}", json_str);

    // Method 2: Using JobSpec::with_native_binary (the recommended way)
    println!("\n2. JobSpec with native binary:");
    let job_spec = JobSpec::with_native_binary(test_binary.clone());
    println!("Job type: {}", job_spec.job_type);
    println!("Payload: {}", serde_json::to_string_pretty(&job_spec.payload)?);

    // Method 3: Manual construction (for RPC submission)
    println!("\n3. Manual construction for RPC:");
    let rpc_payload = serde_json::json!({
        "NativeBinary": {
            "binary": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &test_binary)
        }
    });
    println!("RPC payload:\n{}", serde_json::to_string_pretty(&rpc_payload)?);

    // Verify deserialization works
    println!("\n4. Deserialization test:");
    let deserialized: JobPayload = serde_json::from_value(rpc_payload.clone())?;
    match deserialized {
        JobPayload::NativeBinary { binary } => {
            println!("✓ Successfully deserialized binary: {:?}", binary);
            assert_eq!(binary, test_binary);
        }
        _ => {
            println!("✗ Unexpected payload type");
        }
    }

    println!("\n=== Summary ===");
    println!("The proper way to submit VM jobs through RPC:");
    println!("1. Set job_type to 'vm_execute'");
    println!("2. Create payload as JSON with structure:");
    println!("   {{\"NativeBinary\": {{\"binary\": \"<base64_encoded_binary>\"}}}}");
    println!("3. Submit through ClientRpcRequest::JobSubmit");
    println!("\nNo RPC protocol extension needed - the system already handles binaries properly!");

    Ok(())
}