# VM-Based Job Execution with Hyperlight

This directory contains examples of guest binaries that can be executed in Hyperlight micro-VMs by the Aspen job system.

## Overview

Hyperlight provides lightweight virtual machine isolation for executing untrusted code. Each job runs in its own micro-VM with:
- 1-2ms startup time
- ~5MB memory overhead
- No kernel or OS inside the VM
- Restricted host API access

## Guest Binary Examples

### echo-worker
A minimal example that echoes input back with a prefix.
- Demonstrates basic input/output
- Uses `no_std` for minimal binary size
- Shows host function calls (logging)

### data-processor
A more complex example with JSON processing.
- Deserializes JSON input
- Performs data transformation
- Uses host-provided timestamp
- Returns structured JSON output

## Building Guest Binaries

Guest binaries need to be compiled for the target architecture with specific flags:

```bash
# Build a single worker
./scripts/build-guest.sh echo-worker

# Build all workers
for worker in echo-worker data-processor; do
    ./scripts/build-guest.sh $worker
done
```

The built binaries will be placed in `target/guest-binaries/`.

### Build Requirements

- Rust toolchain with `x86_64-unknown-linux-musl` target (for static linking)
- Or standard `x86_64-unknown-linux-gnu` target (with dynamic linking)

Install musl target:
```bash
rustup target add x86_64-unknown-linux-musl
```

## Security Model

### Guest Isolation
- Each VM runs in hardware-enforced isolation (KVM on Linux, Hyper-V on Windows)
- No access to host filesystem
- No network access by default
- Limited to configured memory (default: 1MB)
- Execution timeout enforced

### Host API Access
Guests can only call registered host functions:
- `hl_println`: Log messages to host
- `hl_get_time`: Get current Unix timestamp

Additional functions can be registered but should be carefully audited.

## Writing Guest Binaries

### Basic Structure

```rust
use aspen_jobs_guest::*;

fn my_handler(input: &[u8]) -> Vec<u8> {
    // Process input
    let result = process(input);
    // Return output
    result
}

// Define the entry point
define_job_handler!(my_handler);
```

### With JSON Support

```rust
use aspen_jobs_guest::*;
use serde_json::json;

fn process_json(input: JobInput) -> JobOutput {
    // Access input payload
    let value = input.payload["key"].as_str().unwrap_or("default");

    // Return success with data
    JobOutput::success(json!({
        "processed": true,
        "result": value.to_uppercase()
    }))
}

// Automatic JSON serialization/deserialization
define_json_handler!(process_json);
```

## Performance Characteristics

### Startup Times
- Cold start: 1-2ms (creating new VM)
- Warm start: <1ms (reusing VM)
- Nix build + execute: 5-10s first time, <2ms cached

### Memory Usage
- Per VM overhead: ~5MB
- Guest memory limit: 1MB (configurable)
- Binary size: 50-500KB typical

### Execution Overhead
- Host function calls: ~10μs
- Context switches: ~1μs
- Overall overhead vs native: ~10-20%

## Troubleshooting

### KVM Not Available
```
Error: KVM not available
```
Solution: Ensure KVM is enabled in BIOS and the kvm kernel modules are loaded:
```bash
sudo modprobe kvm
sudo modprobe kvm_intel  # or kvm_amd
```

### Binary Too Large
```
Error: Binary too large: X bytes (max: 52428800 bytes)
```
Solution: Optimize binary size:
- Use `opt-level = "z"` in Cargo.toml
- Enable LTO: `lto = true`
- Strip symbols: `strip = true`
- Use `no_std` if possible

### Guest Execution Failed
Common causes:
- Binary not properly linked (use musl for static linking)
- Missing execute function export
- Panic in guest code
- Timeout exceeded

## Integration with Job System

### Submitting VM Jobs

```rust
use aspen_jobs::JobSpec;

// Use pre-built binary
let binary = std::fs::read("target/guest-binaries/echo-worker")?;
let job = JobSpec::with_native_binary(binary)
    .payload(json!({"message": "Hello"}))
    .timeout(Duration::from_secs(1));

// Or build with Nix
let job = JobSpec::with_nix_flake("github:myorg/workers", "echo")
    .payload(json!({"message": "Hello"}));

// Submit to job queue
let job_id = job_manager.submit(job).await?;
```

### Worker Pool Configuration

The job system automatically routes jobs requiring isolation to the HyperlightWorker:

```rust
let pool = WorkerPool::new(store)
    .register_handler("vm_execute", HyperlightWorker::new()?)
    .start(4).await?;
```

Jobs tagged with `requires_isolation` or using `JobSpec::with_isolation(true)` will be executed in VMs.

## Examples

Run the complete demo:
```bash
# Build guest binaries
./scripts/build-guest.sh echo-worker
./scripts/build-guest.sh data-processor

# Run the demo
cargo run --example vm_job_demo --features vm-executor
```

This will demonstrate:
1. Echo worker execution
2. Data processing with JSON
3. Nix build and execute (if Nix is available)