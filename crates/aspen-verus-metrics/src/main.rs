//! Verus Metrics CLI - Verify sync between production and Verus code.
//!
//! Usage:
//!   verus-metrics check [OPTIONS]
//!   verus-metrics coverage [OPTIONS]

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Context;
use anyhow::Result;
use aspen_verus_metrics::DEFAULT_VERIFIED_CRATES;
use aspen_verus_metrics::VerificationEngine;
use aspen_verus_metrics::output::OutputFormat;
use aspen_verus_metrics::output::render;
use clap::Parser;
use clap::Subcommand;

#[derive(Parser)]
#[command(name = "verus-metrics")]
#[command(about = "Verify sync between production verified functions and Verus specs")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check for drift between production and Verus code
    Check {
        /// Show verbose output with detailed comparisons
        #[arg(short, long)]
        verbose: bool,

        /// Check only the specified crate
        #[arg(short, long)]
        crate_name: Option<String>,

        /// Output format (terminal, json)
        #[arg(short = 'f', long, default_value = "terminal")]
        format: String,

        /// Output file path (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Exit with error on any drift (for CI)
        #[arg(long)]
        strict: bool,

        /// Root directory of the project
        #[arg(long)]
        root: Option<PathBuf>,
    },

    /// Show coverage metrics
    Coverage {
        /// Check only the specified crate
        #[arg(short, long)]
        crate_name: Option<String>,

        /// Output format (terminal, json)
        #[arg(short = 'f', long, default_value = "terminal")]
        format: String,

        /// Root directory of the project
        #[arg(long)]
        root: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    match run() {
        Ok(success) => {
            if success {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(e) => {
            eprintln!("Error: {:#}", e);
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<bool> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check {
            verbose,
            crate_name,
            format,
            output,
            strict,
            root,
        } => {
            let root_dir = find_root_dir(root)?;
            let crates = get_crate_list(crate_name);
            let output_format: OutputFormat = format.parse().map_err(|e: String| anyhow::anyhow!(e))?;

            let engine = VerificationEngine::new(root_dir, crates, verbose);
            let report = engine.verify()?;

            let output_str = render(&report, output_format, verbose);

            if let Some(output_path) = output {
                fs::write(&output_path, &output_str).context("Failed to write output file")?;
                if verbose {
                    eprintln!("Report written to: {}", output_path.display());
                }
            } else {
                print!("{}", output_str);
            }

            if strict && report.has_drift() {
                Ok(false)
            } else {
                Ok(true)
            }
        }

        Commands::Coverage {
            crate_name,
            format,
            root,
        } => {
            let root_dir = find_root_dir(root)?;
            let crates = get_crate_list(crate_name);
            let output_format: OutputFormat = format.parse().map_err(|e: String| anyhow::anyhow!(e))?;

            let engine = VerificationEngine::new(root_dir, crates, false);
            let report = engine.verify()?;

            // For coverage, just show summary stats
            match output_format {
                OutputFormat::Terminal => {
                    println!("=== Verus Coverage Report ===\n");
                    for crate_report in &report.crates {
                        println!(
                            "{}: {:.1}% ({} matches, {} drifts, {} missing)",
                            crate_report.name,
                            crate_report.coverage_percent(),
                            crate_report.matches,
                            crate_report.drifts,
                            crate_report.missing_production
                        );
                    }
                    println!();
                    println!("Overall: {:.1}%", report.summary.coverage_percent);
                }
                OutputFormat::Json => {
                    let json = serde_json::json!({
                        "crates": report.crates.iter().map(|c| {
                            serde_json::json!({
                                "name": c.name,
                                "coverage_percent": c.coverage_percent(),
                                "matches": c.matches,
                                "drifts": c.drifts,
                                "missing": c.missing_production,
                            })
                        }).collect::<Vec<_>>(),
                        "overall_coverage_percent": report.summary.coverage_percent,
                    });
                    println!("{}", serde_json::to_string_pretty(&json).unwrap());
                }
            }

            Ok(true)
        }
    }
}

/// Find the project root directory.
fn find_root_dir(explicit: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(root) = explicit {
        return Ok(root);
    }

    // Try to find root by looking for Cargo.toml with workspace
    let mut current = std::env::current_dir()?;

    loop {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = fs::read_to_string(&cargo_toml)?;
            if content.contains("[workspace]") {
                return Ok(current);
            }
        }

        if !current.pop() {
            break;
        }
    }

    // Fallback to current directory
    Ok(std::env::current_dir()?)
}

/// Get the list of crates to check.
fn get_crate_list(explicit: Option<String>) -> Vec<String> {
    if let Some(name) = explicit {
        vec![name]
    } else {
        DEFAULT_VERIFIED_CRATES.iter().map(|s| s.to_string()).collect()
    }
}
