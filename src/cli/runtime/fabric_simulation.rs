#[path = "fabric_simulation/command.rs"]
pub(crate) mod command;
#[path = "fabric_simulation/ops.rs"]
mod ops;

pub(crate) fn run_fabric_simulation_command(command: command::FabricSimulationCommand) -> molten::error::Result<()> {
    match command {
        command::FabricSimulationCommand::Preflight => ops::preflight(),
        command::FabricSimulationCommand::Run { out } => ops::run(out),
        command::FabricSimulationCommand::Replay { report } => ops::replay(report),
        command::FabricSimulationCommand::Shrink { out } => ops::shrink(out),
        command::FabricSimulationCommand::Inspect { report } => ops::inspect(report),
        command::FabricSimulationCommand::Export { out } => ops::export(out),
    }
}
