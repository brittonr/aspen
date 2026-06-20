#[path = "plugin/command.rs"]
pub(crate) mod command;
#[path = "plugin/io.rs"]
mod io;
#[path = "plugin/ops.rs"]
mod ops;

pub(crate) type PluginCommand = command::Top;

pub(crate) fn run_plugin_command(command: PluginCommand) -> molten::error::Result<()> {
    match command {
        PluginCommand::Install {
            manifest,
            registry,
            out,
        } => ops::install(manifest, registry, out),
        PluginCommand::RunFixture { state_root, out } => ops::run_fixture(state_root, out),
        PluginCommand::Show { artifact } => ops::show(artifact),
    }
}
