mod local;
mod release;

type Command = super::command::Command;
type Outcome<T> = molten::error::Result<T>;

pub(super) fn run(command: Command) -> Outcome<()> {
    match command {
        command @ Command::LocalNode { .. } => local::local_node(command),
        command @ Command::NixReleaseExport { .. } => local::nix_release_export(command),
        command @ Command::NixReleaseVerify { .. } => local::nix_release_verify(command),
        command @ Command::ReleaseBundleExport { .. } => release::bundle_export(command),
        command @ Command::ReleaseBundleVerify { .. } => release::bundle_verify(command),
        command @ Command::ReleasePromote { .. } => release::promote(command),
        command @ Command::ReleasePromotionSummary { .. } => release::promotion_summary(command),
        command @ Command::ReleaseExport { .. } => release::export(command),
        command @ Command::ReleaseExportVerify { .. } => release::export_verify(command),
        Command::Show { artifact } => show(artifact),
    }
}

fn show(artifact: std::path::PathBuf) -> Outcome<()> {
    let value = super::io::read_preserves_file(&artifact)?;
    println!("{}", molten::operator_dogfood::operator_dogfood_summary(&value)?);
    Ok(())
}

fn wrong_handler(name: &str) -> molten::error::MoltenError {
    molten::error::MoltenError::invalid_harness(format!("dogfood {name} handler called with another command"))
}
