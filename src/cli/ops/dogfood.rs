#[path = "dogfood/archive.rs"]
mod archive;
#[path = "dogfood/command.rs"]
mod command;
#[path = "dogfood/io.rs"]
mod io;
#[path = "dogfood/ops.rs"]
mod ops;
#[path = "dogfood/signed.rs"]
mod signed;

pub(crate) type DogfoodCommand = command::Command;

pub(crate) fn run_dogfood_command(command: DogfoodCommand) -> molten::error::Result<()> {
    ops::run(command)
}
