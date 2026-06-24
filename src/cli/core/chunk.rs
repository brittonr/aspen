#[path = "chunk/command.rs"]
mod command;
#[path = "chunk/io.rs"]
mod io;
#[path = "chunk/ops.rs"]
mod ops;

pub(crate) type Top = command::Top;

pub(crate) fn run(command: Top) -> molten::error::Result<()> {
    ops::run(command)
}
