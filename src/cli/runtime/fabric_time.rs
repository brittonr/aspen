#[path = "fabric_time/command.rs"]
pub(crate) mod command;
#[path = "fabric_time/ops.rs"]
mod ops;

pub(crate) fn run_fabric_time_command(command: command::FabricTimeCommand) -> molten::error::Result<()> {
    match command {
        command::FabricTimeCommand::RunFixture { profile, out } => ops::run_fixture(profile.into(), out),
        command::FabricTimeCommand::Show { report } => ops::show(report),
    }
}
