#[path = "command/base.rs"]
pub(crate) mod base;
#[path = "command/refs.rs"]
pub(crate) mod refs;
#[path = "command/sync.rs"]
pub(crate) mod sync;
#[path = "command/worker.rs"]
pub(crate) mod worker;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, clap::Subcommand)]
pub(crate) enum Top {
    Install(base::Install),
    Show(base::Show),
    Run(base::Run),
    Plan(base::Plan),
    Profile(base::Profile),
    FusionPreview(base::FusionPreview),
    SyncPlan(sync::Plan),
    SyncLoopback(sync::Loopback),
    AdmitPlan(sync::AdmitPlan),
    AdmitLoopback(sync::AdmitLoopback),
    ExecuteLoopback(sync::ExecuteLoopback),
    WorkerRequest(worker::Request),
    WorkerRunLocal(worker::RunLocal),
    WorkerScheduleLocal(worker::ScheduleLocal),
    RefSubmit(refs::Submit),
    RefExecute(refs::Execute),
    Status(refs::Status),
    ReceiptShow(refs::ReceiptShow),
}
