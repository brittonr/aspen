#[path = "retention/clearance.rs"]
mod clearance;
#[path = "retention/command.rs"]
pub(crate) mod command;
#[path = "retention/core.rs"]
mod core;
#[path = "retention/io.rs"]
mod io;
#[path = "retention/ops.rs"]
mod ops;
#[path = "retention/send.rs"]
mod send;
#[path = "retention/workflow.rs"]
mod workflow;

pub(crate) type RetentionCommand = command::Top;

pub(crate) fn run_retention_command(command: RetentionCommand) -> molten::error::Result<()> {
    match command {
        RetentionCommand::Class(args) => core::class(args),
        RetentionCommand::Pin(args) => core::pin(args),
        RetentionCommand::Unpin(args) => core::unpin(args),
        RetentionCommand::Admit(args) => core::admit(args),
        RetentionCommand::Clearance(args) => clearance::record(args),
        RetentionCommand::ClearanceRequest(args) => clearance::request(args),
        RetentionCommand::ClearanceRespond(args) => clearance::respond(args),
        RetentionCommand::ClearanceImport(args) => clearance::import(args),
        RetentionCommand::LiveRequestSend(args) => send::request(args),
        RetentionCommand::LiveResponseSend(args) => send::response(args),
        RetentionCommand::LiveImportWorkflow(args) => workflow::import(args),
        RetentionCommand::LiveLoopback(args) => workflow::loopback(args),
        RetentionCommand::Explain(args) => ops::explain(args),
        RetentionCommand::BundleExport(args) => ops::bundle_export(args),
        RetentionCommand::BundleVerify(args) => ops::bundle_verify(args),
        RetentionCommand::GcPlan(args) => ops::gc_plan(args),
        RetentionCommand::GcApplyPlan(args) => ops::gc_apply_plan(args),
        RetentionCommand::GcAudit(args) => ops::gc_audit(args),
        RetentionCommand::Check(args) => ops::check(args),
        RetentionCommand::RunFixture(args) => ops::run_fixture(args),
        RetentionCommand::Show(args) => ops::show(args),
    }
}
