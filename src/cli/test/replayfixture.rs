#[path = "replayfixture/command.rs"]
pub(crate) mod command;
#[path = "replayfixture/io.rs"]
mod io;
#[path = "replayfixture/ops.rs"]
mod ops;

pub(crate) type ReplayFixtureCommand = command::Top;

pub(crate) fn run_replay_fixture_command(command: ReplayFixtureCommand) -> molten::error::Result<()> {
    ops::run(command)
}
