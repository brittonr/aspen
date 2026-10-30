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

/// Pilot operations: run|provision ROOT ACTUAL_PROVIDER_REV; advance ROOT 1|2; check ROOT.
#[derive(clap::Parser)]
#[command(name = "world-witness-consumer-pilot")]
enum Operation {
    Run { root: std::path::PathBuf, revision: String },
    Provision { root: std::path::PathBuf, revision: String },
    Advance { root: std::path::PathBuf, generation: u64 },
    Check { root: std::path::PathBuf },
    IndexServe { root: std::path::PathBuf },
    IndexInitialize { root: std::path::PathBuf },
}

fn execute(operation: Operation) -> Result<bool> {
    match operation {
        Operation::Run { root, revision } => campaign::run(&root, &revision)?,
        Operation::Provision { root, revision } => provision::run(&root, &revision)?,
        Operation::Advance { root, generation } => session::advance(&root, generation)?,
        Operation::Check { root } => return session::check(&root),
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
