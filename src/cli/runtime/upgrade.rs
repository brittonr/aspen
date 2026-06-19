#[path = "upgrade/command.rs"]
mod command;
#[path = "upgrade/io.rs"]
mod io;
#[path = "upgrade/ops.rs"]
mod ops;

pub(crate) type UpgradeCommand = command::Command;

pub(crate) fn run_upgrade_command(command: UpgradeCommand) -> molten::error::Result<()> {
    ops::run(command)
}
