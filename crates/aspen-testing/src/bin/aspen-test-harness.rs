use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use aspen_testing::run_report;
use aspen_testing::suite_inventory::InventoryPaths;
use aspen_testing::suite_inventory::ensure_inventory_is_current;
use aspen_testing::suite_inventory::load_inventory;
use aspen_testing::suite_inventory::write_inventory;
use clap::Parser;
use clap::Subcommand;

#[derive(Debug, Parser)]
#[command(name = "aspen-test-harness")]
#[command(about = "Validate and export shared test harness suite inventory")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Regenerate the committed suite inventory from Nickel manifests.
    Export,
    /// Fail if the committed suite inventory is stale or invalid.
    Check,
    /// Generate a JSON run report from nextest JUnit XML.
    Report {
        /// Path to nextest JUnit XML file.
        #[arg(long, default_value = "target/nextest/default/junit.xml")]
        junit_xml: PathBuf,
        /// Output path for the JSON report (stdout if omitted).
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Print coverage-by-layer summary from suite inventory.
    Coverage,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let paths = InventoryPaths::default();
    let inventory = load_inventory(&paths)?;

    match cli.command {
        Command::Export => {
            write_inventory(&inventory, &paths.output_path)?;
            println!("wrote {}", paths.output_path.display());
            Ok(())
        }
        Command::Check => {
            ensure_inventory_is_current(&inventory, &paths.output_path)?;
            println!("suite inventory is current");
            Ok(())
        }
        Command::Report { junit_xml, output } => {
            let report = run_report::parse_junit_xml(&junit_xml).map_err(|e| anyhow::anyhow!("{e}"))?;
            let json = serde_json::to_string_pretty(&report).context("serializing report")?;
            match output {
                Some(path) => {
                    std::fs::write(&path, &json).with_context(|| format!("writing {}", path.display()))?;
                    println!("wrote report to {}", path.display());
                }
                None => println!("{json}"),
            }
            Ok(())
        }
        Command::Coverage => {
            let coverage = run_report::coverage_by_layer(&inventory.suites);
            let json = serde_json::to_string_pretty(&coverage).context("serializing coverage")?;
            println!("{json}");
            Ok(())
        }
    }
}
