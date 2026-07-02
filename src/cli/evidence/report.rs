#[path = "report/command.rs"]
mod args;
mod command {
    pub(crate) use super::args::*;
}
#[path = "report/ops.rs"]
mod actions;
#[path = "report/io.rs"]
mod io;
mod ops {
    pub(super) use super::actions::*;
}

pub(crate) type ReportCommand = command::Command;

pub(crate) fn run_report_command(command: ReportCommand) -> molten::error::Result<()> {
    ops::run(command)
}
