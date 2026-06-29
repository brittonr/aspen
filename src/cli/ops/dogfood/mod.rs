#[path = "archive.rs"]
mod archive;
#[path = "command.rs"]
mod command;
#[path = "io.rs"]
mod io;
#[path = "ops/mod.rs"]
mod ops;
#[path = "signed.rs"]
mod signed;

pub(crate) type DogfoodCommand = command::Command;

pub(crate) fn run_dogfood_command(command: DogfoodCommand) -> molten::error::Result<()> {
    ops::run(command)
}
