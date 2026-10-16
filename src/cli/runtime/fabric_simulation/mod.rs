#[path = "command.rs"]
pub(crate) mod command;
#[path = "ops.rs"]
mod ops;

pub(crate) fn run_command(command: command::ScenarioAction) -> molten::error::Result<()> {
    match command {
        command::ScenarioAction::Preflight => ops::preflight(),
        command::ScenarioAction::Run { out } => ops::run(out),
        command::ScenarioAction::Replay { report } => ops::replay(report),
        command::ScenarioAction::Shrink { out } => ops::shrink(out),
        command::ScenarioAction::Inspect { report } => ops::inspect(report),
        command::ScenarioAction::Export { out } => ops::export(out),
    }
}
