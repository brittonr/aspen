#[path = "schema/command.rs"]
pub(crate) mod command;
#[path = "schema/io.rs"]
mod io;
#[path = "schema/ops.rs"]
mod ops;

pub(crate) type Command = command::Top;

pub(crate) fn run(command: Command) -> molten::error::Result<()> {
    match command {
        command @ Command::Identity { .. } => ops::identity(command),
        command @ Command::Alias { .. } => ops::alias(command),
        command @ Command::Compat { .. } => ops::compat(command),
        command @ Command::SearchFingerprint { .. } => ops::search_fingerprint(command),
    }
}
