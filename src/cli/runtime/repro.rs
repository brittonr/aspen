#[path = "repro/bundle.rs"]
mod bundle;
#[path = "repro/command.rs"]
mod command;
#[path = "repro/io.rs"]
mod io;
#[path = "repro/ops.rs"]
mod ops;

pub(crate) type ReproCommand = command::Top;

pub(crate) fn run_repro_command(command: ReproCommand) -> molten::error::Result<()> {
    ops::run(command)
}
