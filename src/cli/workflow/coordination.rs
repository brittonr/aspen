#[path = "coordination/bounded.rs"]
mod bounded;
#[path = "coordination/command.rs"]
mod command;
#[path = "coordination/io.rs"]
mod io;
#[path = "coordination/ops.rs"]
mod ops;

pub(crate) type CoordinationCommand = command::Command;

pub(crate) fn run_coordination_command(command: CoordinationCommand) -> molten::error::Result<()> {
    ops::run(command)
}
