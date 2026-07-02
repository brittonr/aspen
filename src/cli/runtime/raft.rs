#[path = "raft/command.rs"]
mod args;
mod command {
    pub(crate) use super::args::*;
}
#[path = "raft/io.rs"]
mod io;
#[path = "raft/ops.rs"]
mod ops;

pub(crate) type RaftCommand = command::Command;

pub(crate) fn run_raft_command(command: RaftCommand) -> molten::error::Result<()> {
    ops::run(command)
}
