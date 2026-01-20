//! CLI for generating Nickel contracts from Rust configuration structs.

use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use aspen_nickel_gen::generate_nickel_contracts;
use aspen_nickel_gen::parse_config_file;
use clap::Parser;

/// Generate Nickel configuration contracts from Rust struct definitions.
#[derive(Parser, Debug)]
#[command(name = "aspen-nickel-gen")]
#[command(version, about, long_about = None)]
struct Args {
    /// Rust source file(s) to parse.
    #[arg(required = true)]
    input: Vec<PathBuf>,

    /// Output file path. If not specified, prints to stdout.
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Only process structs matching this pattern.
    #[arg(short, long)]
    filter: Option<String>,

    /// Verbose output.
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut all_config = aspen_nickel_gen::ParsedConfig::default();

    for input_path in &args.input {
        if args.verbose {
            eprintln!("Parsing: {}", input_path.display());
        }

        let config =
            parse_config_file(input_path).with_context(|| format!("Failed to parse {}", input_path.display()))?;

        if args.verbose {
            eprintln!("  Found {} structs, {} enums", config.structs.len(), config.enums.len());
        }

        // Merge into all_config
        for s in config.structs {
            if let Some(ref filter) = args.filter {
                if !s.name.contains(filter) {
                    continue;
                }
            }
            all_config.structs.push(s);
        }
        for e in config.enums {
            if let Some(ref filter) = args.filter {
                if !e.name.contains(filter) {
                    continue;
                }
            }
            all_config.enums.push(e);
        }
    }

    if all_config.structs.is_empty() && all_config.enums.is_empty() {
        anyhow::bail!("No configuration structs or enums found in input files");
    }

    let nickel_output = generate_nickel_contracts(&all_config);

    if let Some(output_path) = args.output {
        std::fs::write(&output_path, &nickel_output)
            .with_context(|| format!("Failed to write {}", output_path.display()))?;

        if args.verbose {
            eprintln!("Wrote {} bytes to {}", nickel_output.len(), output_path.display());
        }
    } else {
        print!("{nickel_output}");
    }

    Ok(())
}
