#[derive(Debug, clap::Args)]
pub(crate) struct Command {
    #[command(subcommand)]
    action: Action,
}

#[derive(Debug, clap::Subcommand)]
enum Action {
    /// Verify exact portable inputs. Does not authorize or start a node.
    Verify {
        /// Independently selected operator cohort; never learned from the bundle.
        #[arg(long)]
        policy: std::path::PathBuf,
        #[arg(long)]
        bundle: std::path::PathBuf,
    },
}

pub(crate) fn run(input: Command) -> molten::error::Result<()> {
    match input.action {
        Action::Verify { policy, bundle } => {
            let report = molten::node_startup_evidence::verify(&policy, &bundle)?;
            let json = serde_json::to_string(&report).map_err(|error| {
                molten::error::MoltenError::invalid_harness(format!("startup-evidence-report: {error}"))
            })?;
            println!("{json}");
            Ok(())
        }
    }
}
