#[path = "system_extension/command.rs"]
pub(crate) mod command;
#[path = "system_extension/ops.rs"]
mod ops;

pub(crate) type SystemExtensionCommand = command::Top;

pub(crate) fn run_system_extension_command(command: SystemExtensionCommand) -> molten::error::Result<()> {
    match command {
        SystemExtensionCommand::RunFixture { profile, out } => ops::run_fixture(profile.into(), out),
        SystemExtensionCommand::Show { status } => ops::show(status),
    }
}
