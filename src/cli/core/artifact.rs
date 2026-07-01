#[path = "artifact/command.rs"]
pub(crate) mod command;
#[path = "artifact/io.rs"]
mod io;
#[path = "artifact/ops.rs"]
mod ops;

pub(crate) type Command = command::Top;

pub(crate) fn run(command: Command) -> molten::error::Result<()> {
    match command {
        command @ Command::Install { .. } => ops::install(command),
        command @ Command::List { .. } => ops::list(command),
        command @ Command::View { .. } => ops::view(command),
        command @ Command::NameSet { .. } => ops::name_set(command),
        command @ Command::NameShow { .. } => ops::name_show(command),
        command @ Command::Deps { .. } => ops::deps(command),
        command @ Command::Closure { .. } => ops::closure(command),
        command @ Command::Impact { .. } => ops::impact(command),
        command @ Command::IndexRebuild { .. } => ops::index_rebuild(command),
    }
}
