#[path = "job/command.rs"]
pub(crate) mod command;
#[path = "job/core.rs"]
mod core;
#[path = "job/io.rs"]
pub(crate) mod transfer;
pub(crate) mod io {
    pub(crate) use super::transfer::*;
}
#[path = "job/refs.rs"]
mod refs;
#[path = "job/schedule.rs"]
mod timeline;
mod schedule {
    pub(crate) use super::timeline::*;
}
#[path = "job/worker.rs"]
mod agent;
#[path = "job/sync.rs"]
mod sync;
mod worker {
    pub(crate) use super::agent::*;
}

const COORDINATION_CLI_BATCH_REF_LIMIT: usize = 4096;
const COORDINATION_CLI_BATCH_EVIDENCE_LIMIT: usize = 16384;
const JOB_CLI_EVIDENCE_LIMIT: usize = 64;
const JOB_WORKER_CLI_REF_LIMIT: usize = 4096;
const _: () = assert!(COORDINATION_CLI_BATCH_REF_LIMIT <= 100_000);
const _: () = assert!(COORDINATION_CLI_BATCH_EVIDENCE_LIMIT <= 100_000);
const _: () = assert!(JOB_CLI_EVIDENCE_LIMIT <= 100_000);
const _: () = assert!(JOB_WORKER_CLI_REF_LIMIT <= 100_000);

pub(crate) type JobCommand = command::Top;

pub(crate) fn run_job_command(command: JobCommand) -> molten::error::Result<()> {
    match command {
        JobCommand::Install(args) => core::install(args),
        JobCommand::Show(args) => core::show(args),
        JobCommand::Run(args) => core::run(args),
        JobCommand::Plan(args) => core::plan(args),
        JobCommand::Profile(args) => core::profile(args),
        JobCommand::FusionPreview(args) => core::fusion_preview(args),
        JobCommand::SyncPlan(args) => sync::plan(args),
        JobCommand::SyncLoopback(args) => sync::loopback(args),
        JobCommand::AdmitPlan(args) => sync::admit_plan(args),
        JobCommand::AdmitLoopback(args) => sync::admit_loopback(args),
        JobCommand::ExecuteLoopback(args) => sync::execute_loopback(args),
        JobCommand::WorkerRequest(args) => worker::request(args),
        JobCommand::WorkerRunLocal(args) => worker::run_local(args),
        JobCommand::WorkerScheduleLocal(args) => worker::schedule_local(args),
        JobCommand::RefSubmit(args) => refs::submit(args),
        JobCommand::RefExecute(args) => refs::execute(args),
        JobCommand::Status(args) => refs::status(args),
        JobCommand::ReceiptShow(args) => refs::receipt_show(args),
    }
}
