#[path = "rewrite/command.rs"]
mod args;
mod command {
    pub(crate) use super::args::*;
}
#[path = "rewrite/input.rs"]
mod payload;
mod input {
    pub(super) use super::payload::*;
}
#[path = "rewrite/io.rs"]
mod io;
#[path = "rewrite/ops.rs"]
mod ops;

pub(crate) type RewriteCommand = command::Command;

pub(crate) fn run_rewrite_command(command: RewriteCommand) -> molten::error::Result<()> {
    ops::run(command)
}
