#[path = "command.rs"]
pub(crate) mod command;
#[path = "ops.rs"]
mod ops;

pub(crate) type Command = command::Top;

pub(crate) fn run_command(command: Command) -> molten::error::Result<()> {
    match command {
        Command::RunFixture { profile, out } => ops::run_fixture(profile.into(), out),
        Command::Show { status } => ops::show(status),
    }
}
