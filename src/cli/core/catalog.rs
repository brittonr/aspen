#[path = "catalog/command.rs"]
mod command;
#[path = "catalog/filter.rs"]
mod filter;
#[path = "catalog/io.rs"]
mod io;
#[path = "catalog/ops.rs"]
mod ops;

pub(crate) type CatalogCommand = command::Command;

pub(crate) fn run_catalog_command(command: CatalogCommand) -> molten::error::Result<()> {
    ops::run(command)
}
