//! Explicit private same-host consumer pilot. Fixed fixture issuer keys are never
//! production authority. Operator supplies the actual reviewed provider revision.

#[path = "world-witness-consumer-pilot/authority.rs"]
mod authority;
#[path = "world-witness-consumer-pilot/campaign.rs"]
mod campaign;
#[path = "world-witness-consumer-pilot/domain.rs"]
mod domain;
#[path = "world-witness-consumer-pilot/index.rs"]
mod index;
#[path = "world-witness-consumer-pilot/payload.rs"]
mod payload;
#[path = "world-witness-consumer-pilot/provision.rs"]
mod provision;
#[path = "world-witness-consumer-pilot/session.rs"]
mod session;

type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

/// Pilot operations: run|provision ROOT ACTUAL_PROVIDER_REV [--provider-program PATH];
/// advance ROOT 1|2|3; check ROOT; abandon ROOT; resolve-orphan ROOT.
#[derive(clap::Parser)]
#[command(name = "world-witness-consumer-pilot")]
enum Operation {
    Run {
        root: std::path::PathBuf,
        revision: String,
        /// Separately built release-channel `provider` executable; default is the built-in child.
        #[arg(long)]
        provider_program: Option<std::path::PathBuf>,
    },
    Provision {
        root: std::path::PathBuf,
        revision: String,
        #[arg(long)]
        provider_program: Option<std::path::PathBuf>,
    },
    Advance {
        root: std::path::PathBuf,
        generation: u64,
    },
    Check {
        root: std::path::PathBuf,
    },
    Abandon {
        root: std::path::PathBuf,
    },
    ResolveOrphan {
        root: std::path::PathBuf,
    },
    IndexServe {
        root: std::path::PathBuf,
    },
    IndexInitialize {
        root: std::path::PathBuf,
    },
}

fn execute(operation: Operation) -> Result<bool> {
    match operation {
        Operation::Run {
            root,
            revision,
            provider_program,
        } => campaign::run(&root, &revision, provider_program.as_deref())?,
        Operation::Provision {
            root,
            revision,
            provider_program,
        } => provision::run(&root, &revision, provider_program.as_deref())?,
        Operation::Advance { root, generation } => session::advance(&root, generation)?,
        Operation::Check { root } => return session::check(&root),
        Operation::Abandon { root } => session::abandon(&root)?,
        Operation::ResolveOrphan { root } => session::resolve(&root)?,
        Operation::IndexServe { root } => index::serve(&root)?,
        Operation::IndexInitialize { root } => index::initialize(&root.join("index"))?,
    }
    Ok(true)
}

fn main() -> std::process::ExitCode {
    let outcome = <Operation as clap::Parser>::try_parse().map_err(Into::into).and_then(execute);
    match outcome {
        Ok(true) => std::process::ExitCode::SUCCESS,
        Ok(false) => std::process::ExitCode::from(2),
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}
