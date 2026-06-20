#[path = "vat/command.rs"]
pub(crate) mod command;
#[path = "vat/io.rs"]
mod io;
#[path = "vat/ops.rs"]
mod ops;

pub(crate) type VatCommand = command::Top;

pub(crate) fn run_vat_command(command: VatCommand) -> molten::error::Result<()> {
    ops::run(command)
}
