#[path = "transcript/command.rs"]
pub(crate) mod command;
#[path = "transcript/io.rs"]
mod io;
#[path = "transcript/ops.rs"]
mod ops;

pub(crate) type TranscriptCommand = command::Top;

pub(crate) fn run_transcript_command(command: TranscriptCommand) -> molten::error::Result<()> {
    match command {
        command @ TranscriptCommand::Parse { .. } => ops::parse(command),
        command @ TranscriptCommand::Run { .. } => ops::run(command),
        command @ TranscriptCommand::Show { .. } => ops::show(command),
        command @ TranscriptCommand::Render { .. } => ops::render(command),
    }
}
