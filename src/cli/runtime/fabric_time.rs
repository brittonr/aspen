#[path = "fabric_time/command.rs"]
mod command;
#[path = "fabric_time/ops.rs"]
mod ops;

pub(crate) use command::FabricTimeCommand;

pub(crate) fn run_fabric_time_command(command: FabricTimeCommand) -> molten::error::Result<()> {
    match command {
        FabricTimeCommand::RunFixture { profile, out } => ops::run_fixture(profile.into(), out),
        FabricTimeCommand::Show { report } => ops::show(report),
    }
}
