#[path = "coordination/bounded.rs"]
mod limits;
mod bounded {
    pub(super) use super::limits::*;
}
#[path = "coordination/command.rs"]
mod args;
mod command {
    pub(crate) use super::args::*;
}
#[path = "coordination/io.rs"]
mod io;
#[path = "coordination/ops.rs"]
mod ops;

pub(crate) type CoordinationCommand = command::Command;

pub(crate) fn run_coordination_command(command: CoordinationCommand) -> molten::error::Result<()> {
    ops::run(command)
}
