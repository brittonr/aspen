#[path = "prodsoak/command.rs"]
mod args;
mod command {
    pub(crate) use super::args::*;
}
#[path = "prodsoak/io.rs"]
mod io;
#[path = "prodsoak/ops.rs"]
mod ops;

pub(crate) type ProdSoakCommand = command::Command;

pub(crate) fn run_prod_soak_command(command: ProdSoakCommand) -> molten::error::Result<()> {
    ops::run(command)
}
