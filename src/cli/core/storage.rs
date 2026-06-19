#[path = "storage/command.rs"]
pub(crate) mod command;
#[path = "storage/io.rs"]
mod io;
#[path = "storage/ops.rs"]
mod ops;

pub(crate) type StorageCommand = command::Top;

pub(crate) fn run_storage_command(command: StorageCommand) -> molten::error::Result<()> {
    match command {
        command @ StorageCommand::Put { .. } => ops::put(command),
        command @ StorageCommand::Get { .. } => ops::get(command),
        command @ StorageCommand::Recipe { .. } => ops::recipe(command),
        command @ StorageCommand::Migrate { .. } => ops::migrate(command),
        command @ StorageCommand::Verify { .. } => ops::verify(command),
    }
}
