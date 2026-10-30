type FilePath = std::path::PathBuf;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    RunFixture {
        #[arg(long)]
        root: FilePath,
        #[arg(long)]
        configuration: FilePath,
        #[arg(long)]
        output_root: FilePath,
    },
    Replay {
        #[arg(long)]
        root: FilePath,
        #[arg(long)]
        left: FilePath,
        #[arg(long)]
        right: FilePath,
        #[arg(long)]
        output_root: FilePath,
    },
    Show {
        #[arg(long)]
        root: FilePath,
        #[arg(long)]
        status: FilePath,
        #[arg(long, value_enum)]
        kind: StatusKind,
        #[arg(long)]
        receipt: String,
        #[arg(long)]
        profile: String,
        #[arg(long)]
        replay_profile: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub(crate) enum StatusKind { Execution, AdmissionDenial, Replay }

pub(crate) fn run(command: Command) -> super::Outcome<()> {
    match command {
        Command::RunFixture { root, configuration, output_root } => {
            let root = super::directory(&root)?;
            let output = super::directory(&output_root)?;
            let value = molten::spacewasm::operator::run_configuration(&root, &configuration, &output)
                .map_err(super::failure)?;
            super::emit(&value)
        }
        Command::Replay { root, left, right, output_root } => {
            let root = super::directory(&root)?;
            let output = super::directory(&output_root)?;
            let value = molten::spacewasm::operator::replay_configuration(&root, &left, &right, &output)
                .map_err(super::failure)?;
            super::emit(&value)
        }
        Command::Show { root, status, kind, receipt, profile, replay_profile } => {
            let root = super::directory(&root)?;
            let kind = match kind {
                StatusKind::Execution => molten::spacewasm::status::Kind::Execution,
                StatusKind::AdmissionDenial => molten::spacewasm::status::Kind::AdmissionDenial,
                StatusKind::Replay => molten::spacewasm::status::Kind::Replay,
            };
            let display = |profiles: &[&str]| {
                let value = molten::spacewasm::status::readback(&root, &status,
                    molten::spacewasm::status::Expected { kind, receipt: &receipt, profiles })
                    .map_err(super::failure)?;
                println!("{}", value.render());
                Ok(())
            };
            match replay_profile.as_deref() {
                Some(other) => display(&[profile.as_str(), other]),
                None => display(&[profile.as_str()]),
            }
        }
    }
}
