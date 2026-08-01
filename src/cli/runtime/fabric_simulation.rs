#[path = "fabric_simulation/command.rs"]
mod command;
#[path = "fabric_simulation/ops.rs"]
mod ops;

pub(crate) use command::FabricSimulationCommand;

pub(crate) fn run_fabric_simulation_command(command: FabricSimulationCommand) -> molten::error::Result<()> {
    match command {
        FabricSimulationCommand::Preflight => ops::preflight(),
        FabricSimulationCommand::Run { out } => ops::run(out),
        FabricSimulationCommand::Replay { report } => ops::replay(report),
        FabricSimulationCommand::Shrink { out } => ops::shrink(out),
        FabricSimulationCommand::Inspect { report } => ops::inspect(report),
        FabricSimulationCommand::Export { out } => ops::export(out),
    }
}
