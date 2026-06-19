#[path = "remote/command.rs"]
mod command;
#[path = "remote/io.rs"]
mod io;
#[path = "remote/ops.rs"]
mod ops;

pub(crate) type RemoteCommand = command::Command;
pub(crate) type RemoteEnvelopeCommand = command::EnvelopeCommand;

pub(crate) fn run_remote_command(command: RemoteCommand) -> molten::error::Result<()> {
    ops::run(command)
}

pub(crate) fn remote_dataspace_gate_summary(value: &preserves::IOValue) -> molten::error::Result<String> {
    ops::remote_dataspace_gate_summary(value)
}
