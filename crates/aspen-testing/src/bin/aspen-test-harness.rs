use anyhow::Result;
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
    }
}
