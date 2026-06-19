#[path = "service/command.rs"]
mod command;
#[path = "service/io.rs"]
mod io;
#[path = "service/ops.rs"]
mod ops;

pub(crate) type ServiceCommand = command::Command;

pub(crate) fn run_service_command(command: ServiceCommand) -> molten::error::Result<()> {
    ops::run(command)
}
