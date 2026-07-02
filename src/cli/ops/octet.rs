#[path = "octet/command.rs"]
mod args;
#[path = "octet/baseline.rs"]
mod baseline;
mod command {
    pub(crate) use super::args::*;
}
#[path = "octet/io.rs"]
mod io;
#[path = "octet/ops.rs"]
mod ops;

pub(crate) type OctetCommand = command::Command;

pub(crate) fn run_octet_command(command: OctetCommand) -> molten::error::Result<()> {
    ops::run(command)
}
