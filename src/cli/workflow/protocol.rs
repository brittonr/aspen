#[path = "protocol/command.rs"]
mod command;
#[path = "protocol/io.rs"]
mod io;
#[path = "protocol/ops.rs"]
mod ops;

pub(crate) type ProtocolCommand = command::Command;

pub(crate) fn run_protocol_command(command: ProtocolCommand) -> molten::error::Result<()> {
    ops::run(command)
}
