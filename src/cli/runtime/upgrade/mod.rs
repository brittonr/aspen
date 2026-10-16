#[path = "command.rs"]
mod args;
mod command {
    pub(crate) use super::args::*;
}
#[path = "io.rs"]
mod io;
#[path = "ops.rs"]
mod ops;

pub(crate) type UpgradeCommand = command::Command;

pub(crate) fn run_upgrade_command(command: UpgradeCommand) -> molten::error::Result<()> {
    ops::run(command)
}
