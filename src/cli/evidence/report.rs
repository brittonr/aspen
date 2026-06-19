#[path = "report/command.rs"]
mod command;
#[path = "report/io.rs"]
mod io;
#[path = "report/ops.rs"]
mod ops;

pub(crate) type ReportCommand = command::Command;

pub(crate) fn run_report_command(command: ReportCommand) -> molten::error::Result<()> {
    ops::run(command)
}
