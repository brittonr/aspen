#[path = "cache/command.rs"]
pub(crate) mod command;
#[path = "cache/io.rs"]
mod io;
#[path = "cache/ops.rs"]
mod ops;

pub(crate) type Command = command::Top;

pub(crate) fn run(command: Command) -> molten::error::Result<()> {
    match command {
        Command::Put(args) => ops::put(args),
        Command::Get(args) => ops::get(args),
        Command::Status(args) => ops::status(args),
        Command::List(args) => ops::list(args),
        Command::Show(args) => ops::show(args),
        Command::Invalidate(args) => ops::invalidate(args),
        Command::IndexRebuild(args) => ops::index_rebuild(args),
    }
}
