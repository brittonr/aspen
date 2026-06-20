#[path = "ledger/command.rs"]
mod command;
#[path = "ledger/io.rs"]
mod io;
#[path = "ledger/ops.rs"]
mod ops;

pub(crate) type LedgerCommand = command::Command;
pub(crate) type ChainCommand = command::Chain;

pub(crate) fn run_ledger_command(command: LedgerCommand) -> molten::error::Result<()> {
    ops::run_ledger(command)
}

pub(crate) fn run_chain_command(command: ChainCommand) -> molten::error::Result<()> {
    ops::run_chain(command)
}
