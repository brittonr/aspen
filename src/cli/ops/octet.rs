#[path = "octet/baseline.rs"]
mod baseline;
#[path = "octet/command.rs"]
mod command;
#[path = "octet/io.rs"]
mod io;
#[path = "octet/ops.rs"]
mod ops;

pub(crate) type OctetCommand = command::Command;

pub(crate) fn run_octet_command(command: OctetCommand) -> molten::error::Result<()> {
    ops::run(command)
}
