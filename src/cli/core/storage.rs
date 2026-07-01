#[path = "storage/command.rs"]
pub(crate) mod command;
#[path = "storage/io.rs"]
mod io;
#[path = "storage/ops.rs"]
mod ops;

pub(crate) type Command = command::Top;

pub(crate) fn run(command: Command) -> molten::error::Result<()> {
    match command {
        command @ Command::Put { .. } => ops::put(command),
        command @ Command::Get { .. } => ops::get(command),
        command @ Command::Recipe { .. } => ops::recipe(command),
        command @ Command::Migrate { .. } => ops::migrate(command),
        command @ Command::Verify { .. } => ops::verify(command),
    }
}
