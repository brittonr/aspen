#[path = "secrets/command.rs"]
pub(crate) mod command;
#[path = "secrets/io.rs"]
mod io;
#[path = "secrets/ops.rs"]
mod ops;

pub(crate) type SecretsCommand = command::Top;

pub(crate) fn run_secrets_command(command: SecretsCommand) -> molten::error::Result<()> {
    match command {
        SecretsCommand::RunFixture { out } => ops::run_fixture(out),
        SecretsCommand::Show { artifact } => ops::show(artifact),
    }
}
