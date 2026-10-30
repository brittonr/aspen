type FilePath = std::path::PathBuf;
type Outcome<T> = molten::error::Result<T>;

#[derive(Debug, clap::Args)]
pub(crate) struct LiveInput {
    #[arg(long)]
    run_root: FilePath,
    /// Independently reviewed DAG conformance execution artifact.
    #[arg(long)]
    dag_conformance_ref: String,
    #[arg(long)]
    fault: Option<String>,
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    LiveLoopback(LiveInput),
    DistinctProcess {
        #[command(flatten)]
        input: LiveInput,
        #[arg(long)]
        process_binary: FilePath,
    },
    VerifyDetached {
        #[arg(long)]
        receiver_root: FilePath,
        #[arg(long)]
        trusted_context_ref: String,
    },
    #[command(hide = true)]
    FixtureRole {
        #[arg(long)]
        role: String,
        #[arg(long)]
        state_root: FilePath,
        #[arg(long)]
        exchange_root: FilePath,
    },
}

pub(crate) fn run(command: Command) -> Outcome<()> {
    let value = match command {
        Command::LiveLoopback(input) => {
            let run = molten::federation::liveproof::RunInput {
                run_directory: input.run_root, dag_conformance_ref: input.dag_conformance_ref,
            };
            match input.fault.as_deref() {
                Some(fault) => molten::federation::liveproof::loopback_denial(&run, fault)?,
                None => molten::federation::liveproof::loopback(&run)?,
            }
        }
        Command::DistinctProcess { input, process_binary } => {
            let run = molten::federation::liveproof::RunInput {
                run_directory: input.run_root, dag_conformance_ref: input.dag_conformance_ref,
            };
            match input.fault.as_deref() {
                Some(fault) => molten::federation::liveproof::distinct_process_denial(&run, &process_binary, fault)?,
                None => molten::federation::liveproof::distinct_process(&run, &process_binary)?,
            }
        }
        Command::VerifyDetached { receiver_root, trusted_context_ref } => {
            molten::federation::liveproof::verify_detached(&receiver_root, &trusted_context_ref)?
        }
        Command::FixtureRole { role, state_root, exchange_root } => {
            return molten::federation::liveproof::role(&role, &state_root, &exchange_root);
        }
    };
    println!("{}", molten::preserves_rail::to_text(&value)?);
    Ok(())
}
