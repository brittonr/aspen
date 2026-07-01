#[path = "transcript/command.rs"]
pub(crate) mod command;
#[path = "transcript/io.rs"]
mod io;
#[path = "transcript/ops.rs"]
mod ops;

pub(crate) type Command = command::Top;

pub(crate) fn run(command: Command) -> molten::error::Result<()> {
    match command {
        command @ Command::Parse { .. } => ops::parse(command),
        command @ Command::Run { .. } => ops::run(command),
        command @ Command::Show { .. } => ops::show(command),
        command @ Command::Render { .. } => ops::render(command),
    }
}
