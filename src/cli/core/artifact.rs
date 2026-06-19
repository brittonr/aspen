#[path = "artifact/command.rs"]
pub(crate) mod command;
#[path = "artifact/io.rs"]
mod io;
#[path = "artifact/ops.rs"]
mod ops;

pub(crate) type ArtifactCommand = command::Top;

pub(crate) fn run_artifact_command(command: ArtifactCommand) -> molten::error::Result<()> {
    match command {
        command @ ArtifactCommand::Install { .. } => ops::install(command),
        command @ ArtifactCommand::List { .. } => ops::list(command),
        command @ ArtifactCommand::View { .. } => ops::view(command),
        command @ ArtifactCommand::NameSet { .. } => ops::name_set(command),
        command @ ArtifactCommand::NameShow { .. } => ops::name_show(command),
        command @ ArtifactCommand::Deps { .. } => ops::deps(command),
        command @ ArtifactCommand::Closure { .. } => ops::closure(command),
        command @ ArtifactCommand::Impact { .. } => ops::impact(command),
        command @ ArtifactCommand::IndexRebuild { .. } => ops::index_rebuild(command),
    }
}
