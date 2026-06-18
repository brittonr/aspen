#[path = "command/base.rs"]
pub(crate) mod base;
#[path = "command/live.rs"]
pub(crate) mod live;
#[path = "command/ops.rs"]
pub(crate) mod ops;

use clap::Subcommand;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
pub(crate) enum Top {
    Class(base::Class),
    Pin(base::Pin),
    Unpin(base::Unpin),
    Admit(base::Admit),
    #[command(name = "remote-clearance")]
    Clearance(base::Record),
    #[command(name = "remote-clearance-request")]
    ClearanceRequest(base::Request),
    #[command(name = "remote-clearance-respond")]
    ClearanceRespond(base::Respond),
    #[command(name = "remote-clearance-import")]
    ClearanceImport(base::Import),
    #[command(name = "remote-clearance-live-request-send")]
    LiveRequestSend(live::RequestSend),
    #[command(name = "remote-clearance-live-response-send")]
    LiveResponseSend(live::ResponseSend),
    #[command(name = "remote-clearance-live-import-workflow")]
    LiveImportWorkflow(live::ImportWorkflow),
    #[command(name = "remote-clearance-live-loopback")]
    LiveLoopback(live::Loopback),
    Explain(ops::Explain),
    BundleExport(ops::BundleExport),
    BundleVerify(ops::BundleVerify),
    GcPlan(ops::GcPlan),
    GcApplyPlan(ops::GcApplyPlan),
    GcAudit(ops::GcAudit),
    Check(ops::Check),
    RunFixture(ops::RunFixture),
    Show(ops::Show),
}
