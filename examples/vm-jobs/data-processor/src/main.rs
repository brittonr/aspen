//! Data processor worker for VM job execution.
//!
//! This worker demonstrates JSON processing in a guest binary,
//! including deserialization, transformation, and serialization.

use aspen_jobs_guest::*;
use serde_json::json;

/// Process job input and return transformed data.
fn process_data(input: JobInput) -> JobOutput {
    // Log that we're processing
    println("Processing data in VM...");

    // Extract value from payload
    let value = input.payload["value"]
        .as_i64()
        .unwrap_or(0);

    // Perform some computation
    let doubled = value * 2;
    let squared = value * value;

    // Get timestamp from host
    let timestamp = get_timestamp();

    // Create result
    let result = json!({
        "processed": true,
        "timestamp": timestamp,
        "input_value": value,
        "doubled": doubled,
        "squared": squared,
        "metadata": {
            "processor_version": "1.0.0",
            "vm_execution": true
        }
    });

    println(&format!("Computed: {} -> doubled: {}, squared: {}", value, doubled, squared));

    JobOutput::success(result)
}

// Use the JSON handler macro for automatic serialization
define_json_handler!(process_data);

fn main() {
    // Entry point for standard builds
    // The actual work happens through the execute function
}