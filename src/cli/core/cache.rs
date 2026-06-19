#[path = "cache/command.rs"]
pub(crate) mod command;
#[path = "cache/io.rs"]
mod io;
#[path = "cache/ops.rs"]
mod ops;

pub(crate) type CacheCommand = command::Top;

pub(crate) fn run_cache_command(command: CacheCommand) -> molten::error::Result<()> {
    match command {
        CacheCommand::Put(args) => ops::put(args),
        CacheCommand::Get(args) => ops::get(args),
        CacheCommand::Status(args) => ops::status(args),
        CacheCommand::List(args) => ops::list(args),
        CacheCommand::Show(args) => ops::show(args),
        CacheCommand::Invalidate(args) => ops::invalidate(args),
        CacheCommand::IndexRebuild(args) => ops::index_rebuild(args),
    }
}
