#[path = "command.rs"]
mod args;
mod command {
    pub(crate) use super::args::*;
}
#[path = "input.rs"]
mod payload;
mod input {
    pub(super) use super::payload::*;
}
#[path = "io.rs"]
mod io;
#[path = "ops.rs"]
mod ops;

pub(crate) type RewriteCommand = command::Command;

pub(crate) fn run_rewrite_command(command: RewriteCommand) -> molten::error::Result<()> {
    ops::run(command)
}
