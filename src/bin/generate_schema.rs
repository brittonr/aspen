//! JSON Schema generator for Aspen configuration types.
//!
//! This binary generates JSON Schema files for Aspen configuration types.
//! These schemas can be converted to Nickel contracts using `json-schema-to-nickel`.
//!
//! # Usage
//!
//! ```bash
//! # Generate JSON schemas
//! cargo run --bin aspen-generate-schema -- --output-dir ./schemas
//!
//! # Convert to Nickel contracts (requires json-schema-to-nickel)
//! nix run github:nickel-lang/json-schema-to-nickel -- ./schemas/node_config.json > contracts/node_config.ncl
//! ```

use std::fs;
use std::path::PathBuf;

use schemars::schema_for;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    let output_dir = if args.len() > 2 && args[1] == "--output-dir" {
        PathBuf::from(&args[2])
    } else {
        PathBuf::from("./schemas")
    };

    fs::create_dir_all(&output_dir)?;

    // Generate NodeConfig schema (top-level cluster configuration)
    let node_config_schema = schema_for!(aspen_cluster::config::NodeConfig);
    let node_config_json = serde_json::to_string_pretty(&node_config_schema)?;
    fs::write(output_dir.join("node_config.json"), node_config_json)?;
    println!("Generated: node_config.json");

    // Generate PipelineConfig schema (CI/CD pipeline configuration)
    // PipelineConfig is re-exported at the crate root
    let pipeline_config_schema = schema_for!(aspen_ci::PipelineConfig);
    let pipeline_config_json = serde_json::to_string_pretty(&pipeline_config_schema)?;
    fs::write(output_dir.join("pipeline_config.json"), pipeline_config_json)?;
    println!("Generated: pipeline_config.json");

    println!("\nSchemas written to: {}", output_dir.display());
    println!("\nTo convert to Nickel contracts:");
    println!(
        "  nix run github:nickel-lang/json-schema-to-nickel -- {} > contracts/node_config.ncl",
        output_dir.join("node_config.json").display()
    );

    Ok(())
}
