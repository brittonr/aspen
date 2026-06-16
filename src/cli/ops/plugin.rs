use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Subcommand;
use molten::error::MoltenError;
use molten::error::Result;
use molten::plugin_host;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::to_text;

#[derive(Debug, Subcommand)]
pub(crate) enum PluginCommand {
    Install {
        manifest: PathBuf,
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    RunFixture {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    Show {
        artifact: PathBuf,
    },
}

pub(crate) fn run_plugin_command(command: PluginCommand) -> Result<()> {
    match command {
        PluginCommand::Install {
            manifest,
            registry,
            out,
        } => {
            let manifest_value = read_preserves_file(&manifest)?;
            let receipt = plugin_host::install_plugin(&registry, &manifest_value)?;
            write_file(&out, &to_text(&receipt.value)?)?;
            println!(
                "plugin install decision={} receipt={} manifest={} out={}",
                receipt.decision,
                receipt.receipt_ref,
                receipt.manifest_ref,
                out.display()
            );
            Ok(())
        }
        PluginCommand::RunFixture { state_root, out } => {
            let run = plugin_host::minimal_plugin_fixture(&state_root)?;
            fs::create_dir_all(&out).map_err(MoltenError::from)?;
            write_file(&out.join("report.preserves"), &to_text(&run.report_value)?)?;
            write_indexed_values(&out, "evidence", &run.evidence_values)?;
            println!(
                "plugin fixture decision={} manifest={} install={} health={} removal={} out={}",
                run.decision,
                run.manifest_ref,
                run.install_receipt_ref,
                run.health_receipt_ref,
                run.removal_receipt_ref,
                out.display()
            );
            Ok(())
        }
        PluginCommand::Show { artifact } => {
            let value = read_preserves_file(&artifact)?;
            println!("{}", plugin_host::plugin_summary(&value)?);
            Ok(())
        }
    }
}

fn write_indexed_values(out: &Path, prefix: &str, values: &[preserves::IOValue]) -> Result<()> {
    for (index, value) in values.iter().enumerate() {
        write_file(&out.join(format!("{prefix}-{index}.preserves")), &to_text(value)?)?;
    }
    Ok(())
}

fn read_preserves_file(path: &Path) -> Result<preserves::IOValue> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    parse_text(&text)
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, contents).map_err(MoltenError::from)
}
