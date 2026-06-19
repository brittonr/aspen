#[path = "rewrite/command.rs"]
mod command;
#[path = "rewrite/input.rs"]
mod input;
#[path = "rewrite/io.rs"]
mod io;
#[path = "rewrite/ops.rs"]
mod ops;

pub(crate) type RewriteCommand = command::Command;

pub(crate) fn run_rewrite_command(command: RewriteCommand) -> molten::error::Result<()> {
    ops::run(command)
}
