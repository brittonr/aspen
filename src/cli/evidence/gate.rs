#[path = "gate/command.rs"]
mod args;
mod command {
    pub(crate) use super::args::*;
}
#[path = "gate/io.rs"]
mod io;
#[path = "gate/ops.rs"]
mod ops;

pub(crate) type GateCommand = command::Command;

pub(crate) fn run_gate_command(command: GateCommand) -> molten::error::Result<()> {
    ops::run(command)
}
