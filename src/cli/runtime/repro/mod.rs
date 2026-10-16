#[path = "bundle.rs"]
mod bundle;
#[path = "command.rs"]
mod command;
#[path = "io.rs"]
mod io;
#[path = "ops.rs"]
mod ops;

pub(crate) type ReproCommand = command::Top;

pub(crate) fn run_repro_command(command: ReproCommand) -> molten::error::Result<()> {
    ops::run(command)
}
