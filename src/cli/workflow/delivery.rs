#[path = "delivery/command.rs"]
mod command;
#[path = "delivery/io.rs"]
mod io;
#[path = "delivery/ops.rs"]
mod ops;

pub(crate) type DeliveryCommand = command::Command;

pub(crate) fn run_delivery_command(command: DeliveryCommand) -> molten::error::Result<()> {
    ops::run(command)
}
