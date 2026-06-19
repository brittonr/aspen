#[path = "chunk/command.rs"]
mod command;
#[path = "chunk/io.rs"]
mod io;
#[path = "chunk/ops.rs"]
mod ops;

pub(crate) type ChunkCommand = command::Command;

pub(crate) fn run_chunk_command(command: ChunkCommand) -> molten::error::Result<()> {
    ops::run(command)
}
