use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Subcommand;
use molten::error::MoltenError;
use molten::error::Result;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::to_text;
use molten::secrets;

#[derive(Debug, Subcommand)]
pub(crate) enum SecretsCommand {
    RunFixture {
        #[arg(long)]
        out: PathBuf,
    },
    Show {
        artifact: PathBuf,
    },
}

pub(crate) fn run_secrets_command(command: SecretsCommand) -> Result<()> {
    match command {
        SecretsCommand::RunFixture { out } => {
            let run = secrets::run_secrets_fixture()?;
            fs::create_dir_all(&out).map_err(MoltenError::from)?;
            write_file(&out.join("report.preserves"), &to_text(&run.value)?)?;
            write_file(&out.join("secret.preserves"), &to_text(&run.secret.value)?)?;
            write_file(&out.join("encrypted-ref.preserves"), &to_text(&run.encrypted.value)?)?;
            write_file(&out.join("redaction-marker.preserves"), &to_text(&run.marker.value)?)?;
            write_file(&out.join("redaction-transform.preserves"), &to_text(&run.transform.value)?)?;
            write_file(&out.join("reveal-denied.preserves"), &to_text(&run.reveal_denied.value)?)?;
            write_file(&out.join("reveal-pass.preserves"), &to_text(&run.reveal_pass.value)?)?;
            write_file(&out.join("decrypt-denied.preserves"), &to_text(&run.decrypt_denied.value)?)?;
            write_file(&out.join("decrypt-pass.preserves"), &to_text(&run.decrypt_pass.value)?)?;
            write_file(&out.join("commitment-replay.preserves"), &to_text(&run.replay.value)?)?;
            write_file(&out.join("cleanup.preserves"), &to_text(&run.cleanup.value)?)?;
            write_file(&out.join("private-bundle-profile.preserves"), &to_text(&run.private_bundle.value)?)?;
            write_indexed_values(&out, "evidence", &run.evidence_values)?;
            write_file(&out.join("summary.txt"), &secrets::fixture_report_summary(&run.value)?)?;
            println!("secrets fixture ok report={} out={}", run.report_ref, out.display());
            Ok(())
        }
        SecretsCommand::Show { artifact } => {
            let value = read_preserves_file(&artifact)?;
            match secrets::fixture_report_summary(&value) {
                Ok(summary) => println!("{summary}"),
                Err(_) => println!("{}", secrets::secrets_summary(&value)?),
            }
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
