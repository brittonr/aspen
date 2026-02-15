//! Verus Metrics CLI - Verify sync between production and Verus code.
//!
//! Usage:
//!   verus-metrics check [OPTIONS]
//!   verus-metrics coverage [OPTIONS]
//!   verus-metrics watch [OPTIONS]

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Context;
use anyhow::Result;
use aspen_verus_metrics::DEFAULT_VERIFIED_CRATES;
use aspen_verus_metrics::Severity;
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

        /// Output format (terminal, json, github, markdown)
        #[arg(short = 'f', long, default_value = "terminal")]
        format: String,

        /// Output file path (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Exit with error on any drift (for CI) - equivalent to --fail-on error
        #[arg(long)]
        strict: bool,

        /// Exit with error if issues at or above this severity level are found
        #[arg(long, value_name = "LEVEL")]
        fail_on: Option<String>,

        /// Only show issues at or above this severity level
        #[arg(long, value_name = "LEVEL", default_value = "info")]
        min_severity: String,

        /// Root directory of the project
        #[arg(long)]
        root: Option<PathBuf>,
    },

    /// Show coverage metrics
    Coverage {
        /// Check only the specified crate
        #[arg(short, long)]
        crate_name: Option<String>,

        /// Output format (terminal, json, github, markdown)
        #[arg(short = 'f', long, default_value = "terminal")]
        format: String,

        /// Root directory of the project
        #[arg(long)]
        root: Option<PathBuf>,
    },

    /// Watch for file changes and continuously validate
    Watch {
        /// Debounce time in milliseconds
        #[arg(long, default_value = "500")]
        debounce_ms: u64,

        /// Clear terminal between runs
        #[arg(long, default_value = "true")]
        clear: bool,

        /// Check only the specified crate
        #[arg(short, long)]
        crate_name: Option<String>,

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
            fail_on,
            min_severity,
            root,
        } => {
            let root_dir = find_root_dir(root)?;
            let crates = get_crate_list(crate_name);
            let output_format: OutputFormat = format.parse().map_err(|e: String| anyhow::anyhow!(e))?;

            // Parse severity levels
            let min_sev: Severity = min_severity.parse().map_err(|e: String| anyhow::anyhow!(e))?;
            let fail_sev: Option<Severity> = if strict {
                Some(Severity::Error)
            } else {
                fail_on.map(|s| s.parse()).transpose().map_err(|e: String| anyhow::anyhow!(e))?
            };

            let engine = VerificationEngine::new(root_dir, crates, verbose);
            let report = engine.verify()?;

            let output_str = render(&report, output_format, verbose, min_sev);

            if let Some(output_path) = output {
                fs::write(&output_path, &output_str).context("Failed to write output file")?;
                if verbose {
                    eprintln!("Report written to: {}", output_path.display());
                }
            } else {
                print!("{}", output_str);
            }

            // Check if we should fail based on severity threshold
            let should_fail = fail_sev.is_some_and(|threshold| report.has_issues_at_or_above(threshold));
            Ok(!should_fail)
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
                OutputFormat::Json | OutputFormat::GithubActions | OutputFormat::Markdown => {
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
                    println!("{}", serde_json::to_string_pretty(&json)?);
                }
            }

            Ok(true)
        }

        Commands::Watch {
            debounce_ms,
            clear,
            crate_name,
            root,
        } => {
            let root_dir = find_root_dir(root)?;
            let crates = get_crate_list(crate_name);

            aspen_verus_metrics::watch::run_watch(&root_dir, &crates, debounce_ms, clear)?;
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
