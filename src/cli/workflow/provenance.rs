#[path = "provenance/command.rs"]
mod args;
mod command {
    pub(crate) use super::args::*;
}
#[path = "provenance/input.rs"]
mod input;
#[path = "provenance/io.rs"]
mod io;
#[path = "provenance/ops.rs"]
mod ops;

const PROVENANCE_CLI_EVIDENCE_LIMIT: usize = 64;
const _: () = assert!(PROVENANCE_CLI_EVIDENCE_LIMIT <= 100_000);

pub(crate) type ProvenanceCommand = command::Command;

pub(crate) fn run_provenance_command(command: ProvenanceCommand) -> molten::error::Result<()> {
    ops::run(command)
}
