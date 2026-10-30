#[path = "bounded.rs"]
mod limits;
mod bounded {
    pub(super) use super::limits::*;
}
#[path = "command.rs"]
mod args;
mod command {
    pub(crate) use super::args::*;
}
#[path = "io.rs"]
mod io;
#[path = "ops.rs"]
mod ops;

pub(crate) type CoordinationCommand = command::Command;

pub(crate) fn run_coordination_command(command: CoordinationCommand) -> molten::error::Result<()> {
    ops::run(command)
}
