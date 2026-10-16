#[path = "command.rs"]
pub(crate) mod command;
#[path = "ops.rs"]
mod ops;

pub(crate) fn run_command(command: command::ClockAction) -> molten::error::Result<()> {
    match command {
        command::ClockAction::RunFixture { profile, out } => ops::run_fixture(profile.into(), out),
        command::ClockAction::Show { report } => ops::show(report),
    }
}
