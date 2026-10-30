type FilePath = std::path::PathBuf;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    RunFixture {
        #[arg(long)]
        bundle_root: FilePath,
        #[arg(long)]
        staging_root: FilePath,
        #[arg(long)]
        output_root: FilePath,
    },
    Show {
        #[arg(long)]
        output_root: FilePath,
    },
    Replay {
        #[arg(long)]
        bundle_root: FilePath,
        #[arg(long)]
        staging_root: FilePath,
        #[arg(long)]
        output_root: FilePath,
    },
    #[cfg(feature = "component-rpc-telemetry")]
    TelemetryFixture {
        #[arg(long)]
        bundle_root: FilePath,
        #[arg(long)]
        staging_root: FilePath,
        #[arg(long)]
        rpc_output_root: FilePath,
        #[arg(long)]
        sink_root: FilePath,
        #[arg(long)]
        sink_ref: String,
    },
    #[cfg(feature = "component-rpc-telemetry")]
    TelemetryShow {
        #[arg(long)]
        sink_root: FilePath,
    },
}

pub(crate) fn run(command: Command) -> super::Outcome<()> {
    match command {
        Command::RunFixture { bundle_root, staging_root, output_root } => {
            let package = super::directory(&bundle_root)?;
            let staging = super::directory(&staging_root)?;
            let output = super::directory(&output_root)?;
            let value = super::run_async(molten::component_rpc::operator::run(&package, &staging, &output))?;
            super::emit(&value)
        }
        Command::Show { output_root } => {
            let output = super::directory(&output_root)?;
            let value = molten::component_rpc::operator::show(&output).map_err(super::failure)?;
            super::emit(&value)
        }
        Command::Replay { bundle_root, staging_root, output_root } => {
            let package = super::directory(&bundle_root)?;
            let staging = super::directory(&staging_root)?;
            let output = super::directory(&output_root)?;
            let bundle = molten::component_rpc::materialization::Bundle::load_package(&package, &staging)
                .map_err(super::failure)?;
            let result = molten::component_rpc::operator::replay(&output, &bundle).map_err(super::failure)?;
            println!("component-rpc replay verified: {result}");
            Ok(())
        }
        #[cfg(feature = "component-rpc-telemetry")]
        Command::TelemetryFixture { bundle_root, staging_root, rpc_output_root, sink_root, sink_ref } => {
            let package = super::directory(&bundle_root)?;
            let staging = super::directory(&staging_root)?;
            let rpc_output = super::directory(&rpc_output_root)?;
            let sink = super::directory(&sink_root)?;
            let value = super::run_async(molten::component_rpc::telemetry::operator::run_fixture(
                molten::component_rpc::telemetry::operator::Input {
                    package: &package, staging: &staging, rpc_output: &rpc_output, sink: &sink, sink_ref: &sink_ref,
                },
            ))?;
            super::emit(&value)
        }
        #[cfg(feature = "component-rpc-telemetry")]
        Command::TelemetryShow { sink_root } => {
            let sink = super::directory(&sink_root)?;
            let value = molten::component_rpc::telemetry::operator::show(&sink).map_err(super::failure)?;
            super::emit(&value)
        }
    }
}
