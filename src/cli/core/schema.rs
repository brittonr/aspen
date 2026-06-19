#[path = "schema/command.rs"]
pub(crate) mod command;
#[path = "schema/io.rs"]
mod io;
#[path = "schema/ops.rs"]
mod ops;

pub(crate) type SchemaCommand = command::Top;

pub(crate) fn run_schema_command(command: SchemaCommand) -> molten::error::Result<()> {
    match command {
        command @ SchemaCommand::Identity { .. } => ops::identity(command),
        command @ SchemaCommand::Alias { .. } => ops::alias(command),
        command @ SchemaCommand::Compat { .. } => ops::compat(command),
        command @ SchemaCommand::SearchFingerprint { .. } => ops::search_fingerprint(command),
    }
}
