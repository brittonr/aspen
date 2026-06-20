#[path = "gate/command.rs"]
mod command;
#[path = "gate/io.rs"]
mod io;
#[path = "gate/ops.rs"]
mod ops;

pub(crate) type GateCommand = command::Command;

pub(crate) fn run_gate_command(command: GateCommand) -> molten::error::Result<()> {
    ops::run(command)
}
