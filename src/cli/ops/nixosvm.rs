#[path = "nixosvm/command.rs"]
mod args;
mod command {
    pub(crate) use super::args::*;
}
#[path = "nixosvm/io.rs"]
mod io;
#[path = "nixosvm/ops.rs"]
mod ops;

pub(crate) type NixosVmCommand = command::Command;

pub(crate) fn run_nixos_vm_command(command: NixosVmCommand) -> molten::error::Result<()> {
    ops::run(command)
}
