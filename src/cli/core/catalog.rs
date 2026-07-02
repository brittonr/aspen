#[path = "catalog/command.rs"]
mod args;
mod command {
    pub(crate) use super::args::*;
}
#[path = "catalog/filter.rs"]
mod filter;
#[path = "catalog/io.rs"]
mod io;
#[path = "catalog/ops.rs"]
mod ops;

pub(crate) type Command = command::Command;

pub(crate) fn run(command: Command) -> molten::error::Result<()> {
    ops::run(command)
}
